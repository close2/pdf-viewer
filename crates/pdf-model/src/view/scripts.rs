//! Table 199's four field triggers raised over [`ViewState`], Table 224's `/CO` walked, and every
//! script a host's runner runs handed over with the fields its document's realm holds.
//!
//! RFC 0008's Tier 0 (ADR 1579): where a field's `/K`, `/V` or `/C` is one call of
//! [`crate::aform`]'s library, the call runs at the point this state commits a value; `/F` runs
//! where `crate::appearance` lays the value out, because that is where a value is "formatted to
//! display". Every other script is reported, once, as one this tier does not run.
//!
//! **When each runs is the event model's, and the event model is two sources.** ISO 32000-2
//! §12.6.3's Table 199 names the triggers — `/K` "when the user modifies a character", `/V` "when
//! the field's value is changed", `/C` "to recalculate the value of this field when that of another
//! field changes" — and §12.7.3's Table 224 orders the last: `/CO` is the array "defining the
//! calculation order in which their values will be recalculated when the value of any field
//! changes". What the standard does not say is when a value typed character by character has
//! *changed*; Adobe's *JavaScript for Acrobat API Reference*, "Form event processing", draws it:
//! keystrokes, then a keystroke with `willCommit` set, then validate, then calculate, then format.
//! So a host's whole value per keystroke is `/K` in its typing form ([`ViewState::set_field`]),
//! and the commit is its own call ([`ViewState::commit_field`]), which runs `/K`'s commit form, then
//! `/V`, then every `/CO` entry, then `/F` on every field a runner formats. A one-call `/C` is also
//! run after every keystroke — a calculation of the library's is a function of other fields'
//! values, so running it early shows a total the commit would show anyway — and a script's `/C` is
//! run at the commit alone, where the reference puts it, because a script may do more than compute
//! (ADR 1603).
//!
//! **A script that is not one call goes to a runner where a host has supplied one** — RFC 0008
//! section 6.3's policy hook, one place a host supplies ([`ViewState::run_scripts_with`]) and absent
//! by default, which is the level `off` (ADR 1591). The runner is handed data, a [`ScriptEvent`],
//! and answers data, a [`ScriptResult`], so that what runs it can sit across a process boundary;
//! this crate constructs no engine and names none. Every event carries the fields whose state
//! changed since the runner last heard — every field, the first time — so that the realm a runner
//! keeps for the document reads any field without asking (ADR 1602), and every edit a result
//! carries lands in the same log a person's typing does (RFC 0008 section 6.4, ADR 1603).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use super::script_model::{
    CommitKey, ConsoleRequest, DocumentState, FieldState, FontName, Keys, Overrides, Property,
    ScriptEdit, ScriptSite, ViewChange, WindowView,
};
use super::{Entry, ViewState};
use crate::action::{ResetForm, ResetTarget};
use crate::aform::site::{self, Site};
use crate::aform::{Keyed, Keystroke, Trigger};
use crate::forms_data::Import;

/// Most entries of Table 224's `/CO` one recalculation walks.
///
/// The array is the document's, so its length is too; a form with more calculated fields than this
/// is not one any producer writes, and the walk is per keystroke.
const MAX_CALCULATIONS: usize = 4096;

/// Most distinct sentences [`ViewState::script_reports`] keeps.
const MAX_REPORTS: usize = 256;

/// Longest script text, in bytes, handed to a runner.
///
/// Table 221's `/JS` is the document's, so its length is too. A field script is a few hundred bytes
/// and the longest the census population carries is under a hundred kilobytes; a megabyte is not a
/// field script, and decoding one per keystroke is work this bound refuses.
pub(super) const MAX_SCRIPT_BYTES: usize = 1 << 20;

/// Longest one sequence of scripts may take: a commit's walk of `/CO`, or the open sequence.
///
/// Each script is held to its runner's own budget; a sequence is a document's choice of how many
/// scripts to run back to back, and four thousand calculations each just under a field event's
/// budget would hold a commit for minutes. Ten field events' worth is longer than any sequence a
/// form fills with and short enough that a person still reads the stop as one (ADR 1603).
pub(super) const MAX_SEQUENCE_TIME: Duration = Duration::from_secs(1);

/// One script a runner is handed, at one of §12.6.3's sites or the document's open.
///
/// Adobe's "event properties" the site raises — `event.value`, `change`, `selStart`, `selEnd`,
/// `willCommit`, `source`, `targetName` — with the script's own text and the fields the realm is
/// told of. Data rather than a callback, so that it crosses RFC 0008 section 6.2's process boundary
/// as it stands (ADRs 1591, 1602).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptEvent<'a> {
    /// Where the script runs.
    pub site: ScriptSite,
    /// §12.7.4.2's fully qualified name of the event's field — `event.target` — or empty where the
    /// event has none.
    pub field: &'a str,
    /// Adobe's `event.targetName` where the target is not a field: the name a document-level
    /// script carries in Table 32's tree at `Doc/Open`, and an outline item's Table 151 `/Title`
    /// at `Bookmark/Mouse Up` (ADR 1904); empty for every other site.
    pub label: &'a str,
    /// Table 221's `/JS`, as text.
    pub script: &'a str,
    /// The field's text before the event; empty where the event has no field.
    pub value: &'a str,
    /// What a keystroke inserts, replacing the selection: empty at every other event.
    pub change: &'a str,
    /// The selection the change replaces, as byte offsets into `value`.
    pub selection: (usize, usize),
    /// Whether this is the commit rather than a character.
    pub will_commit: bool,
    /// How the person committed, at a commit's own events — its keystroke, its validation and its
    /// field's format — and `None`, Adobe's `0`, at every other (ADR 1626).
    pub commit_key: Option<CommitKey>,
    /// Adobe's `event.fieldFull`: a keystroke into a text field whose characters do not all fit,
    /// by Table 232's `/MaxLen` or by Table 231's `DoNotScroll` (ADR 1626).
    pub field_full: bool,
    /// Adobe's `event.changeEx`: the whole of what was typed where the field is full, the export
    /// value of a choice field's option where the change names one, and the change otherwise.
    pub change_ex: &'a str,
    /// The field whose change a calculation answers — `event.source` — or empty.
    pub source: &'a str,
    /// Every field whose state changed since the runner was last handed an event: all of them the
    /// first time. A runner's realm replaces its record of each by name.
    pub fields: &'a [FieldState],
    /// The zero-based page the event happens on, Adobe's `this.pageNum`.
    pub page: usize,
    /// How many pages the document has, `this.numPages`.
    pub pages: usize,
    /// Whether this view state holds work a save would write and no save has written:
    /// `this.dirty` ([`ViewState::mark_saved`], ADR 1626).
    pub dirty: bool,
    /// The document as a whole, where it has changed since the runner was last handed an event:
    /// always the first time. A runner's realm replaces its record with it.
    pub document: Option<&'a DocumentState>,
    /// The window's view of the document, as a host last told the view state of it: `this.zoom`,
    /// `this.zoomType` and `this.layout` (ADR 1736). Small, so it is handed with every event
    /// rather than measured against what the realm last heard.
    pub view: WindowView,
    /// What the host's keyboard held at the event: `event.shift`, `event.modifier` and
    /// `event.keyDown`, as a host last told the view state ([`ViewState::set_keys`], ADR 1762).
    pub keys: Keys,
    /// The XFA rich text string `event.richValue` is read from, at a rich text field's event:
    /// Table 228's `/RV` where it holds the characters the value does, the value as a save writes
    /// it otherwise (ADR 1635); empty for every other field and site (ADR 1762).
    pub rich_value: &'a str,
}

/// What a [`ScriptRunner`] made of a [`ScriptEvent`].
///
/// A run that did not finish — a budget exceeded, a throw, a refused call left uncaught — changes
/// nothing: `rc` true, no value, no change, no edit, and the sentence saying why in `report` (ADR
/// 1591).
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptResult {
    /// Adobe's `event.rc`: false refuses the keystroke, the commit, the validated value or the
    /// calculated one. No other site listens to it.
    pub rc: bool,
    /// `event.value` where the script changed it: at `/F` the text displayed, at a commit the value
    /// that stands, at `/C` the value calculated.
    pub value: Option<String>,
    /// `event.change` where a keystroke's script changed it.
    pub change: Option<String>,
    /// Every change the script made to the document's fields, in order.
    pub edits: Vec<ScriptEdit>,
    /// Every sentence the run owes a report: each call refused by name, each budget exceeded, each
    /// throw, each line the script logged.
    pub report: Vec<String>,
}

/// What runs the scripts Tier 0 does not: RFC 0008 section 6.3's policy hook.
///
/// A host supplies one through [`ViewState::run_scripts_with`], and only where the reader's level
/// lets scripts run — so the four levels attach to this one place rather than to each trigger:
/// `off` supplies nothing, `on` and `warn` supply a runner, and `ask` supplies one that asks once
/// before its first run and remembers the answer. A view state with nothing supplied reports every
/// such script as one this tier does not run, which is the default (ADR 1591).
///
/// One runner serves one document, and keeps that document's realm across every event it is
/// handed: the functions Table 32's name tree defines at the open are what a field's script calls
/// later (ADR 1602).
pub trait ScriptRunner: std::fmt::Debug + Send + Sync {
    /// Runs one event's script and says what it did.
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult;

    /// Whether the run this runner last answered left its script waiting on a person's answer to a
    /// question it put — `app.alert`, `app.response` — rather than finished (ADR 1627).
    ///
    /// **A run never waits on a person.** A host's thread is the one that draws the question
    /// (ADR 1628), so a script that asks is held where it runs and its run answers at once, having
    /// changed nothing; what it does once answered arrives later, through [`Self::take_resumed`].
    /// A runner that puts no question is never waiting, which is the default.
    fn waiting(&self) -> bool {
        false
    }

    /// The next run that waited on a person and has since finished, in the order they finished:
    /// what [`ViewState::apply_resumed`] applies. `None` where there is none, which is the default.
    fn take_resumed(&self) -> Option<Resumed> {
        None
    }
}

/// A run that waited on a person's answer and has since finished: its site, its field and what it
/// did, applied as that trigger's outcome arriving late (ADR 1627).
#[derive(Debug, Clone, PartialEq)]
pub struct Resumed {
    /// Where the script ran.
    pub site: ScriptSite,
    /// The event's field, or empty.
    pub field: String,
    /// What the whole run did, from its first statement to its last.
    pub result: ScriptResult,
}

/// The runner a [`ViewState`] holds, or none.
///
/// Compared by identity: two view states hold the same runner only where they hold the same one.
#[derive(Debug, Clone, Default)]
pub(super) struct Runner(pub(super) Option<Arc<dyn ScriptRunner>>);

impl PartialEq for Runner {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(mine), Some(theirs)) => Arc::ptr_eq(mine, theirs),
            _ => false,
        }
    }
}

/// What one widget displays through a format script a runner ran: the text Table 199's `/F` left
/// for one value.
///
/// Kept beside the edit log and handed to the appearance through [`super::AnnotationView`], so the
/// page draws what [`ViewState::displayed_value`] answers. `value` is what was formatted, so that
/// a drawing whose value has moved on since shows the value rather than a stale format of another
/// (ADR 1603).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Displayed {
    /// The value the format script was handed.
    pub value: String,
    /// What it displayed.
    pub shown: String,
}

/// A script's `setFocus`, held until a host carries it out: which widget of which field takes the
/// keyboard (ADRs 1615, 1688).
///
/// The widget is a place in the field's list of [`super::widgets_by_field_name`], which is
/// §12.7.4.1's `/Kids` order and the index `getField("name.N")` counts from zero; a `Field` of
/// every widget asks for the first. The place is checked against that list when the script's run
/// is applied, so a host that reads the same list finds the widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusRequest {
    /// The field's fully qualified name.
    pub field: String,
    /// The widget's place among the field's widgets, from zero.
    pub widget: usize,
}

/// The state scripts keep beside the edit log.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Scripting {
    /// What the runner's realm was last told of, or `None` where it has been told nothing.
    told: Option<Box<Told>>,
    /// The properties scripts set, by field name.
    pub(super) overrides: BTreeMap<String, Overrides>,
    /// What each widget displays through a runner's format script.
    pub(super) formatted: BTreeMap<ObjectId, Displayed>,
    /// Every page's index, by object, once a field's page has been asked for.
    pages: Option<BTreeMap<ObjectId, usize>>,
    /// The widget a script last asked the focus for, until a host takes the request.
    focus: Option<FocusRequest>,
    /// The zero-based page a script last turned to, until a host takes the request (ADR 1640).
    page: Option<usize>,
    /// The properties scripts set that a widget's appearance draws, by widget: what
    /// [`super::AnnotationView::scripted`] carries (ADR 1617).
    pub(super) drawn: BTreeMap<ObjectId, Vec<Property>>,
    /// Every widget and member a script set through a `Field` of that widget alone, until a set on
    /// the whole field replaces it: a save writes such a member's field entries — `/DA`, `/Q` — on
    /// no field, which the widget's siblings would inherit (ADR 1664).
    pub(super) one_widget: BTreeSet<(ObjectId, &'static str)>,
    /// What a save last wrote, where a host has said a save happened: what `this.dirty` is
    /// measured against ([`ViewState::mark_saved`]).
    saved: Option<Box<Saved>>,
    /// The field a commit is being made in and how, while its formats run: the one format that
    /// carries `event.commitKey`.
    committing: Option<(String, CommitKey)>,
    /// Every run the runner answered as held — waiting on a person, or queued behind one that is —
    /// in the order handed over, until its outcome arrives ([`ViewState::apply_resumed`]).
    pending: Vec<Pending>,
    /// The timers scripts set and the sounds they asked for, until a host's ticks run the one and
    /// a host takes the other (ADR 1702).
    pub(super) timers: super::script_timers::Timers,
    /// The document's annotations as a realm was told of them, and what scripts set on them (ADR
    /// 1700).
    pub(super) annotations: super::script_annotations::Annotations,
    /// Whether a script set `this.calculate` false, which stops Table 224's `/CO` from being
    /// walked until a script sets it true again (ADR 1724).
    calculations_off: bool,
    /// The window's view as a host last told it, or `None` where no host has
    /// ([`ViewState::set_window_view`], ADR 1736).
    window: Option<WindowView>,
    /// The changes scripts asked of the window's view, in the order asked, until a host takes
    /// them; at most [`MAX_VIEW_CHANGES`].
    views: Vec<ViewChange>,
    /// Whether [`ViewState::run_open_scripts`] has run: until it has, the functions Table 32's
    /// name tree defines do not exist, so no event a host raises runs a script (ADR 1750).
    pub(super) opened: bool,
    /// What the host's keyboard held, as a host last told ([`ViewState::set_keys`], ADR 1762).
    keys: Keys,
    /// The requests scripts made of the host's console, in order, until a host takes them; at
    /// most [`MAX_CONSOLE_REQUESTS`] (ADR 1762).
    console: Vec<ConsoleRequest>,
    /// Every page's words as `getPageNthWord` reads them, once a script of the document has
    /// spelled either member: `None` per page past what the reading reached (ADR 1762).
    words: Option<Vec<Option<Vec<String>>>>,
    /// The outline as a realm is told it, once a script has needed it, and the items scripts
    /// executed (ADR 1934).
    pub(super) bookmarks: super::script_bookmarks::Bookmarks,
}

/// Most console requests a view state holds for a host: a script that clears the console in a
/// loop has asked for its last clear.
const MAX_CONSOLE_REQUESTS: usize = 16;

/// Most pages whose words are read for a document's scripts, and the longest the reading may
/// take: a page's words are its interpretation's text, so reading them costs what drawing the page
/// does, and the reading stops at whichever bound it meets first (ADR 1762).
const MAX_WORD_PAGES: usize = 1024;

/// The longest the reading of a document's words may take; see [`MAX_WORD_PAGES`].
const MAX_WORD_TIME: Duration = Duration::from_secs(2);

/// Most words read of one page.
const MAX_PAGE_WORDS: usize = 1 << 16;

/// Most changes to the window's view a view state holds for a host: a script that sets the zoom
/// in a loop has asked for its last one, and four of each kind is more than any script needs to
/// say where it wants the window (ADR 1736).
const MAX_VIEW_CHANGES: usize = 16;

/// A run its runner held rather than finished, and what its late outcome needs to be applied.
#[derive(Debug, Clone, PartialEq)]
struct Pending {
    /// Where the script ran.
    site: ScriptSite,
    /// The event's field, or empty.
    field: String,
    /// For a commit's keystroke or validation, what each widget showed before the typing began:
    /// what a late refusal puts back.
    before: Option<Vec<(ObjectId, Before)>>,
}

/// Most held runs a view state keeps a record of; the runner's own queue is shorter.
const MAX_PENDING: usize = 256;

/// The parts of a view state a save writes, as the last save wrote them (ADR 1626).
///
/// A copy rather than a counter for [`Told`]'s reason: a comparison cannot miss a site that
/// changes one of them.
#[derive(Debug, Clone, Default, PartialEq)]
struct Saved {
    /// [`ViewState`]'s `edited`.
    edited: BTreeMap<ObjectId, Entry>,
    /// [`ViewState`]'s `imported`.
    imported: BTreeMap<ObjectId, Import>,
    /// [`ViewState`]'s `reset`.
    reset: BTreeSet<ObjectId>,
    /// [`ViewState`]'s `added`.
    added: Vec<super::Added>,
    /// [`ViewState`]'s `retyped`.
    retyped: BTreeMap<ObjectId, String>,
    /// [`ViewState`]'s `filed`.
    filed: Vec<super::Filed>,
    /// [`ViewState`]'s `unfiled`.
    unfiled: Vec<Vec<u8>>,
    /// The properties scripts set that a save writes.
    drawn: BTreeMap<ObjectId, Vec<Property>>,
    /// The `hidden` scripts set on annotations, which a save writes.
    annotations_hidden: BTreeMap<ObjectId, bool>,
    /// The popups scripts opened or closed, as a save writes them.
    popups_opened: BTreeMap<ObjectId, bool>,
}

/// The statements about field values and visibility the realm was last told of: what the delta of
/// the next event is measured against.
///
/// A copy of the four maps an edit, an import, a reset or a hide changes, rather than a counter
/// each of them bumps: a comparison cannot miss a site that writes one, and the maps are as large
/// as the edits a person and a script made, not as the form.
#[derive(Debug, Clone, Default, PartialEq)]
struct Told {
    /// [`ViewState`]'s `edited`.
    edited: BTreeMap<ObjectId, Entry>,
    /// [`ViewState`]'s `imported`.
    imported: BTreeMap<ObjectId, Import>,
    /// [`ViewState`]'s `reset`.
    reset: BTreeSet<ObjectId>,
    /// [`ViewState`]'s `hidden`.
    hidden: BTreeSet<ObjectId>,
    /// [`ViewState`]'s `shown`.
    shown: BTreeSet<ObjectId>,
    /// The properties scripts set.
    overrides: BTreeMap<String, Overrides>,
    /// §8.11's states, which the realm's layers are read from.
    optional_content: Option<crate::optional_content::OptionalContent>,
    /// The annotations, as the realm was last told of them (ADR 1700).
    annotations: Vec<super::AnnotationState>,
    /// Every field name the realm has been told of: a table read for one page at the open names
    /// fewer fields than one read for the whole document later, and a field first met in the
    /// wider one is told then (ADR 1653 section 4).
    names: BTreeSet<String>,
    /// Whether the pages' words had been read when the realm was told (ADR 1762).
    words: bool,
    /// Whether the outline had been read when the realm was told (ADR 1934).
    outline: bool,
}

/// What applying one result's edits did, for the sequence that ran it.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Applied {
    /// Whether a field's value changed.
    pub(super) values: bool,
    /// Whether the script asked for `/CO` to be walked again.
    pub(super) calculate: bool,
    /// Whether what a page draws may have changed: a value, a property, a layer, an annotation,
    /// a reset, a choice, or a walk of `/CO` that may write any of them (ADR 1762). A host draws
    /// the page again only where a run answered this.
    pub(super) drawn: bool,
}

/// What Table 199's `/K`, in its typing form, made of a whole value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    /// The characters stand as typed.
    Stands,
    /// A script rejected the keystroke.
    Rejected,
    /// A script rewrote `event.change`, and what it wrote is what the field takes.
    Rewritten(String),
}

/// What Table 199's `/K` made of a choice field's selection (ADR 1786).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Selected {
    /// The selection stands as the person made it.
    Stands,
    /// A script rejected the keystroke.
    Rejected,
    /// A script rewrote `event.change` to the text of this option of Table 234's `/Opt`, which is
    /// what the field selects.
    Chooses(usize),
}

/// What one widget showed before a person began typing into its field.
///
/// The three statements about a value a typed one replaces, kept so that a refused commit puts
/// back exactly what was there — a reset's `/DV`, an import's value, or an earlier edit.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Before {
    /// The widget's earlier edit, if it had one.
    edited: Option<Entry>,
    /// The widget's import, if it had one.
    imported: Option<Import>,
    /// Whether a reset had named it.
    reset: bool,
}

/// What committing a field did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Committed {
    /// Nothing was being typed into the field, so there was nothing to commit.
    Nothing,
    /// The value stands, rewritten where the keystroke script's commit form rewrote it, and is
    /// displayed through the field's format from now on.
    Accepted,
    /// A script refused the value, and the field shows what it showed before the typing began.
    ///
    /// The sentence names the field and says what the script required: Adobe's library raises an
    /// alert here, and a host that shows this has shown it.
    Refused(String),
}

mod calculation;
mod commit;
mod edits;
mod events;
mod keystroke;
mod queries;
mod state;

/// Whether a script's text spells `getPageNumWords` or `getPageNthWord`, the two members that read
/// the pages' words (ADR 1762).
///
/// Read off the text rather than asked at the call, because a request is data handed across the
/// process boundary before the script runs (ADR 1591): a script that builds either name at run
/// time is told no words, and its call is refused with a sentence saying they were not read.
fn spells_words(script: &str) -> bool {
    script.contains("getPageNumWords") || script.contains("getPageNthWord")
}

/// One page's words: its interpretation's text cut at white space (ADR 1762).
///
/// **The word rule is a choice.** §9.4.4 places glyphs and says nothing of words, and
/// §14.8.2.6.2 leaves an untagged page's words to "heuristics based on information such as glyph
/// positioning on the page"; the readback already holds this program's heuristic — a gap along
/// the line past a share of the font's space is a space (`content::text`, ADRs 1502, 1515) — so a
/// word is a maximal run of characters that are not white space in it. Adobe's reference leaves
/// its own word finder undefined; what punctuation `bStrip` removes is the realm's.
fn page_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .take(MAX_PAGE_WORDS)
        .map(str::to_owned)
        .collect()
}

/// What a keystroke's whole value makes of Adobe's `event.change`, `changeEx` and `fieldFull`
/// (ADR 1626).
///
/// The reference's "event properties" page: `fieldFull` is true where the text does not fit —
/// past `charLimit`, or past the room `doNotScroll` leaves — and then `changeEx` is everything the
/// person tried to enter and `change` what fits; for a list or combo box `changeEx` is the export
/// value of the change. A host hands a whole value, so what fits is its longest prefix within
/// Table 232's `/MaxLen` characters and within the rectangle Table 231's `DoNotScroll` holds the
/// field to — the prefix [`ViewState::set_field`] already accepts under that flag. Where the field
/// is not full, a text field's `changeEx` is its change: a documented choice, since the page
/// defines the property for text fields only where they are full.
struct Typed {
    /// `event.change`: the characters the field takes.
    change: String,
    /// `event.changeEx`.
    change_ex: String,
    /// `event.fieldFull`.
    full: bool,
}

impl Typed {
    /// The three for `text` typed into the field whose widgets are `taking`.
    fn of(document: &Document, taking: &[ObjectId], widget: &Dictionary, text: &str) -> Self {
        let field = crate::appearance::Field::read(document, widget, super::FieldValue::Stored);
        match field.kind {
            Some(crate::appearance::FieldKind::Text) => {
                let mut fits = super::fields::accepted(document, taking, text);
                let limit = field
                    .ancestry
                    .iter()
                    .find_map(|dictionary| document.get_key(dictionary, "MaxLen").as_integer())
                    .and_then(|limit| usize::try_from(limit).ok());
                if let Some(limit) = limit
                    && fits.chars().count() > limit
                {
                    fits = fits.chars().take(limit).collect();
                }
                let full = fits.len() < text.len();
                Self {
                    change: if full { fits } else { text.to_owned() },
                    change_ex: text.to_owned(),
                    full,
                }
            }
            Some(crate::appearance::FieldKind::Choice { .. }) => Self {
                change: text.to_owned(),
                change_ex: export_value(document, &field.ancestry, text)
                    .unwrap_or_else(|| text.to_owned()),
                full: false,
            },
            _ => Self {
                change: text.to_owned(),
                change_ex: text.to_owned(),
                full: false,
            },
        }
    }
}

/// The export value of the option of Table 233's `/Opt` whose text a person sees is `shown`:
/// the first string of a two-string entry, `None` where no entry is a pair showing it.
fn export_value(document: &Document, ancestry: &[Dictionary], shown: &str) -> Option<String> {
    let options = ancestry
        .iter()
        .map(|dictionary| document.get_key(dictionary, "Opt"))
        .find(|value| !matches!(value, Object::Null))?;
    let Object::Array(entries) = options else {
        return None;
    };
    entries.iter().find_map(|entry| {
        let Object::Array(pair) = document.resolve(entry) else {
            return None;
        };
        let text = |index: usize| match pair.get(index).map(|item| document.resolve(item)) {
            Some(Object::String(bytes)) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        };
        (text(1)? == shown).then(|| text(0)).flatten()
    })
}

impl<'a> ScriptEvent<'a> {
    /// An event at `site` on `field` with nothing else stated: no script, no value, no change, no
    /// field told, page one of one.
    #[must_use]
    pub fn at(site: ScriptSite, field: &'a str) -> Self {
        Self {
            site,
            field,
            label: "",
            script: "",
            value: "",
            change: "",
            selection: (0, 0),
            will_commit: false,
            commit_key: None,
            field_full: false,
            change_ex: "",
            source: "",
            fields: &[],
            page: 0,
            pages: 1,
            dirty: false,
            document: None,
            view: WindowView::default(),
            keys: Keys::default(),
            rich_value: "",
        }
    }
}

/// Every key whose value differs between two maps, either way.
fn changed_keys<K: Ord + Clone, V: PartialEq>(
    now: &BTreeMap<K, V>,
    then: &BTreeMap<K, V>,
    out: &mut BTreeSet<K>,
) {
    for (key, value) in now {
        if then.get(key) != Some(value) {
            out.insert(key.clone());
        }
    }
    for key in then.keys() {
        if !now.contains_key(key) {
            out.insert(key.clone());
        }
    }
}

/// Table 224's `/CO`, as the fields it references, in order.
///
/// "An array of indirect references to field dictionaries with calculation actions" — so an entry
/// that is not a reference names no field dictionary and is passed over.
fn calculation_order(document: &Document) -> Vec<ObjectId> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return Vec::new();
    };
    let Object::Array(entries) = document.get_key(form, "CO") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(Object::as_reference)
        .take(MAX_CALCULATIONS)
        .collect()
}

/// Table 221's `/JS` of the ECMAScript action a field states for `trigger`, as text.
///
/// The walk `site::of_widget` makes — the widget, then its `/Parent` chain, nearest first, the
/// first `/AA` that states the trigger owning it — for the text that walk does not carry. `None`
/// where the action is not ECMAScript, where Table 196's `/Next` makes the trigger more than this
/// one action, or where the text cannot be read or is longer than [`MAX_SCRIPT_BYTES`]: each of
/// those stays reported as Tier 0 reports it.
fn supplied_script(document: &Document, widget: &Dictionary, trigger: Trigger) -> Option<String> {
    let mut current = widget.clone();
    for _ in 0..=crate::appearance::MAX_FIELD_ANCESTRY {
        let additional = document.get_key(&current, "AA");
        if let Some(additional) = additional.as_dict()
            && let Some(entry) = additional.get(trigger.key())
        {
            let Object::Dictionary(action) = document.resolve(entry) else {
                return None;
            };
            let follows = match document.get_key(&action, "Next") {
                Object::Null => false,
                Object::Array(items) => !items.is_empty(),
                _ => true,
            };
            if follows {
                return None;
            }
            return script_text(document, &action);
        }
        current = document.get_key(&current, "Parent").as_dict().cloned()?;
    }
    None
}

/// Table 221's `/JS` of one action dictionary, where it is an ECMAScript action whose text can be
/// read within [`MAX_SCRIPT_BYTES`].
pub(super) fn script_text(document: &Document, action: &Dictionary) -> Option<String> {
    match read_script(document, action) {
        ScriptText::Read(text) => Some(text),
        ScriptText::NotScript | ScriptText::Unread(_) => None,
    }
}

/// What one action dictionary's Table 221 `/JS` reads as.
pub(super) enum ScriptText {
    /// The script's text.
    Read(String),
    /// The action is not an ECMAScript action.
    NotScript,
    /// An ECMAScript action whose text is not run, with the phrase saying why, for a report.
    Unread(String),
}

/// Table 221's `/JS` of one action dictionary: its text, or why an ECMAScript action's text is not
/// run (ADR 1950).
///
/// "A text string or text stream containing the ECMAScript script to be executed": §7.9.2.2's text
/// string either way, read through the same decoding a text string always takes. A script past
/// [`MAX_SCRIPT_BYTES`] is not decoded, and the phrase gives its length, so that the one sentence
/// saying it was not run states the bound and not some other reason (trap 5).
pub(super) fn read_script(document: &Document, action: &Dictionary) -> ScriptText {
    if !matches!(document.get_key(action, "S"), Object::Name(ref name) if name.as_bytes() == b"JavaScript")
    {
        return ScriptText::NotScript;
    }
    let unreadable = || {
        ScriptText::Unread(String::from(
            "an ECMAScript action whose /JS is neither a text string nor a text stream this \
             program can decode",
        ))
    };
    let Some(entry) = action.get("JS") else {
        return unreadable();
    };
    let bytes = match document.resolve(entry) {
        Object::String(bytes) => bytes.to_vec(),
        Object::Stream(stream) => match document.decoded_stream_data(&stream) {
            Some(data) => data.to_vec(),
            None => return unreadable(),
        },
        _ => return unreadable(),
    };
    if bytes.len() > MAX_SCRIPT_BYTES {
        return ScriptText::Unread(format!(
            "an ECMAScript action of {} bytes, past the {MAX_SCRIPT_BYTES} a script may be in this \
             program",
            bytes.len()
        ));
    }
    ScriptText::Read(pdf_syntax::text_string(&bytes))
}
