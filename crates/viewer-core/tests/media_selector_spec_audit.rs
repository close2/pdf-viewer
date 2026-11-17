//! Standalone public-reader first-viable selector/handoff regression.
//!
//! ISO 32000-2 §13.2.3.3 says a selector shall play the first viable media rendition
//! encountered. Section 13.2.2 says an unrecognised key in an MH dictionary makes the object
//! non-viable. The first child therefore cannot be selected; the second is a known-type viable
//! clip; the third is a later viable sentinel. Exact xref offsets keep this a valid-document
//! selection test rather than a parser-recovery test.
//!
//! The embedded bytes are a handoff sentinel, not a playable MP4; this asserts
//! rendition selection and the host request, not media decoding or playback.
//! This is independent of the unresolved Table 301 question for MU fully excluded by NU.

#![expect(clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use pdf_model::restriction::Operation;
use viewer_core::{
    Command, DocumentId, Event, Media, RestrictionLevel, RestrictionPolicy, RestrictionScope,
    Viewer,
};

fn selector_with_nonviable_first_and_two_viable_children() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect [0 0 50 50] /A 5 0 R >>",
        "<< /Type /Action /S /Rendition /OP 0 /AN 4 0 R /R 6 0 R >>",
        "<< /Type /Rendition /S /SR /R [7 0 R 10 0 R 13 0 R] >>",
        "<< /Type /Rendition /S /MR /N (rejected) /MH << /Unknown true >> /C 8 0 R >>",
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 9 0 R >>",
        "<< /Type /Filespec /F (rejected.mp4) /UF (rejected.mp4) /EF << /F 16 0 R >> >>",
        "<< /Type /Rendition /S /MR /N (selected) /C 11 0 R >>",
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 12 0 R >>",
        "<< /Type /Filespec /F (selected.mp4) /UF (selected.mp4) /EF << /F 16 0 R >> >>",
        "<< /Type /Rendition /S /MR /N (later) /C 14 0 R >>",
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D 15 0 R >>",
        "<< /Type /Filespec /F (later.mp4) /UF (later.mp4) /EF << /F 16 0 R >> >>",
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nmoov\nendstream",
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

#[test]
fn selector_hands_off_the_first_viable_clip_and_skips_later_viable_clip() {
    const DOCUMENT: DocumentId = DocumentId(1);
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, RestrictionLevel::Off),
        )))
        .for_each(drop);
    let bytes = selector_with_nonviable_first_and_two_viable_children();
    let document = pdf_syntax::Document::open(bytes.clone()).expect("exact-xref fixture opens");
    assert!(!document.was_recovered(), "no parser recovery");
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    let events: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    let handed: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Media {
                content_type,
                media,
                ..
            } => Some((content_type.clone(), media.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        handed,
        vec![(
            Some("video/mp4".to_owned()),
            Media::Embedded {
                name: "selected.mp4".to_owned(),
                bytes: b"moov".to_vec(),
            }
        )],
        "the MH-unknown first child is skipped and later viable children are not selected"
    );
}
