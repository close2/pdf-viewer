//! Constructed annotation borders: defaults and explicit active controls.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are small enough that no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 100×100 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
///
/// Every fixture in this file is one page, a few objects long and numbered from 1, so the offsets
/// are whatever the objects land at — which is what §7.5.4 asks a cross-reference table to say.
fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Assembles a one-page PDF whose page carries one annotation.
///
/// The page's own content stream is empty, so every mark in the raster came from the
/// appearance and nothing has to be subtracted to see it.
fn pdf_with(annotation: &str, appearance_dict: &str, appearance: &str) -> Vec<u8> {
    pdf_over("", annotation, appearance_dict, appearance)
}

/// The same, with content on the page under the annotation.
///
/// §12.5.5 composites an appearance's transparency group "with a backdrop consisting of the page
/// content along with any previously painted annotations", so a fixture about what the group
/// composites *onto* needs a page that has something on it. `page` is drawn in the page's own
/// default user space.
fn pdf_over(page: &str, annotation: &str, appearance_dict: &str, appearance: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form {appearance_dict} /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        page.len().saturating_add(1),
        appearance.len().saturating_add(1)
    );

    assemble(&body)
}

/// Renders a fixture at one pixel per unit onto a transparent background.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "complete cross-reference table");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported")
}

fn at(raster: &pdf_render::Raster, x: u32, y: u32) -> usize {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4)
}

/// Whether anything was painted at a point, given in PDF coordinates.
fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    raster.data[at(raster, x, y) + 3] > 0
}

/// Table 168 defaults an omitted /D on a dashed /BS border to [3] with phase zero.
/// Compare the consumer raster against that explicit value, then prove the fixture can see a
/// different pattern. All three are valid, complete PDFs; only /BS /D differs.
#[test]
fn a_dashed_border_without_a_pattern_uses_the_table_168_default() {
    let draw = |dash: &str| {
        render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /C [1 0 0] \
                 /BS << /W 2 /S /D {dash} >> >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
    };

    let defaulted = draw("");
    let explicit_default = draw("/D [3]");
    let different_pattern = draw("/D [1 3]");

    assert_eq!(defaulted, explicit_default, "omitted /D means [3]");
    assert_ne!(
        defaulted, different_pattern,
        "the control pattern changes the raster"
    );
    assert!(
        (10..90).any(|y| painted(&defaulted, 10, y)),
        "the left edge has visible dash segments"
    );

    // Independent page marks establish phase zero, which default-vs-explicit alone cannot do.
    let page_stroke = |phase: u8| {
        render(pdf_over(
            &format!("1 0 0 RG 2 w [3] {phase} d 11 11 78 78 re S"),
            "<< /Type /Annot /Subtype /Link /Rect [10 10 90 90] /Border [0 0 0] >>",
            "/BBox [0 0 10 10]",
            "",
        ))
    };
    assert_eq!(defaulted, page_stroke(0));
    assert_ne!(defaulted, page_stroke(1));
}

/// Table 169 defaults /BE /S to S (no effect). The empty dictionary must therefore leave the
/// ordinary border unchanged, while /S /C must produce an observable cloud in the same consumer.
#[test]
fn a_border_effect_without_a_style_defaults_to_no_effect() {
    let draw = |effect: &str| {
        render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /C [1 0 0] \
                 /BS << /W 2 >> {effect} >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
    };

    let ordinary = draw("");
    let default_effect = draw("/BE << >>");
    let explicit_no_effect = draw("/BE << /S /S /I 2 >>");
    let cloudy = draw("/BE << /S /C >>");
    assert_eq!(
        ordinary, explicit_no_effect,
        "S means no effect even with an intensity"
    );

    assert_eq!(
        ordinary, default_effect,
        "omitted /S defaults to S (no effect)"
    );
    assert_ne!(ordinary, cloudy, "explicit C is an active-effect control");
    assert!(
        painted(&ordinary, 11, 11),
        "the solid square inks its corner"
    );
    assert!(
        !painted(&cloudy, 11, 11),
        "the cloud leaves the square's corner bare"
    );
    assert!(
        painted(&cloudy, 11, 50),
        "a cloudy cusp reaches the inscribed border"
    );
}

fn link_border(style: &str) -> pdf_render::Raster {
    render(pdf_with(
        &format!(
            "<< /Type /Annot /Subtype /Link /Rect [10 10 90 90] /F 4 /C [1 0 0] /BS << {style} >> >>"
        ),
        "/BBox [0 0 10 10]",
        "",
    ))
}

/// Table 168 gives independent defaults for W and S, even with a present empty BS dictionary.
#[test]
fn an_empty_border_style_defaults_to_one_point_solid() {
    let empty = link_border("");
    assert_eq!(empty, link_border("/W 1 /S /S"));
    assert_eq!(empty, link_border("/W 1"));
    assert_eq!(empty, link_border("/S /S"));
    assert_ne!(empty, link_border("/W 5 /S /S"));
    assert_ne!(empty, link_border("/W 1 /S /D"));
    assert!(painted(&empty, 10, 50));
    assert!(!painted(&empty, 11, 50));
}

/// Table 168 explicitly requires tolerating an unrecognized S by using the solid default.
#[test]
fn an_unknown_border_style_uses_solid_even_with_a_dash_array() {
    let unknown = link_border("/W 2 /S /FutureStyle /D [1 3]");
    assert_eq!(unknown, link_border("/W 2 /S /S"));
    assert_ne!(unknown, link_border("/W 2 /S /D /D [1 3]"));
    assert!(painted(&unknown, 10, 50));
}
