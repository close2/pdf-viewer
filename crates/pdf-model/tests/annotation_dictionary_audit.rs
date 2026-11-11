//! Complete-PDF consumer test for Table 166's page-local /NM identity.
//!
//! Existing Annex O coverage uses a corpus PDF and proves that the page parameter scopes the
//! lookup. This small complete file proves the complementary allowed case: the same /NM may name
//! one annotation on each of two different pages, with each page resolving its own object.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "malformed test fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};

fn two_page_pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [6 0 R] >>",
        "<< /Type /Annot /Subtype /Text /Rect [1 2 3 4] /NM (same-name) >>",
        "<< /Type /Annot /Subtype /Text /Rect [5 6 7 8] /NM (same-name) >>",
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{}\nendobj", index + 1, object);
    }
    let xref = bytes.len();
    let _ = writeln!(bytes, "xref\n0 {}", objects.len() + 1);
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = writeln!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>",
        objects.len() + 1
    );
    let _ = writeln!(bytes, "startxref\n{xref}\n%%EOF");
    bytes.into_bytes()
}

#[test]
fn the_same_nm_names_distinct_annotations_on_distinct_pages() {
    let document = Document::open(two_page_pdf()).expect("complete PDF with exact xref");
    assert!(!document.was_recovered(), "exact xref offsets");
    let pages = pdf_model::Pages::new(&document);
    let first = pages.get(0).expect("first page");
    let second = pages.get(1).expect("second page");

    assert_eq!(
        pdf_model::fragment::annotation_named(&document, &first, b"same-name"),
        Some(ObjectId::new(5, 0)),
        "Table 166 only requires /NM uniqueness within this page"
    );
    assert_eq!(
        pdf_model::fragment::annotation_named(&document, &second, b"same-name"),
        Some(ObjectId::new(6, 0)),
        "page-local lookup must not collapse names to a document-wide identity"
    );
    assert_eq!(
        pdf_model::fragment::annotation_named(&document, &first, b"absent"),
        None,
        "a page with no matching /NM supplies the negative control"
    );
}
