//! Public `GoToDp` leaf navigation and the current interior-node interpretation.
//! The interior policy uses the first descendant leaf in page-tree order; the action
//! clause names a Start page but does not explicitly define a non-leaf target.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixtures; malformed setup must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, Query, Viewer};

fn document_bytes() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /DPartRoot 8 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /DPart 11 0 R /Annots [6 0 R 7 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /DPart 12 0 R >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /DPart 13 0 R >>",
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [5 5 25 25] /Border [0 0 0] \
          /A << /S /GoToDp /Dp 13 0 R >> >>",
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [30 5 50 25] /Border [0 0 0] \
          /A << /S /GoToDp /Dp 10 0 R >> >>",
        "<< /Type /DPartRoot /DPartRootNode 9 0 R >>",
        "<< /Type /DPart /Parent 8 0 R /DParts [11 0 R 10 0 R] >>",
        "<< /Type /DPart /Parent 9 0 R /DParts [12 0 R 13 0 R] >>",
        "<< /Type /DPart /Parent 9 0 R /Start 3 0 R >>",
        "<< /Type /DPart /Parent 10 0 R /Start 4 0 R >>",
        "<< /Type /DPart /Parent 10 0 R /Start 5 0 R >>",
    ];
    let mut bytes = String::from("%PDF-2.0\n");
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
        "trailer\n<< /Size {size} /Root 1 0 R \
         /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
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
fn a_goto_dp_action_on_an_interior_node_shows_its_first_leaf_page() {
    let bytes = document_bytes();
    let syntax = Document::open(bytes.clone()).expect("PDF 2.0 fixture opens");
    assert!(!syntax.was_recovered(), "fixture xref offsets are exact");

    let mut viewer = Viewer::new(400, 300, 1.0);
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
    assert_eq!(page(&viewer), 0);

    // Positive control: a leaf GoToDp names Start directly and reaches page 3.
    let _ = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(6, 0)))
        .count();
    assert_eq!(page(&viewer), 2, "the direct leaf Start control works");

    let _ = viewer.handle(Command::GoTo(PageTarget::First)).count();
    let _ = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(7, 0)))
        .count();
    assert_eq!(
        page(&viewer),
        1,
        "the action names an interior DPart node, whose first descendant leaf starts on page 2"
    );
}
