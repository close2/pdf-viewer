//! Specification audit of ISO 32000-2 §7.7 to §7.12 (review 2234): the resource dictionaries
//! §7.8.3 associates with each kind of content stream, through `pdf-model`'s public walks over a
//! page's resources.
//!
//! Each test is one behaviour finding of `doc/reviews/2234-specification-audit-7-7-7-12.md`,
//! with a control the same fixture passes on main beside the failing half. The fixtures are
//! exact-xref documents built here.

#![expect(
    clippy::expect_used,
    reason = "exact-xref test fixtures and their assertions"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};

/// A classic-xref PDF from object bodies numbered from 1. A body holding `@STREAM@` is a stream:
/// what precedes the marker is the dictionary's entries, what follows it the data.
fn pdf(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let body = match body.split_once("@STREAM@") {
            Some((dict, data)) => format!(
                "<< {dict} /Length {} >>\nstream\n{data}\nendstream",
                data.len().saturating_add(1)
            ),
            None => body.clone(),
        };
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

/// The document of `bytes` and its first page.
fn opened(bytes: Vec<u8>) -> (Document, pdf_model::Page) {
    let document = Document::open(bytes).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    (document, page)
}

/// A page showing one glyph of a Type 3 font whose glyph description paints in a `Separation`
/// space named `/Gold`, the space stated in the glyph stream's own `/Resources` when
/// `in_glyph` and in the font dictionary's `/Resources` otherwise.
fn type3_gold(in_glyph: bool) -> Vec<u8> {
    let space = "/ColorSpace << /CS0 [/Separation /Gold /DeviceCMYK 7 0 R] >>";
    let (font_resources, glyph_resources) = if in_glyph {
        (String::new(), format!("/Resources << {space} >>"))
    } else {
        (format!("/Resources << {space} >>"), String::new())
    };
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        "@STREAM@BT /F1 50 Tf 10 10 Td (a) Tj ET".to_owned(),
        format!(
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 6 0 R >> \
             /Encoding << /Type /Encoding /Differences [97 /a] >> \
             /FirstChar 97 /LastChar 97 /Widths [1000] {font_resources} >>"
        ),
        format!("{glyph_resources} @STREAM@1000 0 d0 /CS0 cs 1 scn 0 0 1000 1000 re f"),
        "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0.2 0.8 0] /N 1 >>".to_owned(),
    ])
}

/// Whether the page's spot colourants include `/Gold`.
fn names_gold(bytes: Vec<u8>) -> bool {
    let (document, page) = opened(bytes);
    pdf_model::colourants::spot_colourants(&document, &page)
        .names()
        .iter()
        .any(|name| name.as_bytes() == b"Gold")
}

/// 2234-B01. §7.8.3 as Errata Collection 3's Issue #128 restates it: a glyph description's named
/// resources "shall be present in the resource dictionary designated by the first Resources entry
/// found" in the glyph stream's own dictionary, then the Type 3 font dictionary, then the page.
/// The interpreter draws the glyph from its own `/Resources` (`content/text/type3.rs`); the walk
/// that decides which spot colourants get a plane reads only the font's.
#[test]
fn a_type3_glyphs_own_resources_name_its_spot_colourant_spec_audit() {
    assert!(
        names_gold(type3_gold(false)),
        "control: the font dictionary's /Resources is walked"
    );
    assert!(
        names_gold(type3_gold(true)),
        "a Separation space in the glyph stream's own /Resources is a colourant the page paints"
    );
}

/// A page whose one optional content group is named by a `BDC /OC` inside `host`: `"form"` for
/// a form `XObject` the page draws, `"pattern"` for a tiling pattern the page fills with, and
/// `"appearance"` for an annotation's normal appearance stream. The group is in each content
/// stream's own `/Resources /Properties`, as §7.8.3 and §8.11.3.2 place it.
fn group_inside(host: &str) -> Vec<u8> {
    let properties = "/Resources << /Properties << /oc1 7 0 R >> >>";
    let marked = "/OC /oc1 BDC 0 0 5 5 re f EMC";
    let (page_resources, contents, annots, hosted) = match host {
        "form" => (
            "/XObject << /X0 6 0 R >>",
            "/X0 Do",
            "",
            format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] {properties} @STREAM@{marked}"
            ),
        ),
        "pattern" => (
            "/Pattern << /P0 6 0 R >>",
            "/Pattern cs /P0 scn 0 0 50 50 re f",
            "",
            format!(
                "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
                 /XStep 10 /YStep 10 {properties} @STREAM@{marked}"
            ),
        ),
        _ => (
            "",
            "",
            "/Annots [8 0 R]",
            format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] {properties} @STREAM@{marked}"
            ),
        ),
    };
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R \
         /OCProperties << /OCGs [7 0 R] /D << /Order [7 0 R] /ListMode /VisiblePages >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << {page_resources} >> /Contents 4 0 R {annots} >>"
        ),
        format!("@STREAM@{contents}"),
        "<< /Producer (audit) >>".to_owned(),
        hosted,
        "<< /Type /OCG /Name (Layer) >>".to_owned(),
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /AP << /N 6 0 R >> >>".to_owned(),
    ])
}

/// Whether the page is found to reference the group, object 7.
fn references_group(host: &str) -> bool {
    let (document, page) = opened(group_inside(host));
    pdf_model::optional_content::groups_referenced_by(&document, &page).contains(&ObjectId {
        number: 7,
        generation: 0,
    })
}

/// 2234-B02. §7.8.3 associates a resource dictionary with every content stream, and §7.8.2
/// lists the content streams that are not a page's: "forms …, patterns …, certain fonts …, and
/// annotation appearances". `groups_referenced_by` answers Table 99's `VisiblePages` ("only those
/// groups in the Order array that are referenced by one or more visible pages") by following
/// the page's resources into forms only, so a group marked inside a tiling pattern's cell is not
/// listed although the page paints it.
#[test]
fn a_group_marked_inside_a_pattern_is_referenced_by_the_page_spec_audit() {
    assert!(
        references_group("form"),
        "control: a group marked inside a form the page draws is referenced"
    );
    assert!(
        references_group("pattern"),
        "a group marked inside a tiling pattern the page paints with is referenced by the page"
    );
}

/// 2234-B02, the annotation route: §12.5.5 draws a normal appearance onto the page, and its
/// stream's own `/Resources /Properties` names the group its `BDC /OC` marks.
#[test]
fn a_group_marked_inside_an_annotation_appearance_is_referenced_by_the_page_spec_audit() {
    assert!(
        references_group("form"),
        "control: a group marked inside a form the page draws is referenced"
    );
    assert!(
        references_group("appearance"),
        "a group marked inside an annotation's normal appearance is referenced by the page"
    );
}
