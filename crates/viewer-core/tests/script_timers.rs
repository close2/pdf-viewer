//! A script's timer counted down in the ticks a host sends, run when its period has elapsed, and a
//! script's sound handed to the host — and a window whose documents set no timer is asked to tick
//! for nothing (ADR 1702).
//!
//! The runner here is the test's own, answering the open action with the edits a script's
//! `app.setTimeOut`, `setInterval` and `beep` make and the timer's own run with a page turn, so
//! what is judged is the viewer's half and never what a script engine does with Adobe's members.

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, Sound};
use viewer_core::{Answer, Command, DocumentId, Event, Query, ScriptRunners, Scripting, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// Three pages and an open action whose script the runner below answers.
fn three_pages() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (go\\(\\);) >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A runner whose open action sets one timer and asks for a warning's sound, and whose timer's
/// run turns to the next page.
#[derive(Debug)]
struct Timing {
    repeat: bool,
}

impl ScriptRunner for Timing {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = match event.site {
            ScriptSite::OpenAction => vec![
                ScriptEdit::Timer {
                    id: 0,
                    script: "next()".to_owned(),
                    period: 500,
                    repeat: self.repeat,
                },
                ScriptEdit::Beep {
                    sound: Sound::Warning,
                },
            ],
            ScriptSite::Timer if event.script == "next()" => vec![ScriptEdit::GoTo {
                page: u32::try_from(event.page.saturating_add(1)).unwrap_or(u32::MAX),
            }],
            _ => Vec::new(),
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Timers(bool);

impl ScriptRunners for Timers {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Timing { repeat: self.0 })
    }
}

/// A viewer with the document open and its open sequence run, and the events the first present
/// caused.
fn opened(runners: Option<Timers>) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    if let Some(runners) = runners {
        viewer
            .handle(Command::Scripts(Scripting::Run(Arc::new(runners))))
            .for_each(drop);
    }
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: three_pages().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events = viewer.handle(Command::Presented).collect();
    (viewer, events)
}

/// The milliseconds the viewer says may pass before a timer is owed.
fn due(viewer: &Viewer) -> Option<u32> {
    match viewer.query(Query::TimerDue) {
        Answer::TimerDue(due) => due,
        // Any other answer is not a number of milliseconds, and the tests read it as none owed.
        _ => None,
    }
}

/// The pages a command's events turned to.
fn turns(events: &[Event]) -> Vec<usize> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged {
                document, index, ..
            } if *document == DOCUMENT => Some(*index),
            _ => None,
        })
        .collect()
}

/// A timeout is owed after its period of ticks and not before, runs once, and is gone.
#[test]
fn a_timeout_runs_once_its_period_of_ticks_has_passed() {
    let (mut viewer, _) = opened(Some(Timers(false)));
    assert_eq!(due(&viewer), Some(500));
    let early: Vec<Event> = viewer.handle(Command::Tick { millis: 400 }).collect();
    assert!(turns(&early).is_empty(), "{early:?}");
    assert_eq!(due(&viewer), Some(100), "a hundred milliseconds are left");
    let owed: Vec<Event> = viewer.handle(Command::Tick { millis: 100 }).collect();
    assert_eq!(turns(&owed), vec![1], "{owed:?}");
    assert_eq!(due(&viewer), None, "a timeout runs once");
    let after: Vec<Event> = viewer.handle(Command::Tick { millis: 5000 }).collect();
    assert!(turns(&after).is_empty(), "{after:?}");
}

/// An interval runs every period, and a late tick runs it once rather than once per period missed.
#[test]
fn an_interval_runs_every_period_and_a_late_tick_runs_it_once() {
    let (mut viewer, _) = opened(Some(Timers(true)));
    let first: Vec<Event> = viewer.handle(Command::Tick { millis: 500 }).collect();
    assert_eq!(turns(&first), vec![1], "{first:?}");
    assert_eq!(due(&viewer), Some(500), "the next period has begun");
    let late: Vec<Event> = viewer.handle(Command::Tick { millis: 1700 }).collect();
    assert_eq!(turns(&late), vec![2], "{late:?}");
}

/// `app.beep` reaches the host as an event after the command whose script asked, once.
#[test]
fn a_scripts_sound_is_an_event_after_the_command_that_asked() {
    let (mut viewer, events) = opened(Some(Timers(false)));
    let beeps: Vec<Sound> = events
        .iter()
        .filter_map(|event| match event {
            Event::Beep { document, sound } if *document == DOCUMENT => Some(*sound),
            _ => None,
        })
        .collect();
    assert_eq!(beeps, vec![Sound::Warning], "{events:?}");
    let next: Vec<Event> = viewer.handle(Command::Tick { millis: 0 }).collect();
    assert!(
        !next.iter().any(|event| matches!(event, Event::Beep { .. })),
        "{next:?}"
    );
}

/// A runner whose open action sets one timeout, and whose timeout's run either only asks for a
/// sound or asks for `/CO` to be walked, which can write what a page draws.
#[derive(Debug)]
struct Ticking {
    draws: bool,
}

impl ScriptRunner for Ticking {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = match event.site {
            ScriptSite::OpenAction => vec![ScriptEdit::Timer {
                id: 0,
                script: "tick()".to_owned(),
                period: 500,
                repeat: false,
            }],
            ScriptSite::Timer if self.draws => vec![ScriptEdit::Calculate],
            ScriptSite::Timer => vec![ScriptEdit::Beep {
                sound: Sound::Default,
            }],
            _ => Vec::new(),
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Tickers(bool);

impl ScriptRunners for Tickers {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Ticking { draws: self.0 })
    }
}

/// The ink of every render a command's events asked for.
fn inks(events: &[Event]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.ink),
            _ => None,
        })
        .collect()
}

/// A timer whose script changes nothing a page draws leaves the page as it was interpreted: the
/// tick it runs at asks for no render, where the same tick under a script that asks for `/CO`'s
/// walk asks for one of new ink (ADRs 1762, 1771).
#[test]
fn a_timer_that_changes_nothing_drawn_does_not_interpret_the_page_again() {
    for draws in [false, true] {
        let mut viewer = Viewer::new(800, 1000, 1.0);
        viewer
            .handle(Command::Scripts(Scripting::Run(Arc::new(Tickers(draws)))))
            .for_each(drop);
        viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes: three_pages().into(),
                password: None,
                fragment: None,
            })
            .for_each(drop);
        viewer.handle(Command::Presented).for_each(drop);
        assert_eq!(due(&viewer), Some(500), "the open action set the timeout");
        let ran: Vec<Event> = viewer.handle(Command::Tick { millis: 500 }).collect();
        assert_eq!(due(&viewer), None, "the timeout ran");
        let expected = usize::from(draws);
        assert_eq!(inks(&ran).len(), expected, "draws {draws}: {ran:?}");
    }
}

/// With no runner nothing runs, so nothing is owed: a window asks and is told to arm nothing.
#[test]
fn a_document_with_no_runner_owes_no_tick() {
    let (viewer, _) = opened(None);
    assert_eq!(due(&viewer), None);
    let empty = Viewer::new(800, 1000, 1.0);
    assert_eq!(due(&empty), None, "nor does a window with no document");
}
