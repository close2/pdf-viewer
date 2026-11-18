//! Public Link activation probes for ISO 32000-2 §12.6.4.14.
//! A controlled runner isolates dispatch and field-edit delivery from ECMAScript evaluation.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixture guards must fail visibly"
)]

use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::restriction::Operation;
use pdf_model::view::{ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner};
use pdf_syntax::Document as SyntaxDocument;
use viewer_core::{
    Answer, Command, DocumentId, Event, PointerAction, Query, RestrictionLevel, RestrictionPolicy,
    RestrictionScope, ScriptRunners, Scripting, Viewer,
};

type Seen = Arc<Mutex<Vec<String>>>;
const DOC: DocumentId = DocumentId(88);
const SCRIPT: &str = "this.getField('Status').value = 'changed-by-rendition-script';";

#[derive(Debug)]
struct Capture(Seen);
impl ScriptRunner for Capture {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.script.to_owned());
        let edits = if event.script == SCRIPT {
            vec![ScriptEdit::Value {
                field: "Status".to_owned(),
                value: "changed-by-rendition-script".to_owned(),
            }]
        } else {
            Vec::new()
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
struct Captures(Seen);
impl ScriptRunners for Captures {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Capture(Arc::clone(&self.0)))
    }
}

/// Exact classic-xref PDF. `p` selects Screen 4's /P; `extra` supplies the wrong-Type control.
/// A valid Link on page 3 invokes the action and names Screen 4 through `/AN`.
fn pdf(p: &str, extra: Option<&str>, screen_listed_on_page3: bool) -> Vec<u8> {
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [{}5 0 R] >>",
            if screen_listed_on_page3 { "4 0 R " } else { "" }
        ),
        format!("<< /Type /Annot /Subtype /Screen /P {p} /Rect [0 0 50 50] >>"),
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [50 0 100 50] /Border [0 0 0] /A 6 0 R >>"
            .to_owned(),
        "<< /Type /Action /S /Rendition /OP 0 /AN 4 0 R /R 7 0 R >>".to_owned(),
        "<< /Type /Rendition /S /MR /N (clip) /C 8 0 R >>".to_owned(),
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 9 0 R /Alt [() (short clip)] >>".to_owned(),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 10 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nmoov\nendstream".to_owned(),
    ];
    let mut objects = objects;
    if let Some(extra) = extra {
        objects.push(extra.to_owned());
    }
    assemble(&objects)
}

/// The script uses the admitted document field API; a valid widget gives its effect a public query.
fn pdf_with_field(rendition: bool) -> Vec<u8> {
    assemble(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm 11 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R 5 0 R 12 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect [0 0 50 50] >>".to_owned(),
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [50 0 100 50] /Border [0 0 0] /A 6 0 R >>".to_owned(),
        if rendition {
            format!("<< /Type /Action /S /Rendition /OP 0 /JS ({SCRIPT}) /AN 4 0 R /R 7 0 R >>")
        } else {
            format!("<< /Type /Action /S /JavaScript /JS ({SCRIPT}) >>")
        },
        "<< /Type /Rendition /S /MR /N (clip) /C 8 0 R >>".to_owned(),
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 9 0 R /Alt [() (short clip)] >>".to_owned(),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 10 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nmoov\nendstream".to_owned(),
        "<< /Fields [13 0 R] /DR << /Font << /Helv 14 0 R >> >> /DA (/Helv 10 Tf 0 g) >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 13 0 R /P 3 0 R /Rect [60 0 90 20] /F 4 /AP << /N 15 0 R >> >>".to_owned(),
        "<< /FT /Tx /T (Status) /V (before) /DA (/Helv 10 Tf 0 g) /Kids [12 0 R] >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 30 20] /Resources << /Font << /Helv 14 0 R >> >> /Length 0 >>\nstream\n\nendstream".to_owned(),
    ])
}

fn assemble(objects: &[String]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(
            format!("{} 0 obj\n{body}\nendobj\n", i.saturating_add(1)).as_bytes(),
        );
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

fn open(bytes: Vec<u8>, runner: Option<Seen>) -> (Viewer, Vec<Event>) {
    let syntax = SyntaxDocument::open(bytes.clone()).expect("exact-xref synthetic PDF opens");
    assert!(!syntax.was_recovered(), "generated xref offsets are exact");
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, RestrictionLevel::Off),
        )))
        .for_each(drop);
    if let Some(seen) = runner {
        viewer
            .handle(Command::Scripts(Scripting::Run(Arc::new(Captures(seen)))))
            .for_each(drop);
    }
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. })),
        "{opened:?}"
    );
    (viewer, opened)
}

/// Click the valid page-3 Link at default-user-space point (75,25).
fn click_link(viewer: &mut Viewer) -> Vec<Event> {
    viewer.handle(Command::Presented).for_each(drop);
    let Answer::Geometry(g) = viewer.query(Query::PageGeometry(0)) else {
        panic!("page geometry available");
    };
    let at = (g.origin.0 + 75.0 * g.scale, g.origin.1 + 75.0 * g.scale);
    assert!(
        matches!(viewer.query(Query::LinkAt(at)), Answer::Link(true)),
        "the public hit test reaches the Link"
    );
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Released,
        })
        .collect()
}
fn media(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, Event::Media { .. }))
        .count()
}
fn field_value(viewer: &Viewer, name: &str) -> String {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("form fields answer");
    };
    fields
        .into_iter()
        .find(|field| field.name.qualified == name)
        .and_then(|field| field.value)
        .map_or_default(|shown| shown.text)
}

#[test]
fn a_valid_page_and_its_annots_entry_are_positive_controls() {
    let (mut viewer, _) = open(pdf("3 0 R", None, true), None);
    let events = click_link(&mut viewer);
    assert_eq!(
        media(&events),
        1,
        "valid attached Page + membership reaches viability and handoff: {events:?}"
    );
}

#[test]
fn a_non_page_p_reference_is_not_executed_even_when_it_has_an_annots_key() {
    // The Filespec at object 11 has an /Annots key. This isolates the /Type guard from membership.
    let (mut viewer, _) = open(
        pdf(
            "11 0 R",
            Some("<< /Type /Filespec /Annots [4 0 R] >>"),
            true,
        ),
        None,
    );
    let events = click_link(&mut viewer);
    assert_eq!(
        media(&events),
        0,
        "wrong /P type must stop execution: {events:?}"
    );
}

#[test]
fn an_enabled_runner_executes_the_rendition_script_and_does_not_use_op_as_fallback() {
    let seen = Seen::default();
    let (mut viewer, _) = open(pdf_with_field(true), Some(Arc::clone(&seen)));
    assert_eq!(field_value(&viewer, "Status"), "before");
    let events = click_link(&mut viewer);
    let scripts = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(
        (scripts, field_value(&viewer, "Status"), media(&events)),
        (
            vec![SCRIPT.to_owned()],
            "changed-by-rendition-script".to_owned(),
            0
        ),
        "enabled in-scope script must run before any OP fallback: {events:?}"
    );
}

#[test]
fn without_a_script_runner_the_op_fallback_remains_observable() {
    let (mut viewer, _) = open(pdf_with_field(true), None);
    assert_eq!(field_value(&viewer, "Status"), "before");
    let events = click_link(&mut viewer);
    assert_eq!(
        media(&events),
        1,
        "no runner means the /OP fallback is the in-scope media handoff: {events:?}"
    );
    assert_eq!(field_value(&viewer, "Status"), "before");
}

#[test]
fn an_ordinary_javascript_action_proves_the_supplied_runner_and_field_edit_route_work() {
    let seen = Seen::default();
    let (mut viewer, _) = open(pdf_with_field(false), Some(Arc::clone(&seen)));
    assert_eq!(field_value(&viewer, "Status"), "before");
    let events = click_link(&mut viewer);
    let scripts = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(scripts, [SCRIPT]);
    assert_eq!(
        field_value(&viewer, "Status"),
        "changed-by-rendition-script"
    );
    assert_eq!(media(&events), 0);
}
