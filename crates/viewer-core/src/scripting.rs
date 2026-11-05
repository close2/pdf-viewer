//! RFC 0008 section 6.3's policy hook as this crate holds it: whether a document's scripts run,
//! and, at the *ask* level, the one question a document puts before its first does.
//!
//! **The level is the host's and the runner is the host's**; what is this crate's is the
//! document. A runner keeps one document's realm (`pdf_model::view::ScriptRunner`), so a host
//! supplies a maker of runners rather than one ([`ScriptRunners`]) and every document this viewer
//! opens is handed one of its own. Rule 2 is why the maker is the host's: the runner that runs a
//! script starts a confined process, and this crate starts none.
//!
//! **The question is asked at the first trigger, never per trigger.** A keystroke script fires per
//! key, so *ask* is one question per document: until it is answered a document's view state holds
//! a [`Withheld`] runner, which runs nothing, says so, and remembers the first script it was
//! handed — the line a person reads before deciding. The answer holds for the document's lifetime,
//! and a `yes` runs what was withheld: the open sequence again where it has run, then Table 224's
//! `/CO` over the values already there (ADR 1616).

use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};

use crate::{AlertButtons, AlertIcon, ScriptAnswer, ScriptQuestion};

/// What makes one runner per document: the host's half of RFC 0008 section 6.3's hook.
///
/// A host whose reader's level runs scripts supplies one in [`Scripting::Run`] or
/// [`Scripting::Ask`]; this crate asks it for a runner each time a document is to run scripts
/// under a new answer, so that two documents never share a realm (ADR 1602).
pub trait ScriptRunners: std::fmt::Debug + Send + Sync {
    /// A runner for one document, holding no realm yet.
    ///
    /// Called when a document opens under [`Scripting::Run`], when one is answered `yes` under
    /// [`Scripting::Ask`], and for every open document when a host sends [`Scripting`] again. A
    /// runner whose construction would cost a process must not pay it here: the first trigger is
    /// where it is started (RFC 0008 section 6.6).
    fn runner(&self) -> Arc<dyn ScriptRunner>;

    /// A runner for one document, and where its scripts' questions to the person wait.
    ///
    /// What this crate calls. The default is [`Self::runner`] with nowhere to wait, which is a
    /// runner whose `app.alert` and `app.response` the person never sees; a maker whose runners
    /// can hold a script on a person's answer overrides it (ADR 1628).
    fn runner_that_asks(&self) -> (Arc<dyn ScriptRunner>, Option<Arc<dyn ScriptAsks>>) {
        (self.runner(), None)
    }
}

/// Where one document's script waits on the person: RFC 0008 section 4.2's `app.alert` and
/// `app.response`, the two calls that need a host.
///
/// **The window's thread never waits.** A question is handed over as data after the command that
/// raised it, [`crate::Event::ScriptAsking`], and the answer comes back as a command,
/// [`crate::Command::AnswerScript`] — so a window that answers a dialogue from its event loop, and
/// one that draws its own card between frames, answer it the same way. What waits is the script,
/// in its confined worker, for as long as its runner's own bound allows; a question whose wait has
/// run out is the runner's to drop and to say so (ADR 1628).
pub trait ScriptAsks: std::fmt::Debug + Send + Sync {
    /// The question a script is waiting on, handed over once: a second call answers `None` until
    /// another script asks.
    fn take_question(&self) -> Option<ScriptQuestion>;

    /// The person's answer to the question last taken. A runner whose script is no longer
    /// waiting — the wait ran out, or the worker was lost — keeps nothing of it.
    fn answer(&self, answer: ScriptAnswer);

    /// Whether the question last taken has been withdrawn since this was last asked: its wait ran
    /// out and the runner answered it as a closed dialogue answers. True once per withdrawal, and
    /// asked before [`Self::take_question`] after every command, which is
    /// [`crate::Event::ScriptQuestionWithdrawn`]'s order (ADR 1643).
    ///
    /// The default is a runner whose questions are never withdrawn — one that waits for as long as
    /// the person takes.
    fn withdrawn(&self) -> bool {
        false
    }
}

/// Whether this viewer runs a document's scripts, as the host's level for them says
/// ([`crate::Command::Scripts`]).
///
/// Three values for the four levels RFC 0008 section 6.3 names, because *warn* and *on* differ in
/// what the runner says rather than in whether it runs — that is the runner's, and so the host's.
/// Closed, and **not** `#[non_exhaustive]`, for `doc/ui-boundary.md`'s reason.
#[derive(Debug, Clone, Default)]
pub enum Scripting {
    /// No runner: every script a view state does not run itself is reported as not run, and no
    /// process is started. **The default** — the owner's answer to RFC 0008's first question
    /// (`doc/questions/A193`).
    #[default]
    Off,
    /// One question per document, [`crate::Event::AskingToRunScripts`], put at its first trigger
    /// and answered with [`crate::Command::AnswerScripts`]; nothing runs until a `yes`.
    Ask(Arc<dyn ScriptRunners>),
    /// Every document is handed a runner when it opens.
    Run(Arc<dyn ScriptRunners>),
}

/// The longest first line a question shows, in characters.
///
/// A script's first line is the document's text, so its length is too; a person reading a card
/// wants enough of it to recognise what the form does, and a minified library is one line.
const FIRST_LINE: usize = 160;

/// What one document's scripts stand at.
#[derive(Debug, Clone, Default)]
pub(crate) struct Consent {
    /// The person's answer under [`Scripting::Ask`], once given: it holds while the document is
    /// open, at whatever level the host sends afterwards.
    pub(crate) answered: Option<bool>,
    /// The runner standing in while the question is unasked or unanswered, or after a `no`.
    pub(crate) withheld: Option<Arc<Withheld>>,
    /// Where the document's runner leaves its scripts' questions to the person, where it has one.
    pub(crate) asks: Option<Arc<dyn ScriptAsks>>,
    /// The notices a person's entry into a calculated field raised, and the messages of the
    /// script tests an XFA form failed, that no window has been shown yet, oldest first, at most
    /// [`MAX_FORM_ALERTS`] (XFA 3.3 pages 383 and 641–642; ADRs 2525, 2696).
    pub(crate) form_alerts: std::collections::VecDeque<FormAlert>,
    /// Whether a notice from [`Self::form_alerts`] has gone out and is not yet answered.
    pub(crate) form_alert_open: bool,
    /// What the `warning` the notice that has gone out asks about is, where it is one: its answer
    /// is the person's choice page 642 or page 385 describes (ADRs 2697, 2698).
    pub(crate) form_alert_warns: Option<Warns>,
    /// Whether a question the runner's script put has gone out and is not yet answered, so that
    /// the answer a window sends is known to be the runner's and not a notice's.
    pub(crate) runner_question_open: bool,
}

/// A notice an XFA form owes the person, and the `warning` it asks about where its answer is the
/// choice page 642 or page 385 gives (ADRs 2697, 2698).
#[derive(Debug, Clone)]
pub(crate) struct FormAlert {
    /// What the window is asked to show.
    pub(crate) question: ScriptQuestion,
    /// The `warning` a "No" dismisses, for a notice that asks.
    pub(crate) warns: Option<Warns>,
}

/// The `warning` a notice asks a person to dismiss or override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Warns {
    /// A calculated field's, by its PDF field name: a "No" leaves the calculated value
    /// undisturbed (page 642; ADR 2697).
    Calculated(String),
    /// The script tests of these containers, by the names the activation gave them: a "No"
    /// returns the person to the form, at the first of them that is a field (page 385; ADR 2698).
    Validated(Vec<String>),
}

/// Most notices a document holds for a window that has not yet shown the ones before them.
pub(crate) const MAX_FORM_ALERTS: usize = 64;

/// What a [`Withheld`] runner has seen.
#[derive(Debug, Default)]
struct Seen {
    /// The first script it was handed: who runs it, and its first line.
    first: Option<(String, String)>,
    /// Whether the question about it has been put.
    asked: bool,
    /// Whether the person answered `no`.
    declined: bool,
}

/// The runner a document holds under [`Scripting::Ask`] until its question is answered `yes`.
///
/// Runs nothing and changes nothing: every result is `rc` true with no value and no edit, so a
/// keystroke goes through and a commit stands, and its one sentence says why the script did not
/// run. What it adds is memory — the first script it is handed is what the question shows.
#[derive(Debug, Default)]
pub(crate) struct Withheld {
    /// What it has seen, behind a lock because a runner is shared (`ScriptRunner: Sync`).
    seen: Mutex<Seen>,
}

impl Withheld {
    /// One that has seen nothing and been answered nothing.
    pub(crate) fn waiting() -> Self {
        Self::default()
    }

    /// One the person has answered `no`.
    pub(crate) fn declined() -> Self {
        Self {
            seen: Mutex::new(Seen {
                declined: true,
                ..Seen::default()
            }),
        }
    }

    /// The question to put now — who runs the first script, and its first line — once, and only
    /// after a script has been handed over.
    pub(crate) fn question(&self) -> Option<(String, String)> {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.asked || seen.declined {
            return None;
        }
        let first = seen.first.clone()?;
        seen.asked = true;
        Some(first)
    }
}

impl ScriptRunner for Withheld {
    fn run_form_script(
        &self,
        _form: &mut pdf_model::xfa::Form<'_>,
        _node: usize,
        language: pdf_model::xfa::FormLanguage,
        script: &str,
        _steps_left: u64,
    ) -> pdf_model::xfa::Ran {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let sentence = if seen.declined {
            "not run: you declined to run this document's scripts, and the answer holds until it \
             closes"
        } else {
            if seen.first.is_none() {
                seen.first = Some((
                    format!("a {} calculation of the document's form", language.name()),
                    first_line(script),
                ));
            }
            "not run: this reader asks before it runs a document's scripts, and the question is \
             waiting on your answer (RFC 0008 section 6.3)"
        };
        pdf_model::xfa::Ran {
            steps: 0,
            outcome: Err(sentence.to_owned()),
        }
    }

    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let sentence = if seen.declined {
            "not run: you declined to run this document's scripts, and the answer holds until it \
             closes"
        } else {
            if seen.first.is_none() {
                seen.first = Some((subject(event), first_line(event.script)));
            }
            "not run: this reader asks before it runs a document's scripts, and the question is \
             waiting on your answer (RFC 0008 section 6.3)"
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: vec![sentence.to_owned()],
        }
    }
}

/// Who runs a script, as a question names it: `the calculate script of Total`.
fn subject(event: &ScriptEvent<'_>) -> String {
    let of = |what: &str| {
        if event.field.is_empty() {
            what.to_owned()
        } else {
            event.field.to_owned()
        }
    };
    match event.site {
        ScriptSite::Field(trigger) => format!("the {} script of {}", trigger.noun(), event.field),
        ScriptSite::Annotation(trigger) => {
            format!("the /{} script of {}", trigger.key(), of("an annotation"))
        }
        ScriptSite::Page(_) => format!("a script of page {}", event.page.saturating_add(1)),
        ScriptSite::OpenAction => "the document's open action".to_owned(),
        ScriptSite::Library => format!("the document-level script {:?}", event.label),
        ScriptSite::Document(trigger) => format!("the document's /{} script", trigger.key()),
        ScriptSite::Timer => "a timer's expression".to_owned(),
        ScriptSite::Outline { .. } => format!("the script of the outline item {:?}", event.label),
    }
}

/// A script's first line that says anything, trimmed and held to [`FIRST_LINE`] characters.
fn first_line(script: &str) -> String {
    let line = script
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if line.chars().count() <= FIRST_LINE {
        return line.to_owned();
    }
    let mut cut: String = line.chars().take(FIRST_LINE).collect();
    cut.push('…');
    cut
}

/// The runner one document holds under `scripting`, given what its reader answered: `None` for no
/// runner at all, which is the level `off`.
///
/// Updates `consent` in passing: a document asked under [`Scripting::Ask`] and not yet answered
/// keeps the [`Withheld`] it has, so that a host sending its level again does not ask again.
pub(crate) fn runner_for(
    scripting: &Scripting,
    consent: &mut Consent,
) -> Option<Arc<dyn ScriptRunner>> {
    consent.asks = None;
    match scripting {
        Scripting::Off => {
            consent.withheld = None;
            None
        }
        Scripting::Run(runners) => {
            consent.withheld = None;
            Some(asking(runners.as_ref(), consent))
        }
        Scripting::Ask(runners) => match consent.answered {
            Some(true) => {
                consent.withheld = None;
                Some(asking(runners.as_ref(), consent))
            }
            Some(false) => {
                let declined = Arc::new(Withheld::declined());
                consent.withheld = Some(Arc::clone(&declined));
                Some(declined)
            }
            None => {
                let waiting = consent
                    .withheld
                    .clone()
                    .unwrap_or_else(|| Arc::new(Withheld::waiting()));
                consent.withheld = Some(Arc::clone(&waiting));
                Some(waiting)
            }
        },
    }
}

/// A runner from `runners`, with where its questions wait kept in `consent`.
fn asking(runners: &dyn ScriptRunners, consent: &mut Consent) -> Arc<dyn ScriptRunner> {
    let (runner, asks) = runners.runner_that_asks();
    consent.asks = asks;
    runner
}

/// The window's view of one document as its realm reads it — `this.zoom`, `this.zoomType`,
/// `this.layout` — from how large the page is drawn, the zoom that chose it and Table 29's
/// arrangement (ADR 1736).
///
/// `magnification` is logical pixels per default user space unit, which the reference's 100 per
/// cent is: the reading a destination's `/XYZ` magnification of 1 already has here. A fitting
/// mode is the zoom type of the same name, and a fixed scale is `NoVary`.
pub(crate) fn window_view(
    zoom: crate::Zoom,
    layout: pdf_model::viewer_preferences::PageLayout,
    magnification: Option<f32>,
) -> pdf_model::view::WindowView {
    use pdf_model::view::ZoomType;
    pdf_model::view::WindowView {
        zoom: magnification.map(|logical| f64::from(logical) * 100.0),
        zoom_type: match zoom {
            crate::Zoom::FitPage => ZoomType::FitPage,
            crate::Zoom::FitWidth => ZoomType::FitWidth,
            crate::Zoom::FitHeight => ZoomType::FitHeight,
            crate::Zoom::Scale(_) | crate::Zoom::In | crate::Zoom::Out => ZoomType::NoVary,
        },
        layout,
    }
}

/// Whether a runner from `scripting` is a real one — one that runs what it is handed — for a
/// document that has `consent`.
pub(crate) fn runs(scripting: &Scripting, consent: &Consent) -> bool {
    match scripting {
        Scripting::Off => false,
        Scripting::Run(_) => true,
        Scripting::Ask(_) => consent.answered == Some(true),
    }
}

/// The notice XFA 3.3 page 642 asks a host to present when a person enters a value into a
/// calculated field whose `override` is `error` or `warning`.
///
/// The words are this program's: the page describes the dialogue and states none. An `error` says
/// the calculated value replaced the entry, and has one button (ADR 2525). A `warning` informs the
/// person that the field is recommended to have its calculated value and gives page 642's two
/// choices as one question: "Yes" overrides, keeping the entry, which already stands; "No"
/// dismisses, leaving the calculated value undisturbed. A dialogue closed without a press answers
/// "No" by [`AlertButtons::dismissed`]'s rule, which is the page's own word for the choice
/// (ADR 2697).
pub(crate) fn override_notice(over: &pdf_model::view::FormOverride) -> FormAlert {
    use pdf_model::view::FormOverrideMode;
    let (message, icon, buttons, warns) = match over.mode {
        FormOverrideMode::Error => (
            format!(
                "{} is a calculated field. The value you entered was replaced by the calculated \
                 value.",
                over.field
            ),
            AlertIcon::Error,
            AlertButtons::Ok,
            None,
        ),
        FormOverrideMode::Warning => (
            format!(
                "{} is a calculated field, and the form recommends its calculated value. Keep \
                 the value you entered instead?",
                over.field
            ),
            AlertIcon::Warning,
            AlertButtons::YesNo,
            Some(Warns::Calculated(over.field.clone())),
        ),
    };
    FormAlert {
        question: ScriptQuestion::Alert {
            message,
            icon,
            buttons,
            title: Some("Calculated field".to_owned()),
        },
        warns,
    }
}

/// The message XFA 3.3 page 383 asks a host to present when containers fail their script tests,
/// one for each group page 386's `validationMessaging` makes of the failures (ADR 2696).
///
/// Each container's line is the form designer's message for the test where the form gives one,
/// and where it gives none page 383 leaves the words to the processor, so they are this
/// program's. An `error` makes the container invalid and a `warning` recommends another value
/// (page 385); the alert carries the heavier of the group's levels. The entry stands either way:
/// page 383 recommends that a person not be held in a field until it validates.
///
/// **A group of warnings asks page 385's choice** (ADR 2698): the page has the message inform the
/// person that the value is recommended to conform and give two choices, to dismiss and return to
/// the form to satisfy the constraint, and to override it. They are one question, as page 642's
/// are (ADR 2697): "Yes" keeps the values entered, which is override, and "No", or a dialogue
/// closed without a press, is dismiss. A group holding an `error` keeps one button, since page 385
/// gives an error no choice.
pub(crate) fn validation_notice(group: &[pdf_model::xfa::Invalid]) -> FormAlert {
    use pdf_model::xfa::Level;
    let mut lines: Vec<String> = group
        .iter()
        .map(|invalid| match (&invalid.message, invalid.level) {
            (Some(said), _) => format!("{}: {said}", invalid.container),
            (None, Level::Error) => format!(
                "{}: the form does not accept this value.",
                invalid.container
            ),
            (None, Level::Warning) => format!(
                "{}: the form recommends a value its check accepts.",
                invalid.container
            ),
        })
        .collect();
    let warned = !group.is_empty() && group.iter().all(|invalid| invalid.level == Level::Warning);
    let (icon, buttons, warns) = if warned {
        lines.push("Keep the value you entered anyway?".to_owned());
        (
            AlertIcon::Warning,
            AlertButtons::YesNo,
            Some(Warns::Validated(
                group
                    .iter()
                    .map(|invalid| invalid.container.clone())
                    .collect(),
            )),
        )
    } else {
        (AlertIcon::Error, AlertButtons::Ok, None)
    };
    FormAlert {
        question: ScriptQuestion::Alert {
            message: lines.join("\n"),
            icon,
            buttons,
            title: Some("Form validation".to_owned()),
        },
        warns,
    }
}

#[cfg(test)]
mod tests {
    use super::{FIRST_LINE, Withheld, first_line};
    use pdf_model::aform::Trigger;
    use pdf_model::view::{ScriptEvent, ScriptRunner, ScriptSite};

    /// The first trigger is remembered and asked about once; every later one is withheld quietly.
    #[test]
    fn a_withheld_runner_asks_once_about_the_first_script_it_was_handed() {
        let withheld = Withheld::waiting();
        assert_eq!(
            withheld.question(),
            None,
            "nothing handed over, nothing to ask"
        );
        let event = ScriptEvent {
            script: "\n  // the total\nevent.value = 1;",
            ..ScriptEvent::at(ScriptSite::Field(Trigger::Calculate), "Total")
        };
        let result = withheld.run(&event);
        assert!(result.rc && result.value.is_none() && result.edits.is_empty());
        assert!(result.report[0].contains("waiting on your answer"));
        let later = ScriptEvent {
            script: "other();",
            ..ScriptEvent::at(ScriptSite::Field(Trigger::Format), "Price1")
        };
        withheld.run(&later);
        assert_eq!(
            withheld.question(),
            Some((
                "the calculate script of Total".to_owned(),
                "// the total".to_owned()
            ))
        );
        assert_eq!(withheld.question(), None, "asked once");
    }

    /// A `no` is said on every script, and never asked again.
    #[test]
    fn a_declined_runner_says_so_and_asks_nothing() {
        let declined = Withheld::declined();
        let result = declined.run(&ScriptEvent::at(
            ScriptSite::Field(Trigger::Calculate),
            "Total",
        ));
        assert!(result.report[0].contains("you declined"));
        assert_eq!(declined.question(), None);
    }

    /// An `error` says the entry was replaced and has one button, because it has nothing for a
    /// window to carry out; a `warning` asks page 642's choice of the field it names (ADR 2697).
    #[test]
    fn an_override_notice_names_the_field_and_says_what_became_of_the_entry() {
        use super::override_notice;
        use crate::{AlertButtons, AlertIcon, ScriptQuestion};
        use pdf_model::view::{FormOverride, FormOverrideMode};
        for (mode, icon, words, offered, warns) in [
            (
                FormOverrideMode::Error,
                AlertIcon::Error,
                "was replaced",
                AlertButtons::Ok,
                false,
            ),
            (
                FormOverrideMode::Warning,
                AlertIcon::Warning,
                "Keep the value you entered",
                AlertButtons::YesNo,
                true,
            ),
        ] {
            let notice = override_notice(&FormOverride {
                field: "form1[0].Total[0]".to_owned(),
                mode,
            });
            assert_eq!(notice.warns.is_some(), warns);
            let ScriptQuestion::Alert {
                message,
                icon: shown,
                buttons,
                ..
            } = notice.question
            else {
                panic!("a notice is an alert");
            };
            assert!(message.contains("form1[0].Total[0]") && message.contains(words));
            assert_eq!((shown, buttons), (icon, offered));
        }
    }

    /// Page 383: the designer's message where the form gives one, this program's words where it
    /// does not; page 385: an `error` in the group makes the alert an error with one button, and
    /// a group of warnings alone a warning asking page 385's dismiss-or-override choice as Yes/No,
    /// naming every container a dismissal returns to. One message per group page 386 makes.
    #[test]
    fn a_validation_notice_says_the_designers_message_and_carries_the_heavier_level() {
        use super::validation_notice;
        use crate::{AlertButtons, AlertIcon, ScriptQuestion};
        use pdf_model::xfa::{Invalid, Level};
        let warned = Invalid {
            container: "form1[0].Amount[0]".to_owned(),
            level: Level::Warning,
            message: Some("At most 100.".to_owned()),
        };
        let refused = Invalid {
            container: "form1[0].Total[0]".to_owned(),
            level: Level::Error,
            message: None,
        };
        for (group, icon, offered, warns) in [
            (
                vec![warned.clone()],
                AlertIcon::Warning,
                AlertButtons::YesNo,
                Some(super::Warns::Validated(vec![
                    "form1[0].Amount[0]".to_owned(),
                ])),
            ),
            (
                vec![warned, refused],
                AlertIcon::Error,
                AlertButtons::Ok,
                None,
            ),
        ] {
            let notice = validation_notice(&group);
            assert_eq!(notice.warns, warns);
            let ScriptQuestion::Alert {
                message,
                icon: shown,
                buttons,
                ..
            } = notice.question
            else {
                panic!("a validation message is an alert");
            };
            assert!(
                message.contains("form1[0].Amount[0]: At most 100."),
                "{message}"
            );
            assert_eq!(
                message.contains("form1[0].Total[0]: the form does not accept"),
                group.len() == 2
            );
            assert_eq!((shown, buttons), (icon, offered));
            assert_eq!(message.contains("Keep the value"), group.len() == 1);
        }
    }

    /// A minified library is one line, and a card shows the start of it.
    #[test]
    fn a_long_first_line_is_cut_where_a_card_can_show_it() {
        let long = "x".repeat(FIRST_LINE * 2);
        let cut = first_line(&long);
        assert_eq!(cut.chars().count(), FIRST_LINE + 1);
        assert!(cut.ends_with('…'));
    }
}
