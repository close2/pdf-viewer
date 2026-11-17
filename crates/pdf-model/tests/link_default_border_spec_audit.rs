//! Public consumer regression for §12.5.4's neither-entry default.
//!
//! The Link has a valid explicit color, no stored AP (Link is an AP-writer exception), and no
//! Border or BS. Its visible result is compared with independent explicit one-point controls.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small fixed fixture: malformed input and invalid raster coordinates should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

fn pdf(extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 80] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /F 4 \
         /C [1 0 0] {extra} >>\nendobj\n"
    );
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }
    let xref_at = pdf.len();
    let size = offsets.len().saturating_add(1);
    writeln!(pdf, "xref\n0 {size}").expect("write xref header");
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write xref row");
    }
    write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    )
    .expect("write trailer");
    pdf.into_bytes()
}

fn draw(extra: &str) -> pdf_render::Raster {
    let document = Document::open(pdf(extra)).expect("valid PDF 1.7");
    assert!(
        !document.was_recovered(),
        "complete exact cross-reference table"
    );
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid Link fixture: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let index = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

#[test]
fn a_link_without_border_entries_uses_the_one_point_solid_default() {
    let omitted = draw("");
    let legacy_explicit = draw("/Border [0 0 1]");
    let style_explicit = draw("/BS << /W 1 /S /S >>");
    let thicker = draw("/Border [0 0 2]");

    assert_eq!(omitted.data, legacy_explicit.data);
    assert_eq!(omitted.data, style_explicit.data);
    assert_ne!(omitted.data, thicker.data, "two points is a live control");
    assert_eq!(pixel(&omitted, 20, 40), [255, 0, 0, 255], "edge is red");
    assert_eq!(
        pixel(&omitted, 21, 40),
        [0, 0, 0, 0],
        "one point inward is clear"
    );
    assert_eq!(
        pixel(&omitted, 19, 40),
        [0, 0, 0, 0],
        "outside Rect stays clear"
    );
}
