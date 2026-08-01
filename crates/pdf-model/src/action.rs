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
//! | `Thread` | §12.6.4.7 | yes — a bead on §12.4.3's article thread, in this file |
//! | `ResetForm` | §12.7.6.3 | yes — a field's value becomes its `/DV`, which changes what is drawn |
//! | `ImportData` | §12.7.6.4 | yes — read, and performed by whoever has the file (§12.7.8) |
//! | `GoToE` | §12.6.4.4 | yes — where the target is embedded in this file, which needs no filesystem |
//! | `Trans` | §12.6.4.15 | yes — read as §12.4.4's transition; playing one is a window's job |
//! | `GoToDp` | §12.6.4.5 | yes — the page §14.12's document part begins at |
//! | everything else | | [`Action::Refused`], by name |
//!
//! The refusals are not laziness and they are not uniform. `GoToR`, `Launch` and
//! `SubmitForm` want a file system or a network, which principle 3's sandbox
//! deliberately withholds (ADR 0014); `JavaScript` is on `CLAUDE.md`'s closed exclusion list;
//! `Sound`, `Movie`, `Rendition` and `GoTo3DView` are clause 13's multimedia, excluded by the
//! same list. A `Thread` action naming *another file* joins the first group, for the
//! same reason `GoToR` is in it. Each keeps its own name in the refusal so that a caller can say which,
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

use crate::article::Bead;
use crate::destination::Destination;

/// Most actions one activation may perform.
///
/// §12.6.2 NOTE 1 recommends that a processor "provide some mechanism for the user to
/// interrupt and manually terminate a sequence of actions", which presumes sequences long
/// enough to want interrupting. This program has no such mechanism, so it has a bound
/// instead: a chain longer than this is a file built to make a reader work, not a document
/// asking for something.
const MAX_ACTIONS: usize = 256;

/// How many Table 205 target dictionaries are followed from one embedded go-to action.
///
/// §12.6.4.4's NOTE asks for exactly this: "[i]t is an error for a target dictionary to have an
/// infinite cycle (for example, one where a target dictionary refers to itself). Interactive PDF
/// processors need to attempt to detect such cases and refuse to execute the action if one is
/// found." A path this long describes a document nested deeper than any real collection.
const MAX_TARGET_DEPTH: usize = 32;

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
    /// §12.6.4.7: jump to a bead on one of §12.4.3's article threads.
    Thread(ThreadJump),
    /// §12.7.6.3: reset form fields to the values the document says they start at.
    ResetForm(ResetForm),
    /// §12.7.6.4: import form data from the file Table 243 names.
    ImportData(ImportData),
    /// §12.6.4.4: go to a destination in a document embedded in this one.
    GoToE(EmbeddedGoTo),
    /// §12.6.4.5: show the page a document part begins at.
    GoToDp(DocumentPartJump),
    /// §12.6.4.15: show the page as it stands, using this transition.
    ///
    /// Table 219's `/Trans` is Table 164's dictionary — the same one a page's own `/Trans` holds
    /// — so this carries [`crate::navigation::Transition`] and nothing of its own. What differs
    /// is *when*: a page's transition plays when the page is turned to, and this one plays in
    /// the middle of a `/Next` chain, which is the whole of what §12.6.4.15 adds.
    Trans(crate::navigation::Transition),
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

/// §12.7.6.3's reset-form action. Table 241.
///
/// The clause states exactly what resetting *is*, and it is a document's own statement rather
/// than an empty value:
///
/// > an interactive PDF processor shall reset selected interactive form fields to their default
/// > values; that is, it shall set the value of the V entry in the field dictionary to that of
/// > the DV entry … If no default value is defined for a field, its V entry shall be removed.
///
/// So this is performable *as a display change*: a field's appearance is built from its value
/// (§12.7.4.3, ADR 0032), and a reset changes which entry that value comes from. Nothing is
/// written to the file — [`crate::view::ViewState`] holds the set of widgets that have been
/// reset, exactly as it holds the annotations a hide action touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetForm {
    /// Table 241's `/Fields`, in whichever of its two forms each element takes.
    ///
    /// Empty means the entry was absent, which the table makes decisive: "[i]f this entry is
    /// omitted, the Include/Exclude flag shall be ignored; all fields in the document's
    /// interactive form are reset."
    pub fields: Vec<ResetTarget>,
    /// Table 242's bit 1: whether `/Fields` says which to reset or which to *spare*.
    ///
    /// §12.7.6.3's Table 242 gives it both readings: clear, and the array "specifies which
    /// fields to reset"; set, and it "indicates which fields to exclude from resetting".
    pub exclude: bool,
}

/// One entry of Table 241's `/Fields`.
///
/// The table permits both spellings in one array — "[e]lements of both kinds may be mixed in the
/// same array" — which is why this is an enum rather than two entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResetTarget {
    /// "[A]n indirect reference to a field dictionary".
    Field(ObjectId),
    /// "[A] text string representing the fully qualified name of a field" (PDF 1.3).
    Name(String),
}

/// §12.7.6.4's import-data action. Table 243.
///
/// The clause is one sentence:
///
/// > Upon invocation of an import-data action, a PDF processor shall import data … from Forms
/// > Data Format (FDF), XFDF (XMLbased Forms Data Format according to ISO 19444-1) or any other
/// > data format that it supports into the document's interactive form from a specified file.
///
/// This action is read here and performed nowhere, for §12.6.4.8's URI's reason (ADR 0070): the
/// two things it needs are a file this program has no filesystem to open and a decision about
/// *which* files a document may name, and both belong to a caller rather than to a renderer.
/// What this crate can do is everything else — [`crate::forms_data::FormsData::read`] reads the
/// file's bytes once somebody has them, and [`crate::view::ViewState::import`] applies it.
///
/// "[O]r any other data format that it supports" is what makes [`Self::format`] worth stating
/// rather than guessing: this program supports FDF and not XFDF, which is ISO 19444-1 and an XML
/// parser rather than a clause of this standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportData {
    /// Table 243's `/F`, "[t]he FDF, XFDF or any other data format file from which to import the
    /// data", as the file names it.
    ///
    /// §7.11.3 makes a file specification "either a string or a dictionary", and both are read
    /// into this — `/UF` first for the dictionary form, which Table 43 makes the Unicode one. It
    /// is a name for a caller to resolve, never a path this crate opens.
    pub file: String,
    /// Which format the name says it is, from nothing more than the file name's extension.
    ///
    /// §12.7.8.1 is where the extension comes from — "[o]n the Microsoft Windows and UNIX
    /// platforms, FDF files shall have the extension .fdf" — and it is a *hint*: the clause
    /// states no way for the action itself to say which format its file is in, so a caller that
    /// opens the file learns the truth from [`crate::forms_data::FormsData::read`], which
    /// answers [`crate::forms_data::FormsDataError::NotFormsData`] for anything that is not one.
    pub format: DataFormat,
}

/// What an import-data action's file name suggests it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    /// `.fdf` — §12.7.8's Forms Data Format, which [`crate::forms_data`] reads.
    Fdf,
    /// `.xfdf` — ISO 19444-1's XML spelling of the same data, which this program does not read.
    Xfdf,
    /// Anything else, which §12.7.6.4's "any other data format that it supports" permits and
    /// this program supports none of.
    Other,
}

/// §12.6.4.4's embedded go-to action. Tables 204 and 205.
///
/// The clause's own vocabulary is worth carrying, because every entry is defined in it:
///
/// > The source is the document containing the embedded go-to action. … The target is the
/// > document in which the destination lives. … The T entry in the action dictionary is a target
/// > dictionary that locates the target in relation to the source, in much the same way that a
/// > relative path describes the physical relationship between two files in a file system.
///
/// **This is the one action of Table 201's twenty that names another document and needs no
/// filesystem to reach it.** A `GoToR` names a file on a disk; a `GoToE` names a file that is
/// *inside the one already open* — §7.11.4's embedded file streams, read since the eighty-sixth
/// session (ADR 0076) — so the whole of the path is bytes this program already holds.
///
/// The destination is deliberately unresolved. §12.3.2's `/D` here is a destination "in the
/// target", and a named one is looked up in the *target's* `/Dests`, so resolving it against the
/// source would answer about the wrong document. [`EmbeddedGoTo::target_in`] opens the target and
/// the caller reads the destination there.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedGoTo {
    /// Table 204's `/D`, "[t]he destination in the target to jump to", as the file states it.
    pub destination: Object,
    /// Table 204's `/T`, flattened into the steps it nests.
    ///
    /// Empty where `/F` is present and `/T` is not, which the table permits: "[o]ptional if F is
    /// present; otherwise required". An empty path with no `/F` is an action naming no target.
    pub path: Vec<TargetStep>,
    /// Table 204's `/F`, "[t]he root document of the target relative to the root document of the
    /// source", where the action names one.
    ///
    /// `None` is the table's own default — "[i]f this entry is absent, the source and target
    /// share the same root document" — and is the only case this program can perform, because
    /// any other root is a file on a disk. Named rather than silently ignored.
    pub root: Option<String>,
    /// Table 204's `/NewWindow`.
    ///
    /// The table makes it a *should* and states the fallback: "[i]f this entry is absent, the
    /// interactive PDF processor should act according to its preference." This program has one
    /// window, so its preference is to replace — and it says so rather than pretending.
    pub new_window: Option<bool>,
}

/// One element of Table 205's path from the source to the target.
///
/// §12.6.4.4, Table 205:
///
/// > R … Specifies the relationship between the current document and the target (which may be an
/// > intermediate target). Valid values are P (the target is the parent of the current document)
/// > and C (the target is a child of the current document).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetStep {
    /// `/R /P`: "the target is the parent of the current document".
    Parent,
    /// `/R /C` with `/N`: a child "located in the `EmbeddedFiles` name tree", by its key there.
    NamedChild(String),
    /// `/R /C` with `/P` and `/A`: a child "associated with a file attachment annotation".
    AttachedChild {
        /// Table 205's `/P`, which page the attachment annotation is on.
        page: AttachmentPage,
        /// Table 205's `/A`, which annotation on that page it is.
        annotation: AttachmentIndex,
    },
}

/// Table 205's `/P`, in both the forms the table gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentPage {
    /// "[I]t specifies the page number (zero-based) in the current document".
    Index(usize),
    /// "[I]t specifies a named destination in the current document that provides the page
    /// number of the file attachment annotation."
    Named(String),
}

/// Table 205's `/A`, in both the forms the table gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentIndex {
    /// "[T]he index (zero-based) of the annotation in the Annots array".
    Index(usize),
    /// "[T]he value of NM in the annotation dictionary".
    Name(String),
}

/// Why an embedded go-to action's target could not be opened.
///
/// Each is a *different* thing a caller may want to say, and none of them is this program
/// failing at something it implements: two are the file naming something it does not contain,
/// one is a file on a disk, and one is the bound §12.6.4.4's own NOTE asks for.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetError {
    /// Table 204's `/F` names a root document other than this one's.
    #[error("the target is in {0}, which this reader has no filesystem to open")]
    AnotherRoot(String),
    /// A `/R /P` step from a document with no parent in this file.
    #[error("§12.6.4.4's path leaves the document this reader opened, which has no parent here")]
    NoParent,
    /// The current document embeds no such file.
    #[error("this document embeds no file called {0}")]
    NoSuchChild(String),
    /// The step names a page or an annotation the current document does not have.
    #[error("§12.6.4.4's path names an attachment this document does not have")]
    NoSuchAttachment,
    /// The embedded bytes are not a PDF this reader can open.
    #[error("the embedded file could not be opened as a PDF: {0}")]
    Unopenable(String),
    /// The action states no path at all.
    #[error("§12.6.4.4's action states neither /F nor /T, so it names no target")]
    NoPath,
}

impl EmbeddedGoTo {
    /// Opens the document Table 205's path leads to, starting from `source`.
    ///
    /// The path is walked one step at a time and the *current document* is what each step is
    /// relative to, which is the clause's own model: "[a]s the hierarchy is navigated, each
    /// intermediate target shall be referred to as the current document. Initially, the source
    /// is the current document."
    ///
    /// A `/R /P` step pops back to the document the walk descended from, which is the only
    /// parent this program can have: an embedded file's parent is the file it was taken out of,
    /// and the document a person opened has no parent inside itself. That is
    /// [`TargetError::NoParent`], and it is the sibling case of §12.6.4.4's EXAMPLE — `/R /P`
    /// then `/R /C` — being performable only from a document that is already a child.
    ///
    /// # Errors
    ///
    /// Every one of [`TargetError`]'s cases, each naming what the file asked for.
    pub fn target_in(&self, source: &Document) -> Result<Document, TargetError> {
        if let Some(root) = &self.root {
            return Err(TargetError::AnotherRoot(root.clone()));
        }
        if self.path.is_empty() {
            return Err(TargetError::NoPath);
        }
        // The documents descended into, in order. The current document is the last of these, or
        // `source` where none has been opened yet — which is what makes a `Parent` step a pop
        // and makes the cycle §12.6.4.4's NOTE warns about impossible: the path is a finite list
        // read once, and each step opens at most one document.
        let mut descended: Vec<Document> = Vec::new();
        for step in &self.path {
            let current = descended.last().unwrap_or(source);
            match step {
                TargetStep::Parent => {
                    if descended.pop().is_none() {
                        return Err(TargetError::NoParent);
                    }
                }
                TargetStep::NamedChild(name) => {
                    let attachment = crate::attachment::attachments(current)
                        .into_iter()
                        .find(|attachment| attachment.name == *name)
                        .ok_or_else(|| TargetError::NoSuchChild(name.clone()))?;
                    descended.push(open_embedded(current, &attachment)?);
                }
                TargetStep::AttachedChild { page, annotation } => {
                    let attachment = attached_file(current, page, annotation)
                        .ok_or(TargetError::NoSuchAttachment)?;
                    descended.push(open_embedded(current, &attachment)?);
                }
            }
        }
        descended.pop().ok_or(TargetError::NoParent)
    }
}

/// Decodes an embedded file stream and opens it as a document.
///
/// The limits are the parent's, so a document nested three deep is held to the same bounds as
/// the one a person opened — which is the whole of what stops a file that embeds itself from
/// being a decompression bomb with extra steps.
fn open_embedded(
    current: &Document,
    attachment: &crate::attachment::Attachment,
) -> Result<Document, TargetError> {
    let data = current
        .decoded_stream_data(&attachment.stream)
        .ok_or_else(|| TargetError::Unopenable("the stream did not decode".to_owned()))?;
    Document::open_with_limits(data.to_vec(), current.limits())
        .map_err(|error| TargetError::Unopenable(error.to_string()))
}

/// Table 205's `/P` and `/A`: the file attachment annotation a child is associated with.
fn attached_file(
    document: &Document,
    page: &AttachmentPage,
    annotation: &AttachmentIndex,
) -> Option<crate::attachment::Attachment> {
    let pages = crate::page::Pages::new(document);
    let index = match page {
        AttachmentPage::Index(index) => *index,
        // "[I]t specifies a named destination in the current document that provides the page
        // number of the file attachment annotation" — so the string is looked up as a
        // destination and only its *page* is used, which is all Table 205 asks of it.
        AttachmentPage::Named(name) => {
            Destination::read(document, &Object::String(name.clone().into_bytes().into()))?
                .page_index(document, &pages)?
        }
    };
    let page = pages.get(index)?;
    let annots = document.get_key(&page.dict, "Annots");
    let annots = annots.as_array()?;
    let found =
        match annotation {
            AttachmentIndex::Index(at) => document.resolve(annots.get(*at)?),
            AttachmentIndex::Name(name) => annots
                .iter()
                .map(|entry| document.resolve(entry))
                .find(|entry| {
                    entry.as_dict().is_some_and(|dict| {
                        document
                            .get_key(dict, "NM")
                            .as_string()
                            .is_some_and(|stated| pdf_syntax::text_string(stated) == *name)
                    })
                })?,
        };
    let dict = found.as_dict()?;
    // §12.5.6.15's `/FS` is the file specification the annotation refers to.
    let specification = document.get_key(dict, "FS");
    crate::attachment::read(document, specification.as_dict()?, String::new())
}

/// Tables 204 and 205, read into [`EmbeddedGoTo`].
///
/// `None` where `/D` is absent, which the table makes required: an action naming no destination
/// has stated nothing to jump to, whatever path it gives.
fn embedded_go_to(document: &Document, dict: &Dictionary) -> Option<Action> {
    let destination = dict.get("D").cloned()?;
    if destination.is_null() {
        return None;
    }
    let mut path = Vec::new();
    let mut target = document.get_key(dict, "T").as_dict().cloned();
    for _ in 0..MAX_TARGET_DEPTH {
        let Some(step) = target else { break };
        let relationship = document.get_key(&step, "R");
        let Some(relationship) = relationship.as_name() else {
            break;
        };
        match relationship.as_bytes() {
            b"P" => path.push(TargetStep::Parent),
            b"C" => match child_step(document, &step) {
                Some(child) => path.push(child),
                // A `/R /C` naming neither a name nor a page-and-annotation pair states no
                // child; the path stops here rather than silently skipping an element, because
                // a shortened path would land in the wrong document.
                None => return Some(Action::Refused(refused(b"GoToE")?)),
            },
            _ => break,
        }
        target = document.get_key(&step, "T").as_dict().cloned();
    }
    Some(Action::GoToE(EmbeddedGoTo {
        destination,
        path,
        root: file_specification(document, dict, "F"),
        new_window: match document.get_key(dict, "NewWindow") {
            Object::Boolean(value) => Some(value),
            _ => None,
        },
    }))
}

/// Table 205's `/R /C`, in whichever of its two forms the step states.
fn child_step(document: &Document, step: &Dictionary) -> Option<TargetStep> {
    if let Some(name) = document.get_key(step, "N").as_string() {
        return Some(TargetStep::NamedChild(pdf_syntax::text_string(name)));
    }
    let page = match document.get_key(step, "P") {
        Object::Integer(index) => AttachmentPage::Index(usize::try_from(index).ok()?),
        Object::String(bytes) => AttachmentPage::Named(pdf_syntax::text_string(&bytes)),
        _ => return None,
    };
    let annotation = match document.get_key(step, "A") {
        Object::Integer(index) => AttachmentIndex::Index(usize::try_from(index).ok()?),
        Object::String(bytes) => AttachmentIndex::Name(pdf_syntax::text_string(&bytes)),
        _ => return None,
    };
    Some(TargetStep::AttachedChild { page, annotation })
}

/// §7.11's file specification under `key`, named rather than opened.
///
/// One line, because [`crate::file_spec`] is where §7.11.1's two forms and Table 43's rule that
/// `/UF` outranks `/F` are read. This used to be three separate readers of the same two keys in
/// two files, each decoding `/F` as a text string — see that module for why the distinction
/// between a text string and a byte string is not cosmetic.
fn file_specification(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    crate::file_spec::FileSpec::parse(document, &document.get_key(dict, key))?.display_name()
}

/// §12.6.4.5's go-to-document-part action. Table 206.
///
/// > A GoToDp action changes the view to the Start page of a specified DPart
///
/// One entry, and it decides which page is shown — which is why §14.12's `inapplicable` rows do
/// not settle this one. The dictionary is kept as its *reference* rather than resolved, because
/// resolving it here would need the page tree that turns its `/Start` into a number, and that is
/// [`DocumentPartJump::page_in`]'s caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentPartJump {
    /// Table 206's `/Dp`, "[t]he indirect reference to a `DPart` dictionary to go to".
    pub part: ObjectId,
}

impl DocumentPartJump {
    /// The zero-based index of the page this action shows, or `None`.
    ///
    /// `None` where `/Dp` does not resolve to a dictionary, where no leaf under it names a
    /// `/Start`, or where the page it names is not in the page tree — three different broken
    /// files and one answer, because a viewer that cannot find the page has nowhere to go.
    #[must_use]
    pub fn page_in(self, document: &Document, pages: &crate::Pages<'_>) -> Option<usize> {
        let part = document.get(self.part);
        let part = part.as_dict()?;
        let page = crate::document_part::first_page(document, part)?;
        pages.index_of(page)
    }
}

/// §12.6.4.7's thread action. Table 209.
///
/// The clause is one sentence — a thread action "jumps to a specified bead on an article
/// thread … in either the current document or a different one" — and the interesting half is
/// Table 209's three spellings of *which* thread, which are the reason this is two enums
/// rather than two references. Resolving them needs the document's articles, so
/// [`ThreadJump::bead_in`] takes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadJump {
    /// Table 209's `/D`, "[t]he destination thread", in whichever form the file states it.
    pub thread: ThreadTarget,
    /// Table 209's `/B`, "[t]he bead in the destination thread", where the file names one.
    ///
    /// Absent means the thread's first bead: the entry is optional, and a thread action with
    /// no bead can only mean the place a thread starts.
    pub bead: Option<BeadTarget>,
}

/// Which thread a [`ThreadJump`] names. Table 209's `/D`, all three forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadTarget {
    /// "An indirect reference to a thread dictionary … the thread shall be in the current file."
    Object(ObjectId),
    /// "The index of the thread within the Threads array … The first thread in the array has
    /// index 0."
    Index(usize),
    /// "The title of the thread as specified in its thread information dictionary."
    ///
    /// The clause states the tie-break itself: "[i]f two or more threads have the same title,
    /// the one appearing first in the document catalog's Threads array shall be used."
    Title(String),
}

/// Which bead a [`ThreadJump`] names. Table 209's `/B`, both forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadTarget {
    /// "An indirect reference to a bead dictionary."
    Object(ObjectId),
    /// "The index of the bead within its thread. The first bead in a thread has index 0."
    Index(usize),
}

impl ThreadJump {
    /// The bead this action jumps to, resolved against a document's articles.
    ///
    /// `None` where the document states no such thread or no such bead — which is a file
    /// naming something it does not contain, and is why this is an `Option` rather than a
    /// page. A caller with a bead turns to [`crate::article::Bead::page_index`].
    #[must_use]
    pub fn bead_in<'a>(&self, articles: &'a crate::article::Articles) -> Option<&'a Bead> {
        let thread = match &self.thread {
            ThreadTarget::Object(id) => articles.threads.iter().find(|thread| thread.id == *id)?,
            ThreadTarget::Index(index) => articles.threads.get(*index)?,
            // "[T]he one appearing first" is `find`'s own answer, since `threads` is in the
            // `/Threads` array's order.
            ThreadTarget::Title(title) => articles
                .threads
                .iter()
                .find(|thread| thread.title.as_deref() == Some(title.as_str()))?,
        };
        match &self.bead {
            None => thread.beads.first(),
            Some(BeadTarget::Index(index)) => thread.beads.get(*index),
            Some(BeadTarget::Object(id)) => thread.beads.iter().find(|bead| bead.id == *id),
        }
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
        b"Thread" => thread(document, dict)?,
        b"ResetForm" => Action::ResetForm(reset_form(document, dict)),
        b"ImportData" => import_data(document, dict)?,
        b"GoToE" => embedded_go_to(document, dict)?,
        // Table 219 makes `/Trans` required, so an action without one has stated no transition
        // and is a dictionary rather than an action.
        b"Trans" => Action::Trans(crate::navigation::transition(document, dict)?),
        // Table 206 makes `/Dp` required and an indirect reference, so an action without one
        // names no document part and is a dictionary rather than an action.
        b"GoToDp" => Action::GoToDp(DocumentPartJump {
            part: match dict.get("Dp") {
                Some(Object::Reference(id)) => *id,
                _ => return None,
            },
        }),
        other => Action::Refused(refused(other)?),
    })
}

/// Table 241's `/Fields` and Table 242's one flag.
///
/// No `None` case: `/Fields` and `/Flags` are both optional, and an action stating neither is the
/// one the table describes — every field in the document reset.
fn reset_form(document: &Document, dict: &Dictionary) -> ResetForm {
    let fields = match document.get_key(dict, "Fields") {
        Object::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                // A reference is the field itself, so it is *not* resolved: what identifies a
                // field here is its object identity, as it is for §12.6.4.11's annotations.
                Object::Reference(id) => Some(ResetTarget::Field(*id)),
                other => match document.resolve(other) {
                    Object::String(bytes) => {
                        Some(ResetTarget::Name(pdf_syntax::text_string(&bytes)))
                    }
                    _ => None,
                },
            })
            .collect(),
        _ => Vec::new(),
    };
    ResetForm {
        fields,
        // Table 242 numbers "from 1 (low-order)", so bit 1 is the value 1.
        exclude: document
            .get_key(dict, "Flags")
            .as_integer()
            .is_some_and(|flags| flags & 1 != 0),
    }
}

/// Table 243's `/F`, in both the forms §7.11.3 gives a file specification.
///
/// `None` where the entry is absent or is neither — the table makes `/F` required, so an
/// import-data action naming no file has stated nothing to import, which is a dictionary rather
/// than an action.
fn import_data(document: &Document, dict: &Dictionary) -> Option<Action> {
    // §7.11.1's two forms and Table 43's `/UF`-over-`/F` rule are `file_spec`'s.
    let file = file_specification(document, dict, "F")?;
    // The extension, case-insensitively: §12.7.8.1 states the letters and nothing about their
    // case, and a file specification is written by whichever platform exported the data.
    let extension = std::path::Path::new(&file)
        .extension()
        .map(|extension| extension.to_string_lossy().into_owned())
        .unwrap_or_default();
    let format = if extension.eq_ignore_ascii_case("fdf") {
        DataFormat::Fdf
    } else if extension.eq_ignore_ascii_case("xfdf") {
        DataFormat::Xfdf
    } else {
        DataFormat::Other
    };
    Some(Action::ImportData(ImportData { file, format }))
}

/// Table 209's `/D` and `/B`, with `/F` deciding that this is another file's thread.
///
/// `None` where `/D` is absent or is none of its three types: the entry is required, and an
/// action naming no thread has stated nothing to jump to. A `/F` produces the refusal instead
/// of a jump, for `GoToR`'s reason — the thread is in a file this reader has no filesystem to
/// open — and it is a *refusal* rather than a silence because the file said where it was.
fn thread(document: &Document, dict: &Dictionary) -> Option<Action> {
    if dict.get("F").is_some() {
        return Some(Action::Refused(refused(b"Thread")?));
    }
    let thread = match dict.get("D")? {
        // A reference is the thread itself; `document.get_key` would resolve it away, and the
        // clause names the *object*.
        Object::Reference(id) => ThreadTarget::Object(*id),
        stated => match document.resolve(stated) {
            Object::Integer(index) => ThreadTarget::Index(usize::try_from(index).ok()?),
            Object::String(bytes) => ThreadTarget::Title(pdf_syntax::text_string(&bytes)),
            _ => return None,
        },
    };
    let bead = match dict.get("B") {
        Some(Object::Reference(id)) => Some(BeadTarget::Object(*id)),
        Some(stated) => match document.resolve(stated) {
            Object::Integer(index) => Some(BeadTarget::Index(usize::try_from(index).ok()?)),
            _ => None,
        },
        None => None,
    };
    Some(Action::Thread(ThreadJump { thread, bead }))
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
        b"Launch" => "Launch: running an application, which the sandbox withholds",
        b"Thread" => {
            "Thread: a thread in another file, which this reader has no filesystem to open"
        }
        b"Sound" => "Sound: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"Movie" => "Movie: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"Rendition" => "Rendition: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        b"GoTo3DView" => "GoTo3DView: clause 13's 3D, excluded by CLAUDE.md principle 5",
        b"JavaScript" => "JavaScript: excluded by CLAUDE.md principle 5",
        b"RichMediaExecute" => {
            "RichMediaExecute: clause 13's multimedia, excluded by CLAUDE.md principle 5"
        }
        b"SubmitForm" => "SubmitForm: §12.7.6.2's submission, which needs a network",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Action, AttachmentIndex, AttachmentPage, BeadTarget, Change, HideTarget, Named,
        TargetError, TargetStep, ThreadTarget, read,
    };
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

    /// Table 209's three spellings of a thread, and both spellings of a bead.
    ///
    /// A reference, an index into `/Threads` and a title all name the same thread here, so a
    /// form dropped from the reader fails this rather than falling through to `None` in a
    /// document nobody has. The title form is the one with a rule attached: "[i]f two or more
    /// threads have the same title, the one appearing first in the document catalog's Threads
    /// array shall be used", which the second thread below exists to check.
    #[test]
    fn a_thread_action_names_its_thread_in_all_three_forms() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 5 0 R /Threads [2 0 R 3 0 R] >>",
            "<< /F 7 0 R /I << /Title (Man Bites Dog) >> >>",
            "<< /F 9 0 R /I << /Title (Man Bites Dog) >> >>",
            "<< /S /Thread /D 3 0 R /B 1 >>",
            "<< /Type /Pages /Kids [6 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 5 0 R >>",
            "<< /N 8 0 R /P 6 0 R /R [0 0 10 10] >>",
            "<< /N 7 0 R /P 6 0 R /R [0 0 10 10] >>",
            "<< /N 9 0 R /P 6 0 R /R [0 0 10 10] >>",
        ]);
        let articles = crate::article::Articles::read(&doc);

        let jump = |body: &str| {
            let object = pdf_syntax::Parser::new(body.as_bytes())
                .parse_object()
                .expect("a dictionary");
            match read(&doc, &object).as_slice() {
                [Action::Thread(jump)] => Some(jump.clone()),
                _ => None,
            }
        };

        let by_reference = jump("<< /S /Thread /D 2 0 R >>").expect("a thread action");
        assert_eq!(by_reference.thread, ThreadTarget::Object(id(2)));
        assert_eq!(
            by_reference.bead_in(&articles).map(|bead| bead.id),
            Some(id(7))
        );

        let by_index = jump("<< /S /Thread /D 1 >>").expect("a thread action");
        assert_eq!(by_index.thread, ThreadTarget::Index(1));
        assert_eq!(
            by_index.bead_in(&articles).map(|bead| bead.id),
            Some(id(9)),
            "index 1 is the second thread, whose only bead is 9"
        );

        let by_title = jump("<< /S /Thread /D (Man Bites Dog) >>").expect("a thread action");
        assert_eq!(
            by_title.thread,
            ThreadTarget::Title("Man Bites Dog".to_owned())
        );
        assert_eq!(
            by_title.bead_in(&articles).map(|bead| bead.id),
            Some(id(7)),
            "two threads share the title and the first in /Threads wins"
        );

        let bead_by_index = jump("<< /S /Thread /D 0 /B 1 >>").expect("a thread action");
        assert_eq!(bead_by_index.bead, Some(BeadTarget::Index(1)));
        assert_eq!(
            bead_by_index.bead_in(&articles).map(|bead| bead.id),
            Some(id(8))
        );

        let bead_by_reference = jump("<< /S /Thread /D 0 /B 8 0 R >>").expect("a thread action");
        assert_eq!(bead_by_reference.bead, Some(BeadTarget::Object(id(8))));
        assert_eq!(
            bead_by_reference.bead_in(&articles).map(|bead| bead.id),
            Some(id(8))
        );

        assert_eq!(
            jump("<< /S /Thread /D 4 >>")
                .expect("a thread action")
                .bead_in(&articles),
            None,
            "a thread index the document does not have names no bead"
        );
    }

    /// A thread action with an `/F` is another file's, and is refused by name.
    ///
    /// Table 209: "[t]he file containing the thread. If this entry is absent, the thread is in
    /// the current file." So the entry's presence is the whole test, and the refusal is
    /// `GoToR`'s — a filesystem this reader deliberately does not have.
    #[test]
    fn a_thread_in_another_file_is_refused_by_name() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /Thread /F (other.pdf) /D 0 >>",
        ]);
        let actions = read(&doc, &Object::Reference(id(2)));
        let [Action::Refused(why)] = actions.as_slice() else {
            panic!("one refusal, got {actions:?}");
        };
        assert!(why.starts_with("Thread:"), "{why}");
    }
    /// §12.6.4.15: a transition action carries Table 164's dictionary and nothing else.
    ///
    /// Table 219 makes `/Trans` required, so an action without one has stated no transition —
    /// and the dictionary it carries is read by the same function a *page's* `/Trans` is, which
    /// is what the two clauses being one table means.
    #[test]
    fn a_transition_action_carries_table_164s_dictionary() {
        let document = document(&[
            "<< /Type /Action /S /Trans /Trans << /S /Wipe /D 2 /Di 180 >> >>",
            "<< /Type /Action /S /Trans >>",
        ]);

        let read_one = |number: u32| read(&document, &document.get(ObjectId::new(number, 0)));
        match read_one(1).as_slice() {
            [Action::Trans(transition)] => {
                assert_eq!(
                    transition.style,
                    crate::navigation::Style::Wipe,
                    "Table 164's /S"
                );
                assert!((transition.duration - 2.0).abs() < 1e-6, "/D");
                assert_eq!(
                    transition.direction,
                    crate::navigation::Direction::Degrees(180.0),
                    "/Di"
                );
            }
            other => panic!("one transition action, got {other:?}"),
        }
        assert!(
            read_one(2).is_empty(),
            "an action stating no /Trans has stated nothing"
        );
    }

    /// §12.6.4.4's own EXAMPLE, all three relationships, read as the paths they describe.
    ///
    /// The clause writes them out as three action dictionaries — a child, the parent, and a
    /// sibling reached by going up and back down — and the third is the one that shows why a
    /// path is a *list*: `/T` nests, and nesting is the only way a target dictionary states more
    /// than one step.
    #[test]
    fn the_clauses_three_example_relationships_read_as_paths() {
        let paths = |target: &str| {
            let document =
                document(&[
                    format!("<< /Type /Action /S /GoToE /D (Chapter 1) /T {target} >>").as_str(),
                ]);
            let catalog = document.get(ObjectId::new(1, 0));
            match read(&document, &catalog).as_slice() {
                [Action::GoToE(target)] => target.path.clone(),
                other => panic!("one embedded go-to, got {other:?}"),
            }
        };

        assert_eq!(
            paths("<< /R /C /N (Embedded document) >>"),
            [TargetStep::NamedChild("Embedded document".to_owned())]
        );
        assert_eq!(paths("<< /R /P >>"), [TargetStep::Parent]);
        assert_eq!(
            paths("<< /R /P /T << /R /C /N (Another embedded document) >> >>"),
            [
                TargetStep::Parent,
                TargetStep::NamedChild("Another embedded document".to_owned())
            ],
            "a sibling is the parent and then a child"
        );
    }

    /// Table 205's other child form: an attachment annotation, named by page and index or by
    /// `/NM`, in either of the two spellings the table gives each.
    #[test]
    fn a_child_may_be_named_by_the_annotation_it_is_attached_to() {
        let step = |entries: &str| {
            let document = document(&[format!(
                "<< /Type /Action /S /GoToE /D (d) /T << /R /C {entries} >> >>"
            )
            .as_str()]);
            let catalog = document.get(ObjectId::new(1, 0));
            match read(&document, &catalog).as_slice() {
                [Action::GoToE(target)] => target.path.first().cloned(),
                other => panic!("one embedded go-to, got {other:?}"),
            }
        };

        assert_eq!(
            step("/P 2 /A 0"),
            Some(TargetStep::AttachedChild {
                page: AttachmentPage::Index(2),
                annotation: AttachmentIndex::Index(0)
            })
        );
        assert_eq!(
            step("/P (start) /A (the file)"),
            Some(TargetStep::AttachedChild {
                page: AttachmentPage::Named("start".to_owned()),
                annotation: AttachmentIndex::Name("the file".to_owned())
            })
        );
    }

    /// A path this reader cannot walk is named rather than performed, and each reason is its
    /// own: a root on a disk, a parent this file does not contain, a child it does not embed.
    #[test]
    fn a_target_this_file_does_not_contain_is_named() {
        let action = |entries: &str| {
            let document =
                document(&[format!("<< /Type /Action /S /GoToE /D (d) {entries} >>").as_str()]);
            let catalog = document.get(ObjectId::new(1, 0));
            let read = read(&document, &catalog);
            match read.as_slice() {
                [Action::GoToE(target)] => (target.clone(), document),
                other => panic!("one embedded go-to, got {other:?}"),
            }
        };

        let refusal = |entries: &str| {
            let (target, document) = action(entries);
            target.target_in(&document).err()
        };

        assert_eq!(
            refusal("/F (elsewhere.pdf) /T << /R /C /N (x) >>"),
            Some(TargetError::AnotherRoot("elsewhere.pdf".to_owned()))
        );
        assert_eq!(refusal("/T << /R /P >>"), Some(TargetError::NoParent));
        assert_eq!(
            refusal("/T << /R /C /N (absent.pdf) >>"),
            Some(TargetError::NoSuchChild("absent.pdf".to_owned()))
        );
        assert_eq!(refusal(""), Some(TargetError::NoPath));
    }
}
