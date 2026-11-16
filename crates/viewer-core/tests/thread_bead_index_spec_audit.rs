//! Table 209 nonzero bead index through the public Viewer action route.
//! Explicit B0 and B1 choose distinct pages in a two-bead circular thread.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixtures; malformed setup must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};
use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

fn document_bytes() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Threads [8 0 R] >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /Annots [6 0 R 7 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /B [9 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /B [10 0 R] >>",
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [5 5 25 25] /Border [0 0 0] \
          /A << /S /Thread /D 0 /B 0 >> >>",
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [30 5 50 25] /Border [0 0 0] \
          /A << /S /Thread /D 0 /B 1 >> >>",
        "<< /Type /Thread /F 9 0 R /I << /Title (two page article) >> >>",
        "<< /Type /Bead /T 8 0 R /N 10 0 R /V 10 0 R /P 4 0 R /R [10 10 40 60] >>",
        "<< /Type /Bead /T 8 0 R /N 9 0 R /V 9 0 R /P 5 0 R /R [20 20 80 80] >>",
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = bytes.len();
    let size = objects.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

#[expect(
    clippy::panic,
    reason = "the public query must answer for the open test fixture"
)]
fn page(viewer: &Viewer) -> usize {
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("open viewer reports the current page");
    };
    index
}

#[test]
fn a_thread_action_bead_index_selects_the_requested_bead_page() {
    let bytes = document_bytes();
    let syntax = Document::open(bytes.clone()).expect("exact-xref PDF opens");
    assert!(!syntax.was_recovered(), "fixture xref offsets are exact");

    let mut viewer = Viewer::new(300, 200, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );

    // Table 209 explicitly numbers the first bead zero; this is the target control.
    let _ = viewer
        .handle(Command::Activate(ObjectId::new(6, 0)))
        .count();
    assert_eq!(page(&viewer), 1);

    let _ = viewer
        .handle(Command::GoTo(viewer_core::PageTarget::First))
        .count();
    let _ = viewer
        .handle(Command::Activate(ObjectId::new(7, 0)))
        .count();
    assert_eq!(
        page(&viewer),
        2,
        "`/B 1` resolves to the second bead, on page 3"
    );
}
