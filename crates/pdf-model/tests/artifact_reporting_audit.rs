//! Exact-xref integration assertion for §14.8.2.2 artifact reporting.
//!
//! Effective #484 makes the two marked-content artifact forms descriptive rather than a
//! producer MUST. This checks the reader's public reporting policy: preserve drawn text, report
//! the declared artifact separately from content classified by absence, and keep real content
//! in structure order. Artifact classification does not itself require a consumer to omit text.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "bounded exact-xref fixture and explicitly checked artifact count"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, structure::Tree};
use pdf_syntax::{Document, ObjectId};

fn tagged_page() -> Document {
    let content = concat!(
        "BT /F1 12 Tf 10 80 Td /P <</MCID 0>> BDC (Real) Tj EMC ",
        "0 -20 Td /Artifact <</Type /Pagination /Subtype /Header ",
        "/Attached [/Top /Right /Bottom] /BBox [0 50 200 75]>> BDC (Header) Tj EMC ",
        "0 -20 Td (Loose) Tj ET"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 4 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 8 0 R >> >> /Contents 5 0 R /StructParents 0 >>".to_owned(),
        "<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 7 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len() + 1),
        "<< /Type /StructElem /S /P /P 4 0 R /Pg 3 0 R /K 0 >>".to_owned(),
        "<< /Nums [0 [6 0 R]] >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];

    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1).expect("String writes do not fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1)
        .expect("String writes do not fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("String writes do not fail");
    }
    write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("String writes do not fail");
    let document = Document::open(bytes.into_bytes()).expect("valid fixture");
    assert!(
        !document.was_recovered(),
        "the fixture uses exact xref offsets"
    );
    document
}

#[test]
fn artifact_reporting_does_not_remove_text_and_absence_is_separate_from_declaration() {
    let document = tagged_page();
    let page = Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    assert!(
        interpretation.glyphs > 0,
        "font substitution must be available"
    );

    assert!(interpretation.text.contains("Real"));
    assert!(interpretation.text.contains("Header"));
    assert!(interpretation.text.contains("Loose"));
    assert_eq!(
        interpretation.artifacts.len(),
        2,
        "one declared and one by absence"
    );
    let declared = &interpretation.artifacts[0];
    let absent = &interpretation.artifacts[1];
    assert_eq!(
        interpretation
            .text
            .get(declared.range.clone())
            .map(str::trim),
        Some("Header")
    );
    assert_eq!(declared.found, pdf_model::content::ArtifactSource::Declared);
    assert_eq!(declared.artifact.attached, [true, true, false, true]);
    assert_eq!(declared.artifact.bbox, Some([0.0, 50.0, 200.0, 75.0]));
    assert_eq!(
        interpretation.text.get(absent.range.clone()).map(str::trim),
        Some("Loose")
    );
    assert_eq!(absent.found, pdf_model::content::ArtifactSource::Absence);

    let tree = Tree::of(&document).expect("structure root");
    assert_eq!(
        tree.logical_text(&document, ObjectId::new(3, 0), &interpretation)
            .as_deref()
            .map(str::trim),
        Some("Real"),
        "logical order includes the reachable marked content, not the artifact ranges"
    );
}
