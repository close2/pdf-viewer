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
//! | `Named` | §12.6.4.12 | yes — Table 215's four page commands |
//! | `URI` | §12.6.4.8 | yes — the URI, resolved; opening it is the caller's |
//! | everything else | | [`Action::Refused`], by name |
//!
//! The refusals are not laziness and they are not uniform. `GoToR`, `GoToE`, `Launch`,
//! `ImportData` and `SubmitForm` want a file system or a network, which principle 3's sandbox
//! deliberately withholds (ADR 0014); `JavaScript` is on `CLAUDE.md`'s closed exclusion list;
//! `Sound`, `Movie`, `Rendition` and `GoTo3DView` are clause 13's multimedia, excluded by the
//! same list; `Trans`, `Thread`, `ResetForm` and `GoToDp` are viewer behaviour this program
//! has not built yet. Each keeps its own name in the refusal so that a caller can say which,
//! rather than "an action".
//!
//! # A URI action is read here and performed nowhere
//!
//! §12.6.4.8 says a URI action "causes a URI to be resolved", and *resolving* is two things
//! that this crate can do and one it cannot. It can decide what the URI is — Table 210's
//! `/URI` against Table 211's `/Base`, by RFC 3986 section 5's algorithm in [`crate::uri`]
//! — and it can apply `/IsMap`'s coordinates. What it cannot do is fetch anything, and it deliberately
//! does not: handing a document-controlled URI to a browser is a decision about this machine,
//! so [`Action::Uri`] carries the answer and the caller decides whether to open it.
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
    /// §12.6.4.12: one of Table 215's four named page commands.
    Named(Named),
    /// §12.6.4.8: a URI to resolve, already resolved as far as the document states it.
    Uri(Uri),
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

/// §12.6.4.12's named action: one of Table 215's four page commands.
///
/// The table is short and its four entries are the whole of it — "Go to the next page of the
/// document", the previous, the first and the last — which is why this is an enum of four and
/// not a string. §12.6.4.12 says a processor "shall support" them and that further names may
/// be added, and its NOTE says a document using a non-standard name "is not portable".
///
/// A name outside the four produces no action at all rather than a [`Action::Refused`],
/// because the clause states that case itself: if a processor "does not recognise the name,
/// it shall take no action". Doing nothing is conformance here, not a silence — the corpus's
/// one example is a `/Print`, which is a viewer command and not a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Named {
    /// `NextPage`.
    NextPage,
    /// `PrevPage`.
    PrevPage,
    /// `FirstPage`.
    FirstPage,
    /// `LastPage`.
    LastPage,
}

impl Named {
    /// One of Table 215's four names, or `None` for a name this processor does not recognise.
    fn read(name: &Name) -> Option<Self> {
        match name.as_bytes() {
            b"NextPage" => Some(Self::NextPage),
            b"PrevPage" => Some(Self::PrevPage),
            b"FirstPage" => Some(Self::FirstPage),
            b"LastPage" => Some(Self::LastPage),
            _ => None,
        }
    }

    /// The page this command reaches from `current`, in a document of `pages` pages.
    ///
    /// Clamped at both ends: Table 215 names four movements and no document has a page before
    /// its first or after its last, so the next page of the last page is the last page. `None`
    /// for an empty document, which has no page to reach.
    #[must_use]
    pub fn page_from(self, current: usize, pages: usize) -> Option<usize> {
        let last = pages.checked_sub(1)?;
        Some(match self {
            Self::NextPage => current.saturating_add(1).min(last),
            Self::PrevPage => current.saturating_sub(1).min(last),
            Self::FirstPage => 0,
            Self::LastPage => last,
        })
    }
}

/// §12.6.4.8's URI action. Table 210.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri {
    /// The URI to resolve: Table 210's `/URI`, against Table 211's `/Base` where one exists.
    ///
    /// Resolution is RFC 3986 section 5's, which §12.6.4.8 defers to by naming that RFC as
    /// where a URI is described. Where the document states no `/Base` and the reference is a partial
    /// one, this is the document's own string unchanged and [`Self::relative`] says so.
    pub uri: String,
    /// Whether the URI is still a relative reference, and so incomplete.
    ///
    /// §12.6.4.8: with no base URI, partial URIs "shall be interpreted relative to the
    /// location of the document itself" — which is a fact about where the file was opened
    /// from and not about the file, so this crate cannot finish the job. The caller opened
    /// the document and can.
    pub relative: bool,
    /// Table 210's `/IsMap`, **default `false`**.
    ///
    /// True asks for the cursor's position to be appended, which [`Self::at_position`] does.
    /// The clause bounds where it applies: the entry "applies only to actions triggered by
    /// the user's clicking an annotation; it shall be ignored for actions associated with
    /// outline items or with a document's `OpenAction` entry", so a caller that is not
    /// following a click never asks.
    pub is_map: bool,
}

impl Uri {
    /// §12.6.4.8's `/IsMap` coordinates, appended as the clause's EXAMPLE 1 states them.
    ///
    /// `point` is the cursor in *user* space — the clause has the caller transform it from
    /// device space first — and `rect` is the annotation's `/Rect` as
    /// `[llx, lly, urx, ury]`. The offset is from the rectangle's **upper-left** corner, so
    /// the y term counts downwards from `ury` while the x term counts rightwards from `llx`,
    /// and the pair is rounded to integers before it is appended after a `?` and separated by
    /// a comma. EXAMPLE 2 is the shape: `http://www.iso.org/intro?100,200`.
    ///
    /// Answers [`Self::uri`] unchanged when `/IsMap` is false, so a caller may call it for
    /// every click without asking whether the entry is set.
    #[must_use]
    pub fn at_position(&self, point: (f32, f32), rect: [f32; 4]) -> String {
        if !self.is_map {
            return self.uri.clone();
        }
        let (x, y) = point;
        let [llx, _, _, ury] = rect;
        let across = (x - llx).round();
        let down = (ury - y).round();
        // A NaN coordinate names no position, and appending one would send a reader to a URI
        // the document did not state.
        if !across.is_finite() || !down.is_finite() {
            return self.uri.clone();
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "rounded and finite, and a page is bounded by §14.11.2's 14 400 units"
        )]
        let (across, down) = (across as i64, down as i64);
        format!("{}?{across},{down}", self.uri)
    }
}

/// §12.6.3's trigger events for an annotation. Table 197.
///
/// The clause's own vocabulary: an additional-actions dictionary "extends the set of events
/// that can trigger the execution of an action", and each entry names one event. The four
/// pointer events say what a mouse is — "a generic pointing device" with a selection button,
/// a location and a notion of focus — which is the same NOTE §12.5.5 makes about the
/// appearances the same four events change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trigger {
    /// `/E`: the cursor enters the annotation's active area.
    Enter,
    /// `/X`: the cursor exits it.
    Exit,
    /// `/D`: a button is pressed inside it.
    Down,
    /// `/U`: a button is released inside it.
    ///
    /// The one entry with a precedence rule, and [`for_annotation`] applies it.
    Up,
    /// `/Fo`: the annotation receives the input focus. Widgets only.
    Focus,
    /// `/Bl`: it loses the input focus. Widgets only.
    Blur,
    /// `/PO`: the page containing it is opened.
    PageOpen,
    /// `/PC`: that page is closed.
    PageClose,
    /// `/PV`: that page becomes visible.
    PageVisible,
    /// `/PI`: that page is no longer visible.
    PageInvisible,
}

impl Trigger {
    /// Table 197's key for this event.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Enter => "E",
            Self::Exit => "X",
            Self::Down => "D",
            Self::Up => "U",
            Self::Focus => "Fo",
            Self::Blur => "Bl",
            Self::PageOpen => "PO",
            Self::PageClose => "PC",
            Self::PageVisible => "PV",
            Self::PageInvisible => "PI",
        }
    }
}

/// §12.6.3's trigger events for a page object. Table 198.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageTrigger {
    /// `/O`: the page is opened.
    Open,
    /// `/C`: the page is closed.
    Close,
}

/// What an annotation performs for one of Table 197's events, in execution order.
///
/// Empty for an annotation with no `/AA`, which is every annotation in 942 of the 974 corpus
/// documents.
///
/// # `/U` is the entry with a rule
///
/// §12.6.3's Table 197, of the mouse-up event:
///
/// > For backward compatibility, the A entry in an annotation dictionary, if present, takes
/// > precedence over this entry
///
/// So an annotation that states both performs its `/A` — which is what `crate::link` already
/// follows on a click — and its `/AA /U` is not reached. That is a *rule about two entries*
/// rather than a preference, and it is the reason this function takes the annotation rather
/// than its `/AA`.
#[must_use]
pub fn for_annotation(document: &Document, annotation: &Dictionary, event: Trigger) -> Vec<Action> {
    if event == Trigger::Up {
        let stated = document.get_key(annotation, "A");
        if !matches!(stated, Object::Null) {
            return read(document, &stated);
        }
    }
    let additional = document.get_key(annotation, "AA");
    let Some(additional) = additional.as_dict() else {
        return Vec::new();
    };
    read(
        document,
        additional.get(event.key()).unwrap_or(&Object::Null),
    )
}

/// What a page performs for one of Table 198's two events, in execution order.
///
/// `/AA` on a page object is **not** one of §7.7.3.4's inheritable entries, so this reads the
/// page's own dictionary and does not walk `/Parent`.
#[must_use]
pub fn for_page(document: &Document, page: &Dictionary, event: PageTrigger) -> Vec<Action> {
    let additional = document.get_key(page, "AA");
    let Some(additional) = additional.as_dict() else {
        return Vec::new();
    };
    let key = match event {
        PageTrigger::Open => "O",
        PageTrigger::Close => "C",
    };
    read(document, additional.get(key).unwrap_or(&Object::Null))
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
        b"Named" => Action::Named(Named::read(document.get_key(dict, "N").as_name()?)?),
        b"URI" => Action::Uri(uri(document, dict)?),
        other => Action::Refused(refused(other)?),
    })
}

/// Table 210's `/URI` and `/IsMap`, with Table 211's `/Base` applied.
///
/// `None` where `/URI` is absent or is not a string: §12.6.4.8 makes the entry required, so a
/// URI action without one has stated nothing to resolve.
///
/// The entry is "encoded in UTF-8", which is what makes this the one place in this tree a PDF
/// string is read as UTF-8 rather than through §7.9.2.2's text-string rules — Table 210 calls
/// it an ASCII string and states the encoding of what those bytes spell, and a `/UTF-16`
/// text string would be a different entry. Bytes that are not UTF-8 are a malformed URI, and
/// [`String::from_utf8_lossy`] keeps the rest of it rather than dropping the link.
fn uri(document: &Document, dict: &Dictionary) -> Option<Uri> {
    let Object::String(bytes) = document.get_key(dict, "URI") else {
        return None;
    };
    let stated = String::from_utf8_lossy(&bytes).into_owned();
    let is_map = boolean(&document.get_key(dict, "IsMap")).unwrap_or(false);

    // An absolute reference is its own answer: RFC 3986 section 5.2.2's first branch takes
    // the whole of it from the reference, and the one thing it would still do — removing the dot
    // segments from its path — is normalisation this module deliberately does not apply to a
    // URI a document stated. So the base is not even looked up, which keeps the catalog out
    // of the 216 of the corpus's 217 URI actions that state a scheme.
    if crate::uri::is_absolute(&stated) {
        return Some(Uri {
            uri: stated,
            relative: false,
            is_map,
        });
    }
    let (uri, relative) = match base_uri(document) {
        Some(base) => {
            let resolved = crate::uri::resolve(&base, &stated);
            let relative = !crate::uri::is_absolute(&resolved);
            (resolved, relative)
        }
        None => (stated, true),
    };
    Some(Uri {
        uri,
        relative,
        is_map,
    })
}

/// Table 211's `/Base`, from the catalog's `/URI` dictionary.
///
/// §12.6.4.8: "[t]o support URI actions, a PDF document's catalog dictionary … may include a
/// URI entry whose value is a URI dictionary", and "[o]nly one entry shall be defined for
/// such a dictionary". One corpus document of 974 states it.
fn base_uri(document: &Document) -> Option<String> {
    let catalog = document.catalog().ok()?;
    let uri = document.get_key(&catalog, "URI");
    let dict = uri.as_dict()?;
    match document.get_key(dict, "Base") {
        Object::String(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        _ => None,
    }
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
        b"Sound" => "Sound: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"Movie" => "Movie: clause 13's multimedia, excluded by CLAUDE.md principle 5",
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
    use super::{Action, Change, HideTarget, Named, read};
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
                Action::Uri(_) => "URI",
                Action::Refused(name) => name.split(':').next().unwrap_or(name),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(kinds, vec!["Hide", "URI", "Launch", "JavaScript"]);
    }

    /// Table 215's four names, and the one the corpus writes that is not among them.
    ///
    /// §12.6.4.12 states what a processor does with the fourth case itself — "if the viewer
    /// does not recognise the name, it shall take no action" — so `/Print` produces no action
    /// rather than a refusal. `Named::page_from` is the movement each one names, clamped: the
    /// page after the last page is the last page.
    #[test]
    fn a_named_action_is_one_of_table_215s_four_page_commands() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /Named /N /NextPage >>",
            "<< /S /Named /N /PrevPage >>",
            "<< /S /Named /N /FirstPage >>",
            "<< /S /Named /N /LastPage >>",
            "<< /S /Named /N /Print >>",
        ]);
        let named = |number| match read(&doc, &Object::Reference(id(number))).as_slice() {
            [Action::Named(named)] => Some(*named),
            [] => None,
            other => panic!("one named action or none, got {other:?}"),
        };
        assert_eq!(named(2), Some(Named::NextPage));
        assert_eq!(named(3), Some(Named::PrevPage));
        assert_eq!(named(4), Some(Named::FirstPage));
        assert_eq!(named(5), Some(Named::LastPage));
        assert_eq!(named(6), None, "/Print is not one of Table 215's four");

        assert_eq!(Named::NextPage.page_from(1, 10), Some(2));
        assert_eq!(
            Named::NextPage.page_from(9, 10),
            Some(9),
            "clamped at the end"
        );
        assert_eq!(Named::PrevPage.page_from(0, 10), Some(0));
        assert_eq!(Named::FirstPage.page_from(7, 10), Some(0));
        assert_eq!(Named::LastPage.page_from(0, 10), Some(9));
        assert_eq!(Named::LastPage.page_from(0, 0), None, "no page to reach");
    }

    /// §12.6.4.8 with Table 211's `/Base`: `issue14802.pdf`'s own pair.
    ///
    /// The document states `/URI << /Base (http://example.com/) >>` in its catalog and a URI
    /// action of `(./relative_link.txt)`, which is the only corpus document that needs the
    /// base at all — and the only one whose `/URI` is a partial reference is a *different*
    /// file, `pr19449.pdf`, which states no base and so stays relative.
    #[test]
    fn a_partial_uri_is_resolved_against_the_documents_base() {
        let doc = document(&[
            "<< /Type /Catalog /URI << /Base (http://example.com/) >> >>",
            "<< /S /URI /URI (./relative_link.txt) >>",
            "<< /S /URI /URI (https://example.invalid/a?b#c) >>",
        ]);
        let read_relative = read(&doc, &Object::Reference(id(2)));
        let [Action::Uri(relative)] = read_relative.as_slice() else {
            panic!("one URI action");
        };
        assert_eq!(relative.uri, "http://example.com/relative_link.txt");
        assert!(!relative.relative, "resolved against the base");
        assert!(!relative.is_map, "Table 210's default");

        let read_absolute = read(&doc, &Object::Reference(id(3)));
        let [Action::Uri(absolute)] = read_absolute.as_slice() else {
            panic!("one URI action");
        };
        assert_eq!(
            absolute.uri, "https://example.invalid/a?b#c",
            "an absolute reference is handed on exactly as the document wrote it"
        );
    }

    /// With no `/Base`, a partial URI stays partial and says so.
    ///
    /// §12.6.4.8 says such a URI is "interpreted relative to the location of the document
    /// itself", which is a fact about where the file was opened from. `pr19449.pdf` writes
    /// `foo.bar.com`, which is a relative reference and not a host name — guessing a scheme
    /// for it would be deciding where the link goes.
    #[test]
    fn a_partial_uri_with_no_base_is_reported_as_relative() {
        let doc = document(&["<< /Type /Catalog >>", "<< /S /URI /URI (foo.bar.com) >>"]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let [Action::Uri(uri)] = actions.as_slice() else {
            panic!("one URI action");
        };
        assert_eq!(uri.uri, "foo.bar.com");
        assert!(uri.relative);
    }

    /// §12.6.4.8's `/IsMap`, its EXAMPLE 1's arithmetic and its EXAMPLE 2's shape.
    ///
    /// The cursor at (150, 642) in user space, inside a rectangle whose lower-left is at
    /// (50, 442) and whose upper-right is at (250, 742): across is 150 − 50 and down is
    /// 742 − 642, so the URI gains `?100,100`. The y term is what the clause's "upper-left
    /// corner" means and is the half a reader gets wrong, because every other rectangle in
    /// this tree is measured from its lower-left.
    #[test]
    fn is_map_appends_the_cursor_relative_to_the_upper_left_corner() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /URI /URI (http://www.iso.org/intro) /IsMap true >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let [Action::Uri(uri)] = actions.as_slice() else {
            panic!("one URI action");
        };
        assert!(uri.is_map);
        assert_eq!(
            uri.at_position((150.0, 642.0), [50.0, 442.0, 250.0, 742.0]),
            "http://www.iso.org/intro?100,100"
        );
        // "If the resulting coordinates (xf, yf) are fractional, they shall be rounded to the
        // nearest integer values."
        assert_eq!(
            uri.at_position((150.4, 641.5), [50.0, 442.0, 250.0, 742.0]),
            "http://www.iso.org/intro?100,101"
        );
    }

    /// Without `/IsMap`, a position changes nothing — the entry is what asks for one.
    #[test]
    fn a_uri_without_is_map_ignores_the_cursor() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /URI /URI (http://example.invalid/) >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let [Action::Uri(uri)] = actions.as_slice() else {
            panic!("one URI action");
        };
        assert_eq!(
            uri.at_position((1.0, 2.0), [0.0, 0.0, 10.0, 10.0]),
            "http://example.invalid/"
        );
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
