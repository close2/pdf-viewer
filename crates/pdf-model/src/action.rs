//! ISO 32000-2 §12.6's actions: what activating something *does*.
//!
//! §12.6.1 states the point — an annotation or an outline item "may specify an action to
//! perform, such as launching an application, playing a sound, changing an annotation's
//! appearance state" — and Table 201 lists twenty types. This module reads the ones that
//! change what a page **displays**, because that is what this program is: it draws pages.
//!
//! # Which of the twenty are here, and why the other seventeen are not
//!
//! | type | clause | here |
//! |---|---|---|
//! | `GoTo` | §12.6.4.2 | yes — a destination in this document |
//! | `SetOCGState` | §12.6.4.13 | yes — §8.11's layers, which decide what is drawn |
//! | `Hide` | §12.6.4.11 | yes — §12.5.3's Hidden flag, which decides what is drawn |
//! | everything else | | [`Action::Refused`], by name |
//!
//! The refusals are not laziness and they are not uniform. `GoToR`, `GoToE`, `Launch`,
//! `ImportData` and `SubmitForm` want a file system or a network, which principle 3's sandbox
//! deliberately withholds (ADR 0014); `URI` wants a browser; `JavaScript` is on
//! `CLAUDE.md`'s closed exclusion list; `Sound`, `Movie`, `Rendition` and `GoTo3DView` are
//! clause 13's multimedia, excluded by the same list; `Trans`, `Named`, `Thread`, `ResetForm`
//! and `GoToDp` are viewer behaviour this program has not built yet. Each keeps its own name
//! in the refusal so that a caller can say which, rather than "an action".
//!
//! # `/Next` makes an action a tree
//!
//! Table 196's `/Next` is "either a single action dictionary or an array of action
//! dictionaries that shall be performed in order", and each of those may have a `/Next` of its
//! own, so — in the clause's own words — "[t]he actions can thus form a tree instead of a
//! simple linked list". [`read`] flattens that tree into the order it is to be performed in.
//!
//! NOTE 1 also states the one robustness rule the clause gives, and it is a rule about
//! *documents that are wrong*: "self-referential actions ought not be executed more than
//! once". A `/Next` is a reference a file controls, so a cycle is a file a reader must
//! survive; [`read`] visits each action object once and bounds the total.

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::destination::Destination;

/// Most actions one activation may perform.
///
/// §12.6.2 NOTE 1 recommends that a processor "provide some mechanism for the user to
/// interrupt and manually terminate a sequence of actions", which presumes sequences long
/// enough to want interrupting. This program has no such mechanism, so it has a bound
/// instead: a chain longer than this is a file built to make a reader work, not a document
/// asking for something.
const MAX_ACTIONS: usize = 256;

/// What one action does, as far as this program can do it.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// §12.6.4.2: display a destination in this document.
    GoTo(Destination),
    /// §12.6.4.13: set the state of one or more optional content groups.
    SetOcgState(SetOcgState),
    /// §12.6.4.11: hide or show annotations by setting or clearing their Hidden flags.
    Hide(Hide),
    /// An action type this program recognises and does not perform, named.
    ///
    /// A `&'static str` rather than the file's own bytes: the name is one of Table 201's
    /// twenty, matched here, so it is this program's vocabulary and not the document's. An
    /// `/S` that is *not* one of the twenty is not an action at all and produces no entry.
    Refused(&'static str),
}

/// §12.6.4.13's set-OCG-state action. Table 217.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetOcgState {
    /// The changes, in the order `/State` states them, already paired with their groups.
    ///
    /// §12.6.4.13 Table 217:
    ///
    /// > The array elements shall be processed from left to right; each name shall be applied
    /// > to the subsequent groups until the next name is encountered
    ///
    /// A group "may appear more than once in the State array; its state shall be set each
    /// time it is encountered", so this is a list and not a map — EXAMPLE 2 turns on a group
    /// that `[/OFF 1 0 R /Toggle 1 0 R]` names twice.
    pub changes: Vec<(ObjectId, Change)>,
    /// Table 217's `/PreserveRB`, **default `true`**.
    ///
    /// When set, a group turned on takes every other member of its `/RBGroups` radio-button
    /// collection off with it (§8.11.4.5, Table 99). The default being `true` is the shape
    /// this file's own habits warn about: a parameter whose default is the behaviour nobody
    /// implemented is a gap on every document that writes the action.
    pub preserve_radio_buttons: bool,
}

/// One of Table 217's three state names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// `ON` — "sets the state of subsequent groups to ON".
    On,
    /// `OFF` — "sets the state of subsequent groups to OFF".
    Off,
    /// `Toggle` — "reverses the state of subsequent groups".
    Toggle,
}

/// §12.6.4.11's hide action. Table 214.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hide {
    /// What to hide or show: Table 214's `/T`, in all three of its forms.
    pub targets: Vec<HideTarget>,
    /// Table 214's `/H`, **default `true`**: hide (`true`) or show (`false`).
    pub hide: bool,
}

/// One thing a hide action names. Table 214's `/T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HideTarget {
    /// "An indirect reference to an annotation dictionary".
    Annotation(ObjectId),
    /// "A text string giving the fully qualified field name of an interactive form field
    /// whose associated widget annotation or annotations are to be affected".
    ///
    /// Held as the name rather than resolved to annotations, because resolving it means
    /// walking `/AcroForm /Fields` and §12.7.4.2's naming rules, which is
    /// [`crate::view::ViewState`]'s job when the action is performed and not this module's.
    Field(String),
}

/// Reads the action an `/A` entry names, and everything its `/Next` chain performs after it.
///
/// The returned list is in execution order: an action, then its `/Next` subtree, then the
/// next sibling — which is what §12.6.2 NOTE 1 describes as "[a]ctions within each Next array
/// are executed in order, each followed in turn by any actions specified in its Next entry,
/// and so on recursively".
///
/// Empty for anything that is not an action dictionary, which includes the common case of an
/// annotation with no `/A` at all.
#[must_use]
pub fn read(document: &Document, entry: &Object) -> Vec<Action> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    push(document, entry, &mut out, &mut seen);
    out
}

/// Appends one action and its `/Next` subtree.
///
/// `seen` holds the object identity of every action dictionary already visited, which is what
/// makes NOTE 1's "self-referential actions ought not be executed more than once" true here.
/// A *direct* action dictionary has no identity to record; it also cannot be part of a cycle,
/// because nothing can refer back to it.
fn push(document: &Document, entry: &Object, out: &mut Vec<Action>, seen: &mut BTreeSet<ObjectId>) {
    if out.len() >= MAX_ACTIONS {
        return;
    }
    if let Object::Reference(id) = entry
        && !seen.insert(*id)
    {
        return;
    }
    let resolved = document.resolve(entry);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    if let Some(action) = one(document, dict) {
        out.push(action);
    }

    // "The value is either a single action dictionary or an array of action dictionaries
    // that shall be performed in order."
    let next = dict.get("Next").cloned().unwrap_or(Object::Null);
    match document.resolve(&next) {
        Object::Array(items) => {
            for item in &items {
                push(document, item, out, seen);
            }
        }
        Object::Dictionary(_) => push(document, &next, out, seen),
        _ => {}
    }
}

/// Reads one action dictionary, without its `/Next`.
///
/// `None` where `/S` is absent or names nothing in Table 201: §12.6.2 makes `/S` required, so
/// a dictionary without one has not stated an action, and a name outside the table is a type
/// from some future version this reader cannot even name.
fn one(document: &Document, dict: &Dictionary) -> Option<Action> {
    let kind = document.get_key(dict, "S");
    let kind = kind.as_name()?;
    Some(match kind.as_bytes() {
        b"GoTo" => Action::GoTo(Destination::read(document, dict.get("D")?)?),
        b"SetOCGState" => Action::SetOcgState(set_ocg_state(document, dict)),
        b"Hide" => Action::Hide(hide(document, dict)),
        other => Action::Refused(refused(other)?),
    })
}

/// Table 217's `/State` and `/PreserveRB`.
fn set_ocg_state(document: &Document, dict: &Dictionary) -> SetOcgState {
    let mut changes = Vec::new();
    let state = document.get_key(dict, "State");
    if let Some(items) = state.as_array() {
        // "each name shall be applied to the subsequent groups until the next name is
        // encountered" — so a group before any name is governed by nothing and is skipped,
        // which is the only reading of an array that opens with a reference.
        let mut current: Option<Change> = None;
        for item in items {
            match item {
                Object::Name(name) => current = Change::read(name),
                Object::Reference(id) => {
                    if let Some(change) = current {
                        changes.push((*id, change));
                    }
                }
                // A group has to be an indirect object (§8.11.2.2), so a direct dictionary
                // here names nothing this configuration lists and governs nothing.
                _ => {}
            }
        }
    }
    SetOcgState {
        changes,
        preserve_radio_buttons: boolean(&document.get_key(dict, "PreserveRB")).unwrap_or(true),
    }
}

/// Table 214's `/T` and `/H`.
fn hide(document: &Document, dict: &Dictionary) -> Hide {
    let stated = dict.get("T").cloned().unwrap_or(Object::Null);
    let mut targets = Vec::new();
    // The entry is read *unresolved* first, because an annotation target is an indirect
    // reference and resolving it throws away the identity that names the annotation. The
    // array form is the one shape that has to be resolved to be recognised at all.
    match &stated {
        Object::Array(items) => {
            for item in items {
                targets.extend(hide_target(item));
            }
        }
        other => match document.resolve(other) {
            Object::Array(items) => {
                for item in &items {
                    targets.extend(hide_target(item));
                }
            }
            _ => targets.extend(hide_target(other)),
        },
    }
    Hide {
        targets,
        hide: boolean(&document.get_key(dict, "H")).unwrap_or(true),
    }
}

/// §7.3.2's boolean, or `None` for anything else — including an absent entry, which is how
/// both of the defaults above are stated.
fn boolean(object: &Object) -> Option<bool> {
    match object {
        Object::Boolean(value) => Some(*value),
        _ => None,
    }
}

/// One element of `/T`, in either of the two forms Table 214 gives an element.
fn hide_target(item: &Object) -> Option<HideTarget> {
    match item {
        Object::Reference(id) => Some(HideTarget::Annotation(*id)),
        Object::String(bytes) => Some(HideTarget::Field(pdf_syntax::text_string(bytes))),
        _ => None,
    }
}

impl Change {
    /// One of Table 217's three names, or `None` for anything else.
    fn read(name: &Name) -> Option<Self> {
        match name.as_bytes() {
            b"ON" => Some(Self::On),
            b"OFF" => Some(Self::Off),
            b"Toggle" => Some(Self::Toggle),
            _ => None,
        }
    }

    /// Applies this change to a group's current state.
    #[must_use]
    pub fn applied_to(self, state: bool) -> bool {
        match self {
            Self::On => true,
            Self::Off => false,
            Self::Toggle => !state,
        }
    }
}

/// Table 201's other seventeen types, each named rather than lumped together.
///
/// Returning `None` for a name outside the table matters: §12.6.2 says `/S` names a type "see
/// Table 201 for specific values", so a name the table does not hold is not an action this
/// standard defines, and reporting it as a refused action would claim knowledge of it.
fn refused(kind: &[u8]) -> Option<&'static str> {
    Some(match kind {
        b"GoToR" => {
            "GoToR: a destination in another file, which this reader has no filesystem to open"
        }
        b"GoToE" => "GoToE: a destination in an embedded file",
        b"GoToDp" => "GoToDp: a document part, which needs §14.12's part hierarchy",
        b"Launch" => "Launch: running an application, which the sandbox withholds",
        b"Thread" => "Thread: an article thread, which needs a reading-order view",
        b"URI" => "URI: a network resource, which this reader has no network to fetch",
        b"Sound" => "Sound: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"Movie" => "Movie: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"Named" => "Named: a viewer command such as NextPage, which is viewer-ui work",
        b"Rendition" => "Rendition: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"Trans" => "Trans: §12.4.4's page transition, which this viewer does not animate",
        b"GoTo3DView" => "GoTo3DView: clause 13's 3D, excluded by CLAUDE.md principle 5",
        b"JavaScript" => "JavaScript: excluded by CLAUDE.md principle 5",
        b"RichMediaExecute" => {
            "RichMediaExecute: clause 13's multimedia, excluded by CLAUDE.md principle 5"
        }
        b"SubmitForm" => "SubmitForm: §12.7.6.2's submission, which needs a network",
        b"ResetForm" => "ResetForm: §12.7.6.3's reset, which needs a field to be editable first",
        b"ImportData" => "ImportData: §12.7.6.4's FDF import, which needs a filesystem",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{Action, Change, HideTarget, read};
    use pdf_syntax::{Document, Object, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    fn id(number: u32) -> ObjectId {
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// §12.6.4.13's own EXAMPLE 1, read as the clause states it should be.
    ///
    /// > << /S /SetOCGState /State [/OFF 2 0 R 3 0 R /Toggle 16 0 R 19 0 R /ON 5 0 R] >>
    ///
    /// Each name applies to the groups after it until the next name, which is why this is a
    /// list of five pairs and not a dictionary of three names.
    #[test]
    fn a_state_array_applies_each_name_to_the_groups_after_it() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /SetOCGState /State [/OFF 3 0 R 4 0 R /Toggle 5 0 R 6 0 R /ON 7 0 R] >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let [Action::SetOcgState(state)] = actions.as_slice() else {
            panic!("one set-OCG-state action, got {actions:?}");
        };
        assert_eq!(
            state.changes,
            vec![
                (id(3), Change::Off),
                (id(4), Change::Off),
                (id(5), Change::Toggle),
                (id(6), Change::Toggle),
                (id(7), Change::On),
            ]
        );
        assert!(
            state.preserve_radio_buttons,
            "Table 217's default value is true"
        );
    }

    /// §12.6.4.13's EXAMPLE 2: one group named twice, and the last name wins.
    ///
    /// > If the array contained [/OFF 1 0 R /Toggle 1 0 R], the group's state would be ON
    /// > after the action was performed.
    ///
    /// Which only works if the changes are applied in order rather than collapsed per group,
    /// and that is what makes `changes` a list.
    #[test]
    fn a_group_named_twice_is_changed_twice() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /SetOCGState /State [/OFF 3 0 R /Toggle 3 0 R] >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let [Action::SetOcgState(state)] = actions.as_slice() else {
            panic!("one action, got {actions:?}");
        };
        let mut on = true;
        for (_, change) in &state.changes {
            on = change.applied_to(on);
        }
        assert!(on, "OFF then Toggle leaves the group ON");
    }

    /// Table 214's `/T` in all three of its forms, and `/H` defaulting to true.
    #[test]
    fn a_hide_action_names_annotations_and_fields() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /Hide /T [4 0 R (PersonalData.Address.ZipCode)] >>",
            "<< /S /Hide /T 4 0 R /H false >>",
            "<< /Type /Annot /Subtype /Widget >>",
        ]);
        let read_both = read(&doc, &Object::Reference(id(2)));
        let [Action::Hide(both)] = read_both.as_slice() else {
            panic!("one hide action");
        };
        assert_eq!(
            both.targets,
            vec![
                HideTarget::Annotation(id(4)),
                HideTarget::Field("PersonalData.Address.ZipCode".to_owned()),
            ]
        );
        assert!(both.hide, "Table 214's /H defaults to true");

        let read_single = read(&doc, &Object::Reference(id(3)));
        let [Action::Hide(single)] = read_single.as_slice() else {
            panic!("one hide action");
        };
        assert_eq!(single.targets, vec![HideTarget::Annotation(id(4))]);
        assert!(!single.hide);
    }

    /// §12.6.2's `/Next` chain, flattened into execution order.
    ///
    /// The fixture is the clause's own example of the shape — an action with a `/Next` array
    /// whose members have `/Next` entries of their own — so what it pins is that the tree is
    /// walked depth-first rather than breadth-first: an action is "followed in turn by any
    /// actions specified in its Next entry, and so on recursively".
    #[test]
    fn a_next_chain_is_flattened_in_execution_order() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /Hide /T 9 0 R /Next [3 0 R 5 0 R] >>",
            "<< /S /URI /URI (https://example.invalid) /Next 4 0 R >>",
            "<< /S /Launch >>",
            "<< /S /JavaScript /JS (app.alert\\(1\\)) >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let kinds: Vec<&str> = actions
            .iter()
            .map(|action| match action {
                Action::Hide(_) => "Hide",
                Action::Refused(name) => name.split(':').next().unwrap_or(name),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(kinds, vec!["Hide", "URI", "Launch", "JavaScript"]);
    }

    /// §12.6.2 NOTE 1: "self-referential actions ought not be executed more than once".
    ///
    /// `/Next` is a reference the document controls, so a cycle is a file a reader has to
    /// survive rather than a case that cannot arise. Each action object is visited once.
    #[test]
    fn a_cycle_of_next_entries_terminates() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /Hide /T 4 0 R /Next 3 0 R >>",
            "<< /S /Launch /Next 2 0 R >>",
            "<< /Type /Annot >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        assert_eq!(actions.len(), 2, "each action once: {actions:?}");
    }

    /// An `/S` outside Table 201 is not an action, and is not reported as a refused one.
    #[test]
    fn a_name_the_table_does_not_hold_is_not_an_action() {
        let doc = document(&["<< /Type /Catalog >>", "<< /S /Teleport >>"]);
        assert!(read(&doc, &Object::Reference(id(2))).is_empty());
    }
}
