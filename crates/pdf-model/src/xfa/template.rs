//! XFA 3.3's template, read as far as binding a field to its data needs.
//!
//! The template is §K.2's "form template, which describes the characteristics of the form,
//! including its fields, calculations, validations, and formatting". This module reads its
//! containers — `subform`, `subformSet`, `field`, `exclGroup`, `area`, `pageSet`, `pageArea` —
//! with each one's name, its `scope`, its `bind`, its `ui`, its `items` and its `occur`, and
//! nothing of its layout or its scripts: those are RFC 0011 section 5's later steps, and this one
//! is the second (ADR 2396).
//!
//! Two questions are answered from it. **Which container an `AcroForm` field is**: XFA 3.3 chapter
//! 2, *Field Names* (pages 72–73), makes an XFAF form's PDF field name a SOM expression pointing
//! at the field element, and chapter 3, *Reference by Class* and *Transparent Nodes* (pages
//! 94–96), is the part of that grammar the expression uses. **Which data node the container binds
//! to**: chapter 4, *Basic Data Binding to Produce the XFA Form DOM* (pages 171–215) — a direct
//! match by name and by index among same-named siblings, through the transparent nodes page 193
//! lists. XFA 3.3 is cited and paraphrased, never quoted (RFC 0011's preamble).

use super::xml::Tree;

/// The container classes this module reads, by their element names in XFA 3.3's template grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    /// `subform`.
    Subform,
    /// `subformSet`, which page 333 makes the mark of a dynamic form.
    SubformSet,
    /// `field`.
    Field,
    /// `exclGroup`, a set of buttons of which one at most is on.
    ExclGroup,
    /// `area`, which page 193 makes transparent to binding even when it is named.
    Area,
    /// `pageSet`.
    PageSet,
    /// `pageArea`, a master page.
    PageArea,
}

impl Class {
    /// The class an element names, if it is one of the containers.
    fn of(local: &str) -> Option<Self> {
        Some(match local {
            "subform" => Self::Subform,
            "subformSet" => Self::SubformSet,
            "field" => Self::Field,
            "exclGroup" => Self::ExclGroup,
            "area" => Self::Area,
            "pageSet" => Self::PageSet,
            "pageArea" => Self::PageArea,
            _ => return None,
        })
    }

    /// The name `#class` spells this class with (chapter 3, *Reference by Class*).
    fn name(self) -> &'static str {
        match self {
            Self::Subform => "subform",
            Self::SubformSet => "subformSet",
            Self::Field => "field",
            Self::ExclGroup => "exclGroup",
            Self::Area => "area",
            Self::PageSet => "pageSet",
            Self::PageArea => "pageArea",
        }
    }
}

/// A container's `bind match`, the four values chapter 4 page 176 lists; `once` is the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Bind {
    /// `once`: the normal matching rules.
    Once,
    /// `none`: bound to no data node.
    None,
    /// `global`: the field's data may be outside the record.
    Global,
    /// `dataRef`, with the SOM expression its `ref` names.
    DataRef(String),
}

/// One container of the template.
#[derive(Debug, Clone)]
pub(crate) struct Container {
    /// Which container it is.
    pub(crate) class: Class,
    /// Its `name`; `None` where it states none, which page 193 makes it transparent for.
    pub(crate) name: Option<String>,
    /// Whether it states `scope="none"`, the named subform page 194 treats as nameless.
    pub(crate) scope_none: bool,
    /// Its `bind`.
    pub(crate) bind: Bind,
    /// The element its `ui` holds — `textEdit`, `checkButton`, `choiceList`, … — with the
    /// `choiceList`'s `open` beside it.
    pub(crate) ui: Option<(String, Option<String>)>,
    /// Its `items` elements, each with whether it states `save="1"`, and their values in order.
    pub(crate) items: Vec<(bool, Vec<String>)>,
    /// Whether its `value` holds `exData`, rich text whose data is markup.
    pub(crate) rich: bool,
    /// Whether its `occur` bounds differ or are unbounded, page 333's dynamic subform.
    pub(crate) occurs_variably: bool,
    /// The container it is in; `None` for a top-level subform.
    pub(crate) parent: Option<usize>,
    /// Its own containers, in document order.
    pub(crate) children: Vec<usize>,
}

impl Container {
    /// Whether binding passes through this container as though it were not there and its
    /// children were its parent's (chapter 4 pages 193–194): a nameless subform, field or
    /// exclusion group that makes no explicit data reference, a subform stating `scope="none"`,
    /// and an area whatever its name.
    ///
    /// **A subform stating `bind match="none"` is taken as transparent too, and that is a reading
    /// rather than a sentence.** Page 176 says such a node binds to no data node and says nothing
    /// of its children; the only examples (chapter 9, pages 358–359) wrap bound subforms in one,
    /// and the children go on binding by name in the wrapper's parent. The one hybrid in the
    /// corpora that states it, `bug1815476.pdf`, carries a `datasets` packet whose values sit
    /// directly under the root's data group, which is that reading's evidence and not its source
    /// (ADR 2396).
    fn transparent_to_binding(&self) -> bool {
        match self.class {
            Class::Area | Class::SubformSet => true,
            Class::Subform => {
                self.scope_none
                    || self.bind == Bind::None
                    || (self.name.is_none() && !matches!(self.bind, Bind::DataRef(_)))
            }
            Class::Field | Class::ExclGroup => {
                self.name.is_none() && !matches!(self.bind, Bind::DataRef(_))
            }
            Class::PageSet | Class::PageArea => false,
        }
    }
}

/// A template's containers.
#[derive(Debug, Clone)]
pub struct Template {
    containers: Vec<Container>,
    roots: Vec<usize>,
}

/// Where a container's data is, by the binding chapter 4 states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Binding {
    /// A direct match: the data node at this path of `(name, index)` steps below the record, the
    /// data group the root subform binds whatever its name (chapter 4 page 182).
    Direct(Vec<(String, usize)>),
    /// `bind match="global"`: the direct match where there is one, else a data value of this
    /// name outside the record (chapter 6, pages 246–247).
    Global(Vec<(String, usize)>, String),
    /// An explicit data reference, as a path below the record or below `xfa:data`.
    Explicit {
        /// The steps.
        steps: Vec<(String, usize)>,
        /// Whether they start at `xfa:data` (`$data`) rather than at the record (`$record`).
        from_data: bool,
    },
    /// `bind match="none"`: the template binds this container to nothing.
    Nothing,
}

/// Why a container's data could not be found by the part of chapter 4 this module reads.
pub(crate) type Unbindable = &'static str;

impl Template {
    /// Reads the template under `root`, the `template` element of a packet.
    pub(crate) fn read(tree: &Tree, root: usize) -> Self {
        let mut template = Self {
            containers: Vec::new(),
            roots: Vec::new(),
        };
        let mut pending: Vec<(usize, Option<usize>)> = Vec::new();
        if let Some(element) = tree.get(root) {
            pending.extend(element.children.iter().rev().map(|&child| (child, None)));
        }
        // An explicit stack in document order, so a hostile template's depth costs a vector and
        // not the call stack; `xml::MAX_DEPTH` has already bounded it.
        while let Some((index, parent)) = pending.pop() {
            let Some(element) = tree.get(index) else {
                continue;
            };
            let Some(class) = Class::of(&element.local) else {
                continue;
            };
            let at = template.containers.len();
            let mut container = Container {
                class,
                name: element
                    .attribute("name")
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned),
                scope_none: element.attribute("scope") == Some("none"),
                bind: Bind::Once,
                ui: None,
                items: Vec::new(),
                rich: false,
                occurs_variably: false,
                parent,
                children: Vec::new(),
            };
            for &child in &element.children {
                let Some(property) = tree.get(child) else {
                    continue;
                };
                match property.local.as_str() {
                    "bind" => {
                        container.bind = match property.attribute("match") {
                            Some("none") => Bind::None,
                            Some("global") => Bind::Global,
                            Some("dataRef") => {
                                Bind::DataRef(property.attribute("ref").unwrap_or("").to_owned())
                            }
                            _ => Bind::Once,
                        };
                    }
                    "ui" => {
                        container.ui = property
                            .children
                            .iter()
                            .filter_map(|&kind| tree.get(kind))
                            .find(|kind| kind.local != "extras" && kind.local != "picture")
                            .map(|kind| {
                                (
                                    kind.local.clone(),
                                    kind.attribute("open").map(str::to_owned),
                                )
                            });
                    }
                    "items" => {
                        let values = property
                            .children
                            .iter()
                            .filter_map(|&item| tree.get(item))
                            .map(|item| item.text.clone())
                            .collect();
                        container
                            .items
                            .push((property.attribute("save") == Some("1"), values));
                    }
                    "value" => {
                        container.rich |= tree.child(child, "exData").is_some();
                    }
                    "occur" => {
                        let bound = |name: &str, default: i64| {
                            property
                                .attribute(name)
                                .and_then(|value| value.trim().parse::<i64>().ok())
                                .unwrap_or(default)
                        };
                        let (min, max) = (bound("min", 1), bound("max", 1));
                        container.occurs_variably = max < 0 || min != max;
                    }
                    _ => {}
                }
            }
            match parent.and_then(|parent| template.containers.get_mut(parent)) {
                Some(parent) => parent.children.push(at),
                None => template.roots.push(at),
            }
            template.containers.push(container);
            pending.extend(
                element
                    .children
                    .iter()
                    .rev()
                    .map(|&child| (child, Some(at))),
            );
        }
        template
    }

    /// The container at an index this template handed out.
    pub(crate) fn get(&self, index: usize) -> Option<&Container> {
        self.containers.get(index)
    }

    /// Why this template is not an XFAF form's, if it is not: page 333's test, a `subformSet`
    /// or an `occur` whose bounds differ or are unbounded (ADR 2380). Table 29's
    /// `/NeedsRendering` is the catalog's, and the caller reads it.
    #[must_use]
    pub fn dynamic(&self) -> Option<&'static str> {
        if self.containers.iter().any(|c| c.class == Class::SubformSet) {
            return Some("its template holds a subformSet");
        }
        if self.containers.iter().any(|c| c.occurs_variably) {
            return Some("its template lets a container occur a varying number of times");
        }
        None
    }

    /// The container an XFAF form's PDF field name points at (chapter 2, *Field Names*).
    ///
    /// The name is a SOM expression: steps separated by `.`, each a name or a `#class`, each
    /// with an optional `[index]`, counted from 0. Page 72 writes the expression with a leading
    /// `form`; the hybrids of the corpora write it without, starting at the root subform, so a
    /// leading `form` that does not itself resolve is skipped.
    pub(crate) fn resolve(&self, expression: &str) -> Option<usize> {
        let steps: Vec<&str> = expression.split('.').collect();
        self.resolve_steps(&steps)
            .or_else(|| match steps.split_first() {
                Some((&"form", rest)) => self.resolve_steps(rest),
                _ => None,
            })
    }

    fn resolve_steps(&self, steps: &[&str]) -> Option<usize> {
        let mut at: Option<usize> = None;
        for step in steps {
            let (name, index) = match step.split_once('[') {
                Some((name, rest)) => (name, rest.strip_suffix(']')?.trim().parse().ok()?),
                None => (*step, 0_usize),
            };
            let siblings = match at {
                Some(parent) => self.get(parent)?.children.clone(),
                None => self.roots.clone(),
            };
            // Page 96's note: the index of a `#class` counts every true sibling of the class,
            // transparent or not.
            at = Some(if let Some(class) = name.strip_prefix('#') {
                siblings
                    .into_iter()
                    .filter(|&c| self.get(c).is_some_and(|c| c.class.name() == class))
                    .nth(index)?
            } else {
                let mut named = Vec::new();
                self.som_named(&siblings, name, &mut named);
                named.into_iter().nth(index)?
            });
        }
        at
    }

    /// The containers among `siblings` named `name`, nameless ones and areas looked through,
    /// because page 94 makes them invisible to the normal syntax and page 95 areas always.
    fn som_named(&self, siblings: &[usize], name: &str, out: &mut Vec<usize>) {
        for &sibling in siblings {
            let Some(container) = self.get(sibling) else {
                continue;
            };
            match container.name.as_deref() {
                Some(own) if container.class != Class::Area => {
                    if own == name {
                        out.push(sibling);
                    }
                }
                _ => self.som_named(&container.children, name, out),
            }
        }
    }

    /// Where the container at `target` binds, by chapter 4's direct match.
    ///
    /// # Errors
    ///
    /// [`Unbindable`], a sentence naming the construction this module does not follow.
    pub(crate) fn binding(&self, target: usize) -> Result<Binding, Unbindable> {
        let container = self
            .get(target)
            .ok_or("the template has no such container")?;
        if !matches!(container.class, Class::Field | Class::ExclGroup) {
            return Err("it names a template node that is not a field or an exclusion group");
        }
        let mut chain = Vec::new();
        let mut parent = container.parent;
        while let Some(at) = parent {
            chain.push(at);
            parent = self.get(at).and_then(|c| c.parent);
        }
        chain.reverse();
        // The first of the chain is the root subform, which binds the record whatever its name.
        let mut steps = Vec::new();
        let mut scope = chain.first().copied();
        for &ancestor in chain.iter().skip(1) {
            let Some(node) = self.get(ancestor) else {
                continue;
            };
            match node.class {
                Class::PageSet | Class::PageArea => {
                    return Err("it is on a master page, which takes no part in data binding");
                }
                Class::ExclGroup if !node.transparent_to_binding() => {
                    return Err(
                        "it is a button of a named exclusion group, whose value is the group's",
                    );
                }
                _ if node.transparent_to_binding() => {}
                _ => {
                    if node.bind != Bind::Once {
                        return Err("it is inside a subform bound by an explicit data reference");
                    }
                    let name = node
                        .name
                        .clone()
                        .ok_or("a nameless subform binds nothing")?;
                    steps.push((name.clone(), self.index_in_scope(scope, ancestor, &name)));
                    scope = Some(ancestor);
                }
            }
        }
        match &container.bind {
            Bind::None => Ok(Binding::Nothing),
            Bind::DataRef(reference) => explicit(reference, &steps),
            Bind::Once | Bind::Global => {
                let name = container
                    .name
                    .clone()
                    .ok_or("it is nameless and states no data reference, so it binds nothing")?;
                steps.push((name.clone(), self.index_in_scope(scope, target, &name)));
                Ok(if container.bind == Bind::Global {
                    Binding::Global(steps, name)
                } else {
                    Binding::Direct(steps)
                })
            }
        }
    }

    /// How many containers named `name` bind normally in `scope` before `target`, binding's
    /// view of the scope: its transparent containers looked through (page 193) and its master
    /// pages left out. Page 195 pairs same-named siblings by this index.
    fn index_in_scope(&self, scope: Option<usize>, target: usize, name: &str) -> usize {
        let children = match scope.and_then(|scope| self.get(scope)) {
            Some(scope) => scope.children.clone(),
            None => self.roots.clone(),
        };
        let mut seen = 0;
        let mut pending: Vec<usize> = children.into_iter().rev().collect();
        while let Some(at) = pending.pop() {
            if at == target {
                return seen;
            }
            let Some(container) = self.get(at) else {
                continue;
            };
            if matches!(container.class, Class::PageSet | Class::PageArea) {
                continue;
            }
            if container.transparent_to_binding() {
                pending.extend(container.children.iter().rev());
            } else if container.name.as_deref() == Some(name)
                && matches!(container.bind, Bind::Once | Bind::Global)
            {
                seen = seen.saturating_add(1);
            }
        }
        seen
    }
}

/// A `dataRef`'s SOM expression as a path: `$record.` or `$data.` followed by names with
/// optional indices, or `$.` relative to the data group the container's scope binds (chapter 4,
/// *Explicit Data References*, pages 199–200). Anything else — `..`, `*`, a `#class` — is not
/// followed, because page 200 makes node creation fail or multiply under it.
fn explicit(reference: &str, scope: &[(String, usize)]) -> Result<Binding, Unbindable> {
    let refused: Unbindable = "its data reference uses SOM grammar this module does not follow";
    let (from_data, mut steps, rest) = if let Some(rest) = reference.strip_prefix("$record.") {
        (false, Vec::new(), rest)
    } else if let Some(rest) = reference.strip_prefix("$data.") {
        (true, Vec::new(), rest)
    } else if let Some(rest) = reference.strip_prefix("$.") {
        (false, scope.to_vec(), rest)
    } else {
        return Err(refused);
    };
    for step in rest.split('.') {
        let (name, index) = match step.split_once('[') {
            Some((name, index)) => (
                name,
                index
                    .strip_suffix(']')
                    .and_then(|index| index.trim().parse().ok())
                    .ok_or(refused)?,
            ),
            None => (step, 0),
        };
        if name.is_empty() || name.starts_with('#') || name.contains('*') {
            return Err(refused);
        }
        steps.push((name.to_owned(), index));
    }
    Ok(Binding::Explicit { steps, from_data })
}

#[cfg(test)]
mod tests {
    use super::{Binding, Template};
    use crate::xfa::xml::parse;

    fn template(text: &str) -> Template {
        let tree = parse(text.as_bytes()).expect("well formed");
        Template::read(&tree, 0)
    }

    /// Chapter 2's Example 2.21: same-named fields in one subform are told apart by index, and
    /// the name with a leading `form` points where the name without one does.
    #[test]
    fn example_2_21_names_each_row_by_its_index() {
        let t = template(
            "<template><subform name=\"PurchaseOrder\"><subform name=\"Items\">\
             <field name=\"PartNo\"/><field name=\"Price\"/>\
             <field name=\"PartNo\"/><field name=\"Price\"/></subform></subform></template>",
        );
        let second = t
            .resolve("form.PurchaseOrder.Items.PartNo[1]")
            .expect("resolves");
        assert_eq!(
            t.resolve("PurchaseOrder[0].Items[0].PartNo[1]"),
            Some(second)
        );
        assert_eq!(
            t.binding(second),
            Ok(Binding::Direct(vec![
                ("Items".to_owned(), 0),
                ("PartNo".to_owned(), 1)
            ]))
        );
    }

    /// Page 193's Example 4.58: a nameless subform is transparent, so the field it wraps binds
    /// where it would without it — and `#subform[0]` still reaches it by class.
    #[test]
    fn a_nameless_subform_is_transparent_to_binding() {
        let t = template(
            "<template><subform name=\"registration\"><field name=\"first\"/>\
             <subform><field name=\"city\"/></subform></subform></template>",
        );
        let city = t
            .resolve("registration[0].#subform[0].city[0]")
            .expect("resolves");
        assert_eq!(t.resolve("registration.city"), Some(city));
        assert_eq!(
            t.binding(city),
            Ok(Binding::Direct(vec![("city".to_owned(), 0)]))
        );
    }

    /// `match="none"` binds the field to nothing; a field on a master page and a button of a
    /// named exclusion group are said, not guessed at.
    #[test]
    fn what_binds_nothing_is_told_apart_from_what_is_not_followed() {
        let t = template(
            "<template><subform name=\"r\"><pageSet><pageArea name=\"p\">\
             <field name=\"onMaster\"/></pageArea></pageSet>\
             <field name=\"button\"><bind match=\"none\"/></field>\
             <exclGroup name=\"g\"><field name=\"on\"/></exclGroup></subform></template>",
        );
        let button = t.resolve("r.button").expect("button");
        assert_eq!(t.binding(button), Ok(Binding::Nothing));
        let master = t.resolve("r.#pageSet[0].p.onMaster").expect("master");
        assert!(t.binding(master).is_err());
        let member = t.resolve("r.g.on").expect("member");
        assert!(t.binding(member).is_err());
        let group = t.resolve("r.g").expect("group");
        assert_eq!(
            t.binding(group),
            Ok(Binding::Direct(vec![("g".to_owned(), 0)]))
        );
    }

    /// Page 333's two marks of a dynamic template.
    #[test]
    fn a_varying_occurrence_or_a_subform_set_is_dynamic() {
        let fixed = template(
            "<template><subform name=\"r\"><occur min=\"1\" max=\"1\"/></subform></template>",
        );
        assert_eq!(fixed.dynamic(), None);
        let varying =
            template("<template><subform name=\"r\"><occur max=\"-1\"/></subform></template>");
        assert!(varying.dynamic().is_some());
        let set = template("<template><subform name=\"r\"><subformSet/></subform></template>");
        assert!(set.dynamic().is_some());
    }
}
