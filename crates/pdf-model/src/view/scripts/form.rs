//! A hybrid form's automation objects, run through the runner a host supplied (ADR 2525).
//!
//! XFA 3.3 page 379 names the moments a form's calculations are activated: the completion of the
//! merge, and a person's entry into a field. [`ViewState::activate_form_on_merge`] and
//! [`ViewState::activate_form_on_entry`] raise those two over the XFA resource §K.2 lets an
//! interactive form carry, as [`crate::xfa::Resource::activate`] orders them (ADR 2506). **The
//! scripts run in the runner**, never here: this crate reads no script language and does not depend
//! on one (ADR 2482), so [`ScriptRunner::run_form_script`] is the one call that carries a script
//! out, and a runner that has none refuses it by name. What the scripts computed lands in the edit
//! log as a person's typing does, and what `override` asks a host to present is held for it
//! ([`ViewState::take_form_overrides`]).
//!
//! Only a form XFA 3.3 does not call dynamic is activated: a `NeedsRendering` document or a
//! template whose containers occur a varying number of times is the dynamic half's (Q639), and
//! says so once instead of half-running.

use std::sync::Arc;

use super::{BTreeMap, Document, Instant, MAX_SEQUENCE_TIME, ObjectId, ScriptRunner, ViewState};
use crate::xfa::{Change, Form, FormLanguage, Limits, Overridden, Ran, Resource, Runner, Stimulus};

/// Most field names kept as entered by a person: a document's fields are its own, and a name past
/// this many is not remembered for the merge (principle 3).
const MAX_ENTERED_FIELDS: usize = 10_000;

/// The form's resource as this view state has read it: not yet asked, or what the document has.
///
/// Compared by identity, as [`super::Runner`] is, because two view states hold the same resource
/// only where they hold the same one.
#[derive(Debug, Clone, Default)]
pub(super) struct FormResource(Option<Held>);

/// What a document has of an XFA form to activate.
#[derive(Debug, Clone)]
enum Held {
    /// No XFA resource.
    Nothing,
    /// A resource that can be activated.
    Resource(Arc<Resource>),
    /// A resource that cannot, and the sentence saying why (Q639 for the dynamic half).
    Refused(String),
}

impl PartialEq for FormResource {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) | (Some(Held::Nothing), Some(Held::Nothing)) => true,
            (Some(Held::Resource(mine)), Some(Held::Resource(theirs))) => Arc::ptr_eq(mine, theirs),
            (Some(Held::Refused(mine)), Some(Held::Refused(theirs))) => mine == theirs,
            _ => false,
        }
    }
}

/// A field a person's entry hit the `override` of, for a host to present (XFA 3.3 page 642).
///
/// Closed, and not `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormOverride {
    /// The field, by its PDF field name.
    pub field: String,
    /// What the calculation's `override` asks the host to present.
    pub mode: FormOverrideMode,
}

/// What a calculation's `override` asks a host to present when a person enters a value into a
/// calculated field (XFA 3.3 pages 641–642).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormOverrideMode {
    /// `error`: an error message, and the calculated value replaced the entry.
    Error,
    /// `warning`: a warning, and the entry stands; the person's choice to dismiss the warning, which
    /// would put the calculated value back, is not carried out here (ADR 2525).
    Warning,
}

/// A runner seen as the [`Runner`] an activation runs its scripts through, counting the scripts
/// that ran to an end and holding the whole activation to [`MAX_SEQUENCE_TIME`].
///
/// Each script is held to its runner's own deadline, and an activation may run ten thousand of
/// them (`Limits::DEFAULT`), so thirty calculations each just under the deadline would hold the
/// window for seconds. Past the sequence budget a script is not run and reports the largest step
/// count, which ends the activation as a spent step budget does: every value it calculated is
/// dropped, and `timed_out` lets the caller say that it was the clock (ADR 2525).
struct ThroughRunner<'r> {
    runner: &'r dyn ScriptRunner,
    ran: usize,
    started: Instant,
    timed_out: bool,
}

impl<'r> ThroughRunner<'r> {
    fn new(runner: &'r dyn ScriptRunner, started: Instant) -> Self {
        Self {
            runner,
            ran: 0,
            started,
            timed_out: false,
        }
    }
}

impl Runner for ThroughRunner<'_> {
    fn run(
        &mut self,
        form: &mut Form<'_>,
        node: usize,
        language: FormLanguage,
        script: &str,
        steps_left: u64,
    ) -> Ran {
        if self.timed_out || self.started.elapsed() > MAX_SEQUENCE_TIME {
            self.timed_out = true;
            return Ran {
                steps: u64::MAX,
                outcome: Err("the activation ran past its time budget".to_owned()),
            };
        }
        let ran = self
            .runner
            .run_form_script(form, node, language, script, steps_left);
        if ran.outcome.is_ok() {
            self.ran = self.ran.saturating_add(1);
        }
        ran
    }
}

impl ViewState {
    /// Activates the calculations of the document's XFA form for the completion of its merge, in
    /// page 379's order, through the runner a host supplied (XFA 3.3 pages 379 and 407).
    ///
    /// A document with no XFA form, a view state with no runner, and a form that is a dynamic one
    /// run nothing; the last says so in [`Self::script_reports`]. Answers whether a value changed,
    /// which is ink to draw again.
    ///
    /// **A person's entry made before the merge stands.** A runner that arrives late — a reader at
    /// `ask` who answers after typing — finds values a person entered, and a merge that wrote the
    /// calculated value over one would lose input that page 642 says an `override` of `warning`
    /// keeps. So the merge leaves the entered fields as they are and then raises each entry as an
    /// entry, which is the rule page 379 gives it: an `error` replaces the entry and says so, a
    /// `warning` or a disabled calculation keeps it (ADR 2525 item 2). The cost is a second
    /// activation, and the notices arrive with the merge instead of with the keystroke.
    pub fn activate_form_on_merge(&mut self, document: &Document) -> bool {
        self.activate_form(document, None, Instant::now())
    }

    /// Activates the calculations that depend on the field `name`, which a person just committed a
    /// value to, in page 379's order (XFA 3.3 pages 379 and 641–642).
    pub fn activate_form_on_entry(&mut self, document: &Document, name: &str) -> bool {
        if self.scripting.form_entered.len() < MAX_ENTERED_FIELDS {
            self.scripting.form_entered.insert(name.to_owned());
        }
        self.activate_form(document, Some(name), Instant::now())
    }

    /// The fields whose `override` asks a host to present a message since a host last took them, in
    /// the order a person entered them.
    pub fn take_form_overrides(&mut self) -> Vec<FormOverride> {
        std::mem::take(&mut self.scripting.form_overrides)
    }

    /// The form's resource, read the first time it is asked for.
    fn held_form(&mut self, document: &Document) -> &Held {
        self.scripting.form.0.get_or_insert_with(|| {
            let Some(stated) = crate::xfa::stated(document).filter(|stated| stated.resource) else {
                return Held::Nothing;
            };
            if stated.needs_rendering {
                return Held::Refused(
                    "the document asks to be regenerated when opened (Table 29's \
                     /NeedsRendering), so its XFA form is a dynamic one (Q639)"
                        .to_owned(),
                );
            }
            match Resource::read(document) {
                Some(Ok(resource)) => match resource.template.dynamic() {
                    Some(why) => {
                        Held::Refused(format!("its XFA form is a dynamic one: {why} (Q639)"))
                    }
                    None => Held::Resource(Arc::new(resource)),
                },
                Some(Err(error)) => {
                    Held::Refused(format!("its XFA resource does not read: {error}"))
                }
                None => Held::Nothing,
            }
        })
    }

    /// `started` is when the sequence began: a merge and the entries it re-raises are one sequence
    /// and share one budget.
    fn activate_form(
        &mut self,
        document: &Document,
        typed: Option<&str>,
        started: Instant,
    ) -> bool {
        let Some(runner) = self.runner.0.clone() else {
            return false;
        };
        let resource = match self.held_form(document).clone() {
            Held::Resource(resource) => resource,
            Held::Refused(why) => {
                if typed.is_none() {
                    self.report(format!("the XFA form's calculations are not run: {why}"));
                }
                return false;
            }
            Held::Nothing => return false,
        };
        let table = crate::view::field_table(document, crate::view::Omitted::Calculation);
        let fields = self.form_fields(document, &table);
        let entered = match typed {
            Some(name) => vec![name.to_owned()],
            None => Vec::new(),
        };
        let stimulus = match typed {
            Some(_) => Stimulus::Entry(&entered),
            None => Stimulus::Merge,
        };
        // What a person entered before this merge stands under it (the merge's own comment).
        let before_the_merge: Vec<String> = match typed {
            Some(_) => Vec::new(),
            None => self
                .scripting
                .form_entered
                .iter()
                .filter(|name| table.contains_key(*name))
                .cloned()
                .collect(),
        };
        let mut through = ThroughRunner::new(runner.as_ref(), started);
        let activation = resource.activate(&fields, stimulus, &mut through, Limits::DEFAULT);
        if through.ran > 0
            && let Some(note) = runner.form_runs_note()
        {
            self.report(format!("the XFA form: {note}"));
        }
        for sentence in &activation.report {
            self.report(format!("the XFA form: {sentence}"));
        }
        if through.timed_out {
            self.report(format!(
                "the XFA form's calculations were stopped: they ran longer than their budget of \
                 {} ms, and the values they had calculated were dropped (ADR 2525)",
                MAX_SEQUENCE_TIME.as_millis()
            ));
        }
        let mut result = activation.result();
        result.edits.retain(|edit| match edit {
            crate::view::ScriptEdit::Value { field, .. } => !before_the_merge.contains(field),
            _ => true,
        });
        let applied = self.apply_edits(document, &table, &result.edits);
        for Overridden { field, mode } in activation.overridden {
            let mode = match mode {
                crate::xfa::Override::Warning => FormOverrideMode::Warning,
                _ => FormOverrideMode::Error,
            };
            self.scripting
                .form_overrides
                .push(FormOverride { field, mode });
        }
        let mut changed = applied.values;
        for name in &before_the_merge {
            if started.elapsed() > MAX_SEQUENCE_TIME {
                self.report(format!(
                    "the XFA form's entries made before its calculations ran were not all \
                     raised: the sequence ran longer than its budget of {} ms (ADR 2525)",
                    MAX_SEQUENCE_TIME.as_millis()
                ));
                break;
            }
            changed |= self.activate_form(document, Some(name), started);
        }
        changed
    }

    /// Every field of the form by its PDF name with the text it holds now, a person's entry
    /// included: what [`crate::xfa::Resource::activate`] reads the values of.
    fn form_fields(
        &self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
    ) -> Vec<(String, Change)> {
        table
            .iter()
            .filter_map(|(name, widgets)| {
                let text = self.text_of(document, *widgets.first()?)?;
                let change = if text.is_empty() {
                    Change::Cleared
                } else if text.as_bytes() == crate::appearance::OFF {
                    Change::Off
                } else {
                    Change::Text(text)
                };
                Some((name.clone(), change))
            })
            .collect()
    }
}
