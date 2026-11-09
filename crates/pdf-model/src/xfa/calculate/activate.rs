//! What runs, and in what order, when a stimulus reaches a form: XFA 3.3's activation of the
//! automation objects (pages 379–380 and 404–407), as a work list that ends under a budget
//! (ADRs 2506, 2507).
//!
//! This module reads no script language. A [`Runner`] — written by the program that runs scripts,
//! because this crate does not depend on an interpreter (ADR 2482), and the shape of the runner
//! `pdf-script` is given (ADR 1591) — runs one script's text
//! against the [`Form`] it is handed and says what it computed and how many steps it took.
//!
//! **The order** (page 379, restated by the rules of pages 404–407):
//!
//! - A person's entry ([`Stimulus::Entry`]): the field's `change` events, then the calculations
//!   that depend on the value, then the validations. Page 391 says a `change` event is the
//!   person's own and a calculation or a merge does not fire it.
//! - The completion of a merge ([`Stimulus::Merge`]): every calculation, then the validations,
//!   then the `initialize` events. Page 379's general order puts events first; page 407's third
//!   rule is the specific one for this stimulus and puts the `initialize` events last, so the
//!   specific rule is the one followed (ADR 2506).
//! - A person's dismissal of the warning page 642 presents for an entry into a calculated field
//!   ([`Stimulus::Dismissed`]): the field's calculation, which replaces the entry, and the
//!   calculations that depend on it (ADR 2697).
//! - **The script tests** (pages 382 and 385): once the calculations of any stimulus have settled,
//!   the `validate` scripts run on a copy of the form whose assignments are put back, and the
//!   containers whose script returns no true value are answered as page 386's
//!   `validationMessaging` groups them — every container at a merge, and otherwise those the
//!   stimulus reached by the dependency tracking page 382 describes (ADR 2696).
//! - A move of the keyboard focus ([`Stimulus::Exit`], [`Stimulus::Enter`]): the `exit` events of
//!   the field the focus left and of each subform and exclusion group it goes out of, innermost
//!   first, and then the `enter` events of each it comes into, outermost first, and of the field
//!   it reached (pages 390–391); after each, the calculations that depend on what the events
//!   changed. The pages do not order a container's `enter` against the field's; outermost first
//!   is the order page 387 reports Adobe's processor dispatching a propagating event in, an `exit`
//!   innermost first is its mirror, and both are choices (ADR 2566).
//! - A mouse button pressed within a field's region, released within it, and a click within it
//!   ([`Stimulus::MouseDown`], [`Stimulus::MouseUp`], [`Stimulus::Click`]): the field's own
//!   `mouseDown`, `mouseUp` and `click` events (pages 391–392), and then the calculations that
//!   depend on what they changed. Page 390 gives a subform and an exclusion group none of the
//!   three, so the field is the only object they happen on (ADRs 2584, 2627); on a radio button
//!   it is the exclusion group's field whose on value the widget's is (ADR 2698), as it is for
//!   the focus.
//! - The pointer moving into a field's region and out of it ([`Stimulus::MouseEnter`],
//!   [`Stimulus::MouseExit`]), with a button up or down: the field's own `mouseEnter` and
//!   `mouseExit` events (page 391), which page 390 also gives the field alone, and then the
//!   calculations that depend on what they changed (ADR 2647).
//! - The end of the open sequence ([`Stimulus::Ready`]): the `ready` events bound to the Form DOM
//!   by `ref`, which page 388 fires once the merge, its calculations and its validations are done
//!   (page 213's step 7), then the `docReady` events bound to the application, which page 388
//!   puts after the Form DOM's `ready` and before rendering; after each, the calculations that
//!   depend on what the events changed. An event whose `ref` names an object this reader raises
//!   no event for is named in the report as not run (ADR 2606).
//! - **What an activity on an object reaches** (pages 404 and 706–707): the events bound to the
//!   object — its own whose `ref` is `$`, and those of any container whose `ref` names it, the
//!   "come-from" notation — each run as its holder's script (ADR 2699).
//! - **What an activity on an object reaches above it** (page 387): an event whose `listen` is
//!   `refAndDescendents` responds to the activity on every object its `ref` contains as well as
//!   on the `ref` itself, and page 706 makes `refOnly`, the `ref` alone, the default. So each time
//!   an activity happens on a field, subform or exclusion group, the events bound to it run,
//!   then those bound to each enclosing container that listen, innermost first, and then those
//!   bound to the Form DOM that listen, which contains every object of the form; the walk goes no
//!   further, because nothing else contains the object. Page 387 leaves the order to the
//!   processor; the object's own first and upward from it is the direction its sentence gives the
//!   propagation, and a choice (ADR 2626).
//! - Value calculations come before property calculations, and among the property calculations
//!   the inner subform before the outer one (page 405, rule 1, items 4 and 5). Among value
//!   calculations the order is the template's own, except that a calculation comes after the
//!   ones whose containers it reads. Pages 404–405 leave the order of one event's scripts
//!   undefined and report that Adobe's processor runs them in document order, which is the
//!   choice made here for events.
//!
//! **The primary event** (pages 398–400): before each object's own events of an activity a person
//! caused, and those that hear it from above, `$event` is set to that activity on that object, and
//! the calculations it wakes see the same values (ADR 2646). After a field's `change` events,
//! `$event.newText` is placed into the field where its scripts moved `change`, `selStart` or
//! `selEnd`.
//!
//! **The cascade** (pages 379–380): a calculation is run again when a container it names has
//! changed. What it names is what [`Form::resolve`] returned while it ran, so the dependency
//! follows the path the script took. A subform counts as changed whenever a container inside it
//! does (page 379), so a change wakes the calculation of every subform around it. A calculation
//! never wakes itself, because a script that reads its own value is the circular reference page
//! 380 leaves to the form's creator, and two calculations that wake each other for ever are
//! ended by [`Limits`], the means of terminating a seemingly infinite loop that page 380
//! recommends a processor have.
//!
//! **The end.** An activation that exhausts a limit changes nothing: every value it calculated
//! is dropped and the report says so, as a script run that did not finish changes nothing in
//! `pdf-script` (ADR 1603 section 3; ADR 2507 decides the limits). The work list is a set taken smallest first and the loop is
//! bounded by [`Limits::activations`], so a hostile template costs time and never the call
//! stack.

use std::collections::{BTreeMap, BTreeSet};

use super::{Calculation, Event, Exited, Form, Replaced, Scalar, Step, Test, Validity};
use crate::view::{CommitKey, Keys};
use crate::xfa::datasets::Change;
use crate::xfa::template::{Bound, Class, Entry, Language, Override, Performs, Script, ScriptTest};
use crate::xfa::{Resource, Template, validate};

/// What a [`Runner`] says of one script.
#[derive(Debug, Clone, PartialEq)]
pub struct Ran {
    /// How many steps the script took. More than the `steps_left` it was handed says it ran into
    /// that ceiling, and the cascade ends there (page 380).
    pub steps: u64,
    /// The value the script computed, or the sentence saying why it computed none. A script
    /// refused part-way has still made the assignments it made before the refusal.
    pub outcome: Result<Scalar, String>,
}

/// The language of a script a [`Runner`] is handed: the two a `script` element's `contentType`
/// names (XFA 3.3 pages 106 and 412), a script in any other being refused before a runner sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormLanguage {
    /// Chapter 25's language, `application/x-formcalc` or no `contentType`.
    FormCalc,
    /// `application/x-javascript` (page 106), reaching the form through its `rawValue` and its
    /// SOM expressions.
    JavaScript,
}

impl FormLanguage {
    /// The language a script was written in, where a runner is handed it.
    #[must_use]
    pub fn of(language: &Language) -> Option<Self> {
        match language {
            Language::FormCalc => Some(Self::FormCalc),
            Language::JavaScript => Some(Self::JavaScript),
            Language::Other(_) => None,
        }
    }

    /// The language's name in a sentence.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::FormCalc => "FormCalc",
            Self::JavaScript => "JavaScript",
        }
    }
}

/// Runs one script of a form's, in either language page 412 names.
pub trait Runner {
    /// Runs `script`, written in `language`, as the script of the container `node` against
    /// `form`, within `steps_left` steps. The script's assignments go through [`Form::set`].
    fn run(
        &mut self,
        form: &mut Form<'_>,
        node: usize,
        language: FormLanguage,
        script: &str,
        steps_left: u64,
    ) -> Ran;
}

/// The ceilings of one activation, which no single script's budget sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How many scripts may be run, the discovery of what each reads included.
    pub activations: usize,
    /// How many steps the scripts may take between them.
    pub steps: u64,
    /// How many container names the calculations may have named between them, the dependencies
    /// the cascade keeps: a script that names a thousand rows is a form, and a million names in
    /// all is a template built to hold memory (principle 3).
    pub names: usize,
    /// How many bytes of text the scripts may assign between them, an overwrite charged as much as
    /// a first assignment, the discovery runs included: a calculation that builds a megabyte
    /// string and assigns it to every field of a template is memory that no step count sees
    /// (principle 3).
    pub bytes: u64,
}

impl Limits {
    /// Ten thousand runs, ten million steps and 64 MiB of assigned text (ADR 2507): the
    /// census's largest form carries at most a few dozen calculation scripts (ADR 2482), a cascade
    /// settles in a few runs per calculation, and each script is held to a hundred thousand steps
    /// of its own, so this is two orders past what a form needs and a fraction of a second of
    /// work at most.
    pub const DEFAULT: Self = Self {
        activations: 10_000,
        steps: 10_000_000,
        names: 1_000_000,
        bytes: 64 << 20,
    };
}

/// What reached the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stimulus<'a> {
    /// Data binding finished (page 379): every calculation is activated.
    Merge,
    /// A person entered values into fields: the field's `change` events, with `$event.prevText`
    /// what it showed before (page 400).
    Entry {
        /// The fields entered into, by their PDF field names.
        entered: &'a [String],
        /// What each field of `entered`, at the same place, showed before the entry, where the
        /// window said; a place past its end, or `None`, leaves `prevText`, `selStart` and
        /// `selEnd` unsupplied.
        before: &'a [Option<String>],
        /// What the window said of the typing into each field of `entered`, at the same place,
        /// for its `change` event's `$event.fullText` and `$event.keyDown` (page 402); a place
        /// past its end, or `None`, leaves both unsupplied.
        typing: &'a [Option<super::Typing>],
    },
    /// The keyboard focus left the field `left` for the field `reaching`, or for nothing a form
    /// holds (pages 390–391): the `exit` events of the field and of each subform and exclusion
    /// group around it that the focus goes out of.
    Exit {
        /// The field the focus left, by its PDF field name.
        left: &'a str,
        /// The field it went to, where it went to one.
        reaching: Option<&'a str>,
        /// The on value of the button of `left` the focus was on, where `left` is an exclusion
        /// group, which names the group's field it leaves ([`Self::Click`]'s rule; ADR 2698).
        left_button: Option<&'a str>,
        /// The on value of the button of `reaching` it went to, by the same rule.
        reaching_button: Option<&'a str>,
        /// Whether `left` is a choice list whose own `exit` already fired when a person committed
        /// a selection in it ([`Self::Selected`]), with no `enter` since: page 392's XFA 2.4
        /// behaviour pairs each `exit` with an `enter`, so only the containers the focus goes out
        /// of have their `exit` now (ADR 2707).
        exited_at_selection: bool,
        /// What happened to the value of `left` as the focus left it, which its `exit` events
        /// read as `$event.commitKey` (page 402): committed by a tab to a new field or by a click
        /// of the mouse outside it, or not committed, a person having pressed Escape (ADR 2756).
        /// `None` where the window does not say, which leaves the property unoffered (ADR 2726).
        committed: Option<Exited>,
    },
    /// The keyboard focus reached the field `reached` from the field `leaving`, or from nothing a
    /// form holds (pages 390–391): the `enter` events of each subform and exclusion group around
    /// the field that the focus comes into, and of the field.
    Enter {
        /// The field the focus reached, by its PDF field name.
        reached: &'a str,
        /// The field it came from, where it came from one.
        leaving: Option<&'a str>,
        /// The on value of the button of `reached` the focus reached, where `reached` is an
        /// exclusion group ([`Self::Click`]'s rule; ADR 2698).
        reached_button: Option<&'a str>,
        /// The on value of the button of `leaving` it came from, by the same rule.
        leaving_button: Option<&'a str>,
    },
    /// A mouse click occurred within the region of the field `clicked` (page 391): the field's own
    /// `click` events.
    Click {
        /// The field clicked, by its PDF field name.
        clicked: &'a str,
        /// Where `clicked` is an exclusion group, whose PDF field is one for all its buttons, the
        /// on value of the button the pointer was on: the button's export value, which the PDF
        /// field's `/V` holds when it is on and so, by Annex K's rule that the XFA values be
        /// consistent with the `/V` entries, the first `items` value of the group's field the
        /// pointer was within (pages 195–197). `None` for a field that is no exclusion group.
        button: Option<&'a str>,
    },
    /// A mouse button was pressed while the pointer was within the region of the field `pressed`
    /// (page 392): the field's own `mouseDown` events.
    MouseDown {
        /// The field pressed on, by its PDF field name.
        pressed: &'a str,
        /// The on value of the button the pointer was on, where the field is an exclusion group
        /// ([`Self::Click`]'s rule).
        button: Option<&'a str>,
    },
    /// A mouse button was released while the pointer was within the region of the field
    /// `released` (page 392), wherever it was pressed: the field's own `mouseUp` events.
    MouseUp {
        /// The field released on, by its PDF field name.
        released: &'a str,
        /// The on value of the button the pointer was on, where the field is an exclusion group
        /// ([`Self::Click`]'s rule).
        button: Option<&'a str>,
    },
    /// The pointer moved into the region of the field `entered` (page 391), a button up or down:
    /// the field's own `mouseEnter` events.
    MouseEnter {
        /// The field the pointer moved into, by its PDF field name.
        entered: &'a str,
        /// The on value of the button the pointer was on, where the field is an exclusion group
        /// ([`Self::Click`]'s rule).
        button: Option<&'a str>,
    },
    /// The pointer moved out of the region of the field `exited` (page 391), a button up or down:
    /// the field's own `mouseExit` events.
    MouseExit {
        /// The field the pointer moved out of, by its PDF field name.
        exited: &'a str,
        /// The on value of the button the pointer was on, where the field is an exclusion group
        /// ([`Self::Click`]'s rule).
        button: Option<&'a str>,
    },
    /// A person committed a selection in the choice list `field`, by a click on it or Enter, and
    /// the focus stays on it (page 392): the field's own `exit` events, which XFA 2.4 fires even
    /// though the focus remains, and which the versions before it fired at every such commit
    /// (ADR 2707).
    Selected {
        /// The choice list, by its PDF field name.
        field: &'a str,
        /// How the selection was committed, which the `exit` events read as `$event.commitKey`
        /// (page 402): [`CommitKey::Enter`] where the window says Enter committed it, page 402's
        /// 2; `None` for a click on an item, which is within the field and none of page 402's
        /// clicks outside it, and so leaves the property unoffered (ADR 2755).
        committed: Option<CommitKey>,
    },
    /// A person began another selection in the choice list `field` after a [`Self::Selected`],
    /// the focus never having left it (page 392): the field's own `enter` events, with
    /// `$event.reenter` true (page 403), which XFA 2.4 fires and the versions before it did not;
    /// a template that asks for the earlier behaviour (pages 392–393) runs nothing (ADR 2707).
    Reentered {
        /// The choice list, by its PDF field name.
        field: &'a str,
    },
    /// The open sequence has ended (pages 388 and 704): the `ready` events bound to the Form DOM
    /// (`ref="$form"`), then the `docReady` events bound to the application (`ref="$host"`).
    Ready,
    /// A person dismissed the warning page 642 presents for an entry into the field `field`,
    /// whose calculation's `override` is `warning`: the calculated value is left undisturbed, so
    /// the calculation runs again and replaces the entry, and the calculations that depend on it
    /// follow. No event fires; the dismissal is an answer to the processor, not an activity on
    /// the form (ADR 2697).
    Dismissed {
        /// The field, by its PDF field name.
        field: &'a str,
    },
    /// The form data is about to be written out in PDF (page 388's `preSave`): the `preSave`
    /// events bound to the application (`ref="$host"`), the calculations that depend on what they
    /// changed, and then, where the form's `config` lists `preSave` (page 1340), every
    /// container's script test (ADR 2703).
    PreSave,
    /// The form has just been written out in PDF (page 388's `postSave`): the `postSave` events
    /// bound to the application.
    PostSave,
    /// Form data is about to be submitted (page 389's `preSubmit`): the `preSubmit` events bound
    /// to the Form DOM (`ref="$form"`), which page 389 says is the one object the event applies
    /// to, the calculations that depend on what they changed, and then, where the form's `config`
    /// lists `preSubmit`, its default (page 1340), every container's script test (ADR 2703). A
    /// script that sets `$event.cancelAction` aborts the submit ([`Activation::cancelled`]).
    PreSubmit {
        /// The field a click on which raised the submit, by its PDF field name, whose click
        /// gives the event its primary properties (page 398); `None` for a submit no click on a
        /// field raised.
        clicked: Option<&'a str>,
        /// The on value of the button clicked, where `clicked` is an exclusion group
        /// ([`Self::Click`]'s rule).
        button: Option<&'a str>,
    },
    /// A submit has ended, answered, cancelled or failed (page 389's `postSubmit`): the
    /// `postSubmit` events bound to the Form DOM.
    PostSubmit {
        /// The field a click on which raised the submit, as [`Self::PreSubmit`]'s.
        clicked: Option<&'a str>,
        /// Its button's on value, as [`Self::PreSubmit`]'s.
        button: Option<&'a str>,
    },
}

impl Stimulus<'_> {
    /// What the stimulus is to a `submit` an event performs under it.
    fn origin(&self) -> Origin {
        match *self {
            Self::Click { clicked, button } => {
                Origin::Click((clicked.to_owned(), button.map(str::to_owned)))
            }
            Self::PreSubmit { .. } | Self::PostSubmit { .. } => Origin::Submitting,
            _ => Origin::Other,
        }
    }
}

/// What a stimulus is to a `submit` an event performs under it (ADR 2738).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Origin {
    /// A click within this field, with its button's on value: the one stimulus whose field the
    /// `preSubmit` events of the submit read as their primary event, as they read a submit-form
    /// action's click (page 398).
    Click((String, Option<String>)),
    /// A submit's own `preSubmit` or `postSubmit`, under which a `submit` is not performed.
    Submitting,
    /// Any other, which gives the `preSubmit` events no primary event, as page 398 leaves to a
    /// secondary one.
    Other,
}

/// How an activation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// No calculation was left to run.
    Settled,
    /// The named limit was reached and the activation's edits were dropped: `activations`,
    /// `steps`, `names` or `bytes`, each a field of [`Limits`].
    Budget(&'static str),
}

/// A field a person entered a value into whose calculation says the entry is the host's to
/// present (XFA 3.3 pages 641–642): the `error` message, or the `warning`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overridden {
    /// The field, by its PDF field name.
    pub field: String,
    /// What the calculation's `override` says: [`Override::Error`] or [`Override::Warning`].
    pub mode: Override,
}

/// How much a failed script test weighs (XFA 3.3 page 385, the `scriptTest` of page 881).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// The container is invalid, and the processor presents an error message.
    Error,
    /// The container is recommended a value its script accepts, and the processor presents a
    /// warning the person may dismiss or override.
    Warning,
}

/// A container whose `validate` script did not return a true value for the value it holds (XFA
/// 3.3 page 382's script test), or a field whose `validate` picture cannot format what it holds
/// (page 382's format test), which the host presents as page 383 says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    /// The container, by its PDF field name, or by its template name where the field list does
    /// not name it (a subform's).
    pub container: String,
    /// The test it failed: [`Test::Script`] or [`Test::Format`].
    pub test: Test,
    /// What its `validate` element's `scriptTest`, or `formatTest`, makes of the failure.
    pub level: Level,
    /// The form designer's message for the test (the `message` element's `text` named
    /// `scriptTest` or `formatTest`, page 383), where the form gives one; where it gives none,
    /// page 383 leaves the words to the processor.
    pub message: Option<String>,
}

/// What an activation did: the edits the scripts made, beside a person's, and what it refused.
#[derive(Debug, Clone, PartialEq)]
pub struct Activation {
    /// The fields whose value a script changed, by PDF field name, in document order. Empty
    /// where [`Self::ended`] is a budget.
    pub edits: Vec<(String, Scalar)>,
    /// Every sentence the activation owes a report: each script refused or stopped, by the field
    /// it belongs to, and the limit it ended on.
    pub report: Vec<String>,
    /// How it ended.
    pub ended: Ended,
    /// How many scripts were run.
    pub activations: usize,
    /// The containers whose script test fails once the calculations have settled, one `Vec` per
    /// message the person is shown, as page 386's `validationMessaging` groups them (ADR 2696).
    /// A merge tests every container (page 382's initialization); any other stimulus tests those
    /// it changed, those around what it changed, and those whose script read what it changed
    /// (page 382's interactive validation, with the dependency tracking pages 382 and 396
    /// describe). Empty where [`Self::ended`] is a budget.
    pub script_tests: Vec<Vec<Invalid>>,
    /// The fields of a person's entry whose calculation has `override` `error` or `warning`, in
    /// the order entered: the host presents the message page 642 asks for, an error for the first
    /// and a warning for the second. Empty where [`Self::ended`] is a budget, which keeps neither
    /// the calculated values nor the replacement of the entry.
    pub overridden: Vec<Overridden>,
    /// Whether a script of [`Stimulus::PreSave`] or [`Stimulus::PreSubmit`] set
    /// `$event.cancelAction`, which page 402 says "causes the action to abort". Kept where
    /// [`Self::ended`] is a budget: the flag was set before the action was consummated, and the
    /// limit that dropped the scripts' values does not clear it.
    pub cancelled: bool,
    /// The `submit`s the events performed, in the order they fired, each for a host to carry out
    /// as a submit-form action (ADR 2738), at most [`MAX_SUBMITS`]. Empty where [`Self::ended`] is
    /// a budget, which keeps nothing the activation did.
    pub submits: Vec<FormSubmit>,
}

/// The most `submit`s one activation hands a host; the rest are said and not sent. A click a
/// person makes is one submit in any form a producer meant, and the bound keeps a template that
/// repeats one from turning a click into a burst of requests.
pub const MAX_SUBMITS: usize = 4;

/// A `submit` an event performed (XFA 3.3 pages 450–453 and 858–861; ADR 2738): the submit-form
/// action whose composition is the body the `submit` asks for, which a host carries out between
/// the form's `preSubmit` and `postSubmit` events as page 446's standard submit processing has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSubmit {
    /// The submit-form action, `/F` the `submit`'s `target`.
    pub action: crate::action::SubmitForm,
    /// The container whose event performed it and the event's activity, for a sentence.
    pub label: String,
    /// The field the person's action that raised it was on, by PDF field name, and its button's on
    /// value where it is an exclusion group: what the `preSubmit` events read their primary
    /// properties of `$event` from (page 398), as they do for a submit-form action's click.
    pub clicked: Option<(String, Option<String>)>,
    /// What the `submit` asked for that the composition does not do, one sentence each.
    pub owed: Vec<String>,
}

impl Activation {
    /// The activation as the outcome of a script run: each calculated value an edit a person could
    /// have made, so that it reaches the `/V` and the `datasets` packet the way a typed one does
    /// (RFC 0008 section 6.4; ADR 2396).
    #[must_use]
    pub fn result(&self) -> crate::view::ScriptResult {
        crate::view::ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: self
                .edits
                .iter()
                .map(|(field, value)| crate::view::ScriptEdit::Value {
                    field: field.clone(),
                    value: text(value),
                })
                .collect(),
            report: self.report.clone(),
        }
    }
}

/// Runs page 382's format test of each field among `values` whose content `changed`, or of every
/// field where `changed` is `None` (a merge), as page 396 re-evaluates it when content changes
/// (ADR 2727): the field's `validate` picture must format the canonical value of what it holds.
/// Answers the fields it fails, each tested field's verdict for its validation state, and a
/// sentence for each test this reader cannot run, which decides nothing. A field holding nothing
/// is put to no format test (page 396), so it fails none.
fn format_tests(
    (template, locales): (&Template, &crate::xfa::LocaleSet),
    values: &[(String, Change)],
    changed: Option<&BTreeSet<usize>>,
) -> (Vec<Invalid>, Vec<(usize, bool)>, Vec<String>) {
    let mut invalid = Vec::new();
    let mut verdicts = Vec::new();
    let mut sentences = Vec::new();
    for (name, value) in values {
        let Some(target) = template.resolve(name) else {
            continue;
        };
        if changed.is_some_and(|changed| !changed.contains(&target)) {
            continue;
        }
        let text = match value {
            Change::Text(text) => text.as_str(),
            Change::Cleared => "",
            Change::Choices(_) | Change::On(_) | Change::Off => continue,
        };
        match validate::conforms(template, locales, target, text) {
            validate::Conformance::Untested => {
                if text.is_empty() {
                    verdicts.push((target, false));
                }
            }
            validate::Conformance::Passes => verdicts.push((target, false)),
            validate::Conformance::Fails(level, message) => {
                verdicts.push((target, true));
                invalid.push(Invalid {
                    container: name.clone(),
                    test: Test::Format,
                    level,
                    message,
                });
            }
            validate::Conformance::Unrun(reason) => sentences.push(format!(
                "{name}: its format test (XFA 3.3 page 382) cannot be run: {reason}, so it decides \
                 nothing"
            )),
        }
    }
    (invalid, verdicts, sentences)
}

/// Whether a script's value is a true one: page 1061 promotes a value a boolean operation reads
/// to true where it can be fully converted to a nonzero number, and null to false.
pub(super) fn true_value(value: &Scalar) -> bool {
    match value {
        Scalar::Null => false,
        Scalar::Number(n) => *n != 0.0 && !n.is_nan(),
        Scalar::Text(text) => text
            .trim()
            .parse::<f64>()
            .is_ok_and(|n| n.is_finite() && n != 0.0),
    }
}

/// A number as text, with at most eleven fractional digits and no trailing zeros: the precision
/// chapter 25 displays a result at (page 1062), which a field takes as a person would type it.
fn text(value: &Scalar) -> String {
    match value {
        Scalar::Null => String::new(),
        Scalar::Text(text) => text.clone(),
        Scalar::Number(n) => {
            let written = format!("{n:.11}");
            let trimmed = written.trim_end_matches('0').trim_end_matches('.');
            if trimmed == "-0" || trimmed.is_empty() {
                "0".to_owned()
            } else {
                trimmed.to_owned()
            }
        }
    }
}

/// The most sentences one activation reports; past it the rest are counted, so that a template
/// of thousands of refused scripts costs a counter and not its own size in strings.
const MAX_REPORT: usize = 256;

/// The most characters one reported sentence keeps.
const MAX_SENTENCE: usize = 512;

impl Resource {
    /// Whether an event of the template is of `activity`, which lets a host skip the activation of
    /// an activity a person causes on nearly every movement — the pointer crossing a field — in a
    /// form that states no event for it. The walk is one pass over the template's events, which
    /// the XML reader bounds.
    #[must_use]
    pub fn has_activity(&self, activity: &str) -> bool {
        (0..self.template.container_count()).any(|node| {
            self.template.get(node).is_some_and(|container| {
                container
                    .events
                    .iter()
                    .any(|event| event.activity == activity)
            })
        })
    }

    /// Whether the PDF field name `field` names a field whose `ui` holds a `choiceList`, the
    /// field page 392's selection-commit events are fired on.
    #[must_use]
    pub fn is_choice_list(&self, field: &str) -> bool {
        self.template.resolve(field).is_some_and(|node| {
            self.template.get(node).is_some_and(|container| {
                container.class == Class::Field
                    && container
                        .ui
                        .as_ref()
                        .is_some_and(|(ui, _)| ui == "choiceList")
            })
        })
    }

    /// Activates the automation objects a `stimulus` reaches, in page 379's order, and says what
    /// they changed. `fields` are every field of the form by its PDF name with the value its
    /// `/V` carries, a person's entry included.
    ///
    /// `FormCalc` and JavaScript run, the two languages page 412 names (ADR 2545); a script in
    /// another language is refused by name. The script tests of the settled form are in the
    /// result; its null, datatype and format tests are not, because page 1340 runs them at the
    /// occasions a form's `config` names, a save and a submit, which run them over the values they
    /// write ([`Resource::before_save`], [`Resource::before_submit`]), and at an entry they would
    /// name fields a person never reached (ADR 2698).
    ///
    /// The validation state is this activation's alone, so a merge fires every container's
    /// `validationState` event and no other stimulus fires one; [`Self::activate_kept`] keeps the
    /// state from one activation to the next.
    #[must_use]
    pub fn activate(
        &self,
        fields: &[(String, Change)],
        stimulus: Stimulus<'_>,
        runner: &mut dyn Runner,
        limits: Limits,
    ) -> Activation {
        self.activate_kept(fields, stimulus, runner, limits, &mut Validity::default())
    }

    /// Page 382's format test of the fields `entered`, by their PDF field names, as `fields` now
    /// hold them, where no script runs: the test is no script, and page 396 re-evaluates it when
    /// content changes whatever runs the form's scripts (ADR 2727). Each verdict is recorded in
    /// `kept`, and no `validationState` event fires, since an event is a script. Answers the
    /// failures as page 386's `validationMessaging` groups them, and a sentence for each test this
    /// reader cannot run.
    #[must_use]
    pub fn format_tests(
        &self,
        fields: &[(String, Change)],
        entered: &[String],
        kept: &mut Validity,
    ) -> (Vec<Vec<Invalid>>, Vec<String>) {
        let changed: BTreeSet<usize> = entered
            .iter()
            .filter_map(|name| self.template.resolve(name))
            .collect();
        let (invalid, verdicts, sentences) =
            format_tests((&self.template, &self.locales), fields, Some(&changed));
        for (node, failed) in verdicts {
            kept.record(node, Test::Format, failed);
        }
        (validate::grouped(&self.config, invalid), sentences)
    }

    /// [`Self::activate`], with the validation state of the form's containers `kept` from the
    /// activations before it (pages 396–397; ADR 2706). The merge records the state its tests
    /// found and fires every field's, exclusion group's and subform's `validationState` events,
    /// after its `initialize` events, as each object is then initialized; any later activation
    /// records the verdicts of the tests it ran — the script tests it reached, and the null,
    /// datatype and format tests before a save or a submit where the form's `config` lists the
    /// activity — and fires the `validationState` events of each object whose state changed. An
    /// activation that ends on a limit changes nothing, the state included.
    #[must_use]
    pub fn activate_kept(
        &self,
        fields: &[(String, Change)],
        stimulus: Stimulus<'_>,
        runner: &mut dyn Runner,
        limits: Limits,
        kept: &mut Validity,
    ) -> Activation {
        self.activate_held(fields, stimulus, None, runner, limits, kept)
    }

    /// [`Self::activate_kept`], with the keys the window held during the person's action: a
    /// field's `click`, `enter`, `exit`, `mouseDown`, `mouseEnter`, `mouseExit`, `mouseUp` and
    /// `change` events set `$event.modifier` and `$event.shift` from them (pages 398 and 403).
    /// `None` where the window did not say, which leaves both unoffered (ADR 2732).
    #[must_use]
    pub fn activate_held(
        &self,
        fields: &[(String, Change)],
        stimulus: Stimulus<'_>,
        keys: Option<Keys>,
        runner: &mut dyn Runner,
        limits: Limits,
        kept: &mut Validity,
    ) -> Activation {
        let before = kept.clone();
        let mut form = self.form(fields);
        form.limit_text(limits.bytes);
        let initial = form.values.clone();
        let mut session = Session::new(&mut form, runner, limits, &self.template);
        (session.keys, session.origin) = (keys, stimulus.origin());
        let mut typed = Vec::new();
        match stimulus {
            Stimulus::Entry {
                entered,
                before,
                typing,
            } => {
                typed = session.typed(entered, before, typing);
                session.fire("change", Some(&typed), &Bound::Own);
                session.settle(Seeds::Entered(&typed));
            }
            Stimulus::Merge => session.settle(Seeds::All),
            Stimulus::Exit {
                left,
                reaching,
                left_button,
                reaching_button,
                exited_at_selection,
                committed,
            } => {
                session.committed = committed;
                let mut crossed =
                    session.crossed((left, left_button), reaching.map(|r| (r, reaching_button)));
                if exited_at_selection && !self.template.legacy_events() {
                    let field = session.form.named(left);
                    crossed.retain(|&node| Some(node) != field);
                }
                session.fire("exit", Some(&crossed), &Bound::Own);
                session.settle(Seeds::Changed);
            }
            Stimulus::Selected { field, committed } => session.selection(field, false, committed),
            Stimulus::Reentered { field } => session.selection(field, true, None),
            Stimulus::Enter {
                reached,
                leaving,
                reached_button,
                leaving_button,
            } => {
                let mut crossed = session.crossed(
                    (reached, reached_button),
                    leaving.map(|l| (l, leaving_button)),
                );
                crossed.reverse();
                session.fire("enter", Some(&crossed), &Bound::Own);
                session.settle(Seeds::Changed);
            }
            Stimulus::Click { clicked, button } => session.pointed(clicked, button, "click"),
            Stimulus::MouseDown { pressed, button } => {
                session.pointed(pressed, button, "mouseDown");
            }
            Stimulus::MouseUp { released, button } => {
                session.pointed(released, button, "mouseUp");
            }
            Stimulus::MouseEnter { entered, button } => {
                session.pointed(entered, button, "mouseEnter");
            }
            Stimulus::MouseExit { exited, button } => {
                session.pointed(exited, button, "mouseExit");
            }
            Stimulus::Dismissed { field } => {
                let node: Vec<usize> = session.form.named(field).into_iter().collect();
                session.settle(Seeds::Dismissed(&node));
            }
            Stimulus::PreSave => session.action("preSave", None, &Bound::Host),
            Stimulus::PostSave => session.action("postSave", None, &Bound::Host),
            Stimulus::PreSubmit { clicked, button } => {
                session.action("preSubmit", clicked.map(|c| (c, button)), &Bound::Form);
            }
            Stimulus::PostSubmit { clicked, button } => {
                session.action("postSubmit", clicked.map(|c| (c, button)), &Bound::Form);
            }
            Stimulus::Ready => {
                session.unfollowed();
                session.fire("ready", None, &Bound::Form);
                session.settle(Seeds::Changed);
                session.fire("docReady", None, &Bound::Host);
                session.settle(Seeds::Changed);
            }
        }
        let script_tests = self.validated(
            &mut session,
            stimulus,
            (fields, initial.as_slice(), typed.as_slice()),
            kept,
        );
        if matches!(stimulus, Stimulus::Merge) {
            session.fire("initialize", None, &Bound::Own);
            session.settle(Seeds::Changed);
        }
        self.announce(&mut session, stimulus, kept);
        if session.ended.is_some() {
            *kept = before;
        }
        session.finish(&initial, script_tests)
    }

    /// The validations once the stimulus's scripts have settled: the script tests — every
    /// container's at a merge and before an activity the form's `config` lists (page 1340, the
    /// whole form being what a save or a submit writes), else those the stimulus reached — and,
    /// before a save or a submit, the null, datatype and format tests, each verdict recorded in
    /// `kept` (ADR 2706). Answers the script tests' failures as page 386 groups them.
    fn validated(
        &self,
        session: &mut Session<'_, '_, '_>,
        stimulus: Stimulus<'_>,
        (fields, initial, typed): (&[(String, Change)], &[Scalar], &[usize]),
        kept: &mut Validity,
    ) -> Vec<Vec<Invalid>> {
        if session.ended.is_some() {
            return Vec::new();
        }
        let (before_action, occasion) = match stimulus {
            Stimulus::PreSave => (
                self.config.validates_on_save(),
                Some(validate::Occasion::Save),
            ),
            Stimulus::PreSubmit { .. } => (
                self.config.validates_on_submit(),
                Some(validate::Occasion::Submit),
            ),
            _ => (false, None),
        };
        let changed = session.changed_since(initial, typed);
        let everything = matches!(stimulus, Stimulus::Merge);
        // Page 396 re-evaluates the format test when content changes, and a merge initializes
        // the form, which runs every test page 396 does not keep for a pre-event (page 382). A
        // pre-event's own run is the submit's (below), and a save's is its write's.
        let mut failed = Vec::new();
        if occasion.is_none() {
            let values = session.values_now(fields, initial);
            let (formats, verdicts) = session.format_tests(
                (&self.template, &self.locales),
                &values,
                (!everything).then_some(&changed),
            );
            for (node, failure) in verdicts {
                kept.record(node, Test::Format, failure);
            }
            failed = formats;
        }
        let (tested, verdicts) = if everything || before_action {
            session.script_tests(None)
        } else {
            session.script_tests(Some(&changed))
        };
        for (node, failure) in verdicts {
            kept.record(node, Test::Script, failure);
        }
        // Page 381's table runs the format test before the script test.
        failed.extend(tested);
        if let Some(occasion) = occasion {
            let values = session.values_now(fields, initial);
            let resource = (&self.template, &self.locales, &self.config);
            for (node, test, failed) in validate::verdicts(occasion, resource, &values) {
                kept.record(node, test, failed);
            }
        }
        validate::grouped(&self.config, failed)
    }

    /// Fires the `validationState` events page 396 asks for: every object's when it is first
    /// initialized, which for this form is the merge, and afterwards those of each object whose
    /// state changed, followed by the calculations they woke (ADR 2706).
    fn announce(
        &self,
        session: &mut Session<'_, '_, '_>,
        stimulus: Stimulus<'_>,
        kept: &mut Validity,
    ) {
        if session.ended.is_some() {
            return;
        }
        let announced = if matches!(stimulus, Stimulus::Merge) {
            kept.initialize(&self.template)
        } else {
            kept.changed(&self.template)
        };
        if !announced.is_empty() {
            session.fire("validationState", Some(&announced), &Bound::Own);
            session.settle(Seeds::Changed);
        }
    }
}

/// One event of the template bound to the objects whose activity it responds to (XFA 3.3 pages
/// 706–707): the container holding it, where its `ref` is `$` or absent, or each field, subform
/// and exclusion group its `ref` names — the "come-from" notation of page 404, by which an event
/// placed in one container hears an activity on another (ADR 2699).
struct Hook {
    /// The container the event belongs to, whose script it runs as.
    holder: usize,
    /// The event's place among the holder's events.
    at: usize,
    /// The objects its `ref` names.
    sources: Vec<usize>,
}

/// Which events respond to one activity on an object (XFA 3.3 pages 387 and 706).
struct Listeners {
    /// For each object by position, the events bound to it, by holder and place, in document
    /// order: page 404 leaves the order of an event's scripts to the processor and reports Adobe's
    /// processor running them in document order, which is the choice made here.
    own: BTreeMap<usize, Vec<(usize, usize)>>,
    /// For each container by position, the nearest container around it to which an event of the
    /// activity whose `listen` is `refAndDescendents` is bound.
    nearest: Vec<Option<usize>>,
    /// Each such container's listening events, by holder and place.
    listening: BTreeMap<usize, Vec<(usize, usize)>>,
    /// The events of the activity bound to the Form DOM that listen, which contains every object
    /// of the form, by holder and place.
    form: Vec<(usize, usize)>,
}

impl Listeners {
    /// The events of `activity` among `hooks`, with those bound to the Form DOM in `template`.
    ///
    /// One pass in container order finds each container's nearest listener: the template numbers a
    /// container after the container around it, so a container's parent has its answer before the
    /// container is reached, and the walk from an object to each listener above it steps from
    /// listener to listener.
    fn of(template: &Template, hooks: &[Hook], activity: &str) -> Self {
        let count = template.container_count();
        let event = |holder: usize, at: usize| {
            template
                .get(holder)
                .and_then(|container| container.events.get(at))
        };
        let mut own: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        let mut listening: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        for hook in hooks {
            let Some(stated) = event(hook.holder, hook.at) else {
                continue;
            };
            if stated.activity != activity {
                continue;
            }
            for &source in &hook.sources {
                own.entry(source).or_default().push((hook.holder, hook.at));
                if stated.listens {
                    listening
                        .entry(source)
                        .or_default()
                        .push((hook.holder, hook.at));
                }
            }
        }
        for events in own.values_mut().chain(listening.values_mut()) {
            events.sort_unstable();
            events.dedup();
        }
        let mut form = Vec::new();
        for holder in 0..count {
            if let Some(container) = template.get(holder) {
                for (at, stated) in container.events.iter().enumerate() {
                    if stated.listens && stated.bound == Bound::Form && stated.activity == activity
                    {
                        form.push((holder, at));
                    }
                }
            }
        }
        let mut nearest: Vec<Option<usize>> = vec![None; count];
        if !listening.is_empty() {
            for node in 0..count {
                let Some(parent) = template
                    .get(node)
                    .and_then(|container| container.parent)
                    .filter(|&parent| parent < node)
                else {
                    continue;
                };
                nearest[node] = if listening.contains_key(&parent) {
                    Some(parent)
                } else {
                    nearest[parent]
                };
            }
        }
        Self {
            own,
            nearest,
            listening,
            form,
        }
    }
}

/// What starts a pass of the work list.
#[derive(Clone, Copy)]
enum Seeds<'a> {
    /// Every calculation.
    All,
    /// The calculations that depend on what changed, and the calculations of the containers a
    /// person entered values into that a person may not override.
    Entered(&'a [usize]),
    /// The calculations that depend on what changed.
    Changed,
    /// The calculations of the fields whose `warning` a person dismissed, which page 642 says
    /// leaves the calculated value undisturbed.
    Dismissed(&'a [usize]),
}

/// One activation in progress.
struct Session<'f, 'a, 'r> {
    form: &'f mut Form<'a>,
    template: &'a Template,
    runner: &'r mut dyn Runner,
    calculations: Vec<Calculation>,
    /// What each calculation (by position) named at its last run.
    named: Vec<BTreeSet<usize>>,
    /// The subform calculations by the container they belong to.
    properties: BTreeMap<usize, usize>,
    /// The inverse of `named`: the calculations that name each container.
    readers_of: BTreeMap<usize, BTreeSet<usize>>,
    /// How many names `named` holds in all.
    edges: usize,
    /// The most names `edges` may reach ([`Limits::names`]).
    max_names: usize,
    /// Whether discovery has filled `named` and `order`.
    known: bool,
    /// Each calculation's place in the order its inputs come before it, by position.
    order: Vec<usize>,
    activations_left: usize,
    steps_left: u64,
    run: usize,
    report: Vec<String>,
    reported: BTreeSet<String>,
    unreported: usize,
    ended: Option<&'static str>,
    /// The entries whose `override` the host presents.
    overridden: Vec<Overridden>,
    /// What each entered field's entry replaced, by container, for its `change` event's `$event`.
    replaced: BTreeMap<usize, Replaced>,
    /// The template's events bound to objects, read the first time an activity on an object
    /// fires ([`Self::hooks`]).
    hooks: Option<Vec<Hook>>,
    /// The sentence for each event whose `ref` names nothing this reader raises an event for,
    /// which [`Self::unfollowed`] says.
    unbound: Vec<String>,
    /// Whether a pre-event's script set `$event.cancelAction` ([`Activation::cancelled`]).
    cancelled: bool,
    /// Whether the `enter` events being fired are a choice list's re-entry (page 392), whose
    /// `$event.reenter` is true (page 403).
    reentering: bool,
    /// What happened to the value of the field the focus is leaving, or of the choice list whose
    /// selection was committed, which a field's `exit` gives as `$event.commitKey` (page 402);
    /// `None` where the window did not say.
    committed: Option<Exited>,
    /// The keys the window held during the person's action, which a field's events give as
    /// `$event.modifier` and `$event.shift` (page 403); `None` where the window did not say.
    keys: Option<Keys>,
    /// What the stimulus is to a `submit` an event performs.
    origin: Origin,
    /// The `submit`s performed ([`Activation::submits`]).
    submits: Vec<FormSubmit>,
}

impl<'f, 'a, 'r> Session<'f, 'a, 'r> {
    fn new(
        form: &'f mut Form<'a>,
        runner: &'r mut dyn Runner,
        limits: Limits,
        template: &'a Template,
    ) -> Self {
        let calculations = form.calculations();
        let named = vec![BTreeSet::new(); calculations.len()];
        let properties = calculations
            .iter()
            .enumerate()
            .filter(|(_, calculation)| !calculation.sets_value)
            .map(|(index, calculation)| (calculation.node, index))
            .collect();
        Self {
            form,
            template,
            runner,
            calculations,
            named,
            properties,
            readers_of: BTreeMap::new(),
            edges: 0,
            max_names: limits.names,
            known: false,
            order: Vec::new(),
            activations_left: limits.activations,
            steps_left: limits.steps,
            run: 0,
            report: Vec::new(),
            reported: BTreeSet::new(),
            unreported: 0,
            ended: None,
            overridden: Vec::new(),
            replaced: BTreeMap::new(),
            hooks: None,
            unbound: Vec::new(),
            cancelled: false,
            reentering: false,
            committed: None,
            keys: None,
            origin: Origin::Other,
            submits: Vec::new(),
        }
    }

    /// The container of the choice list `field` (page 392), or `None`, said, where the form's
    /// field list does not name it or its `ui` holds no `choiceList`.
    fn choice_list(&mut self, field: &str) -> Option<usize> {
        let Some(node) = self.form.named(field) else {
            self.say(format!("{field}: the form's field list does not name it"));
            return None;
        };
        if self.is_choice_list(node) {
            return Some(node);
        }
        self.say(format!(
            "{field}: a selection commit fires its events only in a choice list (XFA 3.3 page \
             392), and its ui holds none"
        ));
        None
    }

    /// A choice list's selection commit (page 392; ADR 2707): its own `exit` events, or, for a
    /// `reentry`, its own `enter` events with `$event.reenter` true, which the event model before
    /// XFA 2.4 does not fire; each followed by the calculations they woke. `committed` is how the
    /// selection was committed, which the `exit` reads as `$event.commitKey` (page 402; ADR 2755).
    fn selection(&mut self, field: &str, reentry: bool, committed: Option<CommitKey>) {
        if reentry && self.template.legacy_events() {
            return;
        }
        self.committed = committed.map(Exited::Committed);
        let node: Vec<usize> = self.choice_list(field).into_iter().collect();
        self.reentering = reentry;
        self.fire(
            if reentry { "enter" } else { "exit" },
            Some(&node),
            &Bound::Own,
        );
        self.reentering = false;
        self.settle(Seeds::Changed);
    }

    /// Whether the field `node` supports rich text, the definition page 403 gives
    /// `$event.newContentType`'s `allowRichText`: its `ui` holds a `textEdit` whose
    /// `allowRichText` is 1 (page 865). Every other `ui` takes a value of its own kind and no
    /// styled text.
    fn entry(&self, node: usize) -> Entry {
        self.template
            .get(node)
            .map_or(Entry::PlainTextOnly, |container| container.entry)
    }

    /// Whether the container `node` is a field whose `ui` holds a `choiceList`.
    fn is_choice_list(&self, node: usize) -> bool {
        self.template.get(node).is_some_and(|container| {
            container.class == Class::Field
                && container
                    .ui
                    .as_ref()
                    .is_some_and(|(ui, _)| ui == "choiceList")
        })
    }

    /// `fields` as the activation has left them: each field a script assigned a new value in
    /// takes it, as a person's entry would have put it, and every other keeps what it held.
    fn values_now(&self, fields: &[(String, Change)], initial: &[Scalar]) -> Vec<(String, Change)> {
        fields
            .iter()
            .map(|(name, held)| {
                let now = self
                    .form
                    .named(name)
                    .map(|node| (self.form.value(node), initial.get(node)))
                    .filter(|(now, before)| Some(now) != *before)
                    .map(|(now, _)| match text(&now) {
                        written if written.is_empty() => Change::Cleared,
                        written => Change::Text(written),
                    });
                (name.clone(), now.unwrap_or_else(|| held.clone()))
            })
            .collect()
    }

    /// The containers of the PDF field names a person entered values into, each with what its
    /// entry replaced: a commit replaces the whole of what the field showed, `before` at the
    /// same place.
    fn typed(
        &mut self,
        names: &[String],
        before: &[Option<String>],
        typing: &[Option<super::Typing>],
    ) -> Vec<usize> {
        let mut typed = Vec::new();
        for (at, name) in names.iter().enumerate() {
            match self.form.named(name) {
                Some(node) => {
                    let previous = before.get(at).and_then(Option::as_deref);
                    let entered = text(&self.form.value(node));
                    let typing = typing.get(at).cloned().flatten();
                    self.replaced.insert(
                        node,
                        Replaced::commit(previous, &entered).with_typing(typing),
                    );
                    typed.push(node);
                }
                None => self.say(format!("{name}: the form's field list does not name it")),
            }
        }
        typed
    }

    /// The primary event `activity` on the container `node` is (page 398's table), or `None` for
    /// an activity no person's action raises. The table sets `name` and `target` for every event
    /// of a field and for a subform's `enter` and `exit`, and nothing for an exclusion group's;
    /// a field's `change` sets what the entry replaced as well.
    fn primary(&self, activity: &str, node: usize) -> Option<Event> {
        const PRIMARY: [&str; 9] = [
            "change",
            "click",
            "enter",
            "exit",
            "mouseDown",
            "mouseEnter",
            "mouseExit",
            "mouseUp",
            "validationState",
        ];
        // The field events whose row of the table sets `reenter`.
        const REENTERING: [&str; 6] = [
            "click",
            "enter",
            "mouseDown",
            "mouseEnter",
            "mouseExit",
            "mouseUp",
        ];
        if !PRIMARY.contains(&activity) {
            return None;
        }
        let class = self.template.get(node)?.class;
        let named = !matches!(class, Class::ExclGroup) || activity == "validationState";
        let replaced = match (activity, class) {
            ("change", Class::Field) => self.replaced.get(&node).cloned(),
            _ => None,
        };
        let mut event = Event::new(activity, node, named, replaced);
        // Page 398's table has every one of these events of a field set `modifier` and `shift`,
        // a field's `exit` set `commitKey` and its `change` the two content types, which page
        // 403 says are always the same in this version of XFA.
        if class == Class::Field && activity != "validationState" {
            event = event.with_keys(self.keys);
        }
        if class == Class::Field {
            match activity {
                "exit" => event = event.with_commit_key(self.committed),
                "change" => event = event.with_content_type(self.entry(node)),
                _ => {}
            }
        }
        // Page 403: false on a field that is no choice list and for a choice list's first
        // `enter` after it gains the focus, true for a later one while it keeps the focus.
        Some(match (class, activity) {
            (Class::Field, _) if REENTERING.contains(&activity) && !self.is_choice_list(node) => {
                event.with_reenter(false)
            }
            (Class::Field, "enter") => event.with_reenter(self.reentering),
            _ => event,
        })
    }

    /// Places `$event.newText` into the field `node` once its `change` events have run (page
    /// 400), where they moved `change`, `selStart` or `selEnd` so that it is no longer the entry.
    fn place_new_text(&mut self, node: usize) {
        let Some(entered) = self.replaced.get(&node).map(Replaced::new_text) else {
            return;
        };
        let Some(placed) = self
            .form
            .event()
            .and_then(Event::replaced)
            .map(Replaced::new_text)
        else {
            return;
        };
        if placed == entered {
            return;
        }
        if let Err(why) = self.form.set(node, Scalar::Text(placed)) {
            let label = self.label(node);
            self.say(format!("{label}: $event.newText is not placed: {why}"));
        }
    }

    /// The containers the focus crosses at `field` on its way to or from `other`, innermost first:
    /// the field, then each subform and exclusion group around it that does not also hold
    /// `other`, since page 390 fires a container's `enter` and `exit` only for focus that comes in
    /// from outside it or goes out of it. A field the form's field list does not name crosses
    /// nothing and says so.
    ///
    /// Each name comes with the on value of the button the focus is on, which, where the name is an
    /// exclusion group, makes the focus that of the group's field holding it (page 391's field
    /// `enter` and `exit`; ADR 2698): between two buttons of one group the focus crosses the button
    /// fields and not the group. A group whose button names none of its fields is the focus's
    /// object itself, as it was before a button was named.
    ///
    /// Each step goes to a container's parent, so the walk is at most as long as the template has
    /// containers, whatever a hostile template nests.
    fn crossed(
        &mut self,
        (field, button): (&str, Option<&str>),
        other: Option<(&str, Option<&str>)>,
    ) -> Vec<usize> {
        let Some(node) = self.form.named(field) else {
            self.say(format!("{field}: the form's field list does not name it"));
            return Vec::new();
        };
        let node = self.within(node, button);
        let bound = self.template.container_count();
        let around = |start: Option<usize>| {
            let mut chain = Vec::new();
            let mut at = start;
            while let Some(here) = at {
                if chain.len() >= bound {
                    break;
                }
                chain.push(here);
                at = self.template.get(here).and_then(|c| c.parent);
            }
            chain
        };
        let theirs = other.and_then(|(name, button)| {
            self.form
                .named(name)
                .map(|other| self.within(other, button))
        });
        let theirs: BTreeSet<usize> = around(theirs).into_iter().collect();
        around(Some(node))
            .into_iter()
            .filter(|here| !theirs.contains(here))
            .filter(|&here| {
                self.template.get(here).is_some_and(|c| {
                    matches!(c.class, Class::Field | Class::Subform | Class::ExclGroup)
                })
            })
            .collect()
    }

    /// The field of the exclusion group `node` whose on value is `button`, or `node` itself where
    /// it is no exclusion group or the button names none of its fields.
    fn within(&self, node: usize, button: Option<&str>) -> usize {
        let group = self
            .template
            .get(node)
            .is_some_and(|container| container.class == Class::ExclGroup);
        if !group {
            return node;
        }
        button
            .and_then(|on| self.button_of(node, on))
            .unwrap_or(node)
    }

    /// Fires the `activity` events of a pointer on the field `field` — `click`, `mouseDown`,
    /// `mouseUp`, `mouseEnter` or `mouseExit`, which pages 391–392 give a field and page 390 gives
    /// no subform and no exclusion group — and then the calculations that depend on what they
    /// changed. A name the form's field list does not hold fires nothing and says so.
    ///
    /// Where the name reaches an exclusion group, whose PDF field is one for all its buttons, the
    /// pointer was within the group's field whose on value — its first `items` value, which the
    /// group's value is while it is on (pages 195–197) — is `button`, and the events are that
    /// field's; the group hears them only where its `listen` says so (page 387). A button whose
    /// on value no field of the group holds, and a pointer that names no button, fire nothing and
    /// say so (ADR 2698).
    fn pointed(&mut self, field: &str, button: Option<&str>, activity: &str) {
        match self.pointed_field(field, button) {
            Ok(target) => {
                self.fire(activity, Some(&[target]), &Bound::Own);
                self.settle(Seeds::Changed);
            }
            Err(why) => self.say(format!("{field}: its {activity} events are not run: {why}")),
        }
    }

    /// The field the pointer was within, by [`Self::pointed`]'s rule, or the reason there is none.
    fn pointed_field(&self, field: &str, button: Option<&str>) -> Result<usize, String> {
        let Some(node) = self.form.named(field) else {
            return Err("the form's field list does not name it".to_owned());
        };
        match self.template.get(node).map(|container| container.class) {
            Some(Class::Field) => Ok(node),
            Some(Class::ExclGroup) => {
                button
                    .and_then(|on| self.button_of(node, on))
                    .ok_or_else(|| {
                        let which = button.map_or_else(
                            || "the pointer named none of its buttons".to_owned(),
                            |on| format!("none of its fields has the on value {on:?}"),
                        );
                        format!("it is an exclusion group, and {which}")
                    })
            }
            _ => Err("the template container it names is not a field".to_owned()),
        }
    }

    /// Fires the pre- or post-event `activity` of an action — a save or a submit — on the object
    /// `bound` names (pages 388–389), followed by the calculations that depend on what its
    /// scripts changed (page 379), and keeps whether a script cancelled the action (page 402).
    ///
    /// `from` is the field a click on which raised the action, with its button's on value: page
    /// 398 has a secondary event leave the primary properties as the primary event set them, so
    /// the scripts read the click's `name` and `target` (page 389's example 10.7 tells two
    /// submit buttons apart by `$event.target`). A field this form does not hold, or an exclusion
    /// group's button it does not name, leaves the event with no primary property, and says so.
    /// The action clears `cancelAction` at its beginning (page 402) for either half; page 400's
    /// skipping of an action whose flag was already set before its pre-event is the caller's,
    /// which does not start the action.
    fn action(&mut self, activity: &str, from: Option<(&str, Option<&str>)>, bound: &Bound) {
        let primary = from.and_then(|(field, button)| match self.pointed_field(field, button) {
            Ok(target) => self.primary("click", target),
            Err(why) => {
                self.say(format!(
                    "{field}: the {activity} events run with no primary property of $event, \
                     since the click that raised them was on no field: {why} (XFA 3.3 page 398)"
                ));
                None
            }
        });
        self.form
            .set_event(Some(Event::for_action(primary, activity)));
        self.fire(activity, None, bound);
        self.settle(Seeds::Changed);
        self.cancelled = self.form.event().is_some_and(Event::cancelled);
    }

    /// The field of the exclusion group `group` whose on value, the first value of its first
    /// `items` (page 759's `checkButton`), is `on`: the first in document order where the
    /// template gives two fields one value.
    fn button_of(&self, group: usize, on: &str) -> Option<usize> {
        let container = self.template.get(group)?;
        container.children.iter().copied().find(|&child| {
            self.template.get(child).is_some_and(|button| {
                button.class == Class::Field
                    && button
                        .items
                        .first()
                        .and_then(|(_, values)| values.first())
                        .is_some_and(|value| value == on)
            })
        })
    }

    /// Records a sentence once.
    fn say(&mut self, mut sentence: String) {
        if self.report.len() >= MAX_REPORT {
            self.unreported = self.unreported.saturating_add(1);
            return;
        }
        // A name or a script's own message may be as long as the template, so a sentence is cut
        // where it is kept; the set that dedupes holds no more than the report does.
        if let Some((at, _)) = sentence.char_indices().nth(MAX_SENTENCE) {
            sentence.truncate(at);
        }
        if self.reported.insert(sentence.clone()) {
            self.report.push(sentence);
        }
    }

    /// What the container is called in a sentence.
    fn label(&self, node: usize) -> String {
        self.form
            .names
            .get(node)
            .cloned()
            .flatten()
            .or_else(|| self.template.get(node).and_then(|c| c.name.clone()))
            .unwrap_or_else(|| "an unnamed container".to_owned())
    }

    /// Runs `script` on `form` (the session's or a scratch copy of it) if a limit allows, and
    /// charges what it took. `Err` is the sentence for a script that ran no further.
    fn execute(
        &mut self,
        scratch: Option<&mut Form<'a>>,
        node: usize,
        language: &Language,
        script: &str,
    ) -> Result<Scalar, String> {
        let language = match (FormLanguage::of(language), language) {
            (Some(known), _) => known,
            (None, Language::Other(other)) => {
                return Err(format!("its script is in {other}, which no step runs"));
            }
            (None, _) => return Err("its script is in a language no step runs".to_owned()),
        };
        if self.activations_left == 0 {
            self.ended.get_or_insert("activations");
            return Err("the activation ran out of runs".to_owned());
        }
        if self.steps_left == 0 {
            self.ended.get_or_insert("steps");
            return Err("the activation ran out of steps".to_owned());
        }
        self.activations_left = self.activations_left.saturating_sub(1);
        self.run = self.run.saturating_add(1);
        let handed = self.steps_left;
        let form = match scratch {
            Some(form) => form,
            None => &mut *self.form,
        };
        let ran = self.runner.run(form, node, language, script, handed);
        if form.take_text_exhausted() {
            self.ended.get_or_insert("bytes");
        }
        self.steps_left = handed.saturating_sub(ran.steps);
        if ran.steps > handed {
            self.ended.get_or_insert("steps");
        }
        ran.outcome
    }

    /// Fires the `activity` events bound to `bound` of the containers `only` names, or of every
    /// container, in document order (page 404's order for scripts the standard leaves unordered).
    ///
    /// For an activity on an object (`bound` is [`Bound::Own`]), the events that respond to it are
    /// those bound to it: its own whose `ref` is `$`, and any other container's whose `ref` names
    /// it (pages 706–707, the "come-from" notation of page 404; ADR 2699). Each field, subform and
    /// exclusion group it happens on is followed by the events that hear it from above (page 387):
    /// those bound to each enclosing container that listen, innermost first, and then those bound
    /// to the Form DOM that listen. An activity on a DOM or the application ([`Bound::Form`],
    /// [`Bound::Host`]) happens on no container, so nothing contains it and it reaches no further.
    fn fire(&mut self, activity: &str, only: Option<&[usize]>, bound: &Bound) {
        let nodes: Vec<usize> = match only {
            Some(only) => only.to_vec(),
            None => (0..self.template.container_count()).collect(),
        };
        let heard = match bound {
            Bound::Own => Some(Listeners::of(self.template, self.hooks(), activity)),
            _ => None,
        };
        for node in nodes {
            if self.ended.is_some() {
                return;
            }
            let Some(container) = self.template.get(node) else {
                continue;
            };
            let Some(heard) = &heard else {
                let own: Vec<(usize, usize)> = (0..container.events.len())
                    .filter(|&at| {
                        container.events.get(at).is_some_and(|event| {
                            event.bound == *bound && event.activity == activity
                        })
                    })
                    .map(|at| (node, at))
                    .collect();
                self.dispatch(activity, &own, false);
                continue;
            };
            let event = self.primary(activity, node);
            self.form.set_event(event);
            if let Some(own) = heard.own.get(&node) {
                self.dispatch(activity, own, false);
            }
            if !matches!(
                container.class,
                Class::Field | Class::Subform | Class::ExclGroup
            ) {
                self.place_new_text(node);
                continue;
            }
            let mut above = heard.nearest.get(node).copied().flatten();
            while let Some(listener) = above {
                if let Some(events) = heard.listening.get(&listener) {
                    self.dispatch(activity, events, true);
                }
                if self.ended.is_some() {
                    return;
                }
                above = heard.nearest.get(listener).copied().flatten();
            }
            self.dispatch(activity, &heard.form, true);
            self.place_new_text(node);
        }
    }

    /// The template's events bound to objects, read once an activation: an event whose `ref` is
    /// `$` or absent is bound to its own container, and one whose `ref` is another SOM expression
    /// to each field, subform and exclusion group the expression names from the event's container
    /// (page 706), resolved as a script's path is (pages 114–115). An expression that names none,
    /// or that this reader does not resolve — `$layout`, whose `ready` this reader does not raise,
    /// among them — binds the event to nothing, and [`Self::unfollowed`] says so.
    ///
    /// The objects bound in all are held to [`Limits::names`], so a template of many events each
    /// naming every row costs at most what the cascade's dependencies may. And a resolution that
    /// searches the template is charged as many of [`Limits::steps`] as the template has
    /// containers, the most one level of the search can visit, so a template of a hundred thousand
    /// containers each naming another ends on the steps limit and keeps nothing, as a cascade that
    /// runs too long does, rather than holding the activation for the square of its size.
    fn hooks(&mut self) -> &[Hook] {
        if self.hooks.is_none() {
            let mut hooks = Vec::new();
            let mut unbound = Vec::new();
            let mut held = 0_usize;
            let containers = self.template.container_count();
            for holder in 0..containers {
                let Some(container) = self.template.get(holder) else {
                    continue;
                };
                for (at, event) in container.events.iter().enumerate() {
                    let reference = match &event.bound {
                        Bound::Own => {
                            hooks.push(Hook {
                                holder,
                                at,
                                sources: vec![holder],
                            });
                            continue;
                        }
                        Bound::Other(reference) => reference,
                        Bound::Form | Bound::Host => continue,
                    };
                    let path = Step::path(reference);
                    // `$form`, `$layout` and the other qualified names are refused before any
                    // search, and cost nothing.
                    let searches = path.as_ref().is_ok_and(|path| {
                        path.first().is_none_or(|first| {
                            first.name == "$"
                                || !(first.name.starts_with(['$', '!']) || first.name == "xfa")
                        })
                    });
                    if searches {
                        let cost = u64::try_from(containers).unwrap_or(u64::MAX);
                        if self.steps_left < cost {
                            self.ended.get_or_insert("steps");
                            break;
                        }
                        self.steps_left = self.steps_left.saturating_sub(cost);
                    }
                    let resolved = path
                        .and_then(|path| self.form.resolve_path(holder, &path))
                        .map(|found| {
                            found
                                .into_iter()
                                .filter(|&node| {
                                    self.template.get(node).is_some_and(|c| {
                                        matches!(
                                            c.class,
                                            Class::Field | Class::Subform | Class::ExclGroup
                                        )
                                    })
                                })
                                .collect::<Vec<_>>()
                        });
                    let why = match resolved {
                        Ok(sources) if sources.is_empty() => {
                            "which names no field, subform or exclusion group of the form"
                                .to_owned()
                        }
                        Ok(sources) if held.saturating_add(sources.len()) > self.max_names => {
                            format!(
                                "which names more objects than an activation binds ({})",
                                self.max_names
                            )
                        }
                        Ok(sources) => {
                            held = held.saturating_add(sources.len());
                            hooks.push(Hook {
                                holder,
                                at,
                                sources,
                            });
                            continue;
                        }
                        // `$layout`, `$data` and the other DOMs page 388 and chapter 6 name
                        // generate events this reader does not raise, whatever they would name.
                        Err(_)
                            if reference.trim_start().starts_with(['$', '!'])
                                || reference.trim_start().starts_with("xfa.") =>
                        {
                            "which this reader raises no event for".to_owned()
                        }
                        Err(why) => format!("which this reader does not resolve: {why}"),
                    };
                    let label = self
                        .form
                        .names
                        .get(holder)
                        .cloned()
                        .flatten()
                        .or_else(|| container.name.clone())
                        .unwrap_or_else(|| format!("container {holder}"));
                    unbound.push(format!(
                        "{label}: its {} event is bound to {reference}, {why}, and is not run \
                         (XFA 3.3 pages 404 and 707)",
                        event.activity
                    ));
                }
            }
            self.hooks = Some(hooks);
            self.unbound = unbound;
        }
        self.hooks.as_deref().unwrap_or_default()
    }

    /// Runs the events `events` names, each by its container and its place among that container's
    /// events, in the order given. An event that hears the activity from below (`heard`) is
    /// charged one run of [`Limits::activations`] even where it performs nothing, so the
    /// dispatches an activity reaches above the object are bounded by the runs an activation may
    /// make, whatever a template nests or repeats.
    fn dispatch(&mut self, activity: &str, events: &[(usize, usize)], heard: bool) {
        for &(node, at) in events {
            if self.ended.is_some() {
                return;
            }
            let Some(performs) = self
                .template
                .get(node)
                .and_then(|container| container.events.get(at))
                .map(|event| event.performs.clone())
            else {
                continue;
            };
            let script = match performs {
                Performs::Script(script) => script,
                other => {
                    if heard {
                        if self.activations_left == 0 {
                            self.ended.get_or_insert("activations");
                            return;
                        }
                        self.activations_left = self.activations_left.saturating_sub(1);
                    }
                    self.perform(activity, node, &other);
                    continue;
                }
            };
            if let Err(why) = self.execute(None, node, &script.language, &script.text) {
                let label = self.label(node);
                self.say(format!("{label}: its {activity} event: {why}"));
            }
        }
    }

    /// Carries out an event's property other than its `script` (page 703): a `submit` is held for
    /// the host as a [`FormSubmit`] where it composes, and every other outcome is said (ADR 2738).
    ///
    /// A `submit` under a submit's own `preSubmit` or `postSubmit` is not performed: page 446
    /// pairs each pre-event with its post-event around one transaction and says nothing of a
    /// transaction begun inside another, and one begun there would start the next submit at every
    /// answer a server sends, so it is said instead — a choice where the pages are silent.
    fn perform(&mut self, activity: &str, node: usize, performs: &Performs) {
        let label = self.label(node);
        match performs {
            Performs::Script(_) => {}
            Performs::Submit(submit) => match submit.action() {
                Err(why) => self.say(format!(
                    "{label}: its {activity} event's submit {why} (XFA 3.3 page 858)"
                )),
                Ok(_) if self.origin == Origin::Submitting => self.say(format!(
                    "{label}: its {activity} event's submit is not performed: it would begin a \
                     submit inside the one under way (XFA 3.3 page 446)"
                )),
                Ok(_) if self.submits.len() >= MAX_SUBMITS => self.say(format!(
                    "{label}: its {activity} event's submit is not performed: the activation \
                     has already performed {MAX_SUBMITS}, the most it sends"
                )),
                Ok((action, owed)) => self.submits.push(FormSubmit {
                    action,
                    label: format!("{label}'s {activity} event"),
                    clicked: match &self.origin {
                        Origin::Click(clicked) => Some(clicked.clone()),
                        _ => None,
                    },
                    owed,
                }),
            },
            Performs::Execute(execute) => {
                if let Some(why) = execute.unperformed() {
                    self.say(format!(
                        "{label}: its {activity} event's execute {why} (XFA 3.3 page 722)"
                    ));
                }
            }
            Performs::Unperformed(property) => self.say(format!(
                "{label}: its {activity} event's {property} is not performed (XFA 3.3 page 703)"
            )),
        }
    }

    /// Names every event the form holds whose `ref` binds it to nothing this reader raises an event
    /// for ([`Self::hooks`]), which never runs (pages 404 and 707). [`Self::say`] keeps a sentence
    /// once and caps the report.
    fn unfollowed(&mut self) {
        self.hooks();
        for sentence in std::mem::take(&mut self.unbound) {
            self.say(sentence);
        }
    }

    /// The position in `calculations` of the container's calculation.
    fn index_of(&self, node: usize) -> Option<usize> {
        self.calculations.iter().position(|c| c.node == node)
    }

    /// Records what the calculation at `index` named when it last ran, keeping `readers_of` its
    /// inverse. A calculation does not wake itself (page 380's circular reference is the form
    /// creator's to prevent), so its own container is left out.
    fn record(&mut self, index: usize, mut named: BTreeSet<usize>) {
        let Some(calculation) = self.calculations.get(index) else {
            return;
        };
        named.remove(&calculation.node);
        let old = self
            .named
            .get_mut(index)
            .map(std::mem::take)
            .unwrap_or_default();
        self.edges = self.edges.saturating_sub(old.len());
        for node in &old {
            if let Some(readers) = self.readers_of.get_mut(node) {
                readers.remove(&index);
            }
        }
        self.edges = self.edges.saturating_add(named.len());
        for &node in &named {
            self.readers_of.entry(node).or_default().insert(index);
        }
        if let Some(slot) = self.named.get_mut(index) {
            *slot = named;
        }
        if self.edges > self.max_names {
            self.ended.get_or_insert("names");
        }
    }

    /// How deep the container is: the count of containers above it.
    fn depth(&self, node: usize) -> usize {
        let mut depth = 0_usize;
        let mut at = self.template.get(node).and_then(|c| c.parent);
        while let Some(parent) = at {
            depth = depth.saturating_add(1);
            if depth > self.template.container_count() {
                break;
            }
            at = self.template.get(parent).and_then(|c| c.parent);
        }
        depth
    }

    /// The rank of a calculation in the work list: value calculations before property
    /// calculations, inner before outer among those, then the template's order.
    fn rank(&self, index: usize) -> (bool, usize, usize) {
        match self.calculations.get(index) {
            Some(calculation) if calculation.sets_value => (
                false,
                self.order.get(index).copied().unwrap_or(index),
                index,
            ),
            Some(calculation) => (
                true,
                usize::MAX.saturating_sub(self.depth(calculation.node)),
                index,
            ),
            None => (true, usize::MAX, index),
        }
    }

    /// Learns what every calculation names, by running each once on a copy of the form, because
    /// a person's entry has to find the calculations that read it before any of them has run.
    fn discover(&mut self) {
        self.known = true;
        let mut scratch = self.form.clone();
        let room = scratch.text_room();
        for index in 0..self.calculations.len() {
            let Some(calculation) = self.calculations.get(index).cloned() else {
                continue;
            };
            if calculation.override_mode == Override::Disabled || self.ended.is_some() {
                continue;
            }
            scratch.take_resolved();
            // Whether it ran or was refused, what it resolved before stopping is what it reads.
            let outcome = self.execute(
                Some(&mut scratch),
                calculation.node,
                &calculation.language,
                &calculation.script,
            );
            if let (Ok(value), true) = (outcome, calculation.sets_value) {
                let _ = scratch.set(calculation.node, value);
                if scratch.take_text_exhausted() {
                    self.ended.get_or_insert("bytes");
                }
            }
            let named = scratch.take_resolved();
            self.record(index, named);
        }
        // What the copy stored is charged to the form the activation keeps.
        self.form.limit_text(
            self.form
                .text_room()
                .saturating_sub(room.saturating_sub(scratch.text_room())),
        );
        self.order = self.inputs_first();
    }

    /// The position of each calculation in an order that runs the calculations a calculation reads
    /// before it: Kahn's algorithm over the names they hold, taking the template's order among
    /// those that are ready, and where only a cycle is left, the first of it in the template's
    /// order. The cascade of pages 379–380 needs no order to be right, but it needs this one to be
    /// quick: run in the template's order, a chain whose links come in reverse runs each link
    /// again for every link after it.
    fn inputs_first(&self) -> Vec<usize> {
        let count = self.calculations.len();
        let mut before: Vec<usize> = vec![0; count];
        let mut after: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (index, calculation) in self.calculations.iter().enumerate() {
            if !calculation.sets_value {
                continue;
            }
            let Some(readers) = self.readers_of.get(&calculation.node) else {
                continue;
            };
            for &reader in readers {
                let reads_value = self.calculations.get(reader).is_some_and(|c| c.sets_value);
                if reader != index
                    && reads_value
                    && let (Some(count), Some(list)) =
                        (before.get_mut(reader), after.get_mut(index))
                {
                    *count = count.saturating_add(1);
                    list.push(reader);
                }
            }
        }
        let mut ready: BTreeSet<usize> = (0..count).filter(|&i| before[i] == 0).collect();
        let mut placed = vec![usize::MAX; count];
        let mut next = 0_usize;
        let mut waiting: BTreeSet<usize> = (0..count).filter(|&i| before[i] != 0).collect();
        while next < count {
            let Some(index) = ready.pop_first().or_else(|| waiting.pop_first()) else {
                break;
            };
            waiting.remove(&index);
            if let Some(slot) = placed.get_mut(index) {
                *slot = next;
            }
            next = next.saturating_add(1);
            for &reader in after.get(index).map_or(&[][..], Vec::as_slice) {
                if placed.get(reader) != Some(&usize::MAX) {
                    continue;
                }
                if let Some(count) = before.get_mut(reader) {
                    *count = count.saturating_sub(1);
                    if *count == 0 && waiting.remove(&reader) {
                        ready.insert(reader);
                    }
                }
            }
        }
        placed
    }

    /// What the `override` of each calculation of a field a person entered into does (pages
    /// 641–642): `error` and `ignore` put the calculation back on the work list, so the calculated
    /// value replaces the person's; `warning` leaves the person's value standing. The messages the
    /// page asks to be presented are the host's: [`Activation::overridden`] names the fields they
    /// are owed for, and the report says so in words.
    fn entered_overrides(&mut self, typed: &[usize], wanted: &mut BTreeSet<usize>) {
        for &node in typed {
            let Some(index) = self.index_of(node) else {
                continue;
            };
            let mode = self.calculations.get(index).map(|c| c.override_mode);
            let label = self.label(node);
            if let (Some(mode @ (Override::Error | Override::Warning)), Some(field)) =
                (mode, self.form.names.get(node).cloned().flatten())
            {
                self.overridden.push(Overridden { field, mode });
            }
            match mode {
                Some(Override::Error) => {
                    wanted.insert(index);
                    self.say(format!(
                        "{label}: its calculation has override error, so the value \
                         entered is replaced by the calculated one; the error message \
                         page 642 asks to be presented is the host's to show"
                    ));
                }
                Some(Override::Ignore) => {
                    wanted.insert(index);
                }
                Some(Override::Warning) => self.say(format!(
                    "{label}: its calculation has override warning, so the value entered \
                     stands; the warning page 642 asks to be presented, with its choice \
                     of dismissing or overriding, is the host's to show"
                )),
                Some(Override::Disabled) | None => {}
            }
        }
    }

    /// Runs the work list from `seeds` until nothing is left to run or a limit ends it.
    fn settle(&mut self, seeds: Seeds<'_>) {
        if self.ended.is_some() {
            return;
        }
        let mut wanted: BTreeSet<usize> = BTreeSet::new();
        match seeds {
            Seeds::All => {
                self.discover();
                wanted.extend(0..self.calculations.len());
            }
            Seeds::Entered(typed) => {
                if !self.known {
                    self.discover();
                }
                self.entered_overrides(typed, &mut wanted);
                let mut changed: BTreeSet<usize> = self.form.take_changed();
                changed.extend(typed.iter().copied());
                wanted.extend(self.readers(&changed, None));
            }
            Seeds::Changed => {
                let changed = self.form.take_changed();
                if !self.known && !changed.is_empty() {
                    self.discover();
                }
                wanted.extend(self.readers(&changed, None));
            }
            Seeds::Dismissed(fields) => {
                if !self.known {
                    self.discover();
                }
                for &node in fields {
                    if let Some(index) = self.index_of(node) {
                        wanted.insert(index);
                    }
                }
            }
        }
        wanted.retain(|&index| {
            self.calculations
                .get(index)
                .is_some_and(|c| c.override_mode != Override::Disabled)
        });
        let mut queue: BTreeSet<(bool, usize, usize)> =
            wanted.iter().map(|&index| self.rank(index)).collect();
        let mut queued = wanted;
        while let Some((_, _, index)) = queue.pop_first() {
            queued.remove(&index);
            if self.ended.is_some() {
                return;
            }
            let Some(calculation) = self.calculations.get(index).cloned() else {
                continue;
            };
            self.form.take_resolved();
            let _ = self.form.take_changed();
            let outcome = self.execute(
                None,
                calculation.node,
                &calculation.language,
                &calculation.script,
            );
            let named = self.form.take_resolved();
            match outcome {
                Ok(value) if calculation.sets_value => {
                    let stored = self.form.set(calculation.node, value);
                    if self.form.take_text_exhausted() {
                        self.ended.get_or_insert("bytes");
                    }
                    if let Err(why) = stored {
                        let label = self.label(calculation.node);
                        self.say(format!("{label}: {why}"));
                    }
                }
                Err(why) if self.ended.is_none() => {
                    let label = self.label(calculation.node);
                    self.say(format!("{label}: its calculation: {why}"));
                }
                Ok(_) | Err(_) => {}
            }
            self.record(index, named);
            let changed = self.form.take_changed();
            let woken: BTreeSet<usize> = self
                .readers(&changed, Some(index))
                .into_iter()
                .filter(|j| !queued.contains(j))
                .filter(|&j| {
                    self.calculations
                        .get(j)
                        .is_some_and(|c| c.override_mode != Override::Disabled)
                })
                .collect();
            queue.extend(woken.iter().map(|&index| self.rank(index)));
            queued.extend(woken);
        }
    }

    /// The calculations a change to `changed` wakes, other than `except`: those that named one of
    /// them, and a subform's, which counts as changed whenever a container inside it does
    /// (page 379).
    fn readers(&self, changed: &BTreeSet<usize>, except: Option<usize>) -> Vec<usize> {
        let mut woken: BTreeSet<usize> = changed
            .iter()
            .filter_map(|node| self.readers_of.get(node))
            .flatten()
            .copied()
            .collect();
        let mut enclosing = BTreeSet::new();
        for &node in changed {
            let mut at = self.template.get(node).and_then(|c| c.parent);
            while let Some(parent) = at {
                if !enclosing.insert(parent) {
                    break;
                }
                at = self.template.get(parent).and_then(|c| c.parent);
            }
        }
        woken.extend(
            enclosing
                .iter()
                .filter_map(|node| self.properties.get(node)),
        );
        woken.retain(|&index| Some(index) != except);
        woken.into_iter().collect()
    }

    /// The containers this activation changed: those whose value differs from `initial`, and
    /// those a person `typed` into, whose entry is in `initial` already.
    fn changed_since(&self, initial: &[Scalar], typed: &[usize]) -> BTreeSet<usize> {
        let mut changed: BTreeSet<usize> = typed.iter().copied().collect();
        changed.extend(
            self.form
                .values
                .iter()
                .enumerate()
                .filter(|&(node, value)| initial.get(node) != Some(value))
                .map(|(node, _)| node),
        );
        changed
    }

    /// [`format_tests`] of `values`, each sentence said.
    fn format_tests(
        &mut self,
        resource: (&Template, &crate::xfa::LocaleSet),
        values: &[(String, Change)],
        changed: Option<&BTreeSet<usize>>,
    ) -> (Vec<Invalid>, Vec<(usize, bool)>) {
        let (invalid, verdicts, sentences) = format_tests(resource, values, changed);
        for sentence in sentences {
            self.say(sentence);
        }
        (invalid, verdicts)
    }

    /// Runs the script test of every container whose `validate` element states a script and does
    /// not disable the test (XFA 3.3 pages 382 and 881), and answers the containers it fails.
    ///
    /// `changed` is `None` for a merge, where every container is tested (page 382's
    /// initialization). Otherwise a container's failure is answered where it is among `changed`,
    /// is around one of them — page 382 tests a subform on exit where a value was entered into
    /// a field it contains — or its script read one of them: the dependency tracking page 382
    /// asks for and page 396 says the script test is re-evaluated by. What a script read is what
    /// it resolved while it ran, as a calculation's dependencies are (page 380), so every script
    /// is run and the ones `changed` does not reach are not answered.
    ///
    /// **The test is of a value.** Page 396 applies the script test to no field holding null or
    /// empty content, which the null test answers for. A script is true where its value promotes
    /// to a nonzero number, as chapter 25 promotes a value a boolean operation reads (page 1061),
    /// a JavaScript `true` reaching this module as 1; a script refused or stopped decides nothing
    /// and is reported, not failed.
    ///
    /// **A validation alters nothing.** Page 372 says a validate script must not alter the form
    /// and that a processor may not be able to enforce it, and page 385 says the same of the
    /// script test. This processor can: every script runs on one copy of the form, a value it
    /// assigns is put back before the next one runs, and the assignment is reported. The copy's
    /// assignments are charged to the activation's text budget as discovery's are (ADR 2696).
    fn script_tests(
        &mut self,
        changed: Option<&BTreeSet<usize>>,
    ) -> (Vec<Invalid>, Vec<(usize, bool)>) {
        if changed.is_some_and(BTreeSet::is_empty) {
            return (Vec::new(), Vec::new());
        }
        let tests: Vec<(usize, Script, Level, Option<String>)> =
            (0..self.template.container_count())
                .filter_map(|node| {
                    let container = self.template.get(node)?;
                    let level = match container.pictures.script_test {
                        ScriptTest::Error => Level::Error,
                        ScriptTest::Warning => Level::Warning,
                        ScriptTest::Disabled => return None,
                    };
                    let script = container.validation.clone()?;
                    let message = container.pictures.message("scriptTest").map(str::to_owned);
                    Some((node, script, level, message))
                })
                .collect();
        if tests.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let mut around = BTreeSet::new();
        for &node in changed.into_iter().flatten() {
            let mut at = self.template.get(node).and_then(|c| c.parent);
            while let Some(parent) = at {
                if !around.insert(parent) {
                    break;
                }
                at = self.template.get(parent).and_then(|c| c.parent);
            }
        }
        let mut scratch = self.form.clone();
        // A validation is no person's action, so no primary event is under it (page 398).
        scratch.set_event(None);
        let room = scratch.text_room();
        let mut invalid = Vec::new();
        // Each answered container's verdict, for its validation state (ADR 2706): a field holding
        // nothing is put to no script test (page 396), so it fails none.
        let mut verdicts = Vec::new();
        for (node, script, level, message) in tests {
            if self.ended.is_some() {
                break;
            }
            let holds_a_value = match scratch.value(node) {
                Scalar::Null => false,
                Scalar::Text(text) => !text.is_empty(),
                Scalar::Number(_) => true,
            };
            let valued = self
                .template
                .get(node)
                .is_some_and(|c| matches!(c.class, Class::Field | Class::ExclGroup));
            if valued && !holds_a_value {
                let answered = changed.is_none_or(|changed| changed.contains(&node));
                if answered {
                    verdicts.push((node, false));
                }
                continue;
            }
            scratch.take_resolved();
            let _ = scratch.take_changed();
            let outcome = self.execute(Some(&mut scratch), node, &script.language, &script.text);
            let read = scratch.take_resolved();
            let altered = scratch.take_changed();
            let label = self.label(node);
            if !altered.is_empty() {
                for &at in &altered {
                    if let (Some(slot), Some(kept)) =
                        (scratch.values.get_mut(at), self.form.values.get(at))
                    {
                        slot.clone_from(kept);
                    }
                }
                self.say(format!(
                    "{label}: its validation script assigned a value, which a validation may not \
                     do (XFA 3.3 pages 372 and 385), so the assignment is not kept"
                ));
            }
            let answered = changed.is_none_or(|changed| {
                changed.contains(&node) || around.contains(&node) || !read.is_disjoint(changed)
            });
            if answered && let Ok(value) = &outcome {
                verdicts.push((node, !true_value(value)));
            }
            match outcome {
                Ok(value) if answered && !true_value(&value) => invalid.push(Invalid {
                    container: label,
                    test: Test::Script,
                    level,
                    message,
                }),
                Err(why) if answered && self.ended.is_none() => self.say(format!(
                    "{label}: its validation script: {why}, so its script test decides nothing"
                )),
                Ok(_) | Err(_) => {}
            }
        }
        self.form.limit_text(
            self.form
                .text_room()
                .saturating_sub(room.saturating_sub(scratch.text_room())),
        );
        (invalid, verdicts)
    }

    /// The activation's answer.
    fn finish(mut self, initial: &[Scalar], script_tests: Vec<Vec<Invalid>>) -> Activation {
        if let Some(limit) = self.ended {
            let sentence = format!(
                "the activation ended on its {limit} limit after {} runs, and nothing it \
                 calculated is kept (page 380)",
                self.run
            );
            // The limit's sentence is the one a person most needs, so it is kept past the cap.
            self.report.push(sentence);
        }
        let mut edits = Vec::new();
        if self.ended.is_none() {
            for node in 0..self.form.assigned.len() {
                if !self.form.assigned.get(node).copied().unwrap_or(false)
                    || self.form.values.get(node) == initial.get(node)
                {
                    continue;
                }
                if let Some(name) = self.form.names.get(node).cloned().flatten() {
                    edits.push((name, self.form.value(node)));
                } else {
                    let label = self.label(node);
                    self.say(format!(
                        "{label}: a script set its value, and the form's field list does not \
                         name it, so the value has no field to go to"
                    ));
                }
            }
        }
        if self.unreported > 0 {
            let sentence = format!(
                "and {} more sentences, past the {MAX_REPORT} an activation reports",
                self.unreported
            );
            self.report.push(sentence);
        }
        Activation {
            edits,
            report: self.report,
            ended: self.ended.map_or(Ended::Settled, Ended::Budget),
            activations: self.run,
            script_tests: if self.ended.is_none() {
                script_tests
            } else {
                Vec::new()
            },
            overridden: if self.ended.is_none() {
                self.overridden
            } else {
                Vec::new()
            },
            cancelled: self.cancelled,
            submits: if self.ended.is_none() {
                self.submits
            } else {
                Vec::new()
            },
        }
    }
}
