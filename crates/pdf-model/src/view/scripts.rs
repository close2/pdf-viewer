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

use std::collections::BTreeMap;

use pdf_syntax::{Document, Object, ObjectId};

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
            Site::NotRun(sentence) => self.report(format!("{name}: {sentence}")),
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
                Site::Absent | Site::NotRun(_) => shown.text,
            },
        )
    }

    /// Whether this widget's field is being typed into and has not been committed.
    pub(super) fn is_editing(&self, annotation: ObjectId) -> bool {
        self.uncommitted
            .values()
            .any(|widgets| widgets.iter().any(|(widget, _)| *widget == annotation))
    }

    /// Whether Table 199's `/K`, in its typing form, lets the characters stand.
    ///
    /// The host hands a whole value, so the keystroke is that value replacing the whole of the
    /// field's text — what `AFMergeChange` then reassembles is the value itself.
    pub(super) fn keystroke_stands(
        &mut self,
        document: &Document,
        taking: &[ObjectId],
        text: &str,
    ) -> bool {
        let Some(first) = taking.first().copied() else {
            return true;
        };
        let Some(widget) = document.get(first).as_dict().cloned() else {
            return true;
        };
        match site::of_widget(document, &widget, Trigger::Keystroke) {
            Site::Absent => true,
            Site::NotRun(sentence) => {
                let name = super::field_name_of(document, first);
                self.report(format!("{name}: {sentence}"));
                true
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
                    Ok(Keyed::Accepted { .. }) => true,
                    Ok(Keyed::Rejected { .. }) => false,
                    Err(refusal) => {
                        let name = super::field_name_of(document, first);
                        self.report(format!(
                            "{name}: the field's keystroke script {} did not run: {refusal}",
                            call.function.name()
                        ));
                        true
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
