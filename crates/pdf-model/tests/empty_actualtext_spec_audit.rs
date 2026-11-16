//! Public model regression for present-empty versus absent `/ActualText`.
//!
//! The source is a discretionary line-end hyphen: the printed text is `co-` at the end of one
//! line and `operate` at the start of the next. EC3 §14.9.4 describes `ActualText` as replacement
//! character content. Replacing just that hyphen with an empty string is a credible way to join
//! the word for extraction (`cooperate`). The absent and `(-)` controls preserve the line-end
//! hyphen. Whitespace inferred from positioning is ignored; its exact form is not the oracle. These tests concern `ActualText` only; they make
//! no inference about `/Alt` or `/E`, and the structure test has one marked content item to isolate replacement from element cardinality.

#![expect(clippy::expect_used, reason = "standalone exact-xref test fixture")]

use std::fmt::Write as _;

use pdf_syntax::Document;

#[derive(Clone, Copy)]
enum Location {
    SpanProperty,
    ArtifactProperty,
    StructureElement,
}

fn pdf(location: Location, value: Option<&str>) -> Vec<u8> {
    let (content, has_structure) = match location {
        Location::SpanProperty => {
            let actual = value.map_or_else(String::new, |value| format!(" /ActualText {value}"));
            (
                format!(
                    "BT /F1 12 Tf 10 80 Td (co) Tj /Span <<{actual}>> BDC (-) Tj EMC 0 -14 Td (operate) Tj ET"
                ),
                false,
            )
        }
        Location::ArtifactProperty => {
            let actual = value.map_or_else(String::new, |value| format!(" /ActualText {value}"));
            (
                format!(
                    "BT /F1 12 Tf 10 80 Td (co) Tj /Artifact << /Type /Layout{actual} >> BDC (-) Tj EMC 0 -14 Td (operate) Tj ET"
                ),
                false,
            )
        }
        Location::StructureElement => (
            "BT /F1 12 Tf 10 80 Td (co) Tj /Span << /MCID 0 >> BDC (-) Tj EMC 0 -14 Td (operate) Tj ET".to_owned(),
            true,
        ),
    };
    let element_actual = match (location, value) {
        (Location::StructureElement, Some(value)) => format!(" /ActualText {value}"),
        _ => String::new(),
    };
    let catalog_structure = if has_structure {
        "/StructTreeRoot 6 0 R"
    } else {
        ""
    };
    let page_structure = if has_structure {
        "/StructParents 0"
    } else {
        ""
    };
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_structure} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R {page_structure} >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len().saturating_add(1),
            content
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /StructTreeRoot /K 7 0 R /ParentTree 8 0 R >>".to_owned(),
        format!("<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K [0]{element_actual} >>"),
        "<< /Nums [0 [7 0 R]] >>".to_owned(),
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn text(location: Location, value: Option<&str>) -> (String, usize) {
    let document = Document::open(pdf(location, value)).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref must be authoritative");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the supported fixture should render completely: {:?}",
        interpretation.unsupported
    );
    assert!(
        interpretation.glyphs > 0,
        "the visible control text was interpreted"
    );
    (
        interpretation
            .text
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect(),
        interpretation.glyphs,
    )
}

fn assert_hyphen_case(location: Location) {
    let (absent, absent_glyphs) = text(location, None);
    let (kept_hyphen, kept_glyphs) = text(location, Some("(-)"));
    let (removed_hyphen, removed_glyphs) = text(location, Some("()"));
    assert_eq!(absent, "co-operate", "absence preserves the hyphen");
    assert_eq!(
        kept_hyphen, "co-operate",
        "nonempty replacement preserves hard hyphen"
    );
    assert_eq!(absent_glyphs, kept_glyphs);
    assert_eq!(
        absent_glyphs, removed_glyphs,
        "replacement leaves the interpreted glyph count unchanged"
    );
    assert_eq!(
        removed_hyphen, "cooperate",
        "empty replacement removes the discretionary hyphen"
    );
}

#[test]
fn empty_span_property_actualtext_suppresses_only_discretionary_hyphen() {
    assert_hyphen_case(Location::SpanProperty);
}

#[test]
fn empty_artifact_property_actualtext_suppresses_only_discretionary_hyphen() {
    assert_hyphen_case(Location::ArtifactProperty);
}

#[test]
fn empty_structure_actualtext_suppresses_only_discretionary_hyphen() {
    assert_hyphen_case(Location::StructureElement);
}
