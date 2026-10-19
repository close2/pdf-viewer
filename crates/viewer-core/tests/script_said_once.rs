//! A script the view state does not run is said once, with its own reason, at Table 197's events
//! and at an outline item's activation (ADRs 1950, 1975).
//!
//! The view state says why it did not hand a chain's ECMAScript action over — here, a text past
//! the length a script may be in this program — and a host drops the action path's refusal of
//! the same chain wherever the view state answered for it, so Tier 0's sentence, which is not why
//! the script did not run, is not said beside it. With no runner the view state says nothing and
//! the refusal is the one sentence, as it always was.

#![expect(
    clippy::panic,
    reason = "a test's failure is its purpose, and the geometry helper runs outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner};
use viewer_core::{
    Answer, Command, DocumentId, Event, PointerAction, Query, ScriptRunners, Scripting, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// The page's height, which the y flip is measured about.
const PAGE_HEIGHT: f32 = 400.0;

/// The widget's `/Rect`, in the page's default user space.
const WIDGET: [f32; 4] = [100.0, 100.0, 300.0, 150.0];

/// The link's `/Rect`.
const LINK: [f32; 4] = [100.0, 300.0, 300.0, 350.0];

/// The outline's one item.
const OUTLINE_ITEM: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(7, 0);

/// One byte past the longest script the view state hands a runner, `MAX_SCRIPT_BYTES`.
const PAST: usize = (1 << 20) + 1;

/// The sentence the view state says of a `/JS` of [`PAST`] bytes, after its subject.
fn unread(subject: &str) -> String {
    format!(
        "{subject} holds an ECMAScript action of {PAST} bytes, past the {} a script may be in \
         this program, and it was not run",
        1 << 20
    )
}

/// One page whose widget's `/U`, whose link's `/A` and whose outline item's `/A` each name the
/// same script stream, object 8, of [`PAST`] bytes.
fn document() -> Vec<u8> {
    let statement = "ran();";
    let script = format!(
        "<< /Length {PAST} >>\nstream\n{statement}{}\nendstream",
        " ".repeat(PAST.saturating_sub(statement.len()))
    );
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> /Outlines 6 0 R >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Name) /F 4 /P 3 0 R \
         /Rect [100 100 300 150] /AA << /U << /S /JavaScript /JS 8 0 R >> >> >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [100 300 300 350] /P 3 0 R \
         /A << /S /JavaScript /JS 8 0 R >> >>"
            .to_owned(),
        "<< /Type /Outlines /First 7 0 R /Last 7 0 R /Count 1 >>".to_owned(),
        "<< /Title (Totals) /Parent 6 0 R /A << /S /JavaScript /JS 8 0 R >> >>".to_owned(),
        script,
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

/// A runner that runs nothing; no script of the fixture is short enough to reach it.
#[derive(Debug)]
struct Idle;

impl ScriptRunner for Idle {
    fn run(&self, _event: &ScriptEvent<'_>) -> ScriptResult {
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
struct Idlers;

impl ScriptRunners for Idlers {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Idle)
    }
}

/// A viewer with the document open and page one presented, under a runner or none.
fn opened(runner: bool) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    if runner {
        viewer
            .handle(Command::Scripts(Scripting::Run(Arc::new(Idlers))))
            .for_each(drop);
    }
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: document().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    viewer
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

/// The sentences a press and release on `rect` say.
fn clicked(viewer: &mut Viewer, rect: [f32; 4]) -> Vec<String> {
    let at = on_rect(viewer, rect);
    let mut events: Vec<Event> = Vec::new();
    for action in [
        PointerAction::Moved,
        PointerAction::Pressed,
        PointerAction::Released,
    ] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    said(&events)
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

/// How many of `notes` give Tier 0's reason for a script not run.
fn tier_zero(notes: &[String]) -> usize {
    notes
        .iter()
        .filter(|note| note.contains("a script this tier does not run"))
        .count()
}

/// A release on the widget says the `/U` script's length and the bound, and not Tier 0's reason.
#[test]
fn a_widgets_script_past_the_bound_is_said_once() {
    let mut viewer = opened(true);
    let notes = clicked(&mut viewer, WIDGET);
    assert!(
        notes.contains(&unread("the /U event of the annotation of object 4")),
        "{notes:#?}"
    );
    assert_eq!(tier_zero(&notes), 0, "{notes:#?}");
}

/// A click on the link says its `/A` script's length and the bound, and not Tier 0's reason.
#[test]
fn a_links_script_past_the_bound_is_said_once() {
    let mut viewer = opened(true);
    let notes = clicked(&mut viewer, LINK);
    assert!(
        notes.contains(&unread("the /U event of the annotation of object 5")),
        "{notes:#?}"
    );
    assert_eq!(tier_zero(&notes), 0, "{notes:#?}");
}

/// Activating the outline item says its script's length and the bound, and not Tier 0's reason.
#[test]
fn an_outline_items_script_past_the_bound_is_said_once() {
    let mut viewer = opened(true);
    let events: Vec<Event> = viewer.handle(Command::Activate(OUTLINE_ITEM)).collect();
    let notes = said(&events);
    assert!(
        notes.contains(&unread("the outline item \"Totals\"")),
        "{notes:#?}"
    );
    assert_eq!(tier_zero(&notes), 0, "{notes:#?}");
}

/// The control: with no runner the view state says nothing of the script, and the action path's
/// refusal is the one sentence at each site.
#[test]
fn with_no_runner_the_refusal_is_the_one_sentence() {
    let mut viewer = opened(false);
    let mut notes = clicked(&mut viewer, WIDGET);
    notes.extend(clicked(&mut viewer, LINK));
    let events: Vec<Event> = viewer.handle(Command::Activate(OUTLINE_ITEM)).collect();
    notes.extend(said(&events));
    assert_eq!(tier_zero(&notes), 3, "{notes:#?}");
    assert!(
        notes.iter().all(|note| !note.contains("bytes, past the")),
        "{notes:#?}"
    );
}
