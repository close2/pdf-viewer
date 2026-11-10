//! Independent Table 182 orientation regression. `/AP` is intentionally absent so this tests
//! the reader's constructed-appearance policy, not the producer's separate appearance duties.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "fixed raster dimensions bound indexing and the usize conversion on supported targets"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const BUDGET: u64 = 1 << 24;

fn pdf_with_text_and_quad(subtype: &str, text_matrix: &str, quad_points: &str) -> Vec<u8> {
    let content = format!("BT /F1 10 Tf {text_matrix} Tm (TEST) Tj ET\n");
    let annotation = format!(
        "<< /Type /Annot /Subtype /{subtype} /Rect [0 0 100 100] /F 4 \
         /C [0 0 1] /QuadPoints [{quad_points}] >>"
    );
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n\
         6 0 obj\n{}\nendobj\n",
        content.len(),
        content,
        annotation
    );

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
    let _ = writeln!(out, "trailer\n<< /Size {size} /Root 1 0 R >>");
    let _ = write!(out, "startxref\n{xref_at}\n%%EOF\n");
    out.into_bytes()
}

fn raster(subtype: &str, text_matrix: &str, quad_points: &str) -> pdf_render::Raster {
    let bytes = pdf_with_text_and_quad(subtype, text_matrix, quad_points);
    let parsed = Document::open(bytes).expect("complete PDF syntax");
    assert!(!parsed.was_recovered(), "fixture xref is exact");
    let page = pdf_model::Pages::new(&parsed).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&parsed, &page);
    assert!(
        interpretation.is_complete(),
        "the rotated text and mark are supported: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, BUDGET).expect("valid raster target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterization")
}

fn is_blue(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let offset = (usize::try_from(row.saturating_mul(raster.width).saturating_add(x))
        .expect("small raster"))
    .saturating_mul(4);
    let Some(red) = raster.data.get(offset).copied() else {
        return false;
    };
    let Some(green) = raster.data.get(offset + 1).copied() else {
        return false;
    };
    let Some(blue) = raster.data.get(offset + 2).copied() else {
        return false;
    };
    let Some(alpha) = raster.data.get(offset + 3).copied() else {
        return false;
    };
    blue > red && blue > green && alpha != 0
}

#[test]
fn underline_tracks_the_counterclockwise_quad_text_edge_not_its_opposite() {
    // Parent visually checked Figure84 on physical page508: x1/x2 is the baseline edge,
    // x3/x4 is the upper edge, and the order is counterclockwise in y-up space. The current
    // ADR0043/source claim that the figure shows x1/x2 at the top is incorrect.
    // Horizontal baseline control: Figure84's x1->x2 edge is the underline edge.
    let horizontal = raster("Underline", "1 0 0 1 20 20", "20 20 80 20 80 30 20 30");
    assert!(
        is_blue(&horizontal, 50, 20),
        "the horizontal x1-to-x2 edge is underlined"
    );
    assert!(
        !is_blue(&horizontal, 50, 30),
        "the opposite horizontal edge is clear"
    );

    // Rotate the text baseline up-left. This ordered quad still has positive signed area
    // (counterclockwise), but x1->x2 is now the page-space HIGHER edge: [80,20] -> [20,80].
    // Its midpoint is (50,50); the opposite edge midpoint is (43,43). The glyph's local up axis
    // points southwest, into the quad, so the underline remains on x1->x2. A rule that chooses
    // the lowest page-y edge instead of following the specified text edge paints the wrong edge.
    let rotated = raster(
        "Underline",
        "-0.70710678 0.70710678 -0.70710678 -0.70710678 80 20",
        "80 20 20 80 13 73 73 13",
    );
    assert!(
        is_blue(&rotated, 50, 50),
        "underline follows x1-to-x2 despite page rotation"
    );
    assert!(
        !is_blue(&rotated, 43, 43),
        "the lower page-y opposite edge is not the text baseline or underline location"
    );
}

/// Table182 calls these quadrilaterals, not necessarily parallelograms.
#[test]
fn highlight_preserves_all_four_corners_of_a_counterclockwise_trapezoid() {
    // The word occupies x30..54,y15..22, inside both independently stated quads.
    // The rectangular control differs only at vertex3, so dropping that vertex cannot pass.
    let rectangular = raster("Highlight", "1 0 0 1 30 15", "10 10 90 10 90 40 10 40");
    assert!(
        is_blue(&rectangular, 80, 35),
        "rectangular upper right is highlighted"
    );
    let trapezoid = raster("Highlight", "1 0 0 1 30 15", "10 10 90 10 70 40 20 40");
    assert!(is_blue(&trapezoid, 50, 30), "interior control");
    assert!(is_blue(&trapezoid, 80, 15), "lower-right interior control");
    assert!(!is_blue(&trapezoid, 15, 35), "upper-left exterior control");
    // At y35 the stated right edge has x=73.33; x80 is strictly outside.
    assert!(
        !is_blue(&trapezoid, 80, 35),
        "the far corner must constrain the highlight region"
    );
}
