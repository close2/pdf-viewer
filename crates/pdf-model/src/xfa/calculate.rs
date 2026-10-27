//! The form's calculations and the values they read and write, as the host of a script
//! interpreter sees them — RFC 0011 section 5's step 6 (ADR 2482).
//!
//! This module reads no script language. It lists the `calculate` scripts of the template in
//! document order with their language (XFA 3.3 page 412), and answers the questions an
//! interpreter asks of the form it runs in: which containers a name reaches from the script's
//! container (the search page 114 orders, with the index page 115 infers), what value a
//! container holds, and what a script assigned. The interpreter lives in `pdf-formcalc`, which
//! this crate does not depend on; the two meet in a host the program that runs scripts writes
//! (ADR 2482).
//!
//! Values are typed as chapter 4 types them, by the element the container's `value` holds: the
//! numeric ones are numbers, the rest text, an empty value null.

use super::Resource;
use super::datasets::Change;
use super::template::{Class, Container, Language, Template};

/// A value a script reads from a container or assigns to one.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    /// No value.
    Null,
    /// The value of a numeric container (`integer`, `decimal`, `float`).
    Number(f64),
    /// Any other value.
    Text(String),
}

/// Which occurrence of a name a step of a SOM expression selects (XFA 3.3 pages 94 and 115).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occurrence {
    /// No index stated.
    Same,
    /// `[*]`: every occurrence.
    Every,
    /// `[n]`, counted from 0.
    At(i64),
    /// `[+n]` or `[-n]`, from the script container's own index.
    Relative(i64),
}

/// One step of a SOM expression: a name and the occurrence of it wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The container's name, or `$` for the script's own container.
    pub name: String,
    /// Which occurrence.
    pub occurrence: Occurrence,
}

/// One `calculate` script of the template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calculation {
    /// The container the script belongs to, as [`Form::resolve`] and [`Form::value`] name it.
    pub node: usize,
    /// The container's PDF field name, where the field list the form was made from names it.
    pub field: Option<String>,
    /// The script's language (page 412).
    pub language: Language,
    /// The script's text.
    pub script: String,
    /// Whether the result replaces the container's value: a field's and an exclusion group's does,
    /// a subform's is only the script's effect (page 379).
    pub sets_value: bool,
}

/// The template's containers with the values the form holds in them, and what scripts assigned.
#[derive(Debug, Clone)]
pub struct Form<'a> {
    template: &'a Template,
    values: Vec<Scalar>,
    names: Vec<Option<String>>,
    assigned: Vec<bool>,
}

impl Resource {
    /// The form's containers holding the values in `fields`, every field of the form by its PDF
    /// name with the value its `/V` carries; a container the list does not name holds null.
    #[must_use]
    pub fn form(&self, fields: &[(String, Change)]) -> Form<'_> {
        Form::new(&self.template, fields)
    }
}

impl<'a> Form<'a> {
    fn new(template: &'a Template, fields: &[(String, Change)]) -> Self {
        let size = template.container_count();
        let mut form = Self {
            template,
            values: vec![Scalar::Null; size],
            names: vec![None; size],
            assigned: vec![false; size],
        };
        for (name, change) in fields {
            let Some(at) = template.resolve(name) else {
                continue;
            };
            let Some(container) = template.get(at) else {
                continue;
            };
            if let (Some(value), Some(named)) = (form.values.get_mut(at), form.names.get_mut(at)) {
                *value = scalar(container, change);
                *named = Some(name.clone());
            }
        }
        form
    }

    /// The `calculate` scripts of the form, in document order.
    #[must_use]
    pub fn calculations(&self) -> Vec<Calculation> {
        self.template
            .calculating()
            .into_iter()
            .filter_map(|node| {
                let container = self.template.get(node)?;
                let script = container.calculation.as_ref()?;
                Some(Calculation {
                    node,
                    field: self.names.get(node).cloned().flatten(),
                    language: script.language.clone(),
                    script: script.text.clone(),
                    sets_value: matches!(container.class, Class::Field | Class::ExclGroup),
                })
            })
            .collect()
    }

    /// The containers `path` names from the script container `here`.
    ///
    /// A path starting `$` is qualified and relative to `here` (page 114); any other starts the
    /// search of page 114 at the first name, and an omitted index takes the index of `here`
    /// among its same-named siblings where the name matches several (page 115). An empty list is
    /// a path that names nothing.
    ///
    /// # Errors
    ///
    /// A sentence naming the part of SOM this form does not follow: a qualified reference other
    /// than `$`, or the `parent` property.
    pub fn resolve(&self, here: usize, path: &[Step]) -> Result<Vec<usize>, String> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(Vec::new());
        };
        let qualified = first.name == "$";
        if !qualified && (first.name.starts_with(['$', '!']) || first.name == "xfa") {
            return Err(format!(
                "the qualified reference {} (page 114) other than $",
                first.name
            ));
        }
        if let Some(step) = rest.iter().find(|step| step.name == "parent") {
            return Err(format!("the property {} (page 114)", step.name));
        }
        let mut current = if qualified {
            vec![here]
        } else {
            self.select(
                &self.template.unqualified(here, &first.name),
                first.occurrence,
                here,
                true,
            )
        };
        for step in rest {
            let mut next = Vec::new();
            for &node in &current {
                let named = self.template.named_within(Some(node), &step.name);
                next.extend(self.select(&named, step.occurrence, here, !qualified));
            }
            current = next;
        }
        Ok(current)
    }

    /// The matches `occurrence` selects; `inferred` lets an omitted index among several matches
    /// take `here`'s own (page 115), else the first match stands.
    fn select(
        &self,
        matches: &[usize],
        occurrence: Occurrence,
        here: usize,
        inferred: bool,
    ) -> Vec<usize> {
        let at = |index: i64| {
            usize::try_from(index)
                .ok()
                .and_then(|index| matches.get(index).copied())
                .into_iter()
                .collect()
        };
        match occurrence {
            Occurrence::Every => matches.to_vec(),
            Occurrence::At(index) => at(index),
            Occurrence::Relative(by) => {
                let own = i64::try_from(self.template.own_index(here)).unwrap_or(i64::MAX);
                at(own.saturating_add(by))
            }
            Occurrence::Same if matches.len() <= 1 => matches.to_vec(),
            Occurrence::Same if inferred => {
                at(i64::try_from(self.template.own_index(here)).unwrap_or(i64::MAX))
            }
            Occurrence::Same => at(0),
        }
    }

    /// The value the container `node` holds; null for one that holds none.
    #[must_use]
    pub fn value(&self, node: usize) -> Scalar {
        self.values.get(node).cloned().unwrap_or(Scalar::Null)
    }

    /// Sets the value of the container `node`.
    ///
    /// # Errors
    ///
    /// A sentence naming why the container takes no value: it is not a field or an exclusion
    /// group.
    pub fn set(&mut self, node: usize, value: Scalar) -> Result<(), String> {
        let class = self
            .template
            .get(node)
            .map(|container| container.class)
            .ok_or_else(|| "the form has no such container".to_owned())?;
        if !matches!(class, Class::Field | Class::ExclGroup) {
            return Err(format!("a {} holds no value (page 379)", class.name()));
        }
        if let (Some(slot), Some(flag)) = (self.values.get_mut(node), self.assigned.get_mut(node)) {
            *slot = value;
            *flag = true;
        }
        Ok(())
    }

    /// What scripts assigned: the PDF name of each container that has one, with its value, in
    /// document order.
    #[must_use]
    pub fn edits(&self) -> Vec<(String, Scalar)> {
        self.assigned
            .iter()
            .enumerate()
            .filter(|(_, assigned)| **assigned)
            .filter_map(|(node, _)| {
                Some((self.names.get(node).cloned().flatten()?, self.value(node)))
            })
            .collect()
    }
}

/// Whether the container's `value` holds a numeric element.
fn numeric(container: &Container) -> bool {
    matches!(
        container.value_type.as_deref(),
        Some("integer" | "decimal" | "float")
    )
}

/// The value a `/V` carries, typed by the container it belongs to.
fn scalar(container: &Container, change: &Change) -> Scalar {
    let text = match change {
        Change::Text(text) | Change::On(text) => text.as_str(),
        Change::Choices(values) => values.first().map_or("", String::as_str),
        Change::Off | Change::Cleared => "",
    };
    if text.is_empty() {
        return Scalar::Null;
    }
    if numeric(container)
        && let Ok(number) = text.trim().parse::<f64>()
        && number.is_finite()
    {
        return Scalar::Number(number);
    }
    Scalar::Text(text.to_owned())
}
