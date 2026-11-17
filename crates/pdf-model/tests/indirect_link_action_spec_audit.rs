//! §7.3.10: public link target across direct/indirect action and destination values.
#![expect(
    clippy::expect_used,
    reason = "a malformed small fixture is a test error"
)]

use std::io::Write as _;

use pdf_model::{Pages, link};
use pdf_syntax::Document;

fn pdf(indirect: bool) -> Vec<u8> {
    let action = if indirect {
        "8 0 R"
    } else {
        "<< /S /GoTo /D [4 0 R /Fit] >>"
    };
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [5 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_owned(),
        format!("<< /Type /Annot /Subtype /Link /Rect [10 10 40 30] /A {action} >>"),
        "null".to_owned(),
        "null".to_owned(),
        "<< /S 9 0 R /D 10 0 R >>".to_owned(),
        "/GoTo".to_owned(),
        "[4 0 R 11 0 R]".to_owned(),
        "/Fit".to_owned(),
    ];
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn link_target(indirect: bool) -> Option<usize> {
    let document = Document::open(pdf(indirect)).expect("valid PDF");
    assert!(!document.was_recovered(), "the explicit xref is exact");
    let pages = Pages::new(&document);
    let first = pages.get(0).expect("first page");
    let links = link::links(&document, &first);
    assert_eq!(links.len(), 1);
    link::target(&document, &pages, links.first()?)
}

#[test]
fn an_indirect_goto_and_indirect_destination_operands_reach_the_target_page() {
    assert_eq!(link_target(false), Some(1), "direct positive control");
    assert_eq!(
        link_target(true),
        Some(1),
        "indirect action, /S, /D and /Fit"
    );
}
