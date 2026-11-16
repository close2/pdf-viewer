//! Public viewer regression for ISO 32000-2 §12.4.4.1's omitted /Dur rule.
//!
//! The exact-xref PDF has two
//! stated-duration pages surrounding a non-final page with no /Dur, so document-end behavior
//! cannot make the negative assertion pass vacuously.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small fixed PDF fixture object numbers and page indexes"
)]
#![expect(
    clippy::panic,
    reason = "a fixture that does not reach its assertions must fail rather than pass vacuously"
)]

use std::fmt::Write as _;

use viewer_core::{Command, DocumentId, Event, PageTarget, PresentationMode, Query, Viewer};

const DOC: DocumentId = DocumentId(1);

fn fixture() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R] /Count 4 >>".to_owned(),
        // Positive control: a stated duration advances to the omitted-duration page.
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 7 0 R /Dur 1 >>".to_owned(),
        // The page under test is not last and has no /Dur entry.
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 7 0 R >>".to_owned(),
        // Positive control: manual navigation reaches another page with a stated /Dur.
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 7 0 R /Dur 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> /Contents 7 0 R >>".to_owned(),
        "<< /Length 0 >>\nstream\nendstream".to_owned(),
    ];

    let mut pdf = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = pdf.len();
    let _ = writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    pdf.into_bytes()
}

fn current_page(viewer: &Viewer) -> usize {
    let viewer_core::Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("the open document has a current page");
    };
    index
}

#[test]
fn an_omitted_page_duration_does_not_auto_advance_inside_a_presentation() {
    let bytes = fixture();
    let document = pdf_syntax::Document::open(bytes.clone()).expect("exact-xref fixture opens");
    assert!(!document.was_recovered());
    let mut viewer = Viewer::new(20, 20, 1.0);
    let open_events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        open_events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !open_events
            .iter()
            .any(|event| matches!(event, Event::OpenFailed { .. }))
    );

    let _ = viewer
        .handle(Command::Present(PresentationMode::On))
        .count();
    let first_advance: Vec<_> = viewer.handle(Command::Tick { millis: 1_000 }).collect();
    assert_eq!(current_page(&viewer), 1, "the stated /Dur control advances");
    assert!(
        first_advance
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 1, .. }))
    );

    let omitted_page_ticks: Vec<_> = viewer.handle(Command::Tick { millis: 60_000 }).collect();
    assert_eq!(
        current_page(&viewer),
        1,
        "page 2 has no /Dur and is not the last page"
    );
    assert!(
        !omitted_page_ticks
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "§12.4.4.1 says no /Dur means no automatic advance: {omitted_page_ticks:?}"
    );

    let _ = viewer.handle(Command::GoTo(PageTarget::Next)).count();
    assert_eq!(current_page(&viewer), 2, "manual advance still works");
    let next_stated_duration: Vec<_> = viewer.handle(Command::Tick { millis: 1_000 }).collect();
    assert_eq!(
        current_page(&viewer),
        3,
        "the following stated /Dur advances"
    );
    assert!(
        next_stated_duration
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 3, .. }))
    );
}
