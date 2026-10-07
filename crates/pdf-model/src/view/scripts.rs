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

use super::script_model::{FieldState, Overrides, Property, ScriptEdit, ScriptSite};
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
    /// The field whose change a calculation answers — `event.source` — or empty.
    pub source: &'a str,
    /// Every field whose state changed since the runner was last handed an event: all of them the
    /// first time. A runner's realm replaces its record of each by name.
    pub fields: &'a [FieldState],
    /// The zero-based page the event happens on, Adobe's `this.pageNum`.
    pub page: usize,
    /// How many pages the document has, `this.numPages`.
    pub pages: usize,
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
    #[expect(
        clippy::too_many_lines,
        reason = "one commit's triggers in the reference's order — the keystroke's commit form,                   then validate, each a one-call arm and a runner's arm with the same refusal —                   read top to bottom as the event model they implement"
    )]
    pub fn commit_field(&mut self, document: &Document, name: &str) -> Committed {
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
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Keystroke), name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
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
                    ..ScriptEvent::at(ScriptSite::Field(Trigger::Validate), name)
                };
                match self.run_field_script(document, &table, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
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
        self.refresh_formatted(document, &table);
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

    /// The properties scripts have set on one field, the latest of each member, in the order they
    /// were set.
    ///
    /// What a host drawing its own control over a field reads beside the value: a script's
    /// `textColor`, `fillColor`, `borderStyle`, `alignment`, `charLimit` and `required` are kept
    /// here and are not drawn into the page's appearance (ADR 1603); `display` and `readonly` are
    /// applied as well as kept.
    #[must_use]
    pub fn script_properties(&self, name: &str) -> &[Property] {
        self.scripting
            .overrides
            .get(name)
            .map_or(&[], |overrides| overrides.set.as_slice())
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
        self.displayed_in(document, &super::widgets_by_field_name(document), name)
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
        let table = super::widgets_by_field_name(document);
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
        let fields = self.tell(document, table);
        let result = runner.run(&ScriptEvent {
            fields: &fields,
            pages: crate::page::Pages::new(document).len(),
            ..event
        });
        let applied = self.apply_edits(document, table, &result.edits);
        Some((result, applied))
    }

    /// Every field whose state differs from what the realm was last told, read now, and the realm
    /// counted as told of them.
    fn tell(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) -> Vec<FieldState> {
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
                    if properties.contains(name) || held.iter().any(|w| widgets.contains(w)) {
                        changed.insert(name);
                    }
                }
                changed
            }
        };
        if names.is_empty() {
            return Vec::new();
        }
        let pages = self
            .scripting
            .pages
            .get_or_insert_with(|| crate::page::Pages::new(document).indices())
            .clone();
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
        }));
        states
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
                ScriptEdit::Property { field, property } => {
                    let Some(widgets) = table.get(field).cloned() else {
                        continue;
                    };
                    self.scripting
                        .overrides
                        .entry(field.clone())
                        .or_default()
                        .record(*property);
                    match property {
                        Property::Display(display) => {
                            for widget in &widgets {
                                self.set_hidden(*widget, !display.on_screen());
                            }
                        }
                        // Table 227 bit 1 bars a *user*, and `set_field` is where a user's value
                        // arrives, so that is where the override is read.
                        Property::ReadOnly(_) => {}
                        other => self.report(format!(
                            "{field}: a script set Field.{}; this view state keeps it and the \
                             drawn appearance does not carry it (ADR 1603)",
                            other.member()
                        )),
                    }
                }
                ScriptEdit::Reset { fields } => {
                    let action = ResetForm {
                        fields: fields.iter().cloned().map(ResetTarget::Name).collect(),
                        exclude: false,
                    };
                    self.reset_form(document, &action);
                    applied.values = true;
                }
                ScriptEdit::Calculate => applied.calculate = true,
            }
        }
        applied
    }

    /// Records each of a runner's sentences against the field it ran for.
    pub(super) fn report_each(&mut self, name: &str, sentences: Vec<String>) {
        for sentence in sentences {
            self.report(format!("{name}: {sentence}"));
        }
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
                let event = ScriptEvent {
                    value: &current,
                    change: text,
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
            let event = ScriptEvent {
                value: &shown.text,
                selection: (at, at),
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

    /// Records one sentence of [`Self::script_reports`], once.
    pub(super) fn report(&mut self, sentence: String) {
        if self.script_reports.len() < MAX_REPORTS && !self.script_reports.contains(&sentence) {
            self.script_reports.push(sentence);
        }
    }
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
            source: "",
            fields: &[],
            page: 0,
            pages: 1,
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
