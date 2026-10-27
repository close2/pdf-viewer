//! XFA 3.3's template, read as far as binding a field to its data and writing that data needs.
//!
//! The template is §K.2's "form template, which describes the characteristics of the form,
//! including its fields, calculations, validations, and formatting". This module reads its
//! containers — `subform`, `subformSet`, `field`, `exclGroup`, `area`, `pageSet`, `pageArea` —
//! with each one's name, its `scope`, its `bind`, its `ui`, its `items`, its `occur`, its
//! `value`'s type, its `locale`, the pictures of its `ui`, `bind` and `validate` and its
//! `validate`'s tests and messages, and nothing of its layout or its scripts: those are RFC 0011
//! section 5's later steps (ADRs 2396, 2414, 2450).
//!
//! Two questions are answered from it. **Which container an `AcroForm` field is**: XFA 3.3 chapter
//! 2, *Field Names* (pages 72–73), makes an XFAF form's PDF field name a SOM expression pointing
//! at the field element, and chapter 3, *Reference by Class* and *Transparent Nodes* (pages
//! 94–96), is the part of that grammar the expression uses. **Which data node the container binds
//! to**: chapter 4, *Basic Data Binding to Produce the XFA Form DOM* (pages 171–215) — a direct
//! match by name and by index among same-named siblings, through the transparent nodes page 193
//! lists. XFA 3.3 is cited and paraphrased, never quoted (RFC 0011's preamble).

use super::xml::{Element, Tree};

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
    pub(crate) fn name(self) -> &'static str {
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

/// A container's picture clauses: the `picture` of its `ui` (the edit pattern), its `bind` (the
/// data pattern) and its `validate`, as XFA 3.3 page 161's table names them.
#[derive(Debug, Clone, Default)]
pub(crate) struct Pictures {
    pub(crate) edit: Option<String>,
    pub(crate) bind: Option<String>,
    pub(crate) validate: Option<String>,
    /// Its `validate`'s `formatTest` (page 880).
    pub(crate) format_test: FormatTest,
    /// Its `validate`'s `nullTest` (pages 880–881).
    pub(crate) null_test: NullTest,
    /// Its `validate`'s `message` texts, each by the test its `name` gives it — `nullTest`,
    /// `formatTest` or `scriptTest` (page 383) — and the form designer's words for that test.
    pub(crate) messages: Vec<(String, String)>,
}

impl Pictures {
    /// The form designer's message for `test` (page 383), where the `validate` element gives one.
    pub(crate) fn message(&self, test: &str) -> Option<&str> {
        self.messages
            .iter()
            .find(|(name, _)| name == test)
            .map(|(_, text)| text.as_str())
    }
}

/// A `validate` element's `nullTest` (XFA 3.3 pages 880–881, the levels of page 384).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum NullTest {
    /// No null test: page 881's default, an empty field being acceptable.
    #[default]
    Disabled,
    /// An empty value is warned about, and the person may dismiss or override it (page 384).
    Warning,
    /// An empty value makes the container invalid (page 384).
    Error,
}

/// A `validate` element's `formatTest` (XFA 3.3 page 880, the levels of pages 384–385).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FormatTest {
    /// A value the picture cannot format is warned about, and the person may override it: page
    /// 880's default.
    #[default]
    Warning,
    /// No format test.
    Disabled,
    /// A value the picture cannot format is refused.
    Error,
}

/// The digits a `decimal` element allows on either side of its radix point (XFA 3.3 page 671).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Digits {
    /// `fracDigits`: the most digits after the point; page 671's default is 2 and `-1`, read here
    /// as `None`, allows any number.
    pub(crate) fraction: Option<usize>,
    /// `leadDigits`: the most digits before the point; page 671's default, `-1`, read here as
    /// `None`, retains however many there are.
    pub(crate) lead: Option<usize>,
}

impl Default for Digits {
    fn default() -> Self {
        Self {
            fraction: Some(2),
            lead: None,
        }
    }
}

impl Digits {
    /// The limits `element` states, the defaults for one it does not or states badly: a negative
    /// bound is page 671's `-1`, no limit, and a value that is no integer is not a statement.
    fn of(element: &Element) -> Self {
        let bound = |name: &str, default: Option<usize>| match element
            .attribute(name)
            .and_then(|text| text.trim().parse::<i64>().ok())
        {
            Some(limit) => usize::try_from(limit).ok(),
            None => default,
        };
        let default = Self::default();
        Self {
            fraction: bound("fracDigits", default.fraction),
            lead: bound("leadDigits", default.lead),
        }
    }
}

/// The language of a script, by its `contentType` (XFA 3.3 page 412: absent means `FormCalc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    /// No `contentType`, or `application/x-formcalc`: chapter 25's language.
    FormCalc,
    /// `application/x-javascript`.
    JavaScript,
    /// Any other `contentType`, as written.
    Other(String),
}

impl Language {
    /// The language a `script` element's `contentType` attribute names.
    fn of(content_type: Option<&str>) -> Self {
        match content_type.map(str::trim) {
            None | Some("" | "application/x-formcalc") => Self::FormCalc,
            Some("application/x-javascript") => Self::JavaScript,
            Some(other) => Self::Other(other.to_owned()),
        }
    }
}

/// The `script` of a container's `calculate` element.
#[derive(Debug, Clone)]
pub(crate) struct Script {
    /// Its language.
    pub(crate) language: Language,
    /// Its text, as the template states it.
    pub(crate) text: String,
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
    /// The element its `value` holds — `text`, `integer`, `decimal`, `float`, `date`, `time`,
    /// `dateTime`, … — the content type chapter 4's Rules 2A and 4 (pages 164–165) read.
    pub(crate) value_type: Option<String>,
    /// The digits its `value`'s `decimal` allows (page 671).
    pub(crate) digits: Digits,
    /// Its `locale`, page 1153's second source of the prevailing locale.
    pub(crate) locale: Option<String>,
    /// Its picture clauses.
    pub(crate) pictures: Pictures,
    /// Whether its `occur` bounds differ or are unbounded, page 333's dynamic subform.
    pub(crate) occurs_variably: bool,
    /// Its `calculate`'s script, where it holds a non-blank one (page 379).
    pub(crate) calculation: Option<Script>,
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
                value_type: None,
                digits: Digits::default(),
                locale: element
                    .attribute("locale")
                    .filter(|locale| !locale.trim().is_empty())
                    .map(str::to_owned),
                pictures: Pictures::default(),
                occurs_variably: false,
                calculation: None,
                parent,
                children: Vec::new(),
            };
            read_properties(&mut container, tree, element);
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

    /// The `locale` the container at `index` states or inherits from the nearest container
    /// above it that states one (page 1153's second source; page 153's EXAMPLE 4.33).
    pub(crate) fn locale(&self, index: usize) -> Option<&str> {
        let mut at = Some(index);
        while let Some(container) = at.and_then(|i| self.containers.get(i)) {
            if let Some(locale) = &container.locale {
                return Some(locale);
            }
            at = container.parent;
        }
        None
    }

    /// How many containers the template holds.
    pub(crate) fn container_count(&self) -> usize {
        self.containers.len()
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

    /// The containers named `name` among `parent`'s own, nameless ones and areas looked through
    /// (pages 94 and 95); a `None` parent is the template's top level.
    pub(crate) fn named_within(&self, parent: Option<usize>, name: &str) -> Vec<usize> {
        let mut found = Vec::new();
        match parent {
            Some(at) => {
                if let Some(container) = self.get(at) {
                    self.som_named(&container.children, name, &mut found);
                }
            }
            None => self.som_named(&self.roots, name, &mut found),
        }
        found
    }

    /// The containers an unqualified SOM name reaches from the script container `here`, by page
    /// 114's order: the container's children; the container and its siblings; its parent and the
    /// parent's siblings; and so up to the root, the first level that holds a match deciding.
    pub(crate) fn unqualified(&self, here: usize, name: &str) -> Vec<usize> {
        let below = self.named_within(Some(here), name);
        if !below.is_empty() {
            return below;
        }
        let mut level = Some(here);
        while let Some(at) = level {
            let parent = self.get(at).and_then(|container| container.parent);
            let found = self.named_within(parent, name);
            if !found.is_empty() {
                return found;
            }
            level = parent;
        }
        Vec::new()
    }

    /// The index of the container at `at` among the containers of its own name beside it, which
    /// page 115 infers for a reference that states none; 0 for a nameless one.
    pub(crate) fn own_index(&self, at: usize) -> usize {
        let Some(container) = self.get(at) else {
            return 0;
        };
        let Some(name) = container.name.as_deref() else {
            return 0;
        };
        self.named_within(container.parent, name)
            .iter()
            .position(|&sibling| sibling == at)
            .unwrap_or(0)
    }

    /// The containers with a `calculate` script, in document order (the order the containers
    /// are read in).
    pub(crate) fn calculating(&self) -> Vec<usize> {
        (0..self.containers.len())
            .filter(|&at| self.get(at).is_some_and(|c| c.calculation.is_some()))
            .collect()
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

/// The properties of one container element — its `bind`, `ui`, `items`, `value`, `validate` and
/// `occur` — read into `container`.
fn read_properties(container: &mut Container, tree: &Tree, element: &Element) {
    for &child in &element.children {
        let Some(property) = tree.get(child) else {
            continue;
        };
        match property.local.as_str() {
            "bind" => {
                container.pictures.bind = picture(tree, child);
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
                container.pictures.edit = picture(tree, child);
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
                let kind = property.children.first().and_then(|&kind| tree.get(kind));
                container.value_type = kind.map(|kind| kind.local.clone());
                container.digits = kind.map(Digits::of).unwrap_or_default();
            }
            "validate" => {
                container.pictures.validate = picture(tree, child);
                container.pictures.format_test = match property.attribute("formatTest") {
                    Some("disabled") => FormatTest::Disabled,
                    Some("error") => FormatTest::Error,
                    // Page 880's default, which a value it does not list falls back to as well.
                    _ => FormatTest::Warning,
                };
                container.pictures.null_test = match property.attribute("nullTest") {
                    Some("warning") => NullTest::Warning,
                    Some("error") => NullTest::Error,
                    // Page 881's default, which a value it does not list falls back to as well.
                    _ => NullTest::Disabled,
                };
                container.pictures.messages = tree
                    .child(child, "message")
                    .and_then(|message| tree.get(message))
                    .map(|message| {
                        message
                            .children
                            .iter()
                            .filter_map(|&text| tree.get(text))
                            .filter_map(|text| {
                                let said = text.text.split_whitespace().collect::<Vec<_>>();
                                Some((text.attribute("name")?.to_owned(), said.join(" ")))
                            })
                            .filter(|(_, said)| !said.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
            }
            "calculate" => {
                container.calculation = tree
                    .child(child, "script")
                    .and_then(|script| tree.get(script))
                    .filter(|script| !script.text.trim().is_empty())
                    .map(|script| Script {
                        language: Language::of(script.attribute("contentType")),
                        text: script.text.clone(),
                    });
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
}

/// The text of the `picture` element under `property`, where it states a non-empty one.
fn picture(tree: &Tree, property: usize) -> Option<String> {
    let at = tree.child(property, "picture")?;
    let text = tree.get(at)?.text.trim();
    (!text.is_empty()).then(|| text.to_owned())
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
