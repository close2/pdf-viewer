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
//!
//! The order scripts run in, and the cascade that re-runs a calculation when a value it reads
//! changes, are [`activate`]'s (XFA 3.3 pages 379–380 and 404–407; ADR 2506).

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::Resource;
use super::datasets::Change;
use super::template::{Class, Container, Language, Override, Template};

pub use activate::{
    Activation, Ended, FormLanguage, Invalid, Level, Limits, Overridden, Ran, Runner, Stimulus,
};
pub use event::{Event, Replaced};

mod activate;
mod event;

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

impl Step {
    /// The most steps a SOM expression in a string may have, as a `FormCalc` path may (ADR 2524).
    pub const MAX_STEPS: usize = 64;
    /// The most bytes one name of such an expression may have.
    pub const MAX_NAME_BYTES: usize = 4096;

    /// A SOM expression in a string, as the steps [`Form::resolve`] takes: names joined by `.`,
    /// each with an optional `[n]`, `[*]`, `[+n]` or `[-n]`, a leading `$` for the script's own
    /// container (XFA 3.3 pages 94–96 and 114–115). A JavaScript script hands one to the host as a
    /// string, and an event's `ref` states one (page 706).
    ///
    /// # Errors
    ///
    /// The sentence naming the part of SOM this reader does not take in a string: the descendant
    /// separator `..` (page 104), the class separator `.#` (page 96), a predicate (page 105) and
    /// an escaped character, each refused by name, and an expression past the bounds.
    pub fn path(text: &str) -> Result<Vec<Self>, String> {
        if text.contains("..") {
            return Err("the descendant separator `..` (page 104)".to_owned());
        }
        if text.contains(".#") || text.starts_with('#') {
            return Err("the class separator `.#` (page 96)".to_owned());
        }
        if text.contains(".[") || text.contains(".(") {
            return Err("a predicate (page 105)".to_owned());
        }
        if text.contains('\\') {
            return Err("an escaped character (page 105)".to_owned());
        }
        let mut steps = Vec::new();
        for part in text.split('.') {
            if steps.len() >= Self::MAX_STEPS {
                return Err(format!(
                    "an expression of more than {} names",
                    Self::MAX_STEPS
                ));
            }
            let (name, occurrence) = match part.split_once('[') {
                None => (part, Occurrence::Same),
                Some((name, index)) => {
                    let index = index
                        .strip_suffix(']')
                        .ok_or_else(|| format!("the index of {name} is not closed"))?;
                    (name, Occurrence::of(index.trim())?)
                }
            };
            let name = name.trim();
            if name.is_empty() {
                return Err("an empty name".to_owned());
            }
            if name.len() > Self::MAX_NAME_BYTES {
                return Err(format!(
                    "a name of more than {} bytes",
                    Self::MAX_NAME_BYTES
                ));
            }
            steps.push(Self {
                name: name.to_owned(),
                occurrence,
            });
        }
        Ok(steps)
    }
}

impl Occurrence {
    /// An index of a SOM step, the text between its brackets (page 94).
    fn of(index: &str) -> Result<Self, String> {
        if index == "*" {
            return Ok(Self::Every);
        }
        let parsed = |digits: &str| {
            digits
                .parse::<i64>()
                .map_err(|_| format!("the index [{index}] is not a number"))
        };
        if let Some(by) = index.strip_prefix('+') {
            return Ok(Self::Relative(parsed(by)?));
        }
        if let Some(by) = index.strip_prefix('-') {
            return Ok(Self::Relative(parsed(by)?.saturating_neg()));
        }
        Ok(Self::At(parsed(index)?))
    }
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
    /// What the `calculate` element's `override` lets a person do to the value (page 641).
    pub override_mode: Override,
}

/// The template's containers with the values the form holds in them, and what scripts assigned.
#[derive(Debug, Clone)]
pub struct Form<'a> {
    template: &'a Template,
    values: Vec<Scalar>,
    names: Vec<Option<String>>,
    assigned: Vec<bool>,
    /// The containers a path has resolved to since [`Self::take_resolved`], which is what a
    /// script reads and so what its calculation depends on (page 380).
    resolved: RefCell<BTreeSet<usize>>,
    /// The containers whose value an assignment changed since [`Self::take_changed`].
    changed: BTreeSet<usize>,
    /// How many more bytes of text assignments may store: every assignment is charged its text's
    /// length whether or not it overwrites one, so the form never holds more than it was given
    /// (principle 3). Unlimited until [`Self::limit_text`].
    text_room: u64,
    /// Whether an assignment was refused for want of [`Self::text_room`].
    text_exhausted: bool,
    /// The primary event the scripts now running are under, which `$event` reads (ADR 2646).
    event: Option<Event>,
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
            resolved: RefCell::new(BTreeSet::new()),
            changed: BTreeSet::new(),
            text_room: u64::MAX,
            text_exhausted: false,
            event: None,
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
                    override_mode: container.calc_override,
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
        let found = self.resolve_path(here, path)?;
        // A property of `$event` is no container, so nothing that reads it depends on it.
        let containers = self.template.container_count();
        self.resolved
            .borrow_mut()
            .extend(found.iter().copied().filter(|&node| node < containers));
        Ok(found)
    }

    /// The containers `resolve` records and returns.
    fn resolve_path(&self, here: usize, path: &[Step]) -> Result<Vec<usize>, String> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(Vec::new());
        };
        let qualified = first.name == "$";
        if first.name == "$event" {
            return self.resolve_event(here, rest);
        }
        if first.name == "xfa"
            && let Some((event, rest)) = rest.split_first()
            && event.name == "event"
        {
            return self.resolve_event(here, rest);
        }
        if !qualified && (first.name.starts_with(['$', '!']) || first.name == "xfa") {
            return Err(format!(
                "the qualified reference {} (page 114) other than $",
                first.name
            ));
        }
        if let Some(step) = rest.iter().find(|step| step.name == "parent") {
            return Err(format!("the property {} (page 114)", step.name));
        }
        let current = if qualified {
            vec![here]
        } else {
            self.select(
                &self.template.unqualified(here, &first.name),
                first.occurrence,
                here,
                true,
            )
        };
        Ok(self.descend(current, rest, here, !qualified))
    }

    /// The containers each step of `steps` names within those before it.
    fn descend(
        &self,
        mut current: Vec<usize>,
        steps: &[Step],
        here: usize,
        inferred: bool,
    ) -> Vec<usize> {
        for step in steps {
            let mut next = Vec::new();
            for &node in &current {
                let named = self.template.named_within(Some(node), &step.name);
                next.extend(self.select(&named, step.occurrence, here, inferred));
            }
            current = next;
        }
        current
    }

    /// What a path after `$event` or `xfa.event` names (XFA 3.3 pages 398–404): `target` is the
    /// container the event happened on, the rest of the path continuing from it, and any other
    /// property the node it is, which is a value and has nothing within it.
    fn resolve_event(&self, here: usize, rest: &[Step]) -> Result<Vec<usize>, String> {
        let Some((property, rest)) = rest.split_first() else {
            return Err(
                "$event: an object, of which a script reads a property (XFA 3.3 page 398)"
                    .to_owned(),
            );
        };
        let containers = self.template.container_count();
        match event::property(self.event.as_ref(), containers, &property.name)? {
            event::Named::Target(target) => Ok(self.descend(vec![target], rest, here, false)),
            event::Named::Property(_) if !rest.is_empty() => Err(format!(
                "$event.{}: a value, which holds nothing named {}",
                property.name,
                rest.iter()
                    .map(|step| step.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".")
            )),
            event::Named::Property(node) => Ok(vec![node]),
        }
    }

    /// Makes `event` the primary event the scripts from here on run under, or ends it.
    pub fn set_event(&mut self, event: Option<Event>) {
        self.event = event;
    }

    /// The primary event the scripts run under, as they have left it.
    #[must_use]
    pub fn event(&self) -> Option<&Event> {
        self.event.as_ref()
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
        let containers = self.template.container_count();
        if event::is_property(containers, node) {
            return event::value(self.event.as_ref(), containers, node).unwrap_or(Scalar::Null);
        }
        self.values.get(node).cloned().unwrap_or(Scalar::Null)
    }

    /// Sets the value of the container `node`.
    ///
    /// # Errors
    ///
    /// A sentence naming why the container takes no value: it is not a field or an exclusion
    /// group.
    pub fn set(&mut self, node: usize, value: Scalar) -> Result<(), String> {
        let containers = self.template.container_count();
        if event::is_property(containers, node) {
            if let Scalar::Text(text) = &value
                && u64::try_from(text.len()).unwrap_or(u64::MAX) > self.text_room
            {
                self.text_exhausted = true;
                return Err("the activation ran out of room for text".to_owned());
            }
            let cost = event::assign(self.event.as_mut(), containers, node, value)?;
            self.text_room = self.text_room.saturating_sub(cost);
            return Ok(());
        }
        let class = self
            .template
            .get(node)
            .map(|container| container.class)
            .ok_or_else(|| "the form has no such container".to_owned())?;
        if !matches!(class, Class::Field | Class::ExclGroup) {
            return Err(format!("a {} holds no value (page 379)", class.name()));
        }
        if let Scalar::Text(text) = &value {
            let cost = u64::try_from(text.len()).unwrap_or(u64::MAX);
            if cost > self.text_room {
                self.text_exhausted = true;
                return Err("the activation ran out of room for text".to_owned());
            }
            self.text_room = self.text_room.saturating_sub(cost);
        }
        if let (Some(slot), Some(flag)) = (self.values.get_mut(node), self.assigned.get_mut(node)) {
            if *slot != value {
                self.changed.insert(node);
            }
            *slot = value;
            *flag = true;
        }
        Ok(())
    }

    /// Limits the text assignments store from here on to `bytes` in all; an assignment past it is
    /// refused with an error the script sees, and [`Self::take_text_exhausted`] says so.
    pub fn limit_text(&mut self, bytes: u64) {
        self.text_room = bytes;
    }

    /// How many bytes of text assignments may still be stored.
    #[must_use]
    pub fn text_room(&self) -> u64 {
        self.text_room
    }

    /// Whether an assignment was refused for want of room since the last call.
    pub fn take_text_exhausted(&mut self) -> bool {
        std::mem::take(&mut self.text_exhausted)
    }

    /// The containers [`Self::resolve`] has returned since the last call, which are the ones a
    /// script named and so the ones its result depends on.
    pub fn take_resolved(&self) -> BTreeSet<usize> {
        std::mem::take(&mut *self.resolved.borrow_mut())
    }

    /// The containers whose value a [`Self::set`] changed since the last call.
    pub fn take_changed(&mut self) -> BTreeSet<usize> {
        std::mem::take(&mut self.changed)
    }

    /// The container the PDF field `name` is, where the field list the form was made from named it.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<usize> {
        self.names
            .iter()
            .position(|named| named.as_deref() == Some(name))
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
