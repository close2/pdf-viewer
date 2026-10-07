//! Table 199's four field triggers raised over [`ViewState`], and Table 224's `/CO` walked.
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
//! and the commit is its own call ([`ViewState::commit_field`]), which runs `/K`'s commit form and
//! then `/V`. **`/CO` is walked after every change either makes** — the stronger reading of "when
//! the value of any field changes", and the one that keeps a total following its lines while they
//! are typed; a calculation is a function of other fields' values, so running it early shows a
//! total the commit would show anyway.
//!
//! **A script that is not one call goes to a runner where a host has supplied one** — RFC 0008
//! section 6.3's policy hook, one place a host supplies ([`ViewState::run_scripts_with`]) and absent
//! by default, which is the level `off` (ADR 1591). The runner is handed data, a [`FieldEvent`], and
//! answers data, a [`FieldResult`], so that what runs it can sit across a process boundary; this
//! crate constructs no engine and names none. `/K` and `/F` are handed over; `/V` and `/C` stay
//! reported as scripts this tier does not run.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use super::{Entry, ViewState};
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
const MAX_SCRIPT_BYTES: usize = 1 << 20;

/// One field event Tier 0 does not run, as a [`ScriptRunner`] receives it.
///
/// Table 199's `/K` and `/F` with what Adobe's "event properties" hand a field script —
/// `event.value`, `event.change`, `event.selStart`, `event.selEnd`, `event.willCommit` — and the
/// script's own text. Data rather than a callback, so that it crosses RFC 0008 section 6.2's process
/// boundary as it stands (ADR 1591).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldEvent<'a> {
    /// Which of Table 199's triggers fired.
    pub trigger: Trigger,
    /// §12.7.4.2's fully qualified name of the field.
    pub field: &'a str,
    /// Table 221's `/JS`, as text.
    pub script: &'a str,
    /// The field's text before the event.
    pub value: &'a str,
    /// What a keystroke inserts, replacing the selection: empty at a commit and at `/F`.
    pub change: &'a str,
    /// The selection the change replaces, as byte offsets into `value`.
    pub selection: (usize, usize),
    /// Whether this is the commit rather than a character.
    pub will_commit: bool,
}

/// What a [`ScriptRunner`] made of a [`FieldEvent`].
///
/// A run that did not finish — a budget exceeded, a throw, a refused call left uncaught — changes
/// nothing: `rc` true, no value, no change, and the sentence saying why in `report` (ADR 1591).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldResult {
    /// Adobe's `event.rc`: false refuses the keystroke, or the commit.
    pub rc: bool,
    /// `event.value` where the script changed it: at `/F` the text displayed, at a commit the value
    /// that stands.
    pub value: Option<String>,
    /// `event.change` where a keystroke's script changed it.
    pub change: Option<String>,
    /// Every sentence the run owes a report: each call refused by name, each budget exceeded, each
    /// throw, each line the script logged.
    pub report: Vec<String>,
}

/// What runs the field scripts Tier 0 does not: RFC 0008 section 6.3's policy hook.
///
/// A host supplies one through [`ViewState::run_scripts_with`], and only where the reader's level
/// lets scripts run — so the four levels attach to this one place rather than to each trigger:
/// `off` supplies nothing, `on` and `warn` supply a runner, and `ask` supplies one that asks once
/// before its first run and remembers the answer. A view state with nothing supplied reports every
/// such script as one this tier does not run, which is the default (ADR 1591).
pub trait ScriptRunner: std::fmt::Debug + Send + Sync {
    /// Runs one event's script and says what it did.
    fn run(&self, event: &FieldEvent<'_>) -> FieldResult;
}

/// The runner a [`ViewState`] holds, or none.
///
/// Compared by identity: two view states hold the same runner only where they hold the same one.
#[derive(Debug, Clone, Default)]
pub(super) struct Runner(Option<Arc<dyn ScriptRunner>>);

impl PartialEq for Runner {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(mine), Some(theirs)) => Arc::ptr_eq(mine, theirs),
            _ => false,
        }
    }
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
    /// Commits what a person typed into a field: Table 199's `/K` in its commit form, then `/V`.
    ///
    /// What a host calls when a person leaves the field or presses Enter — the moment Adobe's
    /// "Form event processing" sets `willCommit`. Where either script refuses, the field goes back
    /// to what it showed before the typing began, the way Adobe's `event.rc` set false leaves a
    /// field unchanged; where the keystroke's commit form rewrites the value — a comma decimal
    /// stored with a period, an arbitrary mask's literals put back — the rewrite is the value.
    /// Table 224's `/CO` is walked after either outcome (ADR 1579).
    ///
    /// A script that is not one call of the library is not run and is reported in
    /// [`Self::script_reports`]; the value stands, because a validation nobody ran refused nothing.
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
                        self.revert(before);
                        self.recalculate(document, &table);
                        return Committed::Refused(format!(
                            "{name}: {}",
                            message.unwrap_or_else(|| {
                                "the field's keystroke script refused the value".to_owned()
                            })
                        ));
                    }
                    Err(refusal) => self.report(format!(
                        "{name}: the field's keystroke script {} did not run: {refusal}",
                        call.function.name()
                    )),
                }
            }
            Site::NotRun(sentence) => {
                let at = text.len();
                let event = FieldEvent {
                    trigger: Trigger::Keystroke,
                    field: name,
                    script: "",
                    value: &text,
                    change: "",
                    selection: (at, at),
                    will_commit: true,
                };
                match self.run_supplied(document, &widget, event) {
                    None => self.report(format!("{name}: {sentence}")),
                    Some(result) => {
                        self.report_each(name, result.report);
                        if !result.rc {
                            self.revert(before);
                            self.recalculate(document, &table);
                            return Committed::Refused(format!(
                                "{name}: the field's keystroke script refused the value"
                            ));
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
                    self.revert(before);
                    self.recalculate(document, &table);
                    return Committed::Refused(format!("{name}: {message}"));
                }
                Ok(None) => {}
                Err(refusal) => self.report(format!(
                    "{name}: the field's validate script {} did not run: {refusal}",
                    call.function.name()
                )),
            },
            Site::NotRun(sentence) => self.report(format!("{name}: {sentence}")),
            Site::Absent => {}
        }

        self.recalculate(document, &table);
        Committed::Accepted
    }

    /// Hands every field script Tier 0 does not run at `/K` and `/F` to `runner`, or, with `None`,
    /// to nothing — RFC 0008 section 6.3's level `off`, which is where every view state starts.
    ///
    /// The one place a host's level for scripts reaches this crate (ADR 1591): what the runner may
    /// reach, and whether it asks first, is the host's and the runner's, never decided here.
    pub fn run_scripts_with(&mut self, runner: Option<Arc<dyn ScriptRunner>>) {
        self.runner = Runner(runner);
    }

    /// What Tier 0's dispatch did not run, each sentence once, in the order it was met.
    ///
    /// A field's script that is not one call of the library, a call whose arguments the library
    /// refuses, a `/CO` entry whose `/C` is either — each named with its field, never raised as a
    /// dialog and never repeated (RFC 0008 section 6.8).
    #[must_use]
    pub fn script_reports(&self) -> &[String] {
        &self.script_reports
    }

    /// What one field displays: its value through its one-call format script, or as it stands.
    ///
    /// [`Self::field_value`] answers with the characters a host edits; this answers with what the
    /// page shows when nobody is editing them — Table 199's `/F` "performed before the field is
    /// formatted to display its value" — which is what an assistive technology reading an
    /// unfocused field, or a host drawing its own control over one, wants. A field being typed
    /// into displays as typed. `None` in [`Self::field_value`]'s cases, and a password field
    /// answers with its echo, unformatted (ADR 1579).
    #[must_use]
    pub fn displayed_value(&self, document: &Document, name: &str) -> Option<String> {
        let shown = self.field_value(document, name)?;
        let table = super::widgets_by_field_name(document);
        let widget = table.get(name)?.first().copied()?;
        if shown.obscured || self.is_editing(widget) {
            return Some(shown.text);
        }
        let object = document.get(widget);
        let dictionary = object.as_dict()?;
        Some(
            match site::of_widget(document, dictionary, Trigger::Format) {
                Site::Library(call) => call
                    .format(&shown.text)
                    .map_or(shown.text, |formatted| formatted.text),
                // A runner's sentences are not kept from here — this answers a question and
                // records nothing — and the runner keeps its own log of every run (ADR 1591).
                Site::NotRun(_) => {
                    let at = shown.text.len();
                    let event = FieldEvent {
                        trigger: Trigger::Format,
                        field: name,
                        script: "",
                        value: &shown.text,
                        change: "",
                        selection: (at, at),
                        will_commit: false,
                    };
                    self.run_supplied(document, dictionary, event)
                        .and_then(|result| result.value)
                        .unwrap_or(shown.text)
                }
                Site::Absent => shown.text,
            },
        )
    }

    /// Hands `event` to the supplied runner with the field's script, where there is a runner and
    /// the trigger's action is one ECMAScript action whose text can be read.
    fn run_supplied(
        &self,
        document: &Document,
        widget: &Dictionary,
        event: FieldEvent<'_>,
    ) -> Option<FieldResult> {
        let runner = self.runner.0.as_ref()?;
        let script = supplied_script(document, widget, event.trigger)?;
        Some(runner.run(&FieldEvent {
            script: &script,
            ..event
        }))
    }

    /// Records each of a runner's sentences against the field it ran for.
    fn report_each(&mut self, name: &str, sentences: Vec<String>) {
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
                let name = super::field_name_of(document, first);
                let current = self.text_of(document, first).unwrap_or_default();
                let event = FieldEvent {
                    trigger: Trigger::Keystroke,
                    field: &name,
                    script: "",
                    value: &current,
                    change: text,
                    selection: (0, current.len()),
                    will_commit: false,
                };
                match self.run_supplied(document, &widget, event) {
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

    /// Walks Table 224's `/CO`, running each entry's one-call `/C`.
    ///
    /// In the array's order and once — "the calculation order in which their values will be
    /// recalculated" — so a calculation that changes a field earlier in the array does not restart
    /// the walk; Adobe's reference is silent on that re-entrancy and running the order once is what
    /// the array states (ADR 1579). A calculated value is the document changing its own value,
    /// like §12.7.6.3's reset, so Table 227's `ReadOnly` — a bar on the *user* — does not stop it.
    /// A field being typed into is left alone: what a person is typing is not overwritten under
    /// them.
    pub(super) fn recalculate(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) {
        for field in calculation_order(document) {
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
                Site::NotRun(sentence) => self.report(format!("{name}: {sentence}")),
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
    fn text_of(&self, document: &Document, widget: ObjectId) -> Option<String> {
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
    fn report(&mut self, sentence: String) {
        if self.script_reports.len() < MAX_REPORTS && !self.script_reports.contains(&sentence) {
            self.script_reports.push(sentence);
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
            if !matches!(document.get_key(&action, "S"), Object::Name(ref name) if name.as_bytes() == b"JavaScript")
            {
                return None;
            }
            let follows = match document.get_key(&action, "Next") {
                Object::Null => false,
                Object::Array(items) => !items.is_empty(),
                _ => true,
            };
            if follows {
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
            return Some(pdf_syntax::text_string(&bytes));
        }
        current = document.get_key(&current, "Parent").as_dict().cloned()?;
    }
    None
}
