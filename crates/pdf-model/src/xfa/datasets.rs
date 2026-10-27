//! The `datasets` packet: §K.2's data, "which represents the state of the form", read and kept
//! consistent with the `/V` entries a save writes.
//!
//! §K.2's first writer rule binds this program the moment it saves a hybrid file:
//!
//! > PDF interactive form field objects shall be present for each field specified in the XFA
//! > resource. The XFA field values shall be consistent with the corresponding V entries of the
//! > PDF field objects.
//!
//! So a value written to a field's `/V` is written to the data value that field binds as well
//! (RFC 0011 section 5, step 2; ADR 2396). The packet is spliced, not re-serialised: the data
//! values a person changed get new content, a data value the binding needs and the packet lacks
//! is appended to its data group — which is where chapter 4's re-normalisation puts one (page
//! 210) — and every other byte is the producer's.

use std::fmt::Write as _;

use super::canonical;
use super::config::Config;
use super::locale::{LocaleSet, Patterns};
use super::picture::{self, Category};
use super::template::{Binding, Class, Container, FormatTest, Template};
use super::xml::{self, Tree};

/// A value a save writes to a field's `/V`, in the terms its data value needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A text string: a text field's value, or a single choice's export value.
    Text(String),
    /// An array of text strings: a choice field with several items selected (Table 233).
    Choices(Vec<String>),
    /// A button turned on, with the export value of the widget that is on — Table 230's `/Opt`
    /// entry for it where the field states one, its appearance state's name where it does not.
    On(String),
    /// A button turned off: §12.7.5.2.3's `Off`.
    Off,
    /// No value: the `/V` removed, as §12.7.6.3's reset does where there is no `/DV`.
    Cleared,
}

/// A `datasets` packet as read: its text, its elements, and where `xfa:data` and the record are.
#[derive(Debug, Clone)]
pub struct Datasets {
    text: String,
    tree: Tree,
    /// The `xfa:data` element.
    data: Option<usize>,
}

/// What [`Datasets::keep`] did: the packet's new text, if anything changed, and every field
/// whose value it could not keep consistent, with the reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kept {
    /// The whole packet (or the whole resource stream it is inside), spliced.
    pub text: Option<String>,
    /// The PDF field name and the sentence saying why its data value was not written.
    pub unkept: Vec<(String, String)>,
}

/// One node of the data as the writer sees it: an element of the packet, or one it adds.
struct Node {
    name: String,
    /// The packet's element this is; `None` for a node this writer adds.
    origin: Option<usize>,
    children: Vec<usize>,
    /// New content, already escaped markup.
    content: Option<String>,
}

impl Datasets {
    /// Reads the packet whose text is `text`, `datasets` being the element index of its
    /// `xfa:datasets` element in `tree`.
    pub(crate) fn new(text: String, tree: Tree, datasets: usize) -> Self {
        let data = tree.child(datasets, "data");
        Self { text, tree, data }
    }

    /// The text of the data value a field binds, as XFA 3.3 chapter 4 binds it; `None` where
    /// the packet holds none. A multiple-selection choice list's data group answers with its
    /// values joined by line feeds, which is how page 198 says the form sees them.
    #[must_use]
    pub fn value(&self, template: &Template, field: &str) -> Option<String> {
        let target = template.resolve(field)?;
        let binding = template.binding(target).ok()?;
        let record = self.record()?;
        let found = match binding {
            Binding::Direct(steps) => self.walk(record, &steps),
            Binding::Global(steps, name) => self
                .walk(record, &steps)
                .or_else(|| self.global(record, &name)),
            Binding::Explicit { steps, from_data } => {
                self.walk(if from_data { self.data? } else { record }, &steps)
            }
            Binding::Nothing => None,
        }?;
        let element = self.tree.get(found)?;
        if element.children.is_empty() {
            return Some(element.text.clone());
        }
        let values: Vec<&str> = element
            .children
            .iter()
            .filter_map(|&child| self.tree.get(child))
            .map(|child| child.text.as_str())
            .collect();
        Some(values.join("\n"))
    }

    /// The record: the first data group under `xfa:data`, which the root subform binds whatever
    /// the two are named (chapter 4 page 182).
    fn record(&self) -> Option<usize> {
        self.tree.get(self.data?)?.children.first().copied()
    }

    fn walk(&self, from: usize, steps: &[(String, usize)]) -> Option<usize> {
        let mut at = from;
        for (name, index) in steps {
            at = self
                .tree
                .get(at)?
                .children
                .iter()
                .copied()
                .filter(|&child| self.tree.get(child).is_some_and(|c| &c.local == name))
                .nth(*index)?;
        }
        Some(at)
    }

    /// A global field's data: a data value of its name outside the record, the first of them,
    /// because the record is the first and page 247 picks the global whose index is the
    /// record's number.
    fn global(&self, record: usize, name: &str) -> Option<usize> {
        let data = self.tree.get(self.data?)?;
        let mut pending: Vec<usize> = data
            .children
            .iter()
            .copied()
            .filter(|&child| child != record)
            .rev()
            .collect();
        while let Some(at) = pending.pop() {
            let element = self.tree.get(at)?;
            if element.local == name && element.children.is_empty() {
                return Some(at);
            }
            pending.extend(element.children.iter().rev());
        }
        None
    }

    /// Writes each changed field's value into the data value it binds.
    ///
    /// `changes` are PDF field names — §12.7.4.2's qualified names, which an XFAF form makes SOM
    /// expressions (XFA 3.3 page 72) — with what the save writes to each `/V`; `locales` are the
    /// form's, which a numeric, date or time field's value is canonicalised in (ADR 2414);
    /// `config` is the form's `config` packet, whose default bind pictures a field with none of
    /// its own is written by, and whose extended mapping rules this program does not carry out
    /// leave every field named (XFA 3.3 chapter 15; ADR 2450).
    #[must_use]
    pub fn keep(
        &self,
        template: &Template,
        (locales, config): (&LocaleSet, &Config),
        changes: &[(String, Change)],
    ) -> Kept {
        let mut kept = Kept::default();
        if let Some(reason) = config.unfollowed() {
            kept.unkept = changes
                .iter()
                .map(|(name, _)| (name.clone(), reason.clone()))
                .collect();
            return kept;
        }
        let (Some(data), Some(record)) = (self.data, self.record()) else {
            kept.unkept = changes
                .iter()
                .map(|(name, _)| {
                    (
                        name.clone(),
                        "the XFA datasets packet holds no data group for the form".to_owned(),
                    )
                })
                .collect();
            return kept;
        };
        let mut nodes = Vec::new();
        let data_node = self.overlay(data, &mut nodes);
        // `overlay` numbers the nodes in document order, so the record's node is the first child
        // of the data node, as `record` found it.
        let Some(record_node) = nodes
            .get(data_node)
            .and_then(|n: &Node| n.children.first().copied())
        else {
            return kept;
        };
        let mut any_kept = false;
        for (name, entered) in changes {
            match self.keep_one(
                (template, locales, config),
                &mut nodes,
                (data_node, record_node),
                record,
                name,
                entered,
            ) {
                Ok(()) => any_kept = true,
                Err(reason) => kept.unkept.push((name.clone(), reason)),
            }
        }
        if any_kept {
            kept.text = Some(self.spliced(&nodes, data_node));
        }
        kept
    }

    fn keep_one(
        &self,
        (template, locales, config): (&Template, &LocaleSet, &Config),
        nodes: &mut Vec<Node>,
        (data_node, record_node): (usize, usize),
        record: usize,
        name: &str,
        entered: &Change,
    ) -> Result<(), String> {
        let target = template
            .resolve(name)
            .ok_or("its name points at no field of the XFA template")?;
        let container = template
            .get(target)
            .ok_or("its name points at no field of the XFA template")?;
        let binding = template.binding(target).map_err(str::to_owned)?;
        let content = content(
            template,
            locales,
            target,
            &defaulted(config, container, &binding)?,
            entered,
        )?;
        let node = match binding {
            Binding::Nothing => return Ok(()),
            Binding::Direct(steps) => descend(nodes, record_node, &steps, true)?,
            Binding::Explicit { steps, from_data } => descend(
                nodes,
                if from_data { data_node } else { record_node },
                &steps,
                true,
            )?,
            Binding::Global(steps, global) => {
                if let Ok(node) = descend(nodes, record_node, &steps, false) {
                    node
                } else {
                    let element = self.global(record, &global).ok_or(
                        "it is bound globally, and no data value of its name is outside the record",
                    )?;
                    nodes
                        .iter()
                        .position(|n| n.origin == Some(element))
                        .ok_or("its global data value is not in the packet")?
                }
            }
        };
        let target_node = nodes.get_mut(node).ok_or("the data node vanished")?;
        if !target_node.children.is_empty() && !matches!(content, Content::Values(_)) {
            return Err(
                "its data value holds markup, which this program does not write".to_owned(),
            );
        }
        target_node.children.clear();
        target_node.content = Some(match content {
            Content::Text(text) => xml::escape(&text),
            Content::Values(values) => values.iter().fold(String::new(), |mut out, value| {
                let _ = write!(out, "<value>{}</value>", xml::escape(value));
                out
            }),
        });
        Ok(())
    }

    /// The data subtree under `element` as writer nodes, numbered in document order.
    fn overlay(&self, element: usize, nodes: &mut Vec<Node>) -> usize {
        let at = nodes.len();
        let name = self
            .tree
            .get(element)
            .map(|e| e.local.clone())
            .unwrap_or_default();
        nodes.push(Node {
            name,
            origin: Some(element),
            children: Vec::new(),
            content: None,
        });
        let children = self
            .tree
            .get(element)
            .map(|e| e.children.clone())
            .unwrap_or_default();
        for child in children {
            let node = self.overlay(child, nodes);
            if let Some(parent) = nodes.get_mut(at) {
                parent.children.push(node);
            }
        }
        at
    }

    /// The packet's text with every node that has new content, or new children, spliced in.
    fn spliced(&self, nodes: &[Node], from: usize) -> String {
        let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let mut pending = vec![from];
        while let Some(at) = pending.pop() {
            let Some(node) = nodes.get(at) else {
                continue;
            };
            let Some(element) = node.origin.and_then(|origin| self.tree.get(origin)) else {
                continue;
            };
            let added: String = node
                .children
                .iter()
                .filter(|&&child| nodes.get(child).is_some_and(|c| c.origin.is_none()))
                .map(|&child| rendered(nodes, child))
                .collect();
            let inner = match &node.content {
                Some(content) => Some(content.clone()),
                None if !added.is_empty() => Some(added),
                None => None,
            };
            if let Some(inner) = inner {
                match (&element.content, &node.content) {
                    (Some(range), Some(_)) => edits.push((range.clone(), inner)),
                    (Some(range), None) => edits.push((range.end..range.end, inner)),
                    (None, _) => edits.push((
                        element.start_end.clone(),
                        format!(">{inner}</{}>", element.qualified),
                    )),
                }
            }
            if node.content.is_none() {
                pending.extend(node.children.iter().copied());
            }
        }
        edits.sort_by_key(|(range, _)| range.start);
        let mut out = String::with_capacity(self.text.len());
        let mut from = 0;
        for (range, text) in edits {
            out.push_str(self.text.get(from..range.start).unwrap_or(""));
            out.push_str(&text);
            from = range.end;
        }
        out.push_str(self.text.get(from..).unwrap_or(""));
        out
    }
}

/// A node this writer added, as markup.
fn rendered(nodes: &[Node], at: usize) -> String {
    let Some(node) = nodes.get(at) else {
        return String::new();
    };
    let inner: String = match &node.content {
        Some(content) => content.clone(),
        None => node
            .children
            .iter()
            .map(|&child| rendered(nodes, child))
            .collect(),
    };
    format!("<{0}>{inner}</{0}>", node.name)
}

/// Follows `steps` from `from`, adding a node where the data lacks one and `create` allows.
///
/// Chapter 4 adds a data node for a field that matched none (page 195 under `matchTemplate`,
/// page 210's re-normalisation under `consumeData`), appended after the group's existing
/// children; this does the same, and only where the missing node is the next of its name, so an
/// index past a gap is said rather than filled with nodes nobody's template asked for.
fn descend(
    nodes: &mut Vec<Node>,
    from: usize,
    steps: &[(String, usize)],
    create: bool,
) -> Result<usize, String> {
    let mut at = from;
    for (name, index) in steps {
        let parent = nodes.get(at).ok_or("the data node vanished")?;
        let same: Vec<usize> = parent
            .children
            .iter()
            .copied()
            .filter(|&child| nodes.get(child).is_some_and(|c| &c.name == name))
            .collect();
        at = match same.get(*index) {
            Some(&found) => found,
            None if create && same.len() == *index && xml::is_name(name) => {
                if parent.content.is_some() {
                    return Err("its data group is a data value in the packet".to_owned());
                }
                let added = nodes.len();
                nodes.push(Node {
                    name: name.clone(),
                    origin: None,
                    children: Vec::new(),
                    content: None,
                });
                if let Some(parent) = nodes.get_mut(at) {
                    parent.children.push(added);
                }
                added
            }
            None => {
                return Err(format!(
                    "the data holds {} node(s) named {name} where its binding needs number {}",
                    same.len(),
                    index.saturating_add(1)
                ));
            }
        };
    }
    Ok(at)
}

/// A data value's new content, before escaping.
enum Content {
    Text(String),
    Values(Vec<String>),
}

/// What a container's data value holds for a `/V`, by the container's `ui`.
///
/// - A check button's value is the first of its `items` when on and the second when off, an
///   absent one being empty (XFA 3.3's `checkButton`, page 759).
/// - An exclusion group's value is the on value of the button that is on, and empty when none
///   is (chapter 4, pages 195–197, the group named and its buttons not).
/// - A choice list's is the export value; with `open="multiSelect"` its data is a group of
///   `value` elements, one per selection (page 198).
/// - A text, numeric or date-and-time field's is the text a person typed, canonicalised as
///   [`typed`] says.
fn content(
    template: &Template,
    locales: &LocaleSet,
    target: usize,
    container: &Container,
    entered: &Change,
) -> Result<Content, String> {
    let mismatch = || "its /V is not the kind of value its XFA field holds".to_owned();
    if container.class == Class::ExclGroup {
        return match entered {
            Change::On(export) => {
                let on = container
                    .children
                    .iter()
                    .filter_map(|&child| template.get(child))
                    .filter_map(|button| button.items.first())
                    .filter_map(|(_, values)| values.first())
                    .any(|value| value == export);
                if on {
                    Ok(Content::Text(export.clone()))
                } else {
                    Err(format!(
                        "no button of its exclusion group has the on value {export:?}"
                    ))
                }
            }
            Change::Off | Change::Cleared => Ok(Content::Text(String::new())),
            Change::Text(_) | Change::Choices(_) => Err(mismatch()),
        };
    }
    if container.rich {
        return Err(
            "its XFA value is rich text, whose data is markup this program does not write"
                .to_owned(),
        );
    }
    let (ui, open) = container
        .ui
        .as_ref()
        .map(|(ui, open)| (ui.as_str(), open.as_deref()))
        .ok_or("its XFA field states no ui")?;
    match (ui, entered) {
        ("checkButton", Change::On(_)) => Ok(Content::Text(item(container, 0))),
        ("checkButton", Change::Off | Change::Cleared) => Ok(Content::Text(item(container, 1))),
        ("choiceList", Change::Choices(values)) if open == Some("multiSelect") => {
            Ok(Content::Values(values.clone()))
        }
        ("choiceList", Change::Text(value)) if open == Some("multiSelect") => {
            Ok(Content::Values(vec![value.clone()]))
        }
        ("choiceList", Change::Cleared) if open == Some("multiSelect") => {
            Ok(Content::Values(Vec::new()))
        }
        ("choiceList", Change::Choices(values)) if values.len() == 1 => {
            Ok(Content::Text(values.concat()))
        }
        ("choiceList", Change::Text(value)) => Ok(Content::Text(value.clone())),
        (
            "choiceList" | "textEdit" | "passwordEdit" | "numericEdit" | "dateTimeEdit",
            Change::Cleared,
        ) => Ok(Content::Text(String::new())),
        ("textEdit" | "passwordEdit" | "numericEdit" | "dateTimeEdit", Change::Text(value)) => {
            typed(template, locales, target, container, ui, value).map(Content::Text)
        }
        (
            "checkButton" | "choiceList" | "textEdit" | "passwordEdit" | "numericEdit"
            | "dateTimeEdit",
            _,
        ) => Err(mismatch()),
        (other, _) => Err(format!(
            "its XFA {other} holds a value no /V of this program's writes"
        )),
    }
}

/// The data value of a text, numeric or date-and-time field a person typed `value` into: XFA 3.3
/// chapter 4's input parsing (pages 162–164) turns what was typed into the canonical form the
/// Form DOM holds (page 156, chapter 24), the `validate` picture's format test decides whether the
/// data takes it (page 162), and the `bind` picture writes it in the data's own format (page 176).
///
/// - **The format test**: page 382 passes a value its `validate` picture can format, and page 162
///   updates the data only once the test succeeds. A value the test fails is named and not
///   written, at `error` (page 385's refusal) and at `warning`, page 880's default, alike: a
///   warning's override is the person's to give (page 385), and a save asks no one (ADR 2434).
///   Page 396 exempts an empty value, and `formatTest="disabled"` turns the test off.
/// - **The `bind` picture**: page 176 converts the data from canonical into its local format
///   when it is saved, so the data value is the canonical value written out by it. A value it
///   cannot write is written canonical, as page 493 has a widget show a value its picture cannot
///   format just as it is, and as Rule 2 reads such data back (ADR 2434).
fn typed(
    template: &Template,
    locales: &LocaleSet,
    target: usize,
    container: &Container,
    ui: &str,
    value: &str,
) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let hint = match container.value_type.as_deref() {
        Some("date") => Some(Category::Date),
        Some("time") => Some(Category::Time),
        Some("integer" | "decimal" | "float") => Some(Category::Num),
        Some("text") => Some(Category::Text),
        _ if ui == "numericEdit" => Some(Category::Num),
        _ => None,
    };
    let locale = locales.prevailing(template.locale(target));
    let canonical = canonicalized(locales, container, ui, value, hint, &locale)?;
    let pictures = &container.pictures;
    if let Some(validate) = &pictures.validate
        && pictures.format_test != FormatTest::Disabled
    {
        let level = match pictures.format_test {
            FormatTest::Error => "error",
            FormatTest::Warning | FormatTest::Disabled => "warning",
        };
        match picture::format(validate, hint, &locale, locales, &canonical) {
            Ok(Some(_)) => {}
            Ok(None) => {
                // Page 383: the form designer's message for the test where the form gives one.
                let said = pictures
                    .message("formatTest")
                    .map(|said| format!(" — its form's message for it is {said:?}"))
                    .unwrap_or_default();
                return Err(format!(
                    "its value {canonical:?} fails its validate picture's format test (XFA 3.3 \
                     page 382) at the level {level}{said}, and page 162 updates the data only \
                     once the test succeeds; a warning's override (page 385) is the person's, \
                     and this save asks no one"
                ));
            }
            Err(reason) => {
                return Err(format!(
                    "its validate picture's format test (XFA 3.3 page 382) cannot be run: {reason}"
                ));
            }
        }
    }
    if let Some(bind) = &pictures.bind {
        return Ok(picture::format(bind, hint, &locale, locales, &canonical)
            .map_err(|reason| format!("its bind picture cannot write its data: {reason}"))?
            .unwrap_or(canonical));
    }
    Ok(canonical)
}

/// What a person typed as the Form DOM's canonical value: chapter 4's input parsing (pages
/// 162–164).
///
/// - **Rule 2**: with an edit picture (`ui`'s), the first alternative that matches gives the
///   canonical value; where none matches, the text is taken to be canonical already and is
///   written as typed — page 163's French word for two stays as it was typed. Rule 2 names a
///   `bind` picture beside the edit picture, and page 177 treats a script assigning a value as a
///   person typing one through it, so a field with no edit picture is parsed by its `bind`.
/// - **Rule 2A**: with neither, a `date`, `time`, `float` or `decimal` value is parsed by the
///   locale's default picture — the `med` date or time pattern, the `numeric` number pattern —
///   and accepted as typed where it is already canonical. Page 166 adds that a number is
///   localised only where the `ui` is `numericEdit`, so a `float` or `decimal` elsewhere is not.
/// - **Page 156's exception**: in a `numericEdit` field, text that is not a number counts as
///   zero. Writing zero beside a `/V` that says otherwise is not §K.2's consistency, so such
///   a field is named and not written.
/// - A `dateTime` with no picture is written where it is already canonical and named otherwise:
///   Rule 4 (page 165) joins the default date and time pictures with a `T` and page 1156 with a
///   space, and this program does not choose between them for a person's input.
fn canonicalized(
    locales: &LocaleSet,
    container: &Container,
    ui: &str,
    value: &str,
    hint: picture::Hint,
    locale: &str,
) -> Result<String, String> {
    let value_type = container.value_type.as_deref();
    let numeric = ui == "numericEdit";
    let as_typed = || -> Result<String, String> {
        if numeric && !canonical::is_number(value) {
            return Err(format!(
                "its /V {value:?} is not a number, which XFA 3.3 page 156 evaluates as zero in a \
                 numeric field, and a zero beside that /V would not be consistent with it"
            ));
        }
        Ok(value.to_owned())
    };
    let pictures = &container.pictures;
    if let Some(edit) = pictures.edit.as_ref().or(pictures.bind.as_ref()) {
        return match picture::canonicalize(edit, hint, locale, locales, value)? {
            Some(canonical) => Ok(canonical),
            None => as_typed(),
        };
    }
    let default = match value_type {
        Some("date") => Some((Patterns::Date, "med")),
        Some("time") => Some((Patterns::Time, "med")),
        Some("decimal" | "float") if numeric => Some((Patterns::Number, "numeric")),
        Some("dateTime") if canonical::is_date_time(value) => return Ok(value.to_owned()),
        Some("dateTime") => {
            return Err(format!(
                "its /V {value:?} is a date-time typed with no edit picture, which XFA 3.3 reads \
                 by a default page 165 joins with a T and page 1156 with a space"
            ));
        }
        _ => None,
    };
    let Some((which, name)) = default else {
        return as_typed();
    };
    let already = match which {
        Patterns::Date => canonical::is_date(value),
        Patterns::Time => canonical::is_time(value),
        Patterns::Number => canonical::is_number(value),
    };
    let parsed = locales
        .pattern(locale, which, name)
        .and_then(|pattern| picture::canonicalize(pattern, hint, locale, locales, value));
    match parsed {
        Ok(Some(canonical)) => Ok(canonical),
        Ok(None) => as_typed(),
        Err(_) if already => Ok(value.to_owned()),
        Err(reason) => Err(reason),
    }
}

/// `container` with the default bind picture XFA 3.3 page 527 gives it where its template states
/// none: the `config` packet's `transform` naming the data element it binds, which a direct match
/// and an explicit reference name by their last step and a global binding by its name. A container
/// bound to nothing binds no data element for a `transform` to name (ADR 2450).
fn defaulted(
    config: &Config,
    container: &Container,
    binding: &Binding,
) -> Result<Container, String> {
    let mut container = container.clone();
    if container.pictures.bind.is_none() {
        let named = match binding {
            Binding::Direct(steps) | Binding::Explicit { steps, .. } => {
                steps.last().map(|(name, _)| name.as_str())
            }
            Binding::Global(_, name) => Some(name.as_str()),
            Binding::Nothing => None,
        };
        if let Some(name) = named {
            container.pictures.bind = config.bind_picture(name)?.map(str::to_owned);
        }
    }
    Ok(container)
}

/// A check button's on (`0`) or off (`1`) value: the `items` element's, empty where it states none.
pub(super) fn item(container: &Container, which: usize) -> String {
    container
        .items
        .first()
        .and_then(|(_, values)| values.get(which))
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{Change, Datasets};
    use crate::xfa::config::Config;
    use crate::xfa::locale::LocaleSet;
    use crate::xfa::template::Template;
    use crate::xfa::xml::parse;

    fn template(text: &str) -> Template {
        Template::read(&parse(text.as_bytes()).expect("well formed"), 0)
    }

    fn datasets(text: &str) -> Datasets {
        let tree = parse(text.as_bytes()).expect("well formed");
        Datasets::new(text.to_owned(), tree, 0)
    }

    const TEMPLATE: &str = "<template><subform name=\"order\">\
        <subform name=\"customer\"><field name=\"name\"><ui><textEdit/></ui></field>\
        <field name=\"city\"><ui><textEdit/></ui></field></subform>\
        <field name=\"total\"><ui><numericEdit/></ui></field>\
        <field name=\"region\"><ui><textEdit/></ui><bind match=\"global\"/></field>\
        <field name=\"code\"><ui><textEdit/></ui><bind match=\"dataRef\" ref=\"$data.codes.code[1]\"/></field>\
        <exclGroup name=\"size\"><field><ui><checkButton/></ui><items><text>S</text></items></field>\
        <field><ui><checkButton/></ui><items><text>L</text></items></field></exclGroup>\
        </subform></template>";

    /// Only the changed value's content is replaced; a missing data value is appended to its
    /// group, and a self-closing group is opened to take one (page 210's re-normalisation).
    #[test]
    fn a_splice_changes_the_values_named_and_nothing_else() {
        let t = template(TEMPLATE);
        let d = datasets(
            "<xfa:datasets xmlns:xfa=\"x\"><xfa:data>\n<order>\n <customer><name>Old &amp; gone</name></customer>\n</order>\n</xfa:data></xfa:datasets>",
        );
        let kept = d.keep(
            &t,
            (&LocaleSet::default(), &Config::default()),
            &[
                (
                    "order[0].customer[0].name[0]".to_owned(),
                    Change::Text("New".to_owned()),
                ),
                (
                    "order[0].customer[0].city[0]".to_owned(),
                    Change::Text("Graz".to_owned()),
                ),
                ("order[0].size[0]".to_owned(), Change::On("L".to_owned())),
            ],
        );
        assert!(kept.unkept.is_empty(), "{:?}", kept.unkept);
        assert_eq!(
            kept.text.as_deref(),
            Some(
                "<xfa:datasets xmlns:xfa=\"x\"><xfa:data>\n<order>\n <customer><name>New</name><city>Graz</city></customer>\n<size>L</size></order>\n</xfa:data></xfa:datasets>"
            )
        );
        let empty = datasets("<xfa:datasets><xfa:data><order/></xfa:data></xfa:datasets>");
        let kept = empty.keep(
            &t,
            (&LocaleSet::default(), &Config::default()),
            &[(
                "order.customer.name".to_owned(),
                Change::Text("A".to_owned()),
            )],
        );
        assert_eq!(
            kept.text.as_deref(),
            Some(
                "<xfa:datasets><xfa:data><order><customer><name>A</name></customer></order></xfa:data></xfa:datasets>"
            )
        );
    }

    /// A global field binds its name outside the record when the record has none, and an explicit
    /// `$data` reference is followed from `xfa:data`.
    #[test]
    fn global_and_explicit_bindings_find_their_data() {
        let t = template(TEMPLATE);
        let d = datasets(
            "<datasets><data><order/><shared><region>EU</region></shared>\
             <codes><code>a</code><code>b</code></codes></data></datasets>",
        );
        let kept = d.keep(
            &t,
            (&LocaleSet::default(), &Config::default()),
            &[
                ("order.region".to_owned(), Change::Text("US".to_owned())),
                ("order.code".to_owned(), Change::Text("c".to_owned())),
            ],
        );
        assert!(kept.unkept.is_empty(), "{:?}", kept.unkept);
        let written = datasets(&kept.text.expect("changed"));
        assert_eq!(written.value(&t, "order.region").as_deref(), Some("US"));
        assert_eq!(written.value(&t, "order.code").as_deref(), Some("c"));
    }

    /// What cannot be written is said: text that is no number in a numeric field (XFA 3.3 page
    /// 156), an on value no button of the group has, a name the template does not hold. Nothing
    /// is written for them.
    #[test]
    fn what_cannot_be_kept_is_named() {
        let t = template(TEMPLATE);
        let d = datasets("<datasets><data><order><total>1</total></order></data></datasets>");
        let kept = d.keep(
            &t,
            (&LocaleSet::default(), &Config::default()),
            &[
                ("order.total".to_owned(), Change::Text("1,5".to_owned())),
                ("order.size".to_owned(), Change::On("M".to_owned())),
                ("order.nowhere".to_owned(), Change::Text("x".to_owned())),
            ],
        );
        assert_eq!(kept.text, None);
        assert_eq!(kept.unkept.len(), 3, "{:?}", kept.unkept);
    }
}
