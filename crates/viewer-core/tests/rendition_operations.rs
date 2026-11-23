//! ISO 32000-2 §12.6.4.14's stop, pause and resume, and Table 218's `/OP` 4, as the core hands them
//! to the window that holds the association (ADR 3433).
//!
//! Table 218 makes `/OP` 1 "[s]top any rendition being played in association with the annotation
//! specified by AN", 2 pause it and 3 resume it, and 4 "[p]lay the rendition specified by R,
//! associating it with the annotation specified by AN. If a rendition is already associated with
//! the annotation, resume the rendition if it is paused; otherwise, do nothing." The association
//! lives where the rendition plays, so the core's part is to carry `/AN` with every sound it hands
//! over and to send each operation, with its annotation, to the window — after §12.6.4.14's
//! precondition, which binds "a rendition action" whatever its operation. The wrong answers each
//! test excludes: an operation said "not performed" and dropped, an operation sent for an
//! annotation its page does not list, `/OP` 4 sent as a plain play, and `/OP` performed beside a
//! runner that runs the action's `/JS` instead.

use std::sync::Arc;

use pdf_model::restriction::Operation;
use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner};
use pdf_syntax::ObjectId;
use viewer_core::{
    Associated, Command, DocumentId, Event, MediaOperation, RestrictionLevel, RestrictionPolicy,
    RestrictionScope, ScriptRunners, Scripting, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);
const SCREEN: ObjectId = ObjectId {
    number: 4,
    generation: 0,
};

/// Assembles a document from object bodies, with object 1 the catalog.
fn assembled(objects: &[String]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{body}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// A page with screen annotation 4, whose `/A` is a rendition action stating `entries` and naming
/// annotation 4 as `/AN`, with a WAVE clip as `/R`. `listed` is whether the page's `/Annots` lists
/// the annotation — §12.6.4.14's precondition.
fn document(entries: &str, listed: bool) -> Vec<u8> {
    let annots = if listed { "/Annots [4 0 R]" } else { "" };
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {annots} >>"),
        "<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect [0 0 50 50] /A 5 0 R >>".to_owned(),
        format!("<< /Type /Action /S /Rendition {entries} /AN 4 0 R /R 6 0 R >>"),
        "<< /Type /Rendition /S /MR /N (chime) /C 7 0 R >>".to_owned(),
        "<< /Type /MediaClip /S /MCD /CT (audio/wav) /D 8 0 R /P << /TF (TEMPNEVER) >> >>"
            .to_owned(),
        "<< /Type /Filespec /F (chime.wav) /UF (chime.wav) /EF << /F 9 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nRIFF\nendstream".to_owned(),
    ])
}

/// A runner that runs nothing and answers every script as having run.
#[derive(Debug)]
struct Quiet;
impl ScriptRunner for Quiet {
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
struct Quiets;
impl ScriptRunners for Quiets {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Quiet)
    }
}

/// The events of activating annotation 4 in a window that plays sounds itself, with or without a
/// script runner.
fn activated(bytes: Vec<u8>, runner: bool) -> Vec<Event> {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, RestrictionLevel::On),
        )))
        .for_each(drop);
    viewer.handle(Command::PlaysHere(true)).for_each(drop);
    if runner {
        viewer
            .handle(Command::Scripts(Scripting::Run(Arc::new(Quiets))))
            .for_each(drop);
    }
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Activate(SCREEN)).collect()
}

/// The operations an event list sends.
fn operated(events: &[Event]) -> Vec<(DocumentId, ObjectId, MediaOperation)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::OperateMedia {
                document,
                annotation,
                operation,
            } => Some((*document, *annotation, *operation)),
            _ => None,
        })
        .collect()
}

/// The associations of the sounds an event list hands over.
fn associations(events: &[Event]) -> Vec<Option<Associated>> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Media {
                sound: Some(sound), ..
            } => Some(sound.rendition),
            _ => None,
        })
        .collect()
}

/// Every note the events report.
fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Table 218's `/OP` 1, 2 and 3 each reach the window as the operation on annotation 4, hand over
/// no clip, and say nothing of an operation not performed.
#[test]
fn stop_pause_and_resume_are_sent_to_the_window_for_the_annotation_an_names() {
    for (op, operation) in [
        (1, MediaOperation::Stop),
        (2, MediaOperation::Pause),
        (3, MediaOperation::Resume),
    ] {
        let events = activated(document(&format!("/OP {op}"), true), false);
        assert_eq!(
            operated(&events),
            [(DOCUMENT, SCREEN, operation)],
            "/OP {op}: {events:?}"
        );
        assert!(associations(&events).is_empty(), "/OP {op} plays nothing");
        assert!(
            !notes(&events)
                .iter()
                .any(|note| note.contains("does not perform")),
            "/OP {op}: {:?}",
            notes(&events)
        );
    }
}

/// "Before a rendition action is executed": a stop naming an annotation its page does not list is
/// not sent, and the precondition's sentence is said.
#[test]
fn an_operation_on_an_unlisted_annotation_is_not_sent() {
    let events = activated(document("/OP 1", false), false);
    assert!(operated(&events).is_empty(), "{events:?}");
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("Annots") || note.contains("§12.6.4.14")),
        "{:?}",
        notes(&events)
    );
}

/// `/OP` 0 and `/OP` 4 both play the clip associated with annotation 4; only 4 asks the window
/// to resume the one already associated instead, which the window alone can tell.
#[test]
fn a_played_sound_carries_its_annotation_and_whether_it_resumes_instead() {
    for (op, or_resume) in [(0, false), (4, true)] {
        let events = activated(document(&format!("/OP {op}"), true), false);
        assert_eq!(
            associations(&events),
            [Some(Associated {
                annotation: SCREEN,
                or_resume
            })],
            "/OP {op}: {events:?}"
        );
    }
}

/// Table 218: "If both are present, OP is considered a fallback that shall be executed if the
/// interactive PDF processor is unable to execute `ECMAScripts`." With a runner the `/JS` is run and
/// the stop is not sent; with none (the level `off`) the stop is the fallback and is sent (ADR
/// 2988).
#[test]
fn a_stop_beside_a_script_is_the_fallback_only_where_no_runner_is_supplied() {
    let bytes = || document("/OP 1 /JS (this.dirty = false;)", true);
    assert_eq!(operated(&activated(bytes(), true)), []);
    assert_eq!(
        operated(&activated(bytes(), false)),
        [(DOCUMENT, SCREEN, MediaOperation::Stop)]
    );
}
