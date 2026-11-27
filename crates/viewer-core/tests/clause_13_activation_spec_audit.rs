//! A 3D or rich media annotation a person activates (audit 2278).
//!
//! ISO 32000-2 §13.6.2 gives a 3D annotation two states: "Inactive (the default initial state):
//! the annotation displays the annotation's normal appearance" and "Active: the annotation displays
//! a rendering of the 3D artwork"; §13.7.2.1 gives a rich media annotation the same two. Table 310's
//! `/A` defaults to `XA`, activation by an explicit user action. Rendering the artwork is excluded
//! (`CLAUDE.md`, the clause 13 entry; `doc/scope-ratifications.md`), and an exclusion is refused by
//! name: a press that would activate the artwork says that it is not rendered (trap 5). The control
//! is a movie annotation, whose activation already hands its movie over.

#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure"
)]

use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Assembles a document from object bodies, with object 1 the catalog.
fn assembled(objects: &[&str]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
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

/// One 100-unit page whose one annotation, object 4 over `/Rect [0 0 50 50]`, has the subtype and
/// entries `annotation` states and a normal appearance (object 5); objects 6 and 7 are a movie's
/// embedded file and a 3D stream.
fn document(annotation: &str) -> Vec<u8> {
    let annot =
        format!("<< /Type /Annot {annotation} /Rect [0 0 50 50] /AP << /N 5 0 R >> /F 4 >>");
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        &annot,
        "<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] /Length 0 >>\nstream\n\nendstream",
        "<< /Type /Filespec /F (intro.mov) /UF (intro.mov) /EF << /F 7 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /video#2Fquicktime /Length 4 >>\nstream\nmoov\nendstream",
        "<< /Type /3D /Subtype /U3D /Length 4 >>\nstream\nU3D\0\nendstream",
    ])
}

/// The events of a press and a release in the middle of the annotation.
fn pressed(bytes: Vec<u8>) -> Vec<Event> {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    // The middle of `/Rect [0 0 50 50]` on the 100-unit page, flipped into the viewport.
    let at = (
        geometry.origin.0 + 25.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 25.0) * geometry.scale,
    );
    let mut events = Vec::new();
    for action in [PointerAction::Pressed, PointerAction::Released] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    events
}

/// Every sentence the events say.
fn said(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. }
            | Event::Refused { notes, .. }
            | Event::Warned { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// 2278-B04: a press on a 3D annotation, and on a rich media annotation, says that the artwork the
/// activation would show is not rendered; today it says nothing.
#[test]
fn activating_3d_or_rich_media_artwork_is_refused_by_name_spec_audit() {
    // Control: a movie annotation's press hands the movie over (§12.5.6.17).
    let movie = pressed(document("/Subtype /Movie /Movie << /F 6 0 R >>"));
    assert!(
        movie
            .iter()
            .any(|event| matches!(event, Event::Media { .. })),
        "{movie:?}"
    );
    for (subtype, annotation) in [
        ("3D", "/Subtype /3D /3DD 8 0 R"),
        (
            "RichMedia",
            "/Subtype /RichMedia /RichMediaContent << /Assets << /Names [] >> >>",
        ),
    ] {
        let events = pressed(document(annotation));
        let sentences = said(&events);
        assert!(
            sentences.iter().any(|sentence| sentence.contains("3D")
                || sentence.contains("rich media")
                || sentence.contains("RichMedia")),
            "a press on a {subtype} annotation activates its artwork (Table 310's default /A \
             /XA), which this program does not render, and says nothing: {events:?}"
        );
    }
}
