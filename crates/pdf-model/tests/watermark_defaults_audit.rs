//! Standalone integration test for Table 194's omitted defaults.
//!
//! The fixture owns its assembler and raster assertions.
//! It is PDF 1.7, has exact stream lengths and xref offsets, and supplies Resources on the page
//! and appearance form.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "standalone test fixture; invalid syntax or pixel indexing should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

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
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn watermark(fixed_print: &str) -> Vec<u8> {
    let appearance = "0 0 0 rg 0 0 10 10 re f";
    let empty_content = "";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{empty_content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Watermark /Rect [20 30 60 70] /F 4 \
         /AP << /N 6 0 R >> {fixed_print} >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] \
         /Resources << >> /Length {} >>\nstream\n{appearance}\nendstream\nendobj\n",
        empty_content.len().saturating_add(1),
        appearance.len().saturating_add(1),
    );
    assemble(&body)
}

fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "exact xref fixture");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should render completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list")
}

fn pdf_extent(raster: &pdf_render::Raster) -> (u32, u32, u32, u32) {
    let (mut left, mut bottom, mut right, mut top) = (u32::MAX, u32::MAX, 0, 0);
    let mut drew = false;
    for row in 0..raster.height {
        let y = raster.height.saturating_sub(1).saturating_sub(row);
        for x in 0..raster.width {
            let offset =
                ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
            if raster.data[offset + 3] > 0 {
                drew = true;
                left = left.min(x);
                bottom = bottom.min(y);
                right = right.max(x);
                top = top.max(y);
            }
        }
    }
    assert!(drew, "the fixture must produce nonempty ink");
    (left, bottom, right, top)
}

#[test]
fn omitted_fixed_print_matrix_h_and_v_match_their_explicit_defaults() {
    let omitted = render(watermark("/FixedPrint << /Type /FixedPrint >>"));
    let explicit = render(watermark(
        "/FixedPrint << /Type /FixedPrint /Matrix [1 0 0 1 0 0] /H 0 /V 0 >>",
    ));
    let displaced = render(watermark(
        "/FixedPrint << /Type /FixedPrint /H 0.25 /V 0.5 >>",
    ));

    assert_eq!(pdf_extent(&omitted), (0, 0, 39, 39));
    assert_eq!(pdf_extent(&explicit), (0, 0, 39, 39));
    assert_eq!(pdf_extent(&displaced), (25, 50, 64, 89));
}
