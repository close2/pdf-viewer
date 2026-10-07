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
}

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
    match scripting {
        Scripting::Off => {
            consent.withheld = None;
            None
        }
        Scripting::Run(runners) => {
            consent.withheld = None;
            Some(runners.runner())
        }
        Scripting::Ask(runners) => match consent.answered {
            Some(true) => {
                consent.withheld = None;
                Some(runners.runner())
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

/// Whether a runner from `scripting` is a real one — one that runs what it is handed — for a
/// document that has `consent`.
pub(crate) fn runs(scripting: &Scripting, consent: &Consent) -> bool {
    match scripting {
        Scripting::Off => false,
        Scripting::Run(_) => true,
        Scripting::Ask(_) => consent.answered == Some(true),
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

    /// A minified library is one line, and a card shows the start of it.
    #[test]
    fn a_long_first_line_is_cut_where_a_card_can_show_it() {
        let long = "x".repeat(FIRST_LINE * 2);
        let cut = first_line(&long);
        assert_eq!(cut.chars().count(), FIRST_LINE + 1);
        assert!(cut.ends_with('…'));
    }
}
