//! `FreeText` Table 177 standalone-annotation regressions.
//!
//! These are standalone PDF 1.7 fixtures: `FreeText` is in page /Annots, while `AcroForm`
//! /Fields is empty. They do not use the existing `variable_text.rs` helper, which puts every
//! annotation into /Fields and is therefore a widget-shaped fixture for this subtype.

#![expect(
    clippy::expect_used,
    reason = "fixture and renderer failures should identify the failed test premise"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

/// A PDF 1.7 annotation-only `FreeText` source with a standard-14 font in `AcroForm` DR.
/// `entries` are additional `FreeText` dictionary entries. The file has a complete classic xref.
fn free_text_pdf(entries: &str, contents: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [] /DA (/F1 16 Tf 0 g) /DR << /Font << /F1 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /FreeText /Rect [20 20 180 80] \
         /F 4 /Contents ({contents}) /DA (/F1 16 Tf 0 g) /BS << /W 0 >> {entries} >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n"
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

fn draw(entries: &str, contents: &str) -> pdf_render::Raster {
    let document = Document::open(free_text_pdf(entries, contents)).expect("valid PDF 1.7");
    assert!(!document.was_recovered(), "exact cross references");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list")
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "indices are bounded by the fixed 200 by 100 raster"
)]
fn opacity(raster: &pdf_render::Raster, x: u32, row: u32) -> u8 {
    let index = (row * raster.width + x) as usize * 4;
    raster.data[index + 3]
}

/// Extents in PDF user-space coordinates; the raster's rows run downwards.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "indices are bounded by the fixed 200 by 100 raster"
)]
fn ink_bounds(raster: &pdf_render::Raster) -> (u32, u32, u32, u32) {
    let columns: Vec<_> = (0..raster.width)
        .filter(|x| (0..raster.height).any(|y| opacity(raster, *x, y) > 0))
        .collect();
    let rows: Vec<_> = (0..raster.height)
        .filter(|y| (0..raster.width).any(|x| opacity(raster, x, *y) > 0))
        .map(|row| raster.height - 1 - row)
        .collect();
    (
        *columns.first().expect("text ink has a left edge"),
        *columns.last().expect("text ink has a right edge"),
        *rows.iter().min().expect("text ink has a bottom edge"),
        *rows.iter().max().expect("text ink has a top edge"),
    )
}

/// Table 177 /Q defaults to left justification, whose specified code is /Q 0.
#[test]
fn freetext_missing_q_matches_zero_and_right_quadding_is_a_discriminating_control() {
    let missing = draw("", "MMMM");
    let explicit_left = draw("/Q 0", "MMMM");
    let centered = draw("/Q 1", "MMMM");
    let right = draw("/Q 2", "MMMM");
    assert_eq!(ink_bounds(&missing), ink_bounds(&explicit_left));

    let (left_start, left_end, _, _) = ink_bounds(&explicit_left);
    let (center_start, center_end, _, _) = ink_bounds(&centered);
    let (right_start, right_end, _, _) = ink_bounds(&right);
    assert!(
        (center_start + center_end).abs_diff(200) <= 4,
        "Q=1 should center the word on the page/box midpoint: {center_start}..{center_end}"
    );
    assert!(
        right_start > left_start + 70,
        "Q=2 should shift the same word toward the far side of the 160pt box: left start={left_start}, right start={right_start}"
    );
    assert!(
        right_end > left_end + 70,
        "right edge should move toward x=180"
    );
}

/// Table 177 defines /RD as [left top right bottom]. Each comparison changes only one
/// inset, with the border disabled so its rectangle cannot impersonate text movement.
#[test]
fn freetext_rd_independently_insets_each_edge_of_the_text_box() {
    let left = ink_bounds(&draw("/Q 0", "MMMM"));
    let inset_left = ink_bounds(&draw("/Q 0 /RD [24 0 0 0]", "MMMM"));
    assert_eq!(inset_left.0, left.0 + 24);
    assert_eq!(inset_left.1, left.1 + 24);
    assert_eq!((inset_left.2, inset_left.3), (left.2, left.3));

    let right = ink_bounds(&draw("/Q 2", "MMMM"));
    let inset_right = ink_bounds(&draw("/Q 2 /RD [0 0 30 0]", "MMMM"));
    assert_eq!(inset_right.0 + 30, right.0);
    assert_eq!(inset_right.1 + 30, right.1);
    assert_eq!((inset_right.2, inset_right.3), (right.2, right.3));

    let inset_top = ink_bounds(&draw("/Q 0 /RD [0 12 0 0]", "MMMM"));
    assert_eq!((inset_top.0, inset_top.1), (left.0, left.1));
    assert_eq!(inset_top.2 + 12, left.2);
    assert_eq!(inset_top.3 + 12, left.3);

    // Plenty of lines reach the lower clip edge. Changing only the bottom inset must
    // leave every pixel above y=28 untouched and remove all text below that edge.
    let text = "MMMM MMMMMM MMMMMM MMMMMM MMMMMM MMMMMM MMMMMM MMMMMM";
    let outer = draw("/Q 0", text);
    let inset_bottom = draw("/Q 0 /RD [0 0 0 8]", text);
    assert_eq!(
        (outer.width, outer.height),
        (inset_bottom.width, inset_bottom.height)
    );
    let mut clipped_ink = false;
    for row in 0..outer.height {
        let y = outer.height - 1 - row;
        for x in 0..outer.width {
            if y >= 28 {
                assert_eq!(opacity(&inset_bottom, x, row), opacity(&outer, x, row));
            } else {
                clipped_ink |= opacity(&outer, x, row) != 0;
                assert_eq!(opacity(&inset_bottom, x, row), 0);
            }
        }
    }
    assert!(
        clipped_ink,
        "the original text must reach the removed strip"
    );
}
