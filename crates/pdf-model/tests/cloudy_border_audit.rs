//! Cloudy borders retain independently stated dash and width parameters.

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

/// Table 169 expressly requires a cloudy border to retain the BS dash array.
#[test]
fn a_cloudy_polygon_honours_distinct_border_dash_arrays() {
    let draw = |dash: &str| {
        render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] /F 4 /C [0 0 1] \
                 /Vertices [20 20 80 20 80 80 20 80] /BS << /W 2 /S /D /D {dash} >> \
                 /BE << /S /C /I 2 >> >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
    };

    let long_dashes = draw("[10 5]");
    let short_dashes = draw("[2 3]");
    assert_ne!(
        long_dashes, short_dashes,
        "the active BS pattern changes cloud strokes"
    );
    assert!(
        (20..80).any(|x| painted(&long_dashes, x, 20)),
        "the active cloudy border has visible stroke marks"
    );
    assert!(
        (20..80).any(|x| painted(&short_dashes, x, 20)),
        "the alternate pattern also has visible stroke marks"
    );
}

/// Width is an independent Table 169 duty. Polygon vertices hold the cloud path fixed as BS
/// width changes; /I 2 keeps this implementation's chosen radius above both positive widths.
#[test]
fn a_cloudy_polygon_honours_bs_width_and_zero_suppresses_its_stroke() {
    let draw = |width: u8| {
        render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] /F 4 /C [0 0 1] \
                 /Vertices [20 20 80 20 80 80 20 80] /BS << /W {width} >> \
                 /BE << /S /C /I 2 >> >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
    };

    let no_border = draw(0);
    let thin = draw(1);
    let thick = draw(4);
    assert_ne!(thin, thick, "changing BS width changes the cloud stroke");
    let coverage =
        |r: &pdf_render::Raster| -> u64 { r.data.chunks_exact(4).map(|p| u64::from(p[3])).sum() };
    assert!(coverage(&thin) > 0);
    assert!(
        coverage(&thick) > 2 * coverage(&thin),
        "four-point stroke has substantially greater coverage on the same cloud path"
    );
    assert!(
        (24..29).any(|x| (12..17).any(|y| painted(&thin, x, y))),
        "positive width paints near the cloud apex"
    );
    assert!(
        (24..29).any(|x| (12..17).any(|y| painted(&thick, x, y))),
        "the wider stroke also paints near the cloud apex"
    );
    assert!(
        (0..100).all(|y| (0..100).all(|x| !painted(&no_border, x, y))),
        "zero BS width suppresses the border stroke"
    );
}
