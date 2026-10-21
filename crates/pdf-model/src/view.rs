//! What a *viewer* holds that a document does not: the state actions change.
//!
//! A [`crate::Interpretation`] of a document nobody has acted on is a function of a document and a
//! page, and that stops being true the moment §12.6.4's actions are performed (ADR 0065):
//! §12.6.4.13 sets the state of an optional content group, §12.6.4.11 sets an annotation's Hidden
//! flag, and both decide what the *next* render of the page draws. Neither is written back to the
//! file.
//!
//! So there is a third input, and this is it. [`ViewState::of`] builds the state a document
//! opens in — §8.11.4.5's initial configuration and no hidden annotations — and
//! [`ViewState::perform`] moves it. `crate::interpret` builds a fresh one per page, so
//! nothing that does not want this pays for it; `crate::interpret_with` takes one that has
//! been moved.
//!
//! # Why the state is not in the `Document`
//!
//! `pdf_syntax::Document` is what the file says. A layer a person switched off is not what
//! the file says, and putting it there would make two renders of one page differ for reasons
//! the file cannot explain — which is exactly the property that makes the oracle's comparison
//! meaningful. §8.11.4.5 draws the same line: the initial state is "the state used by all PDF
//! processors", and everything after it is one processor's.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::action::{EmbeddedGoTo, ImportData, Named, ThreadJump, Uri};
use crate::destination::Destination;
use crate::forms_data::Import;

mod actions;
mod annotations;
mod attachments;
mod field_tree;
mod fields;
mod markup;
mod optional_content;
mod pages;
mod script_annotations;
mod script_bookmarks;
mod script_model;
mod script_sites;
mod script_timers;
mod scripts;
mod settings;
mod writing;

use crate::optional_content::{Audience, OptionalContent, Purpose};
pub use annotations::{
    annotation_at, annotation_interacts, hover_changes_appearance, press_changes_appearance,
};
pub(crate) use field_tree::{Omitted, field_table, field_table_on_page, widgets_under};
pub use field_tree::{widgets_by_field_name, widgets_on_page_by_field_name};
pub(crate) use fields::alternative_name;
pub use fields::{field_at, is_file_select};
pub use script_model::{
    Alignment, AnnotationChange, AnnotationReach, AnnotationState, BookmarkState, BorderStyle,
    Colour, CommitKey, ConsoleCommand, ConsoleRequest, Display, DocumentState, DocumentTrigger,
    Face, FieldState, FieldType, FontName, Glyph, InfoEntry, Keys, Layer, MAX_BOOKMARKS, MAX_PAGES,
    PageState, Property, ScriptEdit, ScriptSite, Sound, TextFlag, ViewChange, WidgetState,
    WindowView, ZoomType,
};
pub use script_sites::ScriptsRan;
pub use script_timers::{MAX_TIMERS, MIN_PERIOD};
pub use scripts::{
    Committed, Displayed, FocusRequest, Resumed, ScriptEvent, ScriptResult, ScriptRunner,
};

/// Deepest nesting of `/Kids`, and longest `/Parent` chain, walked in §12.7.4.1's field tree.
///
/// The tree is one a document controls, and `/Kids` may point back up it, so every walk is
/// bounded — which §12.7.4.1 forbids ("\[a]n interactive PDF processor shall not limit the range of
/// inheritance for field dictionaries") and its ledger row records as departed. The bound is the
/// appearance construction's own, [`crate::appearance::MAX_FIELD_ANCESTRY`], because ADR 1198
/// found real fields thirty-two links deep: one number for one tree, so no walk here stops
/// silently short of where the construction reports.
const MAX_FIELD_DEPTH: usize = crate::appearance::MAX_FIELD_ANCESTRY;

/// The document state a viewer holds and §12.6.4's actions change.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// §8.11's layers, in the state the document opened in plus every change performed since.
    ///
    /// `None` for a document with no `/OCProperties`, which §8.11.4.2 makes decisive: without
    /// it "a PDF processor shall ignore any optional content structures in the document".
    optional_content: Option<OptionalContent>,
    /// Annotations §12.6.4.11 has hidden, by object identity.
    ///
    /// A set of *overrides* rather than a copy of every annotation's flags: Table 167's own
    /// Hidden bit is still read from the file, and this is what a hide action added on top.
    /// Table 214's `/H false` removes an entry rather than adding one, which is why showing an
    /// annotation the file itself marks Hidden does nothing — the action "hides or shows one
    /// or more annotations on the screen by setting or clearing their Hidden flags", and this
    /// program does not write to the file.
    hidden: BTreeSet<ObjectId>,
    /// Annotations §12.6.4.11 has shown, by object identity.
    ///
    /// The other half: `/H false` on an annotation whose own `/F` sets Hidden clears the flag
    /// for this viewing session. Two sets rather than a map from identity to boolean because the
    /// common case is that both are empty and neither allocates.
    shown: BTreeSet<ObjectId>,
    /// Widgets §12.7.6.3's reset-form action has reset, by object identity.
    ///
    /// Empty until a reset is performed, which is every document until somebody clicks. A set
    /// rather than a map of new values, because the clause makes the new value a property of
    /// the *field* — its `/DV`, or nothing — rather than of the action.
    reset: BTreeSet<ObjectId>,
    /// Widgets §12.7.8's imported form data has given a new value, by object identity.
    ///
    /// A map rather than a set, which is the whole difference between this and `reset`: a
    /// reset's new value is a property of the field — its own `/DV` — and an import's comes
    /// from another file, so it has to be carried. Kept disjoint from `reset` by construction,
    /// in [`ViewState::import`] and [`ViewState::reset_form`], because both answer the same
    /// question about one widget and the answer is whichever was performed last.
    imported: BTreeMap<ObjectId, Import>,
    /// Pages §12.7.8.3.3's imported templates have added, after the document's own.
    ///
    /// §12.7.7 says what naming a page is *for*: "\[a]n import-data action can add the named page
    /// to the document into which FDF is being imported". Adding is the one operation in this
    /// module that changes how many pages there are, and it belongs here for the same reason a
    /// hidden annotation does — the file says nothing about it, and a second render of the
    /// document without this state has the pages the file has.
    ///
    /// In the order the FDF file's `/Pages` array states.
    appended: Vec<AppendedPage>,
    /// §12.7.8's named pages whose Table 253 `/F` puts them in a **second file**, waiting for it.
    ///
    /// The entry is "\[t]he file containing the named page", and §12.7.6.4 makes a file a
    /// *document* named a question about this machine rather than about the format — so this
    /// crate has no way to open one and must not acquire one (ADR 1155). What it can do is say
    /// precisely what it is waiting for: [`ViewState::file_awaited`] names the file,
    /// [`ViewState::supply_named_pages`] applies every reference into it once a host has
    /// supplied the bytes, and [`ViewState::decline_named_pages`] says what went without. ADR
    /// 1239.
    ///
    /// Empty for every import whose references name pages of this document, which is every FDF
    /// file anybody has.
    awaiting: Vec<AwaitedPage>,
    /// How many second files have been supplied for §12.7.8's named pages, against
    /// `MAX_NAMED_PAGE_FILES`.
    supplied: usize,
    /// Widgets a *person* has typed a value into, by object identity.
    ///
    /// The fourth statement about a field's value and the only one that comes from outside the
    /// document altogether: Table 226's `/V` is the file's, §12.7.6.3's reset takes `/DV`,
    /// §12.7.8's import takes another file's, and this is what somebody entered. Kept here for
    /// the reason all three others are — `CLAUDE.md`'s rule 1 makes `pdf_syntax::Document`
    /// immutable, so an edit is a log beside the file and never a change to it, and
    /// interpretation stays a pure function of the bytes and this state.
    ///
    /// An entry whose [`Entry::value`] is `None` is a value a person *cleared*, which is a
    /// different thing from a widget nobody has touched: the first shows an empty field and the
    /// second shows the file's own `/V`.
    edited: BTreeMap<ObjectId, Entry>,
    /// The file a person chose for each of §12.7.5.3's file-select controls, by field name.
    ///
    /// Table 231 bit 21 makes the *value* a pathname — "the field's text represents the pathname
    /// of a file whose contents shall be submitted as the field's value" — so [`Self::edited`]
    /// carries that text like any other, and what this carries is the half of the sentence a
    /// pathname is not: the contents. They are kept beside the log rather than in it because
    /// [`Entry::value`] is an [`Object`] and a file's bytes are §7.11.4's *stream*, which only a
    /// document has a place for; `CLAUDE.md` rule 1 keeps the document immutable, so the bytes
    /// live here and §12.7.6.2's submission reads them from here (ADR 1216).
    ///
    /// Keyed by §12.7.4.2's fully qualified name, because that is what the value is keyed by and
    /// what a submission names a field with. A field whose value is set any other way loses its
    /// entry here, so a pathname and the contents behind it cannot disagree.
    chosen: BTreeMap<String, ChosenFile>,
    /// Which annotation the pointer is over or pressing, if any (§12.5.5).
    ///
    /// One annotation rather than a set, because a pointer is in one place. `None` means nothing
    /// is interacting with the user, so every annotation shows its normal appearance.
    pointer: Option<(ObjectId, Pointer)>,
    /// How large the page is being drawn, in logical pixels per default user space unit.
    ///
    /// The one thing in this struct that is a property of the *window* rather than of anything
    /// a person did to the document, and it is here because §12.5.3's `NoZoom` makes it decide
    /// a mark: an annotation with that flag "shall always maintain the same fixed size on the
    /// screen". Interpretation therefore has to know the magnification, and rule 1 says the
    /// only way it may is through this state.
    ///
    /// `None` is **not** 1.0 and the distinction is the whole point: it is *nobody has said*,
    /// which is what the corpus gate, the oracle and every caller of [`ViewState::of`] mean.
    /// Under it `NoZoom` changes nothing, so a page rendered at its own scale is the page it
    /// always was.
    ///
    /// [`ViewState::widget_appearances`] is the second such thing, for exactly the reason stated
    /// above (ADR 0245). Two is not a trend, and each is here because a clause makes
    /// interpretation depend on something only a window knows.
    magnification: Option<f32>,
    /// Whether §12.7's form widgets are drawn, or left to whoever asked for the page.
    ///
    /// The second thing in this struct that is a property of the *host* rather than of the
    /// document or of anything a person did to it, and it is here for `magnification`'s reason:
    /// it changes what is drawn, and rule 1 makes this state the only channel by which anything
    /// outside the file may.
    widget_appearances: WidgetAppearances,
    /// §8.11.4.4's answers about this processor: who is reading, and in what language.
    ///
    /// The third thing in this struct that is a property of the *host* rather than of the
    /// document or of anything a person did to it, and it is here for `magnification`'s reason:
    /// Table 100's `/User` and `/Language` categories decide whether a layer is drawn, and rule
    /// 1 makes this state the only channel by which anything outside the file may.
    ///
    /// [`Audience::NONE`] until a host says otherwise, under which both categories are reported
    /// unanswered and the configuration's own state stands. ADR 1106.
    audience: Audience,
    /// What the output being produced is for: §8.11.4.4's event, and §8.9.5.4 step c)'s question.
    ///
    /// The fourth thing in this struct that is a property of the *host* rather than of the
    /// document or of anything a person did to it, and it is here for `magnification`'s reason:
    /// Table 101's `Print` and `Export` events decide whether a layer is drawn and §8.9.5.4's
    /// step c) decides which of an image's variants is, and rule 1 makes this state the only
    /// channel by which anything outside the file may.
    ///
    /// [`Purpose::View`] until an operation says otherwise, which is what every caller that says
    /// nothing gets. ADR 1173.
    purpose: Purpose,
    /// §12.5.6.22's target media — the sheet the output is going onto, where whoever is
    /// producing it knows its dimensions.
    ///
    /// The fifth thing in this struct that is a property of the *host* rather than of the
    /// document, and it is here for `magnification`'s reason: Table 194 measures `/H` and `/V`
    /// "as a percentage of the width of the target media (or if unknown, the width of the page's
    /// MediaBox )", and rule 1 makes this state the only channel by which anything outside the
    /// file may decide a mark.
    ///
    /// `None` is the clause's own other branch rather than a default: Table 193 says that "\[i]f
    /// the dimensions of the target media are not known at the time of drawing, drawing shall be
    /// done relative to the dimensions specified by the page's MediaBox entry", which is what
    /// every caller that states nothing gets and what §12.5.6.22 requires of a screen outright.
    /// In default user space units, the same as the media box it stands in for. ADR 1179.
    #[expect(
        clippy::doc_markdown,
        reason = "verbatim quotations: Table 193 and Table 194 spell MediaBox without backticks"
    )]
    paper: Option<TargetMedia>,
    /// Whether §10.8.3's separation simulation is what the output being produced is to look like.
    ///
    /// The sixth thing in this struct that is a property of the *host* rather than of the
    /// document, and it is here for `magnification`'s reason: the clause conditions the whole
    /// simulation on something no file states —
    ///
    /// > If it is important for the colours of the display for a PDF, on a device that normally
    /// > would not be used to produce separations, to more closely match those produced when
    /// > using separations, then a simulation of the separation process can be performed for the
    /// > output to the non-separation device.
    ///
    /// — and rule 1 makes this state the only channel by which anything outside the file may
    /// decide a mark.
    ///
    /// **A preference and not one of `CLAUDE.md`'s four restriction levels**, on ADR 1189
    /// section 2's division: those four are for the permissions a *document* asserts over the
    /// person reading it, and no entry of any table asks for this. §12.11's
    /// `SeparationSimulation` requirement names the subclause, but a requirement states what a
    /// processor must be able to do rather than what this reader is being asked for now.
    ///
    /// `false` until a person says otherwise, under which §10.8.2's alternate colour space and
    /// its tint transform are what a `Separation` or `DeviceN` colour is drawn through — the
    /// behaviour that clause states as the expected one for a screen. ADR 1228.
    separation_simulation: bool,
    /// Annotations a person has **added**, in the order they added them.
    ///
    /// The fifth thing in this struct that comes from outside the document, and the first that
    /// is not a change to something the file already holds: `CLAUDE.md` permits exactly this —
    /// what a *user* does to an open document is not authoring, and §7.5.6's incremental update
    /// is how it goes back into the file with the producer's bytes untouched underneath.
    ///
    /// Each carries the object number it will be written under, allocated when it is added so
    /// that it has an identity for as long as the document is open — which is what the pointer,
    /// a later edit and the writer all need to name it by.
    added: Vec<Added>,
    /// What a person retyped into a free text annotation or a text note **the file itself
    /// states**.
    ///
    /// The sixth thing in this struct that comes from outside the document, and the only one that
    /// contradicts something the file already says: `added` puts a new object beside the
    /// producer's, and this replaces the `/Contents` of one of theirs. §7.5.6 is what makes that
    /// the same kind of writing rather than a different one — an update states "this object now
    /// reads like this" and leaves the bytes it read like before untouched underneath — which is
    /// why `CLAUDE.md`'s exclusion covers it: what a *user* does to a document already open is not
    /// authoring.
    ///
    /// Keyed by the annotation's object, which is its identity in the file and needs no allocating.
    /// An entry whose string is empty is a person who **cleared** the note, which is a different
    /// thing from never having touched it: the first draws nothing and the second draws Table
    /// 166's `/Contents`.
    retyped: BTreeMap<ObjectId, String>,
    /// §7.11.4's embedded files a person **attached** this viewing session, in the order they did.
    ///
    /// The seventh thing in this struct that comes from outside the document, and the second
    /// that adds objects rather than changing one: a file put into the document is not authoring
    /// any more than an annotation is, and it leaves through the same §7.5.6 update. Each carries
    /// the object numbers its stream and its specification will be written under, allocated when
    /// it was attached for [`Self::added`]'s reason — and where the home is a page, the annotation
    /// that files it is an entry of `added`, so the icon is drawn and written by the code every
    /// other added annotation takes. ADR 0814.
    filed: Vec<Filed>,
    /// The `/EmbeddedFiles` keys of the file's own entries a person **detached** this viewing
    /// session.
    ///
    /// Keys rather than objects, because a name tree files by key (§7.7.4) and what a person
    /// pointed at was a row named by one. What the entry alone reached is marked free when the
    /// save writes the tree without it — §7.5.6's "shall be marked as deleted by means of their
    /// cross-reference entries" — and what another home still reaches is left in use and said.
    unfiled: Vec<Vec<u8>>,
    /// What a host says the time is, for Table 166's `/M` on what a save writes.
    ///
    /// The fourth thing in this struct that is a property of the *host* rather than of the
    /// document, and the only one that changes no pixel: it decides what goes into the file
    /// rather than what is drawn. `CLAUDE.md`'s rule 3 gives this crate no clock, and Table 166
    /// asks for "\[t]he date and time when the annotation was most recently modified" — a fact
    /// about a machine, which nothing here can read and which is therefore supplied or absent.
    ///
    /// `None` until a host says otherwise, under which no `/M` is written at all. ADR 1160.
    modified: Option<pdf_syntax::Date>,
    /// How many object numbers this state has handed out since it last held nothing added.
    ///
    /// One counter for annotations and attachments alike, and it only ever grows while anything
    /// allocated is still held: an attachment detached again and an annotation added after it
    /// would otherwise be given the same number, and the log names annotations by number.
    allocated: u32,
    /// Fields a person is typing into and has not committed, by §12.7.4.2's fully qualified name,
    /// with what each of the field's widgets showed before the typing began.
    ///
    /// Table 199's `/K` runs per character and its `/V` "when the field's value is changed"; a
    /// host that sends a whole value per keystroke has changed the value character by character
    /// and committed it once, and the commit is what [`ViewState::commit_field`] marks. What is
    /// kept is what a refused commit puts back (ADR 1579).
    uncommitted: BTreeMap<String, Vec<(ObjectId, scripts::Before)>>,
    /// What Tier 0's dispatch did not run, each sentence once, in the order it was met.
    ///
    /// RFC 0008 section 6.8's rule for a failing script — reported once per document per site,
    /// never a storm — which a keystroke script raised per character would otherwise break.
    script_reports: Vec<String>,
    /// What runs a field script Tier 0 does not, where a host has supplied one.
    ///
    /// RFC 0008 section 6.3's policy hook: absent is the level `off`, and nothing a host has not
    /// supplied runs (ADR 1591).
    runner: scripts::Runner,
    /// What scripts keep beside the edit log: the properties they set, what each runner-run format
    /// displayed, and what the runner's realm was last told of (ADRs 1602, 1603).
    scripting: scripts::Scripting,
}

/// One annotation a person added, and the page it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    /// The object it will be written under, and its identity while the document is open.
    pub id: ObjectId,
    /// The page it belongs to, which is what `Page::id` answers.
    pub page: ObjectId,
    /// The annotation dictionary, whole, as §7.5.6 will write it.
    pub dict: Dictionary,
}

/// §7.11.4's embedded file a person is putting into the document, and where.
///
/// What `viewer-core`'s edit log resolves an attach to, and what a replay hands back to
/// [`ViewState::attach`]: the bytes shared rather than copied, because a log entry, an undo and
/// a redo all hold the same file.
#[derive(Debug, Clone, PartialEq)]
pub struct Filing {
    /// The file.
    pub bytes: crate::attachment::filing::Payload,
    /// The name it is filed under — §7.7.4's key in the tree, and Table 43's `/F` and `/UF`
    /// either way, which Table 32 says the key "should match".
    pub name: String,
    /// Table 43's `/Desc`, and Table 166's `/Contents` where the home is a page.
    pub description: Option<String>,
    /// Table 44's `/Subtype`, the media type, where the person's program knows one.
    pub media_type: Option<String>,
    /// Which of §7.11.4.1's homes.
    pub home: FilingHome,
}

/// Which of §7.11.4.1's two homes a file goes into.
///
/// > An embedded file stream shall be included in a PDF file in one of the following ways:
///
/// — a file specification's `/EF`, which "shall be used for file attachment annotations …
/// which associate the embedded file with a location on a page in the document", or the
/// `EmbeddedFiles` entry of the name dictionary, which associates it "with the document as a
/// whole".
#[derive(Debug, Clone, PartialEq)]
pub enum FilingHome {
    /// §7.7.4's `/EmbeddedFiles` tree: the document as a whole.
    Document,
    /// §12.5.6.15's annotation on a page, at a rectangle in the page's default user space.
    Page {
        /// The page, which is `Page::id`.
        page: ObjectId,
        /// Table 166's `/Rect`.
        rect: [f32; 4],
    },
}

/// One file a person attached this viewing session, with the numbers it will be written under.
#[derive(Debug, Clone, PartialEq)]
pub struct Filed {
    /// What was attached, and where.
    pub filing: Filing,
    /// §7.7.4's key: [`Filing::name`] as §7.9.2's text string, which is what the tree compares.
    pub key: Vec<u8>,
    /// The object §7.11.4's stream will be written under.
    pub stream: ObjectId,
    /// The object §7.11.3's file specification will be written under.
    pub specification: ObjectId,
    /// The annotation filing it, where the home is a page — an entry of the additions.
    pub annotation: Option<ObjectId>,
}

/// Why [`ViewState::attach`] declined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AttachRefusal {
    /// The document, or this viewing session, already files something under this name.
    ///
    /// §7.9.6: a name tree's keys "shall not overlap", and a second file under one key would be
    /// either a replacement nobody asked for or a tree the clause forbids. One namespace for both
    /// homes, so that a detach by name is never ambiguous.
    #[error("a file is already embedded under this name (ISO 32000-2 §7.9.6)")]
    NameTaken,
}

/// What [`ViewState::detach`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detached {
    /// A file this session attached is forgotten, together with its annotation where it had one.
    Filed,
    /// A file the document's own tree names will be left out of the tree the next save writes.
    Unfiled,
    /// Nothing is embedded under this name in the tree or in this viewing session, so nothing
    /// happened.
    ///
    /// A file §12.5.6.15's annotation carries in the *file's* own `/Annots` is among these:
    /// taking it out would be deleting the producer's annotation, which is `doc/todo/33`'s and
    /// not this verb's.
    Nothing,
}

/// Which of §12.5.6.10's four text markup annotations to add.
///
/// The clause's own list, and the four are one construction with four shapes:
///
/// > Text markup annotations shall appear as highlights, underlines, strikeouts (all PDF 1.3),
/// > or jagged ("squiggly") underlines ( PDF 1.4 ) in the text of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Markup {
    /// `Highlight`, drawn as a wash under `Multiply` so the text stays readable.
    Highlight,
    /// `Underline`, a bar on the bottom edge.
    Underline,
    /// `StrikeOut`, a bar across the middle.
    StrikeOut,
    /// `Squiggly`, a jagged underline.
    Squiggly,
}

impl Markup {
    /// Table 182's `/Subtype` for this markup, which the table requires to be one of
    /// `Highlight`, `Underline`, `Squiggly` or `StrikeOut`.
    const fn subtype(self) -> &'static [u8] {
        match self {
            Self::Highlight => b"Highlight",
            Self::Underline => b"Underline",
            Self::StrikeOut => b"StrikeOut",
            Self::Squiggly => b"Squiggly",
        }
    }
}

/// What the pointer is doing to an annotation, in §12.5.5's terms.
///
/// The clause states the three appearances as three situations rather than as a mode:
///
/// > The normal appearance shall be used when the annotation is not interacting with the
/// > user. … The rollover appearance shall be used when the user moves the cursor into the
/// > annotation's active area without pressing the mouse button. … The down appearance shall
/// > be used when the mouse button is pressed or held down within the annotation's active
/// > area.
///
/// NOTE 2 is worth carrying: "the term mouse denotes a generic pointing device that controls
/// the location of a cursor on the screen and has at least one button".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pointer {
    /// The cursor is inside the annotation's active area with no button pressed.
    Over,
    /// A button is pressed or held down inside it.
    Down,
}

/// What performing an action asks of the viewer that a [`ViewState`] cannot do itself.
///
/// The division is the same one this module exists for. A layer's state and an annotation's
/// Hidden flag are *this document's* state and live here; which page is on screen and what a
/// URI opens are the window's, and no part of them is a fact about the file.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// §12.6.4.2: display this destination. [`Destination::page_index`] turns it into a page.
    Display(Destination),
    /// §12.6.4.12: move as Table 215's command says, from whichever page is being shown.
    Page(Named),
    /// §12.6.4.8: resolve this URI, which means opening it somewhere this program is not.
    Resolve(Uri),
    /// §12.6.4.7: jump to a bead on an article thread.
    ///
    /// Unresolved for [`Self::Display`]'s reason one level further out: a destination needs the
    /// page tree and this needs [`crate::article::Articles`], and neither is part of the state a
    /// click changes. [`crate::action::ThreadJump::bead_in`] turns it into a bead.
    Thread(ThreadJump),
    /// §12.6.4.15: draw the page as it now stands, using this transition.
    ///
    /// §12.6.4.15 is explicit that this is about *when* drawing happens: a processor "shall
    /// normally suspend drawing when such a sequence begins and resume drawing when it ends", and
    /// a transition action in the middle of one says to show the page as the previous action left
    /// it. Suspending and resuming is a window's business — this state has no screen — so the
    /// transition is handed over and the caller decides whether it has one to play.
    Transition(crate::navigation::Transition),
    /// §12.6.4.5: show the page a document part begins at.
    ///
    /// Unresolved for [`Self::Display`]'s reason: finding the page needs the page tree, which is
    /// not part of the state a click changes.
    /// [`crate::action::DocumentPartJump::page_in`] turns it into a page.
    DocumentPart(crate::action::DocumentPartJump),
    /// §12.6.4.4: show a destination in a document embedded in this one.
    ///
    /// Unresolved for [`Self::Display`]'s reason, twice over: the target document has to be
    /// *opened* before its destination means anything, and which document is on screen is the
    /// window's business rather than this state's.
    /// [`crate::action::EmbeddedGoTo::target_in`] opens it.
    Embedded(EmbeddedGoTo),
    /// §12.6.4.3: show a destination in the file Table 203's `/F` names.
    ///
    /// Unresolved for [`Self::Embedded`]'s reason and one more: the document is not in this file
    /// at all, so a party with a filesystem has to supply it before there is anything to read a
    /// destination in. [`crate::action::RemoteGoTo::page_in`] is where it is read once there is.
    /// ADR 1227.
    Remote(crate::action::RemoteGoTo),
    /// §12.7.6.2: submit the form to the URL Table 239 names, which means transmitting it.
    ///
    /// The same division as [`Self::Resolve`] and for the same reason, one step further on:
    /// §12.7.6.2's `shall` is to "transmit the names and values of selected interactive form
    /// fields to a specified uniform resource locator (URL)", and the transmission is a network
    /// request no crate under principle 3's sandbox performs. *Which* names and values, in
    /// *which* format, to *which* URL by *which* method are all functions of the document and of
    /// this state, so a caller composes them with [`crate::submission::compose`] — which needs
    /// the click Table 240 bit 5 asks for, and this function has none — and hands the composed
    /// request to whoever has a network (ADR 1062).
    Submit(crate::action::SubmitForm),
    /// §12.7.6.4: import this file's form data, which means finding and reading it.
    ///
    /// The same division as [`Self::Resolve`], and for the same reason: a document naming a file
    /// is a document asking this machine for something, and whether to give it is not a
    /// rendering decision. A caller that has the bytes hands them to
    /// [`crate::forms_data::FormsData::read`] and then to [`ViewState::import`], which is where
    /// the values become ink.
    Import(ImportData),
    /// §12.6.4.14: a rendition is to be played, stopped, paused or resumed.
    ///
    /// The same division as [`Self::Resolve`]: judging the rendition needs what a host has said
    /// about its reader (`crate::media::Processor`), and handing the clip on is a decision about
    /// this machine. Neither is part of the state a click changes. ADR 1960.
    Rendition(crate::action::Rendition),
}

/// Which statement about a field's value a widget is currently showing.
///
/// Three, and each is a different clause saying where a value comes from. The default is the
/// file's own, which is every widget in every document until something is performed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FieldValue<'a> {
    /// Table 226's `/V`, read up §12.7.4.1's `/Parent` chain.
    #[default]
    Stored,
    /// §12.7.6.3: `/DV` instead, because a reset-form action named this widget.
    Default,
    /// §12.7.4: a value a person entered, which replaces every other statement about it.
    ///
    /// Last because it is latest: the file's `/V`, a reset's `/DV` and an import's value are all
    /// statements made before somebody typed. A `value` of `None` is a field a person *cleared*,
    /// which shows nothing — a different thing from [`Self::Stored`], which shows the file's own
    /// value.
    Edited {
        /// Table 226's `/V` as the edit leaves it, already in the object type the entry takes.
        ///
        /// A string for text and for §12.7.5.2's appearance-state names, and **an array of
        /// strings** where §12.7.5.4's choice field has several items selected — the clause's own
        /// two shapes, resolved once by [`ViewState::set_field`] so that the appearance and the
        /// file cannot disagree about what was entered.
        value: Option<&'a Object>,
        /// Table 234's `/I`, where the edit chose among §12.7.5.4's options.
        ///
        /// Zero-based indices into `/Opt`, ascending. `None` is an edit that named no options —
        /// text typed into a field, a button's state, or a clear — and it is what says the entry
        /// must not survive into the file beside the new `/V`.
        indices: Option<&'a [usize]>,
    },
    /// §12.7.8: a value from an FDF file, which replaces `/V` (§12.7.8.3.2).
    Imported {
        /// Table 249's `/V`.
        ///
        /// `None` is an FDF field that states no `/V` at all, which "replace" makes a field
        /// whose value is *removed* — the same state a reset leaves a field with no `/DV` in,
        /// and drawn the same way. It is a different thing from [`Self::Stored`], which is a
        /// widget nothing has imported into.
        value: Option<&'a Object>,
        /// Table 249's `/Ff`, `/SetFf` and `/ClrFf` over Table 227's field flags.
        ///
        /// Carried with the value because the flags decide how the value is *drawn*: Table 231's
        /// multiline, comb and password bits each change §12.7.4.3's layout of the same string.
        flags: crate::forms_data::FlagChange,
        /// Table 249's `/RV`, the rich text string the import states beside the value.
        ///
        /// `None` where the FDF field states none, which leaves the field's own `/RV` standing:
        /// §12.7.8.3.2 replaces the entries the FDF field states (ADR 1648).
        rich: Option<&'a str>,
    },
}

/// What a person put into one field, as [`ViewState::set_field`] takes it.
///
/// Three variants, because §12.7.5 gives a field's value three shapes and a host has to be able to
/// say all three, and one `Option<String>` is not enough for the third: Table 233 bit 22 — *"(PDF
/// 1.4) If set, more than one of the field's option items may be selected simultaneously; if clear,
/// at most one item shall be selected"* — lets §12.7.5.4's list box hold several items at once, and
/// one string cannot say which several. ADR 0248.
///
/// **A selection is named by index and not by label**, which is the decision worth stating. The
/// clause makes `/V` hold the *labels* — a two-element `/Opt` entry's second element is the one
/// `/V` names — so labels are what reaches the file; but two of Table 234's `/Opt` entries may
/// carry the same label, and a host that answered with labels could not say which of them a
/// person clicked.
/// [`crate::form::ChoiceControl::selected`] already answers in indices, so a host reads and writes
/// the same coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Entered {
    /// The field is cleared: §12.7.6.3's own words for a value that is gone are "its V entry shall
    /// be removed", and this is what a person emptying a field asks for.
    ///
    /// A different state from never having touched the field, which shows Table 226's `/V`.
    #[default]
    Cleared,
    /// Characters, as §7.9.2.2's text string.
    ///
    /// §12.7.5.3's text field, §12.7.5.4's editable combo box — Table 233 bit 19 lets one "include
    /// an editable text box as well as a drop-down list", so its value need not be one of the
    /// options at all — and §12.7.5.2's two toggling buttons, whose value is the appearance-state
    /// name [`crate::form::Widget::on_state`] hands over.
    Text(String),
    /// §12.7.5.4: which of Table 234's `/Opt` entries are selected, as zero-based indices into it.
    ///
    /// An empty list means no item is selected, which §12.7.5.4 makes the same state as
    /// [`Self::Cleared`]: "\[t]he default value of V is null , indicating that no item is currently
    /// selected."
    ///
    /// An index past the end of `/Opt` names nothing and is dropped, and more than one index on a
    /// field whose `MultiSelect` flag is clear is cut to the first — Table 233 bit 22's "at most
    /// one item shall be selected" binds this program because this program is what selects.
    Chosen(Vec<usize>),
}

/// A file a person chose for §12.7.5.3's file-select control, with the bytes behind the pathname.
///
/// Table 231 bit 21 states the whole of what one is:
///
/// > If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select control.
/// > In this case, the field's text represents the pathname of a file whose contents shall be
/// > submitted as the field's value
///
/// — two things, the pathname and the contents, and the second is what no process this crate runs
/// in can fetch: `CLAUDE.md` principle 3 gives it no filesystem. So the contents are supplied from
/// outside by whoever ran a file chooser, and the pathname is what the field then draws. ADR 1216.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenFile {
    /// The pathname the field's text becomes, as the person's own platform spells it.
    ///
    /// §7.11.2's platform-independent spelling of a path is knowledge of a filesystem this crate
    /// has none of, so the name is carried as it was given. A choice, recorded — and the same one
    /// `submission::compose` already makes for a pathname a document states.
    pub pathname: String,
    /// The media type to submit the contents under, where the chooser knew one.
    ///
    /// HTML 4.01 section 17.13.4.2 asks for one and names the fallback, which
    /// `submission::compose` applies: `application/octet-stream`.
    pub media_type: Option<String>,
    /// The file's contents, which is the half of Table 231 bit 21 a pathname is not.
    pub bytes: Vec<u8>,
}

/// One widget's edit, resolved against the document into what Table 226's `/V` will say.
///
/// Resolved once, at [`ViewState::set_field`], rather than at each of the three places that read it
/// — the appearance, the description a host reads back, and the file a save writes. §12.7.5.4's
/// mapping from an index to a label needs the field's `/Opt`, and doing it three times would be
/// three chances for a picture and a file to disagree about what a person chose.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    /// Table 226's `/V`, or `None` for a field that was cleared.
    value: Option<Object>,
    /// Table 234's `/I`, where the edit chose among §12.7.5.4's options.
    indices: Option<Vec<usize>>,
}

/// What one import of one FDF file did to this document.
///
/// Every field here is something a caller should be able to say out loud, and none of them is an
/// error. §12.7.8.3.2 matches by fully qualified name, so a name the form has not got is either
/// the wrong FDF for this document or a form that has changed since the data was exported — and
/// only somebody who can see both files can say which.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Imported {
    /// How many widget annotations took a value.
    pub widgets: usize,
    /// Fully qualified names the FDF file states that this document has no field for.
    pub unmatched: Vec<String>,
    /// How many §12.7.7 template pages were added to the document.
    pub pages: usize,
    /// How many §12.7.8.3.4 annotations were placed on the pages Table 254's `/Page` names.
    pub annotations: usize,
    /// What the FDF file asked for and this document did not do, each with the reason.
    ///
    /// Templates it could not add and `/APRef` appearances it could not make (ADRs 1070, 1235).
    pub refused: Vec<String>,
    /// What the import is **waiting** for, one sentence each: Table 253's `/F`.
    ///
    /// Not a refusal and deliberately a separate list. A page in a second file is a question for
    /// a host rather than something this import declined, and [`ViewState::file_awaited`] is the
    /// file to ask for; a caller with no filesystem turns each of these into a refusal by
    /// declining, which [`ViewState::decline_named_pages`] words. ADR 1239.
    pub awaiting: Vec<String>,
}

/// A page §12.7.8.3.3's import has added to the document.
///
/// Two spellings because Table 253 states two, and the entry that divides them is `/F`: without
/// it "it shall be assumed that the page resides in the associated PDF file", so the page is an
/// object of the document being read; with it the page is in a second file and crosses as a copy
/// that names nothing of it (ADRs 1223, 1239).
#[derive(Debug, Clone, PartialEq)]
pub enum AppendedPage {
    /// A page this document already holds, reached through one of §12.7.7's two name trees.
    Named(ObjectId),
    /// A page of a second file, copied whole.
    ///
    /// Every entry is the other producer's own or §7.7.3.4's inheritance made explicit — its
    /// `/Contents` are §7.8.2's concatenation of that page's own, its `/Resources` are the ones
    /// in effect there, and its boxes and `/Rotate` are what that page states. Nothing is
    /// composed and no mark is this program's, which is `CLAUDE.md`'s provenance test; what is
    /// dropped is `/Parent`, because §7.7.3.4's inheritance runs up a tree that is in the other
    /// file.
    Carried(Dictionary),
}

/// One §12.7.8 named page that Table 253's `/F` puts in a second file.
///
/// > The file containing the named page. If this entry is absent, it shall be assumed that the
/// > page resides in the associated PDF file.
///
/// Held rather than refused since ADR 1239: the bytes are a host's to supply, so what this crate
/// owes is to say which file, which page, and what the page was wanted for.
#[derive(Debug, Clone, PartialEq)]
pub struct AwaitedPage {
    /// Table 253's `/F`, as the FDF file wrote it. Not a path: resolving it is a host's.
    pub file: String,
    /// Table 253's `/Name`, "\[t]he name of the referenced page", looked up in that file.
    pub name: String,
    /// What the page becomes once the file arrives.
    pub wanted: WantedPage,
}

/// Which of §12.7.8's two entries is waiting for a named page.
#[derive(Debug, Clone, PartialEq)]
pub enum WantedPage {
    /// Table 249's `/APRef`: the page becomes this widget's appearance in this state.
    Appearance {
        /// The widget annotation whose appearance it is.
        widget: ObjectId,
        /// Which of Table 170's three — `N`, `R` or `D`.
        state: &'static str,
    },
    /// Table 252's `/TRef`: the page is added to the document, with the template's own fields.
    Template {
        /// Table 252's `/Fields`, applied with the page for `ViewState::append_templates`'s
        /// reason.
        fields: Vec<crate::forms_data::FdfField>,
        /// Table 252's `/Rename`, whose `true` is refused where the template states fields.
        rename: bool,
    },
}

/// Everything this state contributes to one annotation's decision, gathered in one walk.
///
/// A struct rather than a list of arguments because they are asked together, once per annotation
/// per page, and because each defaults to "the file's own answer" — which is what [`Default`]
/// here means and what every annotation in a document nothing has interacted with gets, drawn
/// for a reader looking at the screen.
///
/// Two of them are facts about the *output* rather than about the annotation, and they are here
/// because §12.5.3 and §12.5.6.22 make them this annotation's own questions; see `purpose` and
/// `paper`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AnnotationView<'a> {
    /// §12.6.4.11: `Some(true)` where a hide action hid this annotation, `Some(false)` where one
    /// showed it, `None` where none named it.
    pub hidden_by_action: Option<bool>,
    /// Which of Table 170's three appearances §12.5.5 asks for, given where the pointer is.
    pub appearance: Appearance,
    /// Where this widget's value comes from.
    pub value: FieldValue<'a>,
    /// §12.7.8's `/F`, `/SetF` and `/ClrF`, where an FDF file stated one of them.
    ///
    /// `None` is not "no flags": it is *this state has nothing to say*, and the annotation's own
    /// `/F` stands unchanged. The change is carried rather than the result because Table 249's
    /// two modifying entries are defined *against* the flags the document states, which only the
    /// reader of that dictionary has.
    pub flags: Option<crate::forms_data::FlagChange>,
    /// §12.7.8's `/IF`, where an FDF file stated an icon fit dictionary for this field.
    ///
    /// `None` is *this state has nothing to say*, so Table 192's own `/IF` in the widget's `/MK`
    /// stands. Table 249 and Table 192 name the same Table 250 dictionary, so an imported one
    /// replaces the widget's under §12.7.8.3.2's replacing sentence. ADR 1186.
    pub icon_fit: Option<&'a Dictionary>,
    /// §12.7.8's `/AP`, where an FDF file stated a push-button's appearance for this field.
    ///
    /// `None` is *this state has nothing to say*, so Table 170's own `/AP` in the widget stands.
    /// Table 249 makes the two the same dictionary — its `/AP` is "as shown in "Table 170 -
    /// Entries in an appearance dictionary"" bar its values being streams — so an imported one
    /// replaces the widget's under §12.7.8.3.2's replacing sentence, whole rather than per state.
    ///
    /// Every stream in it is a **copy** made when the FDF file was read, so it names no object of
    /// that file and `crate::annotation::stored_appearance` reads it as it reads any other.
    /// ADR 1223.
    pub imported_appearance: Option<&'a Dictionary>,
    /// What a person retyped into §12.5.6.6's annotation, where they retyped one.
    ///
    /// `None` is *this state has nothing to say*, so Table 166's `/Contents` and Table 177's `/RC`
    /// are read as the file states them. `Some("")` is a note whose text a person **took out**,
    /// which draws nothing — the same distinction [`FieldValue`] draws between a field nobody
    /// touched and one somebody cleared, and for the same reason.
    ///
    /// It is a statement about the *text*, and the appearance follows from it rather than beside
    /// it: §12.5.6.6 has this subtype's appearance generated from these entries by §12.7.4.3, so a
    /// stored `/AP` describing text the annotation no longer has is set aside by
    /// `crate::annotation::decide` where this is `Some`.
    pub contents: Option<&'a str>,
    /// What the output being produced is for — [`ViewState::purpose`].
    ///
    /// **A fact about the output rather than about this annotation, and it is here because
    /// §12.5.3 makes it one of this annotation's own questions**: Table 167's bit 3 decides
    /// whether an annotation is printed and its bit 6 decides whether one is rendered on a
    /// screen, so which of the two rows applies cannot be read out of the annotation dictionary
    /// at all. `crate::annotation::displayed` is where the table's two readings are.
    pub purpose: Purpose,
    /// §12.5.6.22's target media — [`ViewState::paper`].
    ///
    /// Carried for the reason `purpose` is: Table 194's `/H` and `/V` are percentages of the
    /// target media's dimensions, and no dictionary in the file states what sheet an operation
    /// chose. Read only under [`Purpose::Print`]; `None` is Table 193's *not known*, under which
    /// the page's own media box stands in.
    pub paper: Option<TargetMedia>,
    /// Whether this widget's field holds a value a person is typing and has not committed.
    ///
    /// Table 199's `/F` is performed "before the field is formatted to display its value", and a
    /// value still being typed is not yet the one the field displays: Adobe's event model — the
    /// *JavaScript for Acrobat API Reference*'s "Form event processing" — formats after the commit,
    /// and a layout of the formatted text would put a host's caret beside characters it is not
    /// editing. So a field between [`ViewState::set_field`] and [`ViewState::commit_field`] is laid
    /// out as typed, and every other value as its format script writes it (ADR 1579).
    pub editing: bool,
    /// What a host-supplied runner's format script displayed for this widget's value, where one
    /// ran: the appearance draws it where Tier 0 does not run the format itself (ADR 1603).
    pub displayed: Option<&'a Displayed>,
    /// The properties scripts set on this widget's field that its appearance draws — `textColor`,
    /// `fillColor`, `strokeColor`, `borderStyle`, `alignment`, `charLimit` — and `required`, which
    /// a save writes: the latest of each, empty where no script set one (ADR 1617).
    pub scripted: &'a [Property],
}

/// §12.5.6.22's target media, and how the page is placed on it.
///
/// **Two numbers rather than one, because the clause's last sentence has two terms.** After the
/// EXAMPLE it says that "given a matrix B that maps a scaled and rotated page into the default
/// user space, a new matrix shall be computed that cancels out B and translates the origin of the
/// media (e.g., printed page) to the origin of the default user space" — so what a caller states
/// is where the media is *and* what B does to the page, and a watermark is drawn immune to the
/// second.
///
/// [`Self::media`] carries the translation because it is stated in the page's own space, and
/// [`Self::page_scale`] is what is left of B. ADR 1204.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetMedia {
    /// The media rectangle **in the page's own default user space**.
    ///
    /// A rectangle rather than a width and a height: Table 194 measures `/H` and `/V` from the
    /// media's own corner, and a page placed anywhere but at that corner has a media rectangle
    /// whose lower-left coordinates are negative in its own space. That is the translation term
    /// of the sentence above, carried by the value rather than computed from it.
    pub media: [f32; 4],
    /// What B scales the page by when it is placed on that media.
    ///
    /// `1.0` is a page printed at its own size, which is the clause's "usual case where the PDF
    /// page size equals the media size" and every caller this program had before scale modes
    /// existed. A page shrunk to fit the paper states the factor it was shrunk by, and an n-up
    /// composition states the factor its cell imposes — which is the whole of what the two
    /// bullets after the EXAMPLE need, because both ask for the watermark "at the specified
    /// size" on a page that is no longer at its own.
    ///
    /// Cancelling B is a *division*: the watermark's own rectangle is divided by this so that
    /// the page's placement scales it back up to the size the media was measured in.
    pub page_scale: f32,
}

impl TargetMedia {
    /// A page placed on the media at its own size, with its corner at the media's.
    ///
    /// The clause's usual case, and the one a caller that knows a sheet and nothing else has.
    #[must_use]
    pub const fn unplaced(media: [f32; 4]) -> Self {
        Self {
            media,
            page_scale: 1.0,
        }
    }
}

/// Which of Table 170's appearances an annotation shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    /// `/N`, the only entry Table 170 requires, and the one a printer uses.
    #[default]
    Normal,
    /// `/R`, the rollover appearance.
    Rollover,
    /// `/D`, the down appearance.
    Down,
}

/// Who draws §12.7's form fields: this crate, or whoever asked for the page.
///
/// A native host places a real `GtkEntry`, `QLineEdit` or `NSTextField` over the page at each
/// widget's own rectangle, which is what [`crate::form`] describes a form *for*. Unless it can
/// also ask for the page **without** the widgets' own pictures, a person sees every field twice:
/// the control, and the appearance stream underneath it. ADR 0244 measured that at 76 controls
/// over one corpus document's 67 fields, every one sitting on the picture of itself.
///
/// # This is an instruction, not a departure
///
/// §6.3.2.2 places the obligation on a rendering processor and states its own exception in the
/// same sentence:
///
/// > A PDF processor shall also render the appropriate appearance stream for all annotations
/// > (12.5.5, "Appearance streams") which have appearance streams designated for this purpose as
/// > indicated by the annotation flags (see 12.5.3, "Annotation flags"), unless otherwise
/// > instructed.
///
/// [`Self::Delegated`] is that instruction, and it can only come from a host that has undertaken
/// to draw those appearances itself. It is never this crate's own choice: [`ViewState::of`] is
/// [`Self::Drawn`], so a caller that does not ask gets the page §6.3.2.2 describes.
///
/// # Why the widgets and nothing else
///
/// §12.5.6.19 is what makes them separable from everything else clause 12 puts on a page:
///
/// > Interactive forms (see 12.7, "Forms") use widget annotations (PDF 1.2) to represent the
/// > appearance of fields and to manage user interactions.
///
/// A widget annotation **is** a field's appearance, so a host that draws the field has replaced
/// exactly it. Nothing else has a counterpart in a toolkit — §12.5.6.10's markups, §12.5.6.4's
/// icons and §12.5.6.12's stamps are page content and stay on the page — and §12.5.1 draws the
/// same line from the standard's side, where Table 31's `/Tabs` has a value for the widgets alone:
///
/// > W (widgets order): Widget annotations shall be visited in the order in which they appear in
/// > the page Annots array, followed by other annotation types in row order.
///
/// So "leave out the widgets" and "leave out the annotations" are different requests, and only
/// the first is one a form host has any business making.
///
/// # And only the widgets a host was told about
///
/// Narrower than `/Subtype /Widget`, deliberately: §12.7.4.2 leaves a dictionary whose `/Parent`
/// chain never reaches the form "simply a Widget annotation", [`crate::form::fields`] answers
/// nothing for it, no host was handed a control for it, and dropping its appearance would take
/// ink off the page that nothing replaces. [`crate::form::delegated_widgets`] is the set, built
/// from that same call so the two cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetAppearances {
    /// This crate draws them, which is what §6.3.2.2 asks of a processor nobody has instructed.
    #[default]
    Drawn,
    /// The host draws them, so the page is interpreted without them.
    Delegated,
}

/// What a change of magnification obliges a caller to do.
///
/// Two clauses read the magnification and they are owed different things, which is why this is
/// not a `bool`. §12.5.3's `NoZoom` moves an annotation *without changing what the page draws*,
/// so a host holding a picture of the old magnification may go on standing in with it while the
/// new interpretation runs. §8.11.4.5's reapplication can switch a layer off, and a picture
/// holding a layer the document now says is not there is a picture of something else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magnified {
    /// The magnification is what it already was.
    Unchanged,
    /// The page is interpreted again, and the ink it already produced still stands.
    Placement,
    /// The page is interpreted again and what it drew is superseded: §8.11.4.5's reapplication
    /// moved an optional content group.
    Visibility,
}

impl Magnified {
    /// Whether the page has to be interpreted again at all.
    #[must_use]
    pub fn needs_interpreting(self) -> bool {
        !matches!(self, Self::Unchanged)
    }

    /// Whether what the page already drew is superseded rather than merely re-placed.
    #[must_use]
    pub fn supersedes_ink(self) -> bool {
        matches!(self, Self::Visibility)
    }
}

impl ViewState {
    /// The state a document opens in.
    ///
    /// §8.11.4.5's initial configuration — "\[t]his state shall be the initial state used by
    /// all PDF processors" — and nothing hidden beyond what each annotation's own `/F` says.
    #[must_use]
    pub fn of(document: &Document) -> Self {
        Self {
            optional_content: OptionalContent::read(document),
            hidden: BTreeSet::new(),
            shown: BTreeSet::new(),
            reset: BTreeSet::new(),
            imported: BTreeMap::new(),
            edited: BTreeMap::new(),
            chosen: BTreeMap::new(),
            appended: Vec::new(),
            awaiting: Vec::new(),
            supplied: 0,
            pointer: None,
            magnification: None,
            widget_appearances: WidgetAppearances::default(),
            audience: Audience::NONE,
            purpose: Purpose::View,
            paper: None,
            separation_simulation: false,
            added: Vec::new(),
            retyped: BTreeMap::new(),
            filed: Vec::new(),
            unfiled: Vec::new(),
            modified: None,
            allocated: 0,
            uncommitted: BTreeMap::new(),
            script_reports: Vec::new(),
            runner: scripts::Runner::default(),
            scripting: scripts::Scripting::default(),
        }
    }

    /// A number no object in the file and nothing already added or attached is using, and
    /// which nothing after this call will be given.
    ///
    /// The same rule [`writing::Update::beside`] applies, and for the same reason: §7.5.5 makes
    /// `/Size` "one greater than the highest object number used in the file" and 68 of the corpus's
    /// 974 documents write a cross-reference entry beyond their own, so the larger of the two wins.
    /// The count already handed out is added on top, because those numbers are spoken for even
    /// though nothing has been written yet — and it is a count that only grows while anything
    /// allocated is held (see [`Self::allocated`]), so a number is never given twice in one
    /// sitting.
    fn allocate(&mut self, document: &Document) -> ObjectId {
        let highest = document.xref().object_numbers().max().unwrap_or_default();
        let stated = document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .and_then(|size| u32::try_from(size).ok())
            .unwrap_or_default();
        let base = highest.saturating_add(1).max(stated);
        let number = base.saturating_add(self.allocated);
        self.allocated = self.allocated.saturating_add(1);
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// Starts the numbering again once nothing allocated is held any more.
    ///
    /// A replay clears every addition and every attachment before it re-applies the log, and the
    /// numbers a replay hands out have to be the ones the log recorded — so the counter starts
    /// from nothing exactly when the state does.
    fn release_numbers_if_nothing_is_held(&mut self) {
        if self.added.is_empty() && self.filed.is_empty() {
            self.allocated = 0;
        }
    }
}

impl Entry {
    /// This entry as the statement about a value the appearance path reads.
    fn as_field_value(&self) -> FieldValue<'_> {
        FieldValue::Edited {
            value: self.value.as_ref(),
            indices: self.indices.as_deref(),
        }
    }
}

/// A saved document, and what this program declined to put in it.
///
/// More than bytes alone because the rest is not an error and must not be silent: a save that
/// quietly dropped what a person typed would be their work lost without a word, and this one
/// drops some things on purpose and leaves others to the next reader. See [`ViewState::save`]
/// for the clauses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The whole file: the document as it was opened, with §7.5.6's update appended.
    pub bytes: Vec<u8>,
    /// §12.7.4.2's qualified name of every field whose typed value was **not** stored.
    ///
    /// Table 231 bit 14's NOTE, quoted in full on [`ViewState::save`]. Empty for every document
    /// with no password field in it, which is every document in this project's corpus.
    pub withheld: Vec<String>,
    /// Every free text annotation written with `/Contents` and **no** appearance stream.
    ///
    /// The other thing a save drops on purpose, and it drops less than it looks: the annotation
    /// carries what the person typed and Table 177 makes `/DA` Required, so the next reader has
    /// everything §12.5.6.6 needs to generate an appearance — this one could not, and says which
    /// annotations. Empty unless a retyped annotation's `/DA` names a font its document does not
    /// define, or a retyped or added annotation's text is set in a face from this machine whose
    /// licence forbids embedding it (ADR 1425). See [`ViewState::save`].
    pub unappeared: Vec<ObjectId>,
    /// §12.7.4.2's qualified name of every field whose widget was written without a complete
    /// appearance stream, which is what Table 224's `/NeedAppearances` in the written file is for.
    ///
    /// The entry's own row binds a writer "if it has not provided appearance streams for all
    /// visible widget annotations present in the document", so a file this program wrote with the
    /// flag set is a file with something outstanding in it — and the flag names no widget. This
    /// does. Empty for every save where §12.7.4.3's layout produced the whole of every changed
    /// widget's stream, and non-empty exactly where that layout declined or reported — a value
    /// whose characters no font in reach can spell is the standing case (`doc/todo/22`). See
    /// [`ViewState::save`].
    pub unconstructed: Vec<String>,
    /// Every file detached from the tree whose stream another home still reaches, with the
    /// sentence naming the home.
    ///
    /// Not a failure and not silence: §7.11.4.1 gives an embedded file more than one home, so
    /// the tree letting go of an entry is not a deletion where the catalog's `/AF` or a page's
    /// annotation still names the stream — the entry is gone from the tree and the objects stay
    /// in use, and the person who asked is told which. Empty for every detach of a file with one
    /// home, which is every file in the corpus's trees.
    pub still_reached: Vec<(String, String)>,
    /// How many §12.7.8.3.3 template pages an import added that this file was written without.
    ///
    /// Table 252's `/Pages` describes "new pages that shall be added to the target document", the
    /// import adds them to the view ([`ViewState::appended_pages`]), and the update writes no page
    /// into the page tree — so a save after such an import leaves them out, and this says how
    /// many (ADR 2154). Zero for every save no template import came before.
    pub unwritten_pages: usize,
}

/// What a field says now, and whether saying it was Table 231 bit 14's substitution.
///
/// **One value carrying both, deliberately.** The flag is not a second reading of the field: it is
/// a statement about *this string*, and a caller that had to ask a separate question could get an
/// answer that disagreed with the characters it is holding. That is the defect ADR 0247's third
/// amendment closes — a value that is not the value, in a type that does not say so, with the
/// exception discoverable only by reading two doc comments and noticing they interact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownValue {
    /// The characters, or the bullets standing in for them.
    ///
    /// `""` is a field with nothing in it. Whether the field has a text value *at all* is the
    /// `Option` around this struct, not a property of it: a button selects an appearance and a
    /// signature holds a dictionary, and neither answers with a [`ShownValue`].
    pub text: String,
    /// Whether [`Self::text`] is Table 231 bit 14's echo rather than the field's own characters.
    ///
    /// ISO 32000-2 §12.7.5.3, Table 231, bit position 14:
    ///
    /// > If set, the field is intended for entering a secure password that should not be echoed
    /// > visibly to the screen. Characters typed from the keyboard shall instead be echoed in some
    /// > unreadable form, such as asterisks or bullet characters.
    ///
    /// **What a host does with it**: it does not write [`Self::text`] back into its control. ADR
    /// 0201 has a host read a field's value back after every keystroke, because §12.7.5.3's
    /// `DoNotScroll` means the field can take less than was typed; doing that here would replace
    /// what a person typed with a row of bullets and send *those* as the next value. A native
    /// password control draws its own echo from the characters it holds, so there is nothing it
    /// needs this string for.
    pub obscured: bool,
}

/// One glyph of a field's value as §12.7.4.3's layout placed it: [`ViewState::field_glyphs`]'s
/// answer.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldGlyph {
    /// Which line of the field the glyph is on, counted from the first drawn.
    pub line: usize,
    /// The byte range of the value [`ViewState::field_value`] answers with that the glyph shows.
    ///
    /// From the byte its code came from to the byte the next code came from, so a character the
    /// font states no code for, or a pair drawn as one joined form, belongs to the glyph before it.
    pub bytes: std::ops::Range<usize>,
    /// The glyph's box in default user space, four corners anticlockwise from the top left —
    /// corners rather than a rectangle because `/R`, the appearance's `/Matrix` and a `/DA`'s `Tm`
    /// can each turn it. As tall as a caret and as wide as the glyph's advance; a comb's glyph is
    /// as wide as its cell.
    pub quad: [f32; 8],
}

/// What a form field is called: the name that identifies it, and the name to show a person.
///
/// Two names because the standard states two, for two different jobs. §12.7.4.2's fully
/// qualified name is the field's *identity* — it is what [`crate::view::ViewState::set_field`]
/// addresses and what §12.7.6.2 exports — and §14.9.3 makes the other one a `shall`:
///
/// > An alternative name may be specified for an interactive form field (see 12.7, "Forms")
/// > which, if present, shall be used in place of the actual field name when an interactive PDF
/// > processor identifies the field in a user-interface. This alternative name, if provided,
/// > shall be specified using the TU entry of the field dictionary.
///
/// A single string could not carry both: whichever meaning it took, the other would be lost at
/// the caller. This is why the two are handed over together rather than chosen here — the caller
/// knows whether it is addressing the field or naming it to a person, and this type does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName {
    /// §12.7.4.2's fully qualified name, which identifies the field.
    pub qualified: String,
    /// Table 226's `/TU`, where the field states one.
    pub alternative: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{ViewState, widgets_by_field_name};
    use crate::action::read;
    use crate::optional_content::{ListMode, OptionalContent, Presented};
    use pdf_syntax::{Document, Object, ObjectId};

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

    /// §12.6.4.13 turns a layer off, and the next render of the page hides its content.
    #[test]
    fn a_set_ocg_state_action_turns_a_layer_off() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R] /D << >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/OFF 4 0 R] >>",
            "<< /Type /OCG /Name (layer) >>",
        ]);
        let mut state = ViewState::of(&doc);
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(true),
            "§8.11.4.5's default BaseState is ON"
        );

        let actions = read(&doc, &Object::Reference(id(3)));
        assert!(
            state.perform_all(&doc, &actions).is_empty(),
            "a layer change asks the viewer for nothing"
        );
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(false)
        );
    }

    /// Table 217's `/PreserveRB`, default true: turning one group on turns its siblings off.
    ///
    /// Table 99 states the paradigm — "the state of at most one optional content group in
    /// each array shall be ON at a time. If one group is turned ON, all others shall be
    /// turned OFF" — and Table 217's default is what makes it apply without the file asking.
    #[test]
    fn turning_a_radio_button_group_on_turns_its_siblings_off() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R 5 0 R] \
             /D << /BaseState /OFF /ON [4 0 R] /RBGroups [[4 0 R 5 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/ON 5 0 R] >>",
            "<< /Type /OCG /Name (first) >>",
            "<< /Type /OCG /Name (second) >>",
        ]);
        let mut state = ViewState::of(&doc);
        let both = |state: &ViewState| {
            (
                state.optional_content().and_then(|oc| oc.state(id(4))),
                state.optional_content().and_then(|oc| oc.state(id(5))),
            )
        };
        assert_eq!(both(&state), (Some(true), Some(false)));

        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(
            both(&state),
            (Some(false), Some(true)),
            "the collection admits one"
        );
    }

    /// `/PreserveRB false` leaves the siblings alone, which is the entry's whole purpose.
    #[test]
    fn preserve_rb_false_ignores_the_collections() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R 5 0 R] \
             /D << /BaseState /OFF /ON [4 0 R] /RBGroups [[4 0 R 5 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/ON 5 0 R] /PreserveRB false >>",
            "<< /Type /OCG /Name (first) >>",
            "<< /Type /OCG /Name (second) >>",
        ]);
        let mut state = ViewState::of(&doc);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(
            (
                state.optional_content().and_then(|oc| oc.state(id(4))),
                state.optional_content().and_then(|oc| oc.state(id(5))),
            ),
            (Some(true), Some(true)),
            "both on, which the collection would forbid and this entry permits"
        );
    }

    /// §12.7.4.2's own example, built as a field tree and looked up by a hide action.
    ///
    /// > If a field with the partial field name PersonalData has a child whose partial name
    /// > is Address, which in turn has a child with the partial name ZipCode, the fully
    /// > qualified name of this last field is PersonalData.Address.ZipCode
    #[test]
    fn a_hide_action_finds_a_field_by_its_fully_qualified_name() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /Hide /T (PersonalData.Address.ZipCode) >>",
            "<< /T (PersonalData) /Kids [5 0 R] >>",
            "<< /T (Address) /Parent 4 0 R /Kids [6 0 R] >>",
            "<< /T (ZipCode) /Parent 5 0 R /FT /Tx >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(
            table.get("PersonalData.Address.ZipCode").map(Vec::as_slice),
            Some([id(6)].as_slice())
        );

        let mut state = ViewState::of(&doc);
        assert_eq!(state.annotation_hidden(id(6)), None);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(state.annotation_hidden(id(6)), Some(true));
    }

    /// A kid with no `/T` is a widget of its parent's field, not a field of its own.
    ///
    /// §12.7.4.2: "A field dictionary that does not have a partial field name (T entry) of its
    /// own shall not be considered a field but simply a Widget annotation." So one name
    /// reaches two annotations, and hiding it hides both — which is what Table 214 means by
    /// "whose associated widget annotation or annotations are to be affected".
    #[test]
    fn one_field_name_may_reach_several_widgets() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /Hide /T (Signature) /H true >>",
            "<< /T (Signature) /FT /Sig /Kids [5 0 R 6 0 R] >>",
            "<< /Parent 4 0 R /Subtype /Widget >>",
            "<< /Parent 4 0 R /Subtype /Widget >>",
        ]);
        let mut state = ViewState::of(&doc);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(state.annotation_hidden(id(5)), Some(true));
        assert_eq!(state.annotation_hidden(id(6)), Some(true));
    }

    /// A `/Fields` entry no `/T` reaches is not a field, so it is in no key of the table.
    ///
    /// §12.7.4.2's sentence is unconditional — "A field dictionary that does not have a partial
    /// field name ( T entry) of its own shall not be considered a field but simply a Widget
    /// annotation" — and at the root of `/Fields` there is no ancestor to supply one. Table 226
    /// printed `/T` as `(Required)` until Errata Collection 3's Issue #28 wrote *Optional* in its
    /// place, which is what makes this a conforming file rather than a repair.
    ///
    /// The two annotations share nothing, so the failure this pins is a *merge*: keying them
    /// under the empty name made one field of them, and a value entered into it reached both.
    #[test]
    fn a_root_field_that_nothing_names_is_not_a_field() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R 4 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /FT /Tx /Subtype /Widget >>",
            "<< /FT /Tx /Subtype /Widget >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(
            table.keys().collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "neither is a field, so neither is under any name — the empty one included"
        );
    }

    /// A `/T` of zero length is still an entry, so the dictionary is still a field.
    ///
    /// §12.7.4.2 conditions on the *entry* rather than on what it holds, and this is the pair to
    /// the test above: the two look identical to a reader that asked whether the name came out
    /// empty instead of whether the entry was there.
    #[test]
    fn an_empty_partial_name_is_still_a_name() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /T () /FT /Tx /Subtype /Widget >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(table.get("").map(Vec::as_slice), Some([id(3)].as_slice()));
    }

    /// A `/Fields` entry that states a `/Parent` takes its ancestors' name.
    ///
    /// §12.7.3 makes `/Fields` "[a]n array of references to the document's root fields (those with
    /// no ancestors in the field hierarchy)", so an entry stating a `/Parent` contradicts it —
    /// and Table 226's `/Parent` is the file's own statement of which field the entry belongs to.
    /// Taking it is the recovery §12.7.4.2's row argues: the alternative is a radio group read as
    /// nameless because the producer listed the buttons rather than the field above them.
    /// `doc/pdf.js/test/pdfs/opt_demo.pdf` is the corpus witness.
    #[test]
    fn a_root_field_takes_the_name_its_parent_chain_states() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /T (garden) /Kids [4 0 R] >>",
            "<< /T (veg) /Parent 3 0 R /FT /Btn /Kids [5 0 R 6 0 R] >>",
            "<< /Parent 4 0 R /Subtype /Widget >>",
            "<< /Parent 4 0 R /Subtype /Widget >>",
            "<< /S /Hide /T (garden.veg) /H true >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(
            table.get("garden.veg").map(Vec::as_slice),
            Some([id(5), id(6)].as_slice()),
            "both buttons are widgets of the field their /Parent chain names"
        );

        let mut state = ViewState::of(&doc);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(7))));
        assert_eq!(state.annotation_hidden(id(5)), Some(true));
        assert_eq!(state.annotation_hidden(id(6)), Some(true));
    }

    /// Table 99's `/Locked` stops a panel and not §12.6.4.13's action.
    ///
    /// The two halves of §8.11.4.5's interactive paragraph, and the clause draws the line
    /// itself: a locked group "cannot be changed through the user interface", and a processor
    /// "may allow the states … to be changed by means other than the user interface".
    #[test]
    fn a_locked_group_refuses_a_panel_and_not_an_action() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R] \
             /D << /Locked [4 0 R] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/OFF 4 0 R] >>",
            "<< /Type /OCG /Name (layer) >>",
        ]);
        let mut state = ViewState::of(&doc);
        assert!(
            state
                .optional_content()
                .is_some_and(|oc| oc.is_locked(id(4)))
        );
        assert!(!state.set_group(id(4), false), "a panel is refused");
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(true)
        );

        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(false),
            "the action is not bound by the lock"
        );
    }

    /// A panel honours Table 99's radio-button collections without being asked to.
    ///
    /// `/PreserveRB` is a *set-OCG-state action's* entry; Table 99 states the paradigm
    /// unconditionally, so a panel that let two members of one collection be on at once would
    /// be showing a state the configuration says cannot exist.
    #[test]
    fn a_panel_keeps_a_radio_button_collection_to_one() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [3 0 R 4 0 R] \
             /D << /BaseState /OFF /ON [3 0 R] /RBGroups [[3 0 R 4 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /OCG /Name (first) >>",
            "<< /Type /OCG /Name (second) >>",
        ]);
        let mut state = ViewState::of(&doc);
        assert!(state.set_group(id(4), true));
        assert_eq!(
            (
                state.optional_content().and_then(|oc| oc.state(id(3))),
                state.optional_content().and_then(|oc| oc.state(id(4))),
            ),
            (Some(false), Some(true))
        );
        assert!(!state.set_group(id(4), true), "already on: nothing changed");
    }

    /// §8.11.4.3's EXAMPLE 1 and EXAMPLE 2, which are the two shapes `/Order` has.
    ///
    /// EXAMPLE 1 labels its nested arrays — `[(Frog Anatomy) 1 0 R 2 0 R]` — and the clause says
    /// those labels "shall be used to present collections of related optional content groups,
    /// and not to communicate actual nesting". EXAMPLE 2 nests without a label, which is what
    /// "actual nesting of groups in the content, such as for layers with sublayers" looks like.
    /// A panel that drew both the same way would tell a person that a heading is a layer, which
    /// is why the label is an `Option` and not a `String`.
    #[test]
    fn the_order_array_keeps_a_label_apart_from_a_nesting() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [3 0 R 4 0 R 5 0 R] \
             /D << /Order [[(Frog Anatomy) 3 0 R 4 0 R] 5 0 R [3 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /OCG /Name (Skin) >>",
            "<< /Type /OCG /Name (Bones) >>",
            "<< /Type /OCG /Name (Layer 1) >>",
        ]);
        let state = ViewState::of(&doc);
        let content = state.optional_content().expect("a configuration");
        assert_eq!(
            content.presentation(),
            [
                Presented::Collection {
                    label: Some("Frog Anatomy".to_owned()),
                    children: vec![Presented::Group(id(3)), Presented::Group(id(4))],
                },
                Presented::Group(id(5)),
                Presented::Collection {
                    label: None,
                    children: vec![Presented::Group(id(3))],
                },
            ]
        );
        assert_eq!(content.name(&doc, id(5)).as_deref(), Some("Layer 1"));
        assert_eq!(content.list_mode(), ListMode::AllPages);
    }

    /// A group `/Order` names that the document never declared governs nothing and is not shown.
    ///
    /// §8.11.3.2 makes membership of `/OCGs` the test for whether content is optional content at
    /// all, so a switch for a group outside it would do nothing at all when a person moved it.
    #[test]
    fn an_undeclared_group_is_not_presented() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [3 0 R] \
             /D << /Order [3 0 R 4 0 R] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /OCG /Name (declared) >>",
            "<< /Type /OCG /Name (not declared) >>",
        ]);
        let state = ViewState::of(&doc);
        assert_eq!(
            state.optional_content().map(OptionalContent::presentation),
            Some([Presented::Group(id(3))].as_slice())
        );
    }

    /// A cycle in `/Kids` terminates rather than exhausting the stack.
    ///
    /// `/Kids` is a reference the document controls and §12.7.4.1 states no acyclicity rule,
    /// so a field pointing back at its own ancestor is a file a reader has to survive. The
    /// assertion that matters is that this returns at all — without the `seen` set it
    /// recurses until the stack ends — and the leaf beside the cycle is still found, which is
    /// what says the guard skips the loop rather than the subtree.
    #[test]
    fn a_cycle_in_the_field_tree_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /T (a) /Kids [4 0 R] >>",
            "<< /T (b) /Kids [3 0 R 5 0 R] >>",
            "<< /T (c) /FT /Tx >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(
            table.get("a.b.c").map(Vec::as_slice),
            Some([id(5)].as_slice())
        );
        assert!(
            !table.keys().any(|name| name.matches('a').count() > 1),
            "the cycle produced no repeated name: {:?}",
            table.keys().collect::<Vec<_>>()
        );
    }

    /// §12.7.6.3's three shapes of `/Fields`, over one field tree.
    ///
    /// Table 241 makes the absent entry decisive — "all fields in the document's interactive
    /// form are reset" — Table 242's clear flag names what to reset "[a]ll descendants … as
    /// well", and its set flag names what to spare. The tree here has a named subtree so that
    /// the descendant rule is what passes rather than an exact-name match.
    #[test]
    fn a_reset_form_action_names_fields_to_reset_or_fields_to_spare() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [7 0 R 9 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /ResetForm >>",
            "<< /S /ResetForm /Fields [(PersonalData)] >>",
            "<< /S /ResetForm /Fields [(PersonalData)] /Flags 1 >>",
            "<< /S /ResetForm /Fields [8 0 R] >>",
            "<< /T (PersonalData) /Kids [8 0 R] >>",
            "<< /T (ZipCode) /Parent 7 0 R /FT /Tx /Subtype /Widget >>",
            "<< /T (Signature) /FT /Sig /Subtype /Widget >>",
        ]);

        let all = |action: u32| {
            let mut state = ViewState::of(&doc);
            state.perform_all(&doc, &read(&doc, &Object::Reference(id(action))));
            (state.is_reset(id(8)), state.is_reset(id(9)))
        };

        assert_eq!(
            all(3),
            (true, true),
            "no /Fields resets every field there is"
        );
        assert_eq!(
            all(4),
            (true, false),
            "naming the parent resets its descendants and nothing else"
        );
        assert_eq!(
            all(5),
            (false, true),
            "the Include/Exclude flag turns the same array into what to spare"
        );
        assert_eq!(
            all(6),
            (true, false),
            "a field named by reference rather than by name"
        );
    }

    /// Table 241's reference form reaches the referenced field's descendants, in both branches.
    ///
    /// The fixture is the one shape a reference and a name answer differently: object 7 is a
    /// **non-terminal** field, so it is not a widget and no page draws it, and object 10 is a
    /// terminal field whose single widget is a separate `/Kids` entry rather than the merged
    /// dictionary §12.7.4.1 permits. Under Table 242 — "[a]ll descendants of the specified fields
    /// in the field hierarchy are reset as well" — naming either by reference reaches the widget
    /// below it; taking the reference for a widget identity reaches nothing at all.
    ///
    /// The exclude branch is asserted beside it because Errata Collection 3's Issue #174 is what
    /// says the parenthesis binds there too: *(All descendants of the specified fields in the
    /// field hierarchy are also exempt from being reset.)*
    #[test]
    fn a_reset_form_action_named_by_reference_reaches_the_fields_descendants() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [7 0 R 10 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /ResetForm /Fields [7 0 R] >>",
            "<< /S /ResetForm /Fields [7 0 R] /Flags 1 >>",
            "<< /S /ResetForm /Fields [10 0 R] >>",
            "<< /S /ResetForm >>",
            "<< /T (PersonalData) /Kids [8 0 R 9 0 R] >>",
            "<< /T (ZipCode) /Parent 7 0 R /FT /Tx /Subtype /Widget >>",
            "<< /T (City) /Parent 7 0 R /FT /Tx /Subtype /Widget >>",
            "<< /T (Signature) /FT /Sig /Kids [11 0 R] >>",
            "<< /Parent 10 0 R /Subtype /Widget >>",
        ]);

        let all = |action: u32| {
            let mut state = ViewState::of(&doc);
            state.perform_all(&doc, &read(&doc, &Object::Reference(id(action))));
            (
                state.is_reset(id(8)),
                state.is_reset(id(9)),
                state.is_reset(id(11)),
            )
        };

        assert_eq!(
            all(3),
            (true, true, false),
            "a reference to a non-terminal field resets the widgets under it"
        );
        assert_eq!(
            all(4),
            (false, false, true),
            "and the Include/Exclude flag spares those same descendants"
        );
        assert_eq!(
            all(5),
            (false, false, true),
            "a terminal field's widget is a /Kids entry, and the reference reaches it"
        );
        assert_eq!(
            all(6),
            (true, true, true),
            "no /Fields is still every widget the field tree names"
        );

        // The set this holds is of *widget annotations*, which is the whole reason the walk
        // stops at a leaf: the non-terminal field itself is drawn by nothing, and putting it in
        // would make `is_reset` answer about an object no page ever asks about.
        let mut state = ViewState::of(&doc);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert!(
            !state.is_reset(id(7)),
            "the non-terminal field is not itself a widget"
        );
    }
}
