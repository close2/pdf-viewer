//! A hybrid form's automation objects, run through the runner a host supplied (ADR 2525).
//!
//! XFA 3.3 page 379 names the moments a form's calculations are activated: the completion of the
//! merge, and a person's entry into a field. [`ViewState::activate_form_on_merge`] and
//! [`ViewState::activate_form_on_entry`] raise those two over the XFA resource §K.2 lets an
//! interactive form carry, as [`crate::xfa::Resource::activate`] orders them (ADR 2506), and
//! [`ViewState::activate_form_on_exit`] and [`ViewState::activate_form_on_enter`] raise the events
//! pages 390–391 fire when the keyboard focus moves (ADR 2566), and
//! [`ViewState::activate_form_on_click`] the `click` events page 391 fires when a mouse click
//! occurs within a field (ADR 2584), and [`ViewState::activate_form_on_mouse_down`] and
//! [`ViewState::activate_form_on_mouse_up`] the `mouseDown` and `mouseUp` events page 392 fires
//! when a mouse button is pressed or released within one (ADR 2627). **The
//! scripts run in the runner**, never here: this crate reads no script language and does not depend
//! on one (ADR 2482), so [`ScriptRunner::run_form_script`] is the one call that carries a script
//! out, and a runner that has none refuses it by name. What the scripts computed lands in the edit
//! log as a person's typing does, and what `override` asks a host to present is held for it
//! ([`ViewState::take_form_overrides`]), as are the messages of the script tests the form fails
//! ([`ViewState::take_form_validations`], ADR 2696).
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

/// Most validation messages held for a host that has not taken them: a form fails as many script
/// tests as it states, and a host that never asks is not made to hold them all (principle 3).
pub(super) const MAX_HELD_VALIDATIONS: usize = 256;

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

/// What raised an activation: page 379's two stimuli, a move of the keyboard focus (pages
/// 390–391), each half of which is [`Stimulus`]'s own, a click, and the open sequence's end.
#[derive(Debug, Clone, Copy)]
enum Raised<'a> {
    /// The completion of the merge.
    Merge,
    /// A person's entry into the field `name`, which showed `before` before it where the caller
    /// said (XFA 3.3 page 400's `prevText`).
    Entry {
        name: &'a str,
        before: Option<&'a str>,
    },
    /// The focus left `left` for `reaching`, each with the on value of the button it was on where
    /// it is an exclusion group ([`Stimulus::Exit`]).
    Exit {
        left: (&'a str, Option<&'a str>),
        reaching: Option<(&'a str, Option<&'a str>)>,
    },
    /// The focus reached `reached` from `leaving`, alike ([`Stimulus::Enter`]).
    Enter {
        reached: (&'a str, Option<&'a str>),
        leaving: Option<(&'a str, Option<&'a str>)>,
    },
    /// A mouse click occurred within the field of this PDF name, on the button of this on value
    /// where the field is an exclusion group ([`Stimulus::Click`]); the four after it alike.
    Click(&'a str, Option<&'a str>),
    /// A mouse button was pressed within the field of this PDF name.
    MouseDown(&'a str, Option<&'a str>),
    /// A mouse button was released within the field of this PDF name.
    MouseUp(&'a str, Option<&'a str>),
    /// The pointer moved into the region of the field of this PDF name.
    MouseEnter(&'a str, Option<&'a str>),
    /// The pointer moved out of the region of the field of this PDF name.
    MouseExit(&'a str, Option<&'a str>),
    /// A person dismissed the warning page 642 presents for an entry into the field of this PDF
    /// name.
    Dismissed(&'a str),
    /// The open sequence ended: the Form DOM's `ready` and the application's `docReady` (pages 388
    /// and 704).
    Ready,
}

impl<'a> Raised<'a> {
    /// The stimulus this is. An entry's names and what each field showed are owned by the caller,
    /// which builds its stimulus itself, so an entry here names no field.
    fn stimulus(self) -> Stimulus<'a> {
        match self {
            Raised::Merge => Stimulus::Merge,
            Raised::Entry { .. } => Stimulus::Entry {
                entered: &[],
                before: &[],
            },
            Raised::Exit { left, reaching } => Stimulus::Exit {
                left: left.0,
                reaching: reaching.map(|(name, _)| name),
                left_button: left.1,
                reaching_button: reaching.and_then(|(_, button)| button),
            },
            Raised::Enter { reached, leaving } => Stimulus::Enter {
                reached: reached.0,
                leaving: leaving.map(|(name, _)| name),
                reached_button: reached.1,
                leaving_button: leaving.and_then(|(_, button)| button),
            },
            Raised::Click(clicked, button) => Stimulus::Click { clicked, button },
            Raised::MouseDown(pressed, button) => Stimulus::MouseDown { pressed, button },
            Raised::MouseUp(released, button) => Stimulus::MouseUp { released, button },
            Raised::MouseEnter(entered, button) => Stimulus::MouseEnter { entered, button },
            Raised::MouseExit(exited, button) => Stimulus::MouseExit { exited, button },
            Raised::Ready => Stimulus::Ready,
            Raised::Dismissed(field) => Stimulus::Dismissed { field },
        }
    }
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
    ///
    /// **The open sequence ends with `ready`.** Once the merge and the entries it raises again are
    /// done, the `ready` events bound to the Form DOM run, which page 388 fires after the merge's
    /// calculations and validations, and then the `docReady` events bound to the application,
    /// which page 388 puts after the Form DOM's `ready` and before the document is rendered; this
    /// is the one call that raises them, so a sequence raises them once. They are an activation
    /// of their own under the same sequence clock, so a `ready` script that never ends is stopped
    /// within the one second the merge started, keeps nothing and leaves the form as the merge
    /// left it. The same activation names the events whose `listen` or `ref` this reader does not
    /// follow (ADR 2606).
    pub fn activate_form_on_merge(&mut self, document: &Document) -> bool {
        let started = Instant::now();
        let merged = self.activate_form(document, Raised::Merge, started);
        let ready = self.activate_form(document, Raised::Ready, started);
        merged || ready
    }

    /// Activates the calculations that depend on the field `name`, which a person just committed a
    /// value to, in page 379's order (XFA 3.3 pages 379 and 641–642). `before` is what the field
    /// showed before the typing began, where the caller knows it, which the field's `change`
    /// event reads as `$event.prevText` (page 400).
    pub fn activate_form_on_entry(
        &mut self,
        document: &Document,
        name: &str,
        before: Option<&str>,
    ) -> bool {
        if self.scripting.form_entered.len() < MAX_ENTERED_FIELDS {
            self.scripting.form_entered.insert(name.to_owned());
        }
        self.activate_form(document, Raised::Entry { name, before }, Instant::now())
    }

    /// Runs the `exit` events of the keyboard focus leaving the field `left` for the field
    /// `reaching`, or for no field (XFA 3.3 pages 390–391): the field's, and those of each
    /// subform and exclusion group around it that the focus goes out of, followed by the
    /// calculations that depend on what the events changed (page 379). Answers whether a value
    /// changed, which is ink to draw again.
    ///
    /// A host calls it after the commit a loss of focus makes, so that an `exit` script reads the
    /// person's entry. The events are one activation under the sequence budget of one second, and
    /// one that is stopped drops what its scripts changed and leaves the entry as the person made
    /// it.
    ///
    /// Each field comes with the on value of the widget the focus is on
    /// ([`crate::form::button_on_value`]), which, for an exclusion group, names the group's field
    /// whose `exit` and `enter` page 391 fires (ADR 2698); `None` for any other field.
    pub fn activate_form_on_exit(
        &mut self,
        document: &Document,
        left: (&str, Option<&str>),
        reaching: Option<(&str, Option<&str>)>,
    ) -> bool {
        self.activate_form(document, Raised::Exit { left, reaching }, Instant::now())
    }

    /// Runs the `enter` events of the keyboard focus reaching the field `reached` from the field
    /// `leaving`, or from no field (XFA 3.3 pages 390–391): those of each subform and exclusion
    /// group around it that the focus comes into, and the field's, followed by the calculations
    /// that depend on what the events changed (page 379). Answers whether a value changed. Each
    /// field comes with its widget's on value, as [`Self::activate_form_on_exit`]'s do.
    pub fn activate_form_on_enter(
        &mut self,
        document: &Document,
        reached: (&str, Option<&str>),
        leaving: Option<(&str, Option<&str>)>,
    ) -> bool {
        self.activate_form(document, Raised::Enter { reached, leaving }, Instant::now())
    }

    /// Runs the `click` events of the field `clicked`, within whose region a mouse click occurred
    /// (XFA 3.3 page 391), followed by the calculations that depend on what the events changed
    /// (page 379). Answers whether a value changed, which is ink to draw again.
    ///
    /// Page 390 gives a subform and an exclusion group no `click`, so the field is the object the
    /// click happens on, and an enclosing container's event hears it only where its `listen` is
    /// `refAndDescendents` (page 387; ADRs 2584, 2626). Where `clicked` is an exclusion group,
    /// whose PDF field is one for all its buttons, `button` is the on value of the widget clicked
    /// ([`crate::form::button_on_value`]) and names the group's field the click was within (ADR 2698);
    /// the four pointer methods after this one take it alike. The events are one activation under the
    /// sequence budget of one second, and one that is stopped drops what its scripts changed and
    /// leaves every value a person entered as it was.
    pub fn activate_form_on_click(
        &mut self,
        document: &Document,
        clicked: &str,
        button: Option<&str>,
    ) -> bool {
        self.activate_form(document, Raised::Click(clicked, button), Instant::now())
    }

    /// Carries out a person's answer to the warning XFA 3.3 page 642 presents for an entry into
    /// the field `field`, whose calculation's `override` is `warning` ([`FormOverrideMode::Warning`]).
    /// Page 642 gives two choices: to override, which keeps the entry and is what the entry
    /// already did, so `dismissed` false changes nothing; and to dismiss, which leaves the
    /// calculated value undisturbed, so `dismissed` true runs the field's calculation again in an
    /// activation of its own, under the sequence budget of one second, its value replacing the
    /// entry, and the field is no longer one a person entered, which a later merge would leave as
    /// it is (ADR 2697). Answers whether a value changed, which is ink to draw again.
    pub fn answer_form_warning(
        &mut self,
        document: &Document,
        field: &str,
        dismissed: bool,
    ) -> bool {
        if !dismissed {
            return false;
        }
        self.scripting.form_entered.remove(field);
        self.activate_form(document, Raised::Dismissed(field), Instant::now())
    }

    /// Carries out a person's answer to the warning XFA 3.3 page 385 presents when the script tests
    /// of `containers`, whose `scriptTest` is `warning`, fail (ADR 2698). Page 385 gives two
    /// choices: to override, which contravenes the constraint and keeps the value, and is what
    /// the entry already did, so `dismissed` false changes nothing; and to dismiss, by which the
    /// person "wishes to return to the form, so that s/he may satisfy this constraint", so
    /// `dismissed` true asks a host for the keyboard focus on the first widget of the first of
    /// `containers` the document names as a field, as a script's `setFocus` asks for it. A
    /// container that is no field — a subform's script test — has no widget to return to.
    pub fn answer_form_validation(
        &mut self,
        document: &Document,
        containers: &[String],
        dismissed: bool,
    ) {
        if !dismissed {
            return;
        }
        let table = crate::view::widgets_by_field_name(document);
        if let Some(field) = containers
            .iter()
            .find(|name| table.get(*name).is_some_and(|widgets| !widgets.is_empty()))
        {
            self.scripting.focus = Some(super::FocusRequest {
                field: field.clone(),
                widget: 0,
            });
        }
    }

    /// Runs the `mouseDown` events of the field `pressed`, within whose region a mouse button was
    /// pressed (XFA 3.3 page 392), followed by the calculations that depend on what the events
    /// changed (page 379). Answers whether a value changed, which is ink to draw again. The
    /// events reach what [`Self::activate_form_on_click`]'s reach, under the same budget.
    pub fn activate_form_on_mouse_down(
        &mut self,
        document: &Document,
        pressed: &str,
        button: Option<&str>,
    ) -> bool {
        self.activate_form(document, Raised::MouseDown(pressed, button), Instant::now())
    }

    /// Runs the `mouseUp` events of the field `released`, within whose region a mouse button was
    /// released (XFA 3.3 page 392) wherever it was pressed, followed by the calculations that
    /// depend on what the events changed (page 379). Answers whether a value changed. The events
    /// reach what [`Self::activate_form_on_click`]'s reach, under the same budget.
    pub fn activate_form_on_mouse_up(
        &mut self,
        document: &Document,
        released: &str,
        button: Option<&str>,
    ) -> bool {
        self.activate_form(document, Raised::MouseUp(released, button), Instant::now())
    }

    /// Runs the `mouseEnter` events of the field `entered`, into whose region the pointer moved
    /// with a button up or down (XFA 3.3 page 391), followed by the calculations that depend on
    /// what the events changed (page 379). Answers whether a value changed. The events reach what
    /// [`Self::activate_form_on_click`]'s reach, under the same budget (ADR 2647).
    pub fn activate_form_on_mouse_enter(
        &mut self,
        document: &Document,
        entered: &str,
        button: Option<&str>,
    ) -> bool {
        self.activate_form(
            document,
            Raised::MouseEnter(entered, button),
            Instant::now(),
        )
    }

    /// Runs the `mouseExit` events of the field `exited`, out of whose region the pointer moved
    /// with a button up or down (XFA 3.3 page 391), followed by the calculations that depend on
    /// what the events changed (page 379). Answers whether a value changed, under the budget of
    /// [`Self::activate_form_on_mouse_enter`].
    pub fn activate_form_on_mouse_exit(
        &mut self,
        document: &Document,
        exited: &str,
        button: Option<&str>,
    ) -> bool {
        self.activate_form(document, Raised::MouseExit(exited, button), Instant::now())
    }

    /// The fields whose `override` asks a host to present a message since a host last took them, in
    /// the order a person entered them.
    pub fn take_form_overrides(&mut self) -> Vec<FormOverride> {
        std::mem::take(&mut self.scripting.form_overrides)
    }

    /// The messages of the script tests the XFA form failed since a host last took them, in the
    /// order the activations ran: each a group of containers page 386's `validationMessaging`
    /// makes one message of, which page 383 asks a host to present (ADR 2696).
    pub fn take_form_validations(&mut self) -> Vec<Vec<crate::xfa::Invalid>> {
        std::mem::take(&mut self.scripting.form_invalid)
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
    fn activate_form(&mut self, document: &Document, raised: Raised<'_>, started: Instant) -> bool {
        let Some(runner) = self.runner.0.clone() else {
            return false;
        };
        let resource = match self.held_form(document).clone() {
            Held::Resource(resource) => resource,
            Held::Refused(why) => {
                if matches!(raised, Raised::Merge) {
                    self.report(format!("the XFA form's calculations are not run: {why}"));
                }
                return false;
            }
            Held::Nothing => return false,
        };
        // The pointer crosses widgets on nearly every movement, and a form with no event of the
        // crossing has nothing to run, so it is not given a whole activation of the form.
        let crossing = match raised {
            Raised::MouseEnter(..) => Some("mouseEnter"),
            Raised::MouseExit(..) => Some("mouseExit"),
            _ => None,
        };
        if crossing.is_some_and(|activity| !resource.has_activity(activity)) {
            return false;
        }
        let table = crate::view::field_table(document, crate::view::Omitted::Calculation);
        let fields = self.form_fields(document, &table);
        let (entered, shown) = match raised {
            Raised::Entry { name, before } => {
                (vec![name.to_owned()], vec![before.map(str::to_owned)])
            }
            _ => (Vec::new(), Vec::new()),
        };
        let stimulus = match raised {
            Raised::Entry { .. } => Stimulus::Entry {
                entered: &entered,
                before: &shown,
            },
            other => other.stimulus(),
        };
        // What a person entered before this merge stands under it (the merge's own comment).
        let before_the_merge: Vec<String> = match raised {
            Raised::Merge => self
                .scripting
                .form_entered
                .iter()
                .filter(|name| table.contains_key(*name))
                .cloned()
                .collect(),
            _ => Vec::new(),
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
                "the XFA form's scripts were stopped: they ran longer than their budget of \
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
        self.hold_for_host(activation.overridden, activation.script_tests);
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
            changed |= self.activate_form(document, Raised::Entry { name, before: None }, started);
        }
        changed
    }

    /// Holds what an activation asks a host to present: the message page 642 asks for an entry
    /// into a calculated field, and the messages of the script tests the form failed (page 383).
    fn hold_for_host(
        &mut self,
        overridden: Vec<Overridden>,
        script_tests: Vec<Vec<crate::xfa::Invalid>>,
    ) {
        for Overridden { field, mode } in overridden {
            let mode = match mode {
                crate::xfa::Override::Warning => FormOverrideMode::Warning,
                _ => FormOverrideMode::Error,
            };
            self.scripting
                .form_overrides
                .push(FormOverride { field, mode });
        }
        let room = MAX_HELD_VALIDATIONS.saturating_sub(self.scripting.form_invalid.len());
        self.scripting
            .form_invalid
            .extend(script_tests.into_iter().take(room));
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
