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
    CommitKey, DocumentState, FieldState, Overrides, Property, ScriptEdit, ScriptSite,
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
    /// The name a document-level script carries in Table 32's tree, Adobe's `event.targetName` at
    /// `Doc/Open`; empty for every other site.
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
}

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
    /// The popups scripts opened or closed, which a save writes.
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
}

/// What applying one result's edits did, for the sequence that ran it.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Applied {
    /// Whether a field's value changed.
    pub(super) values: bool,
    /// Whether the script asked for `/CO` to be walked again.
    pub(super) calculate: bool,
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

impl ViewState {
    /// Commits what a person typed into a field: Table 199's `/K` in its commit form, then `/V`,
    /// then Table 224's `/CO`, then `/F` wherever a runner formats.
    ///
    /// What a host calls when a person leaves the field or presses Enter — the moment Adobe's
    /// "Form event processing" sets `willCommit`. Where the keystroke's commit form or the validate
    /// script refuses — `event.rc` set false — the field goes back to what it showed before the
    /// typing began and the answer says so; where the keystroke's commit form rewrites the value —
    /// a comma decimal stored with a period, an arbitrary mask's literals put back — the rewrite is
    /// the value. `/CO` is walked after either outcome (ADRs 1579, 1603).
    ///
    /// A script that is not one call of the library, with no runner supplied, is not run and is
    /// reported in [`Self::script_reports`]; the value stands, because a validation nobody ran
    /// refused nothing.
    pub fn commit_field(&mut self, document: &Document, name: &str) -> Committed {
        self.commit_field_by(document, name, CommitKey::Enter)
    }

    /// [`Self::commit_field`], with the way the person committed stated: Adobe's
    /// `event.commitKey`, which the commit's keystroke, its validation and its field's format are
    /// handed (ADR 1626).
    ///
    /// [`Self::commit_field`] is this with [`CommitKey::Enter`], a documented choice for a host
    /// that does not say how the commit came (ADR 1626); a host that knows it was a click or a Tab
    /// says so here.
    #[expect(
        clippy::too_many_lines,
        reason = "one commit's triggers in the reference's order — the keystroke's commit form,                   then validate, each a one-call arm and a runner's arm with the same refusal —                   read top to bottom as the event model they implement"
    )]
    pub fn commit_field_by(
        &mut self,
        document: &Document,
        name: &str,
        key: CommitKey,
    ) -> Committed {
        let Some(before) = self.uncommitted.remove(name) else {
            return Committed::Nothing;
        };
        let widgets: Vec<ObjectId> = before.iter().map(|(widget, _)| *widget).collect();
        let Some(first) = widgets.first().copied() else {
            return Committed::Nothing;
        };
        let Some(widget) = document.get(first).as_dict().cloned() else {
            return Committed::Nothing;
        };
        let table = super::widgets_by_field_name(document);
        let mut text = self.text_of(document, first).unwrap_or_default();

        match site::of_widget(document, &widget, Trigger::Keystroke) {
            Site::Library(call) => {
                let at = text.len();
                let event = Keystroke {
                    value: &text,
                    change: "",
                    selection: (at, at),
                    will_commit: true,
                };
                match call.keystroke(&event) {
                    Ok(Keyed::Accepted {
                        value: Some(rewritten),
                    }) => {
                        self.write_text(&widgets, &rewritten);
                        text = rewritten;
                    }
                    Ok(Keyed::Accepted { value: None }) => {}
                    Ok(Keyed::Rejected { message }) => {
                        return self.refuse_commit(
                            document,
                            &table,
                            before,
                            format!(
                                "{name}: {}",
                                message.unwrap_or_else(|| {
                                    "the field's keystroke script refused the value".to_owned()
                                })
                            ),
                        );
                    }
                    Err(refusal) => self.report(format!(
                        "{name}: the field's keystroke script {} did not run: {refusal}",
                        call.function.name()
                    )),
                }
            }
            Site::NotRun(sentence) => {
                let at = text.len();
                let event = ScriptEvent {
                    value: &text,
                    selection: (at, at),
                    will_commit: true,
                    commit_key: Some(key),
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Keystroke), name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
                        self.hold_before(name, &before);
                        self.report_each(name, result.report);
                        if !result.rc {
                            return self.refuse_commit(
                                document,
                                &table,
                                before,
                                format!("{name}: the field's keystroke script refused the value"),
                            );
                        }
                        if let Some(rewritten) = result.value {
                            self.write_text(&widgets, &rewritten);
                            text = rewritten;
                        }
                    }
                }
            }
            Site::Absent => {}
        }

        match site::of_widget(document, &widget, Trigger::Validate) {
            Site::Library(call) => match call.validate(&text) {
                Ok(Some(message)) => {
                    return self.refuse_commit(
                        document,
                        &table,
                        before,
                        format!("{name}: {message}"),
                    );
                }
                Ok(None) => {}
                Err(refusal) => self.report(format!(
                    "{name}: the field's validate script {} did not run: {refusal}",
                    call.function.name()
                )),
            },
            Site::NotRun(sentence) => {
                // Adobe's "event properties" says both that a validate event does not listen to
                // `rc` and that setting it false invalidates the value; the second is the sentence
                // with an effect, and RFC 0008 section 6.5 takes it (ADR 1603). A value the script
                // writes into `event.value` is not taken: the event validates a value, it does not
                // produce one.
                let event = ScriptEvent {
                    value: &text,
                    selection: (text.len(), text.len()),
                    will_commit: true,
                    commit_key: Some(key),
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Validate), name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
                        self.hold_before(name, &before);
                        self.report_each(name, result.report);
                        if !result.rc {
                            return self.refuse_commit(
                                document,
                                &table,
                                before,
                                format!(
                                    "{name}: the field's validate script refused the value, so \
                                     the field shows what it showed before"
                                ),
                            );
                        }
                    }
                }
            }
            Site::Absent => {}
        }

        self.recalculate_scripts(document, &table, name);
        self.scripting.committing = Some((name.to_owned(), key));
        self.refresh_formatted(document, &table);
        self.scripting.committing = None;
        Committed::Accepted
    }

    /// Puts back what a refused commit replaced, recalculates, and answers the refusal.
    fn refuse_commit(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        before: Vec<(ObjectId, Before)>,
        sentence: String,
    ) -> Committed {
        self.revert(before);
        self.recalculate(document, table);
        self.refresh_formatted(document, table);
        Committed::Refused(sentence)
    }

    /// Hands every script Tier 0 does not run to `runner`, or, with `None`, to nothing — RFC 0008
    /// section 6.3's level `off`, which is where every view state starts.
    ///
    /// The one place a host's level for scripts reaches this crate (ADR 1591): what the runner may
    /// reach, and whether it asks first, is the host's and the runner's, never decided here. A new
    /// runner holds a new realm, so the next event tells it of every field again (ADR 1602).
    pub fn run_scripts_with(&mut self, runner: Option<Arc<dyn ScriptRunner>>) {
        self.runner = Runner(runner);
        self.scripting.told = None;
        self.scripting.formatted.clear();
    }

    /// Walks Table 224's `/CO` whole, every script's `/C` through the runner, then every format the
    /// runner runs: what a host calls when a runner arrives for a document whose values were set
    /// without one, so that a calculated field shows what its script computes from the values
    /// already there (ADR 1616).
    pub fn recalculate_with_runner(&mut self, document: &Document) {
        if self.runner.0.is_none() {
            return;
        }
        // A runner arrives as the document opens, so the walk asks the calculation order's own
        // entries for the roots `/Fields` omits rather than every page (ADR 1653 section 4).
        let table = super::field_table(document, super::Omitted::Calculation);
        self.recalculate_scripts(document, &table, "");
        self.refresh_formatted(document, &table);
    }

    /// What Tier 0's dispatch did not run, and what every runner's script said, each sentence
    /// once, in the order it was met.
    ///
    /// A field's script that is not one call of the library, a call whose arguments the library
    /// refuses, a `/CO` entry whose `/C` is either, a runner's refusals, throws and stops — each
    /// named with its field, never raised as a dialog and never repeated (RFC 0008 section 6.8).
    #[must_use]
    pub fn script_reports(&self) -> &[String] {
        &self.script_reports
    }

    /// The properties scripts have set on one field as a whole, the latest of each member, in the
    /// order they were set; a member set through one widget's `Field` is that widget's alone and is
    /// drawn from [`super::AnnotationView::scripted`] (ADR 1664).
    ///
    /// What a host drawing its own control over a field reads beside the value. A script's
    /// `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment` and `charLimit` are
    /// drawn into the page's appearance as well and `required` is saved (ADR 1617); `display` and
    /// `readonly` are applied as well as kept.
    #[must_use]
    pub fn script_properties(&self, name: &str) -> &[Property] {
        self.scripting
            .overrides
            .get(name)
            .map_or(&[], |overrides| overrides.set.as_slice())
    }

    /// The widget a script's `setFocus` last asked the keyboard focus for, taken: `None` once a
    /// host has taken it, and where no script asked.
    ///
    /// The focus is the host's — which widget a key reaches, and the page turned or the view
    /// scrolled to show it, as Adobe's "Field methods" page describes `setFocus` — so a view state
    /// holds the request and a host carries it out after any call that ran scripts, raising Table
    /// 197's `/Bl` and `/Fo` as a press would (ADR 1615). The latest request stands: a script that
    /// asks twice has asked for the second. The widget is the one the script's `Field` stood for
    /// (ADR 1688).
    pub fn take_focus_request(&mut self) -> Option<FocusRequest> {
        self.scripting.focus.take()
    }

    /// The zero-based page a script's `this.pageNum = n` last turned to, taken: `None` once a host
    /// has taken it, and where no script turned one.
    ///
    /// Which page is shown is the host's, as the focus is, so a view state holds the request and a
    /// host carries it out after any call that ran scripts, as the page turn a person asks for —
    /// Table 198's `/C` of the page left and `/O` of the page reached run as they would for a
    /// person's turn. The page is one of the document's, checked when the script's run was applied;
    /// the latest request stands (ADR 1640).
    pub fn take_page_request(&mut self) -> Option<usize> {
        self.scripting.page.take()
    }

    /// What one field displays: its value through its format script, or as it stands.
    ///
    /// [`Self::field_value`] answers with the characters a host edits; this answers with what the
    /// page shows when nobody is editing them — Table 199's `/F` "performed before the field is
    /// formatted to display its value" — which is what an assistive technology reading an
    /// unfocused field, or a host drawing its own control over one, wants. A field being typed
    /// into displays as typed. `None` in [`Self::field_value`]'s cases, and a password field
    /// answers with its echo, unformatted (ADR 1579).
    ///
    /// A format a runner ran at the last commit or open is answered from what it displayed then,
    /// which is what the page draws; one it has not run is run now, told no field, and nothing it
    /// says is recorded — this answers a question (ADR 1603).
    #[must_use]
    pub fn displayed_value(&self, document: &Document, name: &str) -> Option<String> {
        self.displayed_values(document, [name]).pop().flatten()
    }

    /// [`Self::displayed_value`] for each of several fields, with §12.7.4.1's field tree walked
    /// once for all of them rather than once for each.
    ///
    /// What a host placing a control over every field of a page asks, on every repaint: one walk
    /// per field made that question grow with the square of the form — 0.54 ms became 5.2 ms on
    /// `160F-2019.pdf`'s first page, the corpus's most formatted, in `viewer-core`'s
    /// `examples/fields_cost.rs` (ADR 1604).
    #[must_use]
    pub fn displayed_values<'n>(
        &self,
        document: &Document,
        names: impl IntoIterator<Item = &'n str>,
    ) -> Vec<Option<String>> {
        // Asked on every repaint, so the pages are walked for a root `/Fields` omits only where a
        // name asked is not a field it lists — a document that has one on screen (ADR 1653).
        let names: Vec<&str> = names.into_iter().collect();
        let mut table = super::field_table(document, super::Omitted::Listed);
        if names.iter().any(|name| !table.contains_key(*name)) {
            table = super::widgets_by_field_name(document);
        }
        names
            .into_iter()
            .map(|name| self.displayed_in(document, &table, name))
            .collect()
    }

    /// One field's displayed value, read through a table of the field tree the caller walked.
    fn displayed_in(
        &self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        name: &str,
    ) -> Option<String> {
        let widget = table.get(name)?.first().copied()?;
        let object = document.get(widget);
        let dictionary = object.as_dict()?;
        // `Self::field_value`'s reading, from the same table.
        let shown = crate::appearance::field_text_value(
            document,
            dictionary,
            self.annotation(widget).value,
        )?;
        if shown.obscured || self.is_editing(widget) {
            return Some(shown.text);
        }
        Some(
            match site::of_widget(document, dictionary, Trigger::Format) {
                Site::Library(call) => call
                    .format(&shown.text)
                    .map_or(shown.text, |formatted| formatted.text),
                Site::NotRun(_) => {
                    if let Some(displayed) = self.scripting.formatted.get(&widget)
                        && displayed.value == shown.text
                    {
                        return Some(displayed.shown.clone());
                    }
                    let at = shown.text.len();
                    let event = ScriptEvent {
                        value: &shown.text,
                        selection: (at, at),
                        ..ScriptEvent::at(ScriptSite::Field(Trigger::Format), name)
                    };
                    self.runner
                        .0
                        .as_ref()
                        .zip(supplied_script(document, dictionary, Trigger::Format))
                        .and_then(|(runner, script)| {
                            runner
                                .run(&ScriptEvent {
                                    script: &script,
                                    pages: crate::page::Pages::new(document).len(),
                                    ..event
                                })
                                .value
                        })
                        .unwrap_or(shown.text)
                }
                Site::Absent => shown.text,
            },
        )
    }

    /// Hands a field event to the runner with the field's own script for its trigger, where there
    /// is a runner and the trigger's action is one ECMAScript action whose text can be read.
    fn run_field_script(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        widget: &Dictionary,
        event: ScriptEvent<'_>,
    ) -> Option<ScriptResult> {
        let ScriptSite::Field(trigger) = event.site else {
            return None;
        };
        self.runner.0.as_ref()?;
        let script = supplied_script(document, widget, trigger)?;
        self.run_event(
            document,
            table,
            ScriptEvent {
                script: &script,
                ..event
            },
        )
        .map(|(result, _)| result)
    }

    /// Hands one event to the runner, telling it of every field whose state changed since it last
    /// heard, and applies the edits its result carries.
    ///
    /// `None` where no runner is supplied. The result's `value`, `change` and `rc` are the
    /// caller's to read, because what they mean is the site's.
    pub(super) fn run_event(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        event: ScriptEvent<'_>,
    ) -> Option<(ScriptResult, Applied)> {
        let runner = self.runner.0.clone()?;
        let (fields, whole) = self.tell(document, table);
        let page = match event.site {
            ScriptSite::Field(_) => self
                .field_page(document, table, event.field)
                .unwrap_or(event.page),
            _ => event.page,
        };
        let result = runner.run(&ScriptEvent {
            fields: &fields,
            page,
            pages: crate::page::Pages::new(document).len(),
            dirty: self.unsaved(),
            document: whole.as_ref(),
            ..event
        });
        if runner.waiting() && self.scripting.pending.len() < MAX_PENDING {
            self.scripting.pending.push(Pending {
                site: event.site,
                field: event.field.to_owned(),
                before: None,
            });
        }
        let applied = self.apply_edits(document, table, &result.edits);
        Some((result, applied))
    }

    /// Every page's zero-based index by its object, walked once (ADR 1640).
    pub(super) fn page_indices(&mut self, document: &Document) -> &BTreeMap<ObjectId, usize> {
        self.scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
    }

    /// The zero-based page Table 166's `/P` names for a field's first widget: the page a person is
    /// on when they type into the field, and so what `this.pageNum` reads at its own events, which
    /// a script's page turn counts from (ADR 1640). `None` where the widget names no page.
    fn field_page(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        name: &str,
    ) -> Option<usize> {
        let widget = *table.get(name)?.first()?;
        let page = document
            .get(widget)
            .as_dict()?
            .get("P")
            .and_then(Object::as_reference)?;
        self.scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
            .get(&page)
            .copied()
    }

    /// Every field whose state differs from what the realm was last told, read now, and the
    /// document as a whole where it differs too; the realm counted as told of both.
    fn tell(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) -> (Vec<FieldState>, Option<DocumentState>) {
        let annotations = self.annotation_states(document);
        let whole = match self.scripting.told.as_deref() {
            Some(told)
                if told.optional_content == self.optional_content
                    && told.annotations == annotations =>
            {
                None
            }
            _ => Some(DocumentState {
                annotations: annotations.clone(),
                ..self.document_state(document)
            }),
        };
        let names: BTreeSet<&String> = match self.scripting.told.as_deref() {
            None => table.keys().collect(),
            Some(told) => {
                let mut widgets = BTreeSet::new();
                changed_keys(&self.edited, &told.edited, &mut widgets);
                changed_keys(&self.imported, &told.imported, &mut widgets);
                widgets.extend(self.reset.symmetric_difference(&told.reset).copied());
                widgets.extend(self.hidden.symmetric_difference(&told.hidden).copied());
                widgets.extend(self.shown.symmetric_difference(&told.shown).copied());
                let mut changed: BTreeSet<&String> = BTreeSet::new();
                let mut properties = BTreeSet::new();
                changed_keys(&self.scripting.overrides, &told.overrides, &mut properties);
                for (name, held) in table {
                    if properties.contains(name)
                        || held.iter().any(|w| widgets.contains(w))
                        || !told.names.contains(name)
                    {
                        changed.insert(name);
                    }
                }
                changed
            }
        };
        if names.is_empty() && whole.is_none() {
            return (Vec::new(), None);
        }
        let pages = self
            .scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
            .clone();
        let mut told_names = self
            .scripting
            .told
            .as_deref()
            .map(|told| told.names.clone())
            .unwrap_or_default();
        told_names.extend(names.iter().map(|name| (*name).clone()));
        let states = names
            .into_iter()
            .filter_map(|name| {
                let widgets = table.get(name)?;
                self.field_state(document, name, widgets, &pages)
            })
            .collect();
        self.scripting.told = Some(Box::new(Told {
            edited: self.edited.clone(),
            imported: self.imported.clone(),
            reset: self.reset.clone(),
            hidden: self.hidden.clone(),
            shown: self.shown.clone(),
            overrides: self.scripting.overrides.clone(),
            optional_content: self.optional_content.clone(),
            annotations,
            names: told_names,
        }));
        (states, whole)
    }

    /// Applies a script's edits as edits a person could have made, and says what they changed.
    ///
    /// A value goes into the edit log beside a typed one; `display` into the overrides beside
    /// §12.6.4.11's hide; a reset is §12.7.6.3's; and every property is kept by field name, read
    /// back by the realm and by [`Self::script_properties`]. A field a person is typing into is
    /// not written under them, which is the rule a calculation already keeps (ADR 1603).
    fn apply_edits(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        edits: &[ScriptEdit],
    ) -> Applied {
        let mut applied = Applied::default();
        for edit in edits {
            match edit {
                ScriptEdit::Value { field, value } => {
                    let Some(widgets) = table.get(field) else {
                        continue;
                    };
                    if widgets.iter().any(|widget| self.is_editing(*widget)) {
                        self.report(format!(
                            "{field}: a script set the value while a person was typing into the \
                             field, and what they are typing stands"
                        ));
                        continue;
                    }
                    let differs = widgets
                        .iter()
                        .any(|widget| self.text_of(document, *widget).as_deref() != Some(value));
                    if differs {
                        self.write_text(widgets, value);
                        applied.values = true;
                    }
                }
                ScriptEdit::Property {
                    field,
                    widget,
                    property,
                } => self.apply_property(
                    document,
                    table,
                    (field, *widget),
                    property,
                    &mut applied.values,
                ),
                ScriptEdit::Reset { fields } => {
                    let action = ResetForm {
                        fields: fields.iter().cloned().map(ResetTarget::Name).collect(),
                        exclude: false,
                    };
                    self.reset_form(document, &action);
                    applied.values = true;
                }
                ScriptEdit::Calculate => applied.calculate = true,
                ScriptEdit::Focus { field, widget } => self.ask_focus(table, field, *widget),
                ScriptEdit::Layer {
                    number,
                    generation,
                    on,
                } => self.switch_layer(document, *number, *generation, *on),
                ScriptEdit::Annotation {
                    number,
                    generation,
                    change,
                } => {
                    let annotation = ObjectId {
                        number: *number,
                        generation: *generation,
                    };
                    self.change_annotation(document, annotation, change);
                    applied.values = true;
                }
                ScriptEdit::GoTo { page } => {
                    let pages = crate::page::Pages::new(document).len();
                    match usize::try_from(*page) {
                        Ok(page) if page < pages => self.scripting.page = Some(page),
                        _ => self.report(format!(
                            "a script turned to page {}, and this document has {pages}, so no \
                             page is turned (ADR 1640)",
                            u64::from(*page).saturating_add(1)
                        )),
                    }
                }
                ScriptEdit::Timer {
                    id,
                    script,
                    period,
                    repeat,
                } => {
                    if let Err(refused) = self.scripting.timers.set(*id, script, *period, *repeat) {
                        self.report(refused);
                    }
                }
                ScriptEdit::ClearTimer { id } => self.scripting.timers.clear(*id),
                ScriptEdit::Beep { sound } => self.scripting.timers.beep(*sound),
            }
        }
        applied
    }

    /// Holds a script's `setFocus` for a host, on the widget it names or on its field's first, or
    /// reports why no widget can take it (ADRs 1615, 1688).
    fn ask_focus(
        &mut self,
        table: &BTreeMap<String, Vec<ObjectId>>,
        field: &str,
        widget: Option<u32>,
    ) {
        let Some(widgets) = table.get(field) else {
            self.report(format!(
                "a script asked for the focus on {field}, which is not a field of this document"
            ));
            return;
        };
        let index = widget.map_or(0, |index| usize::try_from(index).unwrap_or(usize::MAX));
        if index >= widgets.len() {
            self.report(format!(
                "{field}: a script asked for the focus on its widget {index}, and the field has \
                 {} widget(s), so the focus stays where it is (ADR 1688)",
                widgets.len()
            ));
            return;
        }
        self.scripting.focus = Some(FocusRequest {
            field: field.to_owned(),
            widget: index,
        });
    }

    /// Applies one property a script set, on the widget it names or on every widget of its field,
    /// and sets `values` where the page has to be drawn again (ADRs 1603, 1617, 1664).
    fn apply_property(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        (field, widget): (&str, Option<u32>),
        property: &Property,
        values: &mut bool,
    ) {
        let Some(all) = table.get(field) else {
            return;
        };
        // A widget-level member set through one widget's `Field` reaches that widget
        // alone; every other member is the field's (ADR 1664).
        let widget = widget.filter(|_| property.is_widget_level());
        let widgets: Vec<ObjectId> = match widget {
            None => all.clone(),
            Some(index) => {
                let Some(one) = usize::try_from(index).ok().and_then(|index| all.get(index)) else {
                    self.report(format!(
                        "{field}: a script set Field.{} on its widget {index}, and \
                         the field has {} widget(s), so nothing is set (ADR 1664)",
                        property.member(),
                        all.len()
                    ));
                    return;
                };
                vec![*one]
            }
        };
        // A style is a ZapfDingbats code, and in any other font the same code is a
        // letter: drawn there, it would be a mark the script did not ask for (ADR
        // 1665).
        if let Property::Style(glyph) = property
            && !widgets.iter().all(|widget| {
                document.get(*widget).as_dict().is_some_and(|dictionary| {
                    crate::appearance::draws_dingbats(document, dictionary)
                })
            })
        {
            self.report(format!(
                "{field}: a script set Field.style to {}, and the field's /DA font is \
                 not ZapfDingbats, in which alone the style's code is its glyph; the \
                 widget is drawn as before (ADR 1665)",
                glyph.adobe()
            ));
            return;
        }
        let member = property.member();
        if widget.is_some() {
            self.scripting
                .one_widget
                .extend(widgets.iter().map(|widget| (*widget, member)));
        } else {
            for widget in &widgets {
                self.scripting.one_widget.remove(&(*widget, member));
            }
        }
        self.scripting
            .overrides
            .entry(field.to_owned())
            .or_default()
            .record(widget, property.clone());
        match property {
            Property::Display(display) => {
                for widget in &widgets {
                    self.set_hidden(*widget, !display.on_screen());
                }
            }
            // Table 227 bit 1 bars a *user*, and `set_field` is where a user's value
            // arrives, so that is where the override is read.
            Property::ReadOnly(_) => {}
            // What the appearance draws, and `required`, which a save writes: kept
            // per widget for `AnnotationView::scripted`, and the page drawn again
            // (ADR 1617).
            // A caption is Table 192's `/CA`, `/AC` or `/RC`, drawn and saved the same
            // way (ADR 1626).
            Property::TextColor(_)
            | Property::FillColor(_)
            | Property::StrokeColor(_)
            | Property::BorderStyle(_)
            | Property::Alignment(_)
            | Property::CharLimit(_)
            | Property::Required(_)
            | Property::Caption(..)
            | Property::Style(_) => {
                for widget in &widgets {
                    let held = self.scripting.drawn.entry(*widget).or_default();
                    held.retain(|kept| !kept.replaces(property));
                    held.push(property.clone());
                }
                *values = true;
            }
            other @ Property::TextFlag(..) => self.report(format!(
                "{field}: a script set Field.{}; this view state keeps it and the \
                 drawn appearance does not carry it (ADR 1603)",
                other.member()
            )),
        }
    }

    /// A script's `OCG.state = …`, made as a person's layer switch makes it, or reported where no
    /// person's switch could make it (ADR 1626).
    ///
    /// Table 99's `/Locked` says a locked group's state "cannot be changed through the user
    /// interface", and its next sentence leaves the rest to the processor: one "may allow the
    /// states of optional content groups to be changed by means other than the user interface,
    /// such as ECMAScript". This program takes the narrower reading for a document's scripts — a
    /// script reaches what the person reading could do by hand (RFC 0008 section 4.2) — so the
    /// change goes through [`ViewState::set_group`], and a locked group stays as it is.
    fn switch_layer(&mut self, document: &Document, number: u32, generation: u16, on: bool) {
        let group = ObjectId { number, generation };
        if self.set_group(group, on) {
            return;
        }
        let name = self
            .optional_content
            .as_ref()
            .and_then(|content| content.name(document, group))
            .unwrap_or_else(|| format!("object {number}"));
        let already = self
            .optional_content
            .as_ref()
            .and_then(|content| content.state(group))
            == Some(on);
        if already {
            return;
        }
        let why = if self
            .optional_content
            .as_ref()
            .is_some_and(|content| content.is_locked(group))
        {
            "Table 99's /Locked names it, and a script switches only what the person reading \
             could switch by hand (ADR 1626)"
        } else {
            "it is not a group whose state this document's configuration lets a switch change"
        };
        self.report(format!(
            "a script set the layer {name:?} {}, and it stays as it was: {why}",
            if on { "on" } else { "off" }
        ));
    }

    /// Records each of a runner's sentences against the field it ran for.
    pub(super) fn report_each(&mut self, name: &str, sentences: Vec<String>) {
        for sentence in sentences {
            self.report(format!("{name}: {sentence}"));
        }
    }

    /// The properties scripts set on this widget's field that its appearance draws (ADR 1617).
    pub(super) fn scripted(&self, annotation: ObjectId) -> &[Property] {
        self.scripting
            .drawn
            .get(&annotation)
            .map_or(&[], Vec::as_slice)
    }

    /// Whether this widget's field is being typed into and has not been committed.
    pub(super) fn is_editing(&self, annotation: ObjectId) -> bool {
        self.uncommitted
            .values()
            .any(|widgets| widgets.iter().any(|(widget, _)| *widget == annotation))
    }

    /// Whether a script has set this field read-only (`Some(true)`), writable (`Some(false)`), or
    /// neither.
    pub(super) fn script_read_only(&self, name: &str) -> Option<bool> {
        self.scripting
            .overrides
            .get(name)
            .and_then(Overrides::read_only)
    }

    /// What one widget displays through a runner's format, where one ran for its current value.
    pub(super) fn displayed(&self, annotation: ObjectId) -> Option<&Displayed> {
        self.scripting.formatted.get(&annotation)
    }

    /// What Table 199's `/K`, in its typing form, makes of the characters.
    ///
    /// The host hands a whole value, so the keystroke is that value replacing the whole of the
    /// field's text — what `AFMergeChange` then reassembles is the value itself, and a runner's
    /// rewritten `event.change` is the whole value the field takes.
    pub(super) fn keystroke_verdict(
        &mut self,
        document: &Document,
        taking: &[ObjectId],
        text: &str,
    ) -> Verdict {
        let Some(first) = taking.first().copied() else {
            return Verdict::Stands;
        };
        let Some(widget) = document.get(first).as_dict().cloned() else {
            return Verdict::Stands;
        };
        match site::of_widget(document, &widget, Trigger::Keystroke) {
            Site::Absent => Verdict::Stands,
            Site::NotRun(sentence) => {
                let table = super::widgets_by_field_name(document);
                let name = table
                    .iter()
                    .find(|(_, widgets)| widgets.contains(&first))
                    .map_or_else(
                        || format!("the field of object {}", first.number),
                        |(name, _)| name.clone(),
                    );
                let current = self.text_of(document, first).unwrap_or_default();
                let typed = Typed::of(document, taking, &widget, text);
                let event = ScriptEvent {
                    value: &current,
                    change: &typed.change,
                    change_ex: &typed.change_ex,
                    field_full: typed.full,
                    selection: (0, current.len()),
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Keystroke), &name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => {
                        self.report(format!("{name}: {sentence}"));
                        Verdict::Stands
                    }
                    Some(result) => {
                        self.report_each(&name, result.report);
                        match (result.rc, result.change) {
                            (false, _) => Verdict::Rejected,
                            (true, Some(change)) => Verdict::Rewritten(change),
                            // A full field takes the change cropped to what fits, which is what
                            // the event handed the script as its change (ADR 1626).
                            (true, None) if typed.full => Verdict::Rewritten(typed.change),
                            (true, None) => Verdict::Stands,
                        }
                    }
                }
            }
            Site::Library(call) => {
                let current = self.text_of(document, first).unwrap_or_default();
                let event = Keystroke {
                    value: &current,
                    change: text,
                    selection: (0, current.len()),
                    will_commit: false,
                };
                match call.keystroke(&event) {
                    Ok(Keyed::Accepted { .. }) => Verdict::Stands,
                    Ok(Keyed::Rejected { .. }) => Verdict::Rejected,
                    Err(refusal) => {
                        let name = super::field_name_of(document, first);
                        self.report(format!(
                            "{name}: the field's keystroke script {} did not run: {refusal}",
                            call.function.name()
                        ));
                        Verdict::Stands
                    }
                }
            }
        }
    }

    /// Records what each widget showed before typing began, the first time a field is typed into.
    pub(super) fn begin_typing(&mut self, name: &str, widgets: &[ObjectId]) {
        if self.uncommitted.contains_key(name) {
            return;
        }
        let before = widgets
            .iter()
            .map(|widget| {
                (
                    *widget,
                    Before {
                        edited: self.edited.get(widget).cloned(),
                        imported: self.imported.get(widget).cloned(),
                        reset: self.reset.contains(widget),
                    },
                )
            })
            .collect();
        self.uncommitted.insert(name.to_owned(), before);
    }

    /// Walks Table 224's `/CO` after a keystroke, running each entry's one-call `/C`.
    ///
    /// In the array's order and once — "the calculation order in which their values will be
    /// recalculated" — so a calculation that changes a field earlier in the array does not restart
    /// the walk; Adobe's reference is silent on that re-entrancy and running the order once is what
    /// the array states (ADR 1579). A calculated value is the document changing its own value,
    /// like §12.7.6.3's reset, so Table 227's `ReadOnly` — a bar on the *user* — does not stop it.
    /// A field being typed into is left alone: what a person is typing is not overwritten under
    /// them. A script's `/C` is the commit's ([`Self::recalculate_scripts`]); with no runner it is
    /// reported here as Tier 0 reports it.
    pub(super) fn recalculate(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) {
        self.walk_calculations(document, table, None);
    }

    /// Walks Table 224's `/CO` after a commit, a reset or a script that asked: each entry's
    /// one-call `/C` as [`Self::recalculate`] runs it, and each script's `/C` through the runner
    /// with `event.source` the field that changed (ADR 1603).
    ///
    /// The script's `event.value` is the value the field takes and `event.rc` set false leaves the
    /// field's value as it was. Every sentence a script's run owes is reported with the entry's
    /// position in the order, so a calculation stopped by its budget names where in the chain it
    /// was; the walk as a whole is held to [`MAX_SEQUENCE_TIME`].
    pub(super) fn recalculate_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        source: &str,
    ) {
        self.walk_calculations(document, table, Some(source));
    }

    /// The walk itself: `source` is `None` after a keystroke and the changed field after a commit.
    fn walk_calculations(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        source: Option<&str>,
    ) {
        let order = calculation_order(document);
        let count = order.len();
        let started = Instant::now();
        for (index, field) in order.into_iter().enumerate() {
            let position = index.saturating_add(1);
            if source.is_some() && started.elapsed() > MAX_SEQUENCE_TIME {
                self.report(format!(
                    "the calculation order was stopped at entry {position} of {count}: the walk \
                     ran longer than its budget of {} ms, and the entries from there on were not \
                     recalculated (ADR 1603)",
                    MAX_SEQUENCE_TIME.as_millis()
                ));
                return;
            }
            let Some(dictionary) = document.get(field).as_dict().cloned() else {
                continue;
            };
            let widgets = super::widgets_under(document, field);
            let name = table
                .iter()
                .find(|(_, held)| widgets.iter().any(|widget| held.contains(widget)))
                .map_or_else(
                    || format!("the field of object {}", field.number),
                    |(name, _)| name.clone(),
                );
            match site::of_field(document, &dictionary, Trigger::Calculate) {
                Site::Absent => {}
                Site::NotRun(sentence) => match source {
                    // A script's calculation runs at the commit; typing leaves it to that.
                    None if self.runner.0.is_some() => {}
                    None => self.report(format!("{name}: {sentence}")),
                    Some(source) => {
                        if widgets.iter().any(|widget| self.is_editing(*widget)) {
                            continue;
                        }
                        let value = widgets
                            .first()
                            .and_then(|widget| self.text_of(document, *widget))
                            .unwrap_or_default();
                        let script = supplied_script(document, &dictionary, Trigger::Calculate);
                        let ran = script.as_deref().and_then(|script| {
                            self.run_event(
                                document,
                                table,
                                ScriptEvent {
                                    script,
                                    value: &value,
                                    source,
                                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Calculate), &name)
                                },
                            )
                        });
                        match ran {
                            None => self.report(format!("{name}: {sentence}")),
                            Some((result, _)) => {
                                self.report_each(
                                    &format!(
                                        "{name} (entry {position} of {count} in the calculation \
                                         order)"
                                    ),
                                    result.report,
                                );
                                if result.rc
                                    && let Some(calculated) = result.value
                                {
                                    self.set_calculated(document, &widgets, &calculated);
                                }
                            }
                        }
                    }
                },
                Site::Library(call) => {
                    let result = {
                        let mut values =
                            |listed: &str| self.terminal_values(document, table, listed);
                        call.calculate(&mut values)
                    };
                    match result {
                        Ok(text) => self.set_calculated(document, &widgets, &text),
                        Err(refusal) => self.report(format!(
                            "{name}: the field's calculate script {} did not run: {refusal}",
                            call.function.name()
                        )),
                    }
                }
            }
        }
    }

    /// Runs every format script the runner runs whose field's value has changed since it last ran,
    /// and keeps what each displayed.
    ///
    /// Adobe's "Form event processing" formats after the calculations, and RFC 0008 section 6.5
    /// formats every field whose value changed: a field is re-formatted where its value differs
    /// from the one its kept display was made from. A field being typed into is left as typed.
    /// What a format's own edits change is applied after it, and the walk does not restart for
    /// them (RFC 0008 section 6.5's re-entrancy rule).
    pub(super) fn refresh_formatted(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) {
        if self.runner.0.is_none() {
            return;
        }
        for (name, widgets) in table {
            let Some(first) = widgets.first().copied() else {
                continue;
            };
            if self.is_editing(first) {
                continue;
            }
            let Some(widget) = document.get(first).as_dict().cloned() else {
                continue;
            };
            if !matches!(
                site::of_widget(document, &widget, Trigger::Format),
                Site::NotRun(_)
            ) {
                continue;
            }
            let Some(shown) = self.field_value(document, name) else {
                continue;
            };
            if shown.obscured
                || self
                    .scripting
                    .formatted
                    .get(&first)
                    .is_some_and(|displayed| displayed.value == shown.text)
            {
                continue;
            }
            let at = shown.text.len();
            let commit_key = self
                .scripting
                .committing
                .as_ref()
                .filter(|(field, _)| field == name)
                .map(|(_, key)| *key);
            let event = ScriptEvent {
                value: &shown.text,
                selection: (at, at),
                commit_key,
                ..ScriptEvent::at(ScriptSite::Field(Trigger::Format), name)
            };
            let Some(result) = self.run_field_script(document, table, &widget, event) else {
                continue;
            };
            self.report_each(name, result.report);
            let displayed = Displayed {
                shown: result.value.unwrap_or_else(|| shown.text.clone()),
                value: shown.text,
            };
            for widget in widgets {
                self.scripting.formatted.insert(*widget, displayed.clone());
            }
        }
    }

    /// The values of every terminal field a name in `AFSimple_Calculate`'s list names.
    ///
    /// The field itself where it is terminal, and every field below it where it is not — a name
    /// followed by `.` in §12.7.4.2's qualified names — one value per field, read from its first
    /// widget, since §12.7.4.1 makes a field's value shared by all of them.
    fn terminal_values(
        &self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        listed: &str,
    ) -> Vec<String> {
        // The table is ordered by name, so the field itself is one lookup and its descendants one
        // contiguous run from `listed.` — a calculation is walked per keystroke, and a scan of the
        // whole table per listed name is quadratic in the form's size.
        let below = format!("{listed}.");
        table
            .get(listed)
            .into_iter()
            .chain(
                table
                    .range(below.clone()..)
                    .take_while(|(name, _)| name.starts_with(&below))
                    .map(|(_, widgets)| widgets),
            )
            .filter_map(|widgets| widgets.first())
            .map(|widget| self.text_of(document, *widget).unwrap_or_default())
            .collect()
    }

    /// A widget's value as the text a script reads: a text field's characters, a button's state
    /// name, a choice field's first selected item.
    pub(super) fn text_of(&self, document: &Document, widget: ObjectId) -> Option<String> {
        let object = document.get(widget);
        let dictionary = object.as_dict()?;
        let field =
            crate::appearance::Field::read(document, dictionary, self.annotation(widget).value);
        match document.resolve(field.value.as_ref()?) {
            Object::Name(name) => Some(String::from_utf8_lossy(name.as_bytes()).into_owned()),
            Object::Integer(value) => Some(value.to_string()),
            Object::Real(value) => Some(crate::aform::number_text(value)),
            other => crate::variable_text::value_text(document, &other),
        }
    }

    /// Puts a calculated value into a field's widgets, where it differs from what they show.
    fn set_calculated(&mut self, document: &Document, widgets: &[ObjectId], text: &str) {
        if widgets.iter().any(|widget| self.is_editing(*widget)) {
            return;
        }
        let differs = widgets
            .iter()
            .any(|widget| self.text_of(document, *widget).as_deref() != Some(text));
        if differs {
            self.write_text(widgets, text);
        }
    }

    /// Writes a text value into widgets as an edit, the latest of the four statements about it.
    fn write_text(&mut self, widgets: &[ObjectId], text: &str) {
        let entry = Entry {
            value: Some(Object::String(
                pdf_syntax::text_string::encode_text_string(text).into(),
            )),
            indices: None,
        };
        for widget in widgets {
            self.reset.remove(widget);
            self.imported.remove(widget);
            self.edited.insert(*widget, entry.clone());
        }
    }

    /// Puts back what each widget showed before typing began.
    fn revert(&mut self, before: Vec<(ObjectId, Before)>) {
        for (widget, was) in before {
            match was.edited {
                Some(entry) => self.edited.insert(widget, entry),
                None => self.edited.remove(&widget),
            };
            match was.imported {
                Some(import) => self.imported.insert(widget, import),
                None => self.imported.remove(&widget),
            };
            if was.reset {
                self.reset.insert(widget);
            } else {
                self.reset.remove(&widget);
            }
        }
    }

    /// Keeps what a commit's widgets showed before the typing began beside the run the runner just
    /// held, so that a late refusal can put it back.
    fn hold_before(&mut self, name: &str, before: &[(ObjectId, Before)]) {
        if let Some(pending) = self.scripting.pending.last_mut()
            && pending.field == name
            && pending.before.is_none()
            && matches!(
                pending.site,
                ScriptSite::Field(Trigger::Keystroke | Trigger::Validate)
            )
        {
            pending.before = Some(before.to_vec());
        }
    }

    /// Applies every run the runner held and has since finished, as the outcome of the trigger
    /// that ran it arriving late, and answers whether anything a page draws changed (ADR 1627).
    ///
    /// **What a host calls once a person has answered a script's question** — after it hands the
    /// answer to the runner — and it may call it after any command, since with nothing finished it
    /// does nothing. A run held on a person changed nothing when it was handed over, so its whole
    /// outcome is applied here, in the order the runs finished: every edit as an edit, every
    /// sentence reported. The event's own answer is applied where its trigger can still take it: a
    /// commit's keystroke or validation that refuses puts back what the field showed before the
    /// typing began, unless a person has begun typing into it again; a commit's keystroke that
    /// rewrites the value writes the rewrite; a calculation's value is set; and a format's text is
    /// what the field displays. Every other site takes its edits alone, as it would have at once.
    /// Table 224's `/CO` is walked again where a value changed.
    pub fn apply_resumed(&mut self, document: &Document) -> bool {
        let Some(runner) = self.runner.0.clone() else {
            return false;
        };
        // Asked after every command, so nothing is read until a run has finished: the table walks
        // every page, and an open must not (ADR 1653 section 4).
        let Some(first) = runner.take_resumed() else {
            return false;
        };
        let table = super::widgets_by_field_name(document);
        let (mut changed, mut calculate) = (false, false);
        let mut next = Some(first);
        while let Some(resumed) = next.take() {
            let pending = self
                .scripting
                .pending
                .iter()
                .position(|held| held.site == resumed.site && held.field == resumed.field)
                .map(|at| self.scripting.pending.remove(at));
            let subject = if resumed.field.is_empty() {
                "a script that waited on an answer".to_owned()
            } else {
                resumed.field.clone()
            };
            self.report_each(&subject, resumed.result.report.clone());
            let applied = self.apply_edits(document, &table, &resumed.result.edits);
            changed |= applied.values;
            calculate |= applied.calculate;
            changed |= self.apply_late_event(document, &table, &resumed, pending);
            next = runner.take_resumed();
        }
        if changed || calculate {
            self.recalculate_scripts(document, &table, "");
            self.refresh_formatted(document, &table);
        }
        changed || calculate
    }

    /// The part of a late outcome that is its event's answer: what [`Self::apply_resumed`] says
    /// each site takes. Answers whether a value or a displayed text changed.
    fn apply_late_event(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        resumed: &Resumed,
        pending: Option<Pending>,
    ) -> bool {
        let ScriptSite::Field(trigger) = resumed.site else {
            return false;
        };
        let name = resumed.field.as_str();
        let Some(widgets) = table.get(name).cloned() else {
            return false;
        };
        let typing = widgets.iter().any(|widget| self.is_editing(*widget));
        let before = pending.and_then(|pending| pending.before);
        let result = &resumed.result;
        match (trigger, before) {
            (Trigger::Keystroke | Trigger::Validate, Some(before)) if !result.rc => {
                if typing {
                    self.report(format!(
                        "{name}: its script refused the value once its question was answered, and \
                         the field is being typed into again, so what is typed stands"
                    ));
                    return false;
                }
                self.revert(before);
                self.report(format!(
                    "{name}: its script refused the value once its question was answered, so the \
                     field shows what it showed before the typing began"
                ));
                true
            }
            (Trigger::Keystroke, Some(_)) => match &result.value {
                Some(rewritten) if !typing => {
                    self.write_text(&widgets, rewritten);
                    true
                }
                _ => false,
            },
            (Trigger::Calculate, _) => match &result.value {
                Some(calculated) if result.rc => {
                    self.set_calculated(document, &widgets, calculated);
                    true
                }
                _ => false,
            },
            (Trigger::Format, _) => {
                let Some(shown) = self.field_value(document, name) else {
                    return false;
                };
                let displayed = Displayed {
                    shown: result.value.clone().unwrap_or_else(|| shown.text.clone()),
                    value: shown.text,
                };
                for widget in &widgets {
                    self.scripting.formatted.insert(*widget, displayed.clone());
                }
                true
            }
            _ => false,
        }
    }

    /// Records that a save has written everything this view state holds: what `this.dirty` is
    /// measured against from now on (ADR 1626).
    ///
    /// **What a host calls once [`ViewState::save`]'s update is in a file.** The host keeps its
    /// own mark of unsaved work, which is the reader's and which no script moves; this is the one a
    /// script reads.
    pub fn mark_saved(&mut self) {
        self.scripting.saved = Some(Box::new(self.written()));
    }

    /// The parts of this view state a save writes, copied.
    fn written(&self) -> Saved {
        Saved {
            edited: self.edited.clone(),
            imported: self.imported.clone(),
            reset: self.reset.clone(),
            added: self.added.clone(),
            retyped: self.retyped.clone(),
            filed: self.filed.clone(),
            unfiled: self.unfiled.clone(),
            drawn: self.scripting.drawn.clone(),
            annotations_hidden: self.scripting.annotations.hidden.clone(),
            popups_opened: self.scripting.annotations.opened.clone(),
        }
    }

    /// Whether this view state holds work a save would write that no save has written:
    /// `this.dirty`.
    ///
    /// Measured against what [`Self::mark_saved`] last recorded, or against a view state as a
    /// document opens — holding no edit — where no save has been marked.
    pub(super) fn unsaved(&self) -> bool {
        match self.scripting.saved.as_deref() {
            Some(saved) => {
                saved.edited != self.edited
                    || saved.imported != self.imported
                    || saved.reset != self.reset
                    || saved.added != self.added
                    || saved.retyped != self.retyped
                    || saved.filed != self.filed
                    || saved.unfiled != self.unfiled
                    || saved.drawn != self.scripting.drawn
                    || saved.annotations_hidden != self.scripting.annotations.hidden
                    || saved.popups_opened != self.scripting.annotations.opened
            }
            None => {
                !(self.edited.is_empty()
                    && self.imported.is_empty()
                    && self.reset.is_empty()
                    && self.added.is_empty()
                    && self.retyped.is_empty()
                    && self.filed.is_empty()
                    && self.unfiled.is_empty()
                    && self.scripting.drawn.is_empty()
                    && self.scripting.annotations.hidden.is_empty()
                    && self.scripting.annotations.opened.is_empty())
            }
        }
    }

    /// Records one sentence of [`Self::script_reports`], once.
    pub(super) fn report(&mut self, sentence: String) {
        if self.script_reports.len() < MAX_REPORTS && !self.script_reports.contains(&sentence) {
            self.script_reports.push(sentence);
        }
    }
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
                let mut fits = super::accepted(document, taking, text);
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
///
/// "A text string or text stream containing the ECMAScript script to be executed": §7.9.2.2's text
/// string either way, read through the same decoding a text string always takes.
pub(super) fn script_text(document: &Document, action: &Dictionary) -> Option<String> {
    if !matches!(document.get_key(action, "S"), Object::Name(ref name) if name.as_bytes() == b"JavaScript")
    {
        return None;
    }
    let bytes = match document.resolve(action.get("JS")?) {
        Object::String(bytes) => bytes.to_vec(),
        Object::Stream(stream) => document.decoded_stream_data(&stream)?.to_vec(),
        _ => return None,
    };
    if bytes.len() > MAX_SCRIPT_BYTES {
        return None;
    }
    Some(pdf_syntax::text_string(&bytes))
}
