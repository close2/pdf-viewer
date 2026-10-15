//! ISO 32000-2 §12.6.4.14's rendition action, as far as a processor that plays nothing performs it.
//!
//! `CLAUDE.md`'s clause 13 entry excludes the playing and admits two things: §13.2.2's viability
//! reported in the clause's own terms, and the clip handed to this machine's player at the reader's
//! level. Each test below names the wrong answer it excludes — a clip handed over against a level
//! that refuses it, a clip withheld at the default level, a hand-off made from a screen annotation
//! §12.6.4.14 says may not be executed — because a test that only wanted an event would pass on a
//! viewer that handed over everything (ADRs 1960, 1961).

use pdf_model::restriction::Operation;
use viewer_core::{
    Command, DocumentId, Event, Media, RestrictionLevel, RestrictionPolicy, RestrictionScope,
    Viewer,
};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

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

/// A page with one screen annotation whose `/A` plays an embedded `video/mp4` clip.
///
/// `permissions` is Table 285's `/P`, and `listed` whether the annotation is in its page's
/// `/Annots` — the second half of §12.6.4.14's precondition.
fn document(permissions: &str, listed: bool) -> Vec<u8> {
    let annots = if listed { "/Annots [4 0 R]" } else { "" };
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {annots} >>"),
        "<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect [0 0 50 50] /A 5 0 R >>".to_owned(),
        "<< /Type /Action /S /Rendition /OP 0 /AN 4 0 R /R 6 0 R >>".to_owned(),
        "<< /Type /Rendition /S /MR /N (clip) /C 7 0 R >>".to_owned(),
        format!(
            "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 8 0 R {permissions} /Alt [() (A \
             short clip)] >>"
        ),
        "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 9 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nmoov\nendstream".to_owned(),
    ])
}

/// Opens the document under a level for the hand-off and activates the screen annotation.
fn activated(bytes: Vec<u8>, level: RestrictionLevel) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    (viewer, events)
}

/// The clips an event list hands over.
fn handed(events: &[Event]) -> Vec<(Option<String>, Media)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Media {
                content_type,
                media,
                ..
            } => Some((content_type.clone(), media.clone())),
            _ => None,
        })
        .collect()
}

/// Every sentence the events report.
fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// At the default level the document's `/TF` is not consulted, and the decoded clip is handed over
/// after the report that says it is viable and what its `/Alt` says.
#[test]
fn a_viable_clip_is_reported_and_handed_over_at_the_default_level() {
    let (_, events) = activated(document("", true), RestrictionLevel::Off);
    assert_eq!(
        handed(&events),
        [(
            Some("video/mp4".to_owned()),
            Media::Embedded {
                name: "clip.mp4".to_owned(),
                bytes: b"moov".to_vec(),
            }
        )]
    );
    let said = notes(&events);
    assert!(
        said.iter().any(|note| note.contains("is viable (§13.2.2)")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|note| note.contains("\"A short clip\"")),
        "§14.9.2.4's default text is the report's first reader: {said:?}"
    );
}

/// Table 286's default is `TEMPNEVER`, so a reader obeying the document is told why and handed
/// nothing.
#[test]
fn a_reader_obeying_the_document_is_refused_a_clip_that_never_permits_a_temporary_file() {
    let (_, events) = activated(document("", true), RestrictionLevel::On);
    assert!(handed(&events).is_empty(), "{events:?}");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Refused { operation: Operation::HandOverMedia, notes, .. }
                if notes.iter().any(|note| note.contains("TEMPNEVER"))
        )),
        "{events:?}"
    );
}

/// `TEMPALWAYS` withholds nothing, so the same level hands the clip over.
#[test]
fn a_clip_that_always_permits_a_temporary_file_is_handed_over_to_an_obeying_reader() {
    let (_, events) = activated(
        document("/P << /TF (TEMPALWAYS) >>", true),
        RestrictionLevel::On,
    );
    assert_eq!(handed(&events).len(), 1, "{events:?}");
}

/// At *ask* the clip waits on the answer, and a `yes` releases exactly what was asked about.
#[test]
fn an_asked_reader_is_handed_the_clip_on_a_yes() {
    let (mut viewer, events) = activated(document("", true), RestrictionLevel::Ask);
    assert!(handed(&events).is_empty(), "{events:?}");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Asking {
                operation: Operation::HandOverMedia,
                ..
            }
        )),
        "{events:?}"
    );
    let answered: Vec<Event> = viewer
        .handle(Command::Answer {
            document: DOCUMENT,
            proceed: true,
        })
        .collect();
    assert_eq!(handed(&answered).len(), 1, "{answered:?}");
}

/// §12.6.4.14: "the interactive PDF processor shall make sure that the P entry of the screen
/// annotation dictionary references a valid page object and that the annotation is present in the
/// page object's Annots array" — here it is not, so nothing is judged or handed over.
#[test]
fn a_screen_annotation_its_page_does_not_list_executes_nothing() {
    let (_, events) = activated(document("", false), RestrictionLevel::Off);
    assert!(handed(&events).is_empty(), "{events:?}");
    let said = notes(&events);
    assert!(
        said.iter()
            .any(|note| note.contains("not in its page's /Annots")),
        "{said:?}"
    );
}
