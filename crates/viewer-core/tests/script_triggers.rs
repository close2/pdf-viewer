//! Table 197's six pointer and focus events of a widget and Table 198's page open and close, each
//! handed to the document's runner at the event that raises it, and the refusal of the same
//! ECMAScript actions said only where no runner was supplied (ADR 1752).
//!
//! The runner here is the test's own and records the site of every script it is handed, so what is
//! judged is the viewer's half — which event reaches the runner, in which order — and never what a
//! script engine does with Adobe's members.

#![expect(
    clippy::panic,
    reason = "a test's failure is its purpose, and the geometry helper runs outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::action::{PageTrigger, Trigger};
use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, PointerAction, Query, ScriptRunners, Scripting,
    Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// The widget's `/Rect`, in the page's default user space.
const RECT: [f32; 4] = [100.0, 100.0, 300.0, 150.0];

/// The page's height, which the y flip is measured about.
const PAGE_HEIGHT: f32 = 400.0;

/// A script action whose text names its event.
fn script(name: &str) -> String {
    format!("<< /S /JavaScript /JS ({name}) >>")
}

/// Two pages, the first carrying a text field whose `/AA` states all six of Table 197's pointer and
/// focus events and a link whose `/A` is a script, and both pages stating Table 198's `/O` and
/// `/C`.
fn two_pages() -> Vec<u8> {
    let widget = format!(
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Name) /F 4 /P 3 0 R \
         /Rect [100 100 300 150] /AA << /E {} /X {} /D {} /U {} /Fo {} /Bl {} >> >>",
        script("E"),
        script("X"),
        script("D"),
        script("U"),
        script("Fo"),
        script("Bl"),
    );
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [5 0 R 6 0 R] \
             /AA << /O {} /C {} >> >>",
            script("O1"),
            script("C1")
        ),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /AA << /O {} /C {} >> >>",
            script("O2"),
            script("C2")
        ),
        widget,
        format!(
            "<< /Type /Annot /Subtype /Link /Rect [100 300 300 350] /A {} >>",
            script("L")
        ),
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

/// Every script a runner was handed, as its site and its text.
type Handed = Arc<Mutex<Vec<(ScriptSite, String)>>>;

/// A runner that records what it is handed and changes nothing.
#[derive(Debug)]
struct Recording(Handed);

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((event.site, event.script.to_owned()));
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Recorders(Handed);

impl ScriptRunners for Recorders {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Recording(Arc::clone(&self.0)))
    }
}

/// A viewer with the document open and page one presented, under a recording runner or none.
fn opened(handed: Option<&Handed>) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    if let Some(handed) = handed {
        viewer
            .handle(Command::Scripts(Scripting::Run(Arc::new(Recorders(
                Arc::clone(handed),
            )))))
            .for_each(drop);
    }
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: two_pages().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    viewer
}

/// The middle of the widget, in the window's pixels.
fn on_widget(viewer: &Viewer) -> (f32, f32) {
    on_rect(viewer, RECT)
}

/// The middle of `rect` on page one, in the window's pixels.
fn on_rect(viewer: &Viewer, rect: [f32; 4]) -> (f32, f32) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    let (x, y) = (
        f32::midpoint(rect[0], rect[2]),
        f32::midpoint(rect[1], rect[3]),
    );
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (PAGE_HEIGHT - y) * geometry.scale,
    )
}

/// A point on page one outside the widget and the link, between the two.
fn beside_widget(viewer: &Viewer) -> (f32, f32) {
    on_rect(viewer, [100.0, 270.0, 300.0, 280.0])
}

/// The events of one pointer message.
fn point(viewer: &mut Viewer, at: (f32, f32), action: PointerAction) -> Vec<Event> {
    viewer.handle(Command::Pointer { at, action }).collect()
}

/// Every sentence the events say.
fn said(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// A cursor entering, a press, a release, the cursor leaving and a press beside the widget hand
/// the runner `/E`, `/D`, `/Fo`, `/U`, `/X` and `/Bl` in §12.6.3's order, and say no refusal.
#[test]
fn a_widgets_pointer_and_focus_events_reach_the_runner_in_their_order() {
    let handed = Handed::default();
    let mut viewer = opened(Some(&handed));
    handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    let (on, beside) = (on_widget(&viewer), beside_widget(&viewer));
    let mut events = Vec::new();
    events.extend(point(&mut viewer, beside, PointerAction::Moved));
    events.extend(point(&mut viewer, on, PointerAction::Moved));
    events.extend(point(&mut viewer, on, PointerAction::Pressed));
    events.extend(point(&mut viewer, on, PointerAction::Released));
    events.extend(point(&mut viewer, beside, PointerAction::Moved));
    events.extend(point(&mut viewer, beside, PointerAction::Pressed));
    events.extend(point(&mut viewer, beside, PointerAction::Released));
    let sites: Vec<(ScriptSite, String)> = handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let annotation = |trigger, text: &str| (ScriptSite::Annotation(trigger), text.to_owned());
    assert_eq!(
        sites,
        vec![
            annotation(Trigger::Enter, "E"),
            annotation(Trigger::Down, "D"),
            annotation(Trigger::Focus, "Fo"),
            annotation(Trigger::Up, "U"),
            annotation(Trigger::Exit, "X"),
            annotation(Trigger::Blur, "Bl"),
        ],
        "{events:?}"
    );
    let refused: Vec<String> = said(&events)
        .into_iter()
        .filter(|note| note.contains("JavaScript"))
        .collect();
    assert!(refused.is_empty(), "{refused:?}");
}

/// The control: with no runner, nothing is handed over and the release says the script was not
/// run, as it did before the runner had a site here.
#[test]
fn with_no_runner_the_scripts_refusal_is_said() {
    let mut viewer = opened(None);
    let on = on_widget(&viewer);
    let mut events = Vec::new();
    events.extend(point(&mut viewer, on, PointerAction::Moved));
    events.extend(point(&mut viewer, on, PointerAction::Pressed));
    events.extend(point(&mut viewer, on, PointerAction::Released));
    let refused = said(&events)
        .into_iter()
        .filter(|note| note.contains("declines — JavaScript"))
        .count();
    assert!(refused >= 4, "{events:?}");
}

/// A page turn hands the runner the page left's `/C` and then the page reached's `/O`; page one's
/// `/O` was the open sequence's, at the first present, and only once.
#[test]
fn a_page_turn_hands_the_runner_the_close_and_then_the_open() {
    let handed = Handed::default();
    let mut viewer = opened(Some(&handed));
    let at_open: Vec<(ScriptSite, String)> = handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .drain(..)
        .collect();
    assert_eq!(
        at_open,
        vec![(ScriptSite::Page(PageTrigger::Open), "O1".to_owned())],
        "the open sequence ran page one's /O once"
    );
    let events: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    let turned: Vec<(ScriptSite, String)> = handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(
        turned,
        vec![
            (ScriptSite::Page(PageTrigger::Close), "C1".to_owned()),
            (ScriptSite::Page(PageTrigger::Open), "O2".to_owned()),
        ],
        "{events:?}"
    );
    assert!(
        !said(&events).iter().any(|note| note.contains("JavaScript")),
        "{events:?}"
    );
}

/// A click on a link whose `/A` is a script hands the runner the link's `/U`, which Table 197's
/// precedence makes the `/A`, and says no refusal of it.
#[test]
fn a_links_click_hands_its_script_to_the_runner() {
    let handed = Handed::default();
    let mut viewer = opened(Some(&handed));
    handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    let on = on_rect(&viewer, [100.0, 300.0, 300.0, 350.0]);
    let mut events = Vec::new();
    events.extend(point(&mut viewer, on, PointerAction::Moved));
    events.extend(point(&mut viewer, on, PointerAction::Pressed));
    events.extend(point(&mut viewer, on, PointerAction::Released));
    let sites: Vec<(ScriptSite, String)> = handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(
        sites,
        vec![(ScriptSite::Annotation(Trigger::Up), "L".to_owned())],
        "{events:?}"
    );
    assert!(
        !said(&events).iter().any(|note| note.contains("JavaScript")),
        "{events:?}"
    );
}
