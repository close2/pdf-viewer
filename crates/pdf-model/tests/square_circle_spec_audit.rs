//! Square and Circle constructed appearance geometry and interior-colour component counts.
//!
//! The construction cases intentionally omit AP to reach this project's constructed-appearance fallback.
//! These are syntactically complete PDFs (exact stream lengths, valid xref, no recovery), but
//! a producer would violate Table 166 by omitting AP for a nondegenerate Square/Circle. The
//! tests guard the documented reader-robustness choice in ADR 0030 and do not claim that
//! construction is a reader-side shall imposed by §12.5.6.8.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a test fixture or rendering failure should fail at its source; image coordinates are bounded by these fixtures"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    writeln!(out, "xref\n0 {size}").expect("xref header");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("xref entry");
    }
    write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    )
    .expect("trailer");
    out.into_bytes()
}

/// Build a one-page PDF 1.7 with an annotation only in /Annots and complete classic xref.
fn pdf(annotation: &str) -> Vec<u8> {
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n"
    ))
}

fn stored_square_with_black_border() -> Vec<u8> {
    let stream = "0 G 1 w 0.5 0.5 79 79 re S";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> /Length {} >>\n\
         stream\n{stream}\nendstream\nendobj\n",
        stream.len() + 1
    );
    assemble(&body)
}

fn render_bytes(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("syntactically complete PDF");
    assert!(!document.was_recovered(), "the exact xref is used");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "no mark in these fixtures is unsupported: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterizer supports the display list")
}

fn render(annotation: &str) -> pdf_render::Raster {
    render_bytes(pdf(annotation))
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> usize {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4)
}

fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    raster.data[pixel(raster, x, y) + 3] > 0
}

fn colour(raster: &pdf_render::Raster, x: u32, y: u32) -> (u8, u8, u8) {
    let at = pixel(raster, x, y);
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

fn alpha(raster: &pdf_render::Raster, x: u32, y: u32) -> u8 {
    raster.data[pixel(raster, x, y) + 3]
}

fn extent(raster: &pdf_render::Raster) -> (u32, u32, u32, u32) {
    let mut bounds = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..raster.height {
        for x in 0..raster.width {
            if painted(raster, x, y) {
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
        }
    }
    bounds
}

/// §12.5.4's missing-/Border-and-/BS case is a solid one-point border.
#[test]
fn square_without_border_entries_uses_one_point_solid_with_stated_colour() {
    let control = render_bytes(stored_square_with_black_border());
    assert!(
        painted(&control, 10, 50),
        "stored-AP control draws its outline"
    );
    assert_eq!(colour(&control, 10, 50), (0, 0, 0));
    assert_eq!(alpha(&control, 50, 50), 0);

    let square = render("<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /C [0] >>");
    assert!(
        square.data == control.data,
        "default border matches the explicit one-point solid AP"
    );
    assert!(painted(&square, 10, 50), "the default outline reaches Rect");
    assert_eq!(
        colour(&square, 10, 50),
        (0, 0, 0),
        "the specified stroke colour is black"
    );
    assert_eq!(
        alpha(&square, 50, 50),
        0,
        "a border alone leaves the center empty"
    );
}

/// Table 180 /RD is [left, top, right, bottom], and unequal Rect axes still define an ellipse.
#[test]
fn square_and_circle_rd_use_the_asymmetric_left_top_right_bottom_differences() {
    let full = render(
        "<< /Type /Annot /Subtype /Square /Rect [10 10 90 70] /F 4 \
         /IC [0 0 1] /BS << /W 0 >> >>",
    );
    assert_eq!(
        extent(&full),
        (10, 10, 89, 69),
        "no RD leaves shape at Rect"
    );

    let square = render(
        "<< /Type /Annot /Subtype /Square /Rect [10 10 90 70] /F 4 \
         /IC [1 0 0] /RD [10 15 20 5] /BS << /W 0 >> >>",
    );
    // Independent arithmetic: x=[10+10, 90-20]=[20,70], y=[10+5,70-15]=[15,55].
    assert_eq!(extent(&square), (20, 15, 69, 54));
    assert_eq!(colour(&square, 40, 35), (255, 0, 0));
    for (x, y) in [(19, 35), (70, 35), (40, 14), (40, 55)] {
        assert!(!painted(&square, x, y), "outside inner square at ({x},{y})");
    }

    let circle = render(
        "<< /Type /Annot /Subtype /Circle /Rect [10 10 90 70] /F 4 \
         /IC [0 0 1] /RD [10 15 20 5] /BS << /W 0 >> >>",
    );
    // The same [20,70] × [15,55] box is an ellipse of radii 25 and 20, not a forced circle.
    assert!(painted(&circle, 45, 35), "ellipse centre");
    for (x, y) in [(22, 35), (67, 35), (45, 17), (45, 52)] {
        assert!(painted(&circle, x, y), "near an ellipse axis at ({x},{y})");
    }
    for (x, y) in [(21, 16), (68, 16), (21, 53), (68, 53)] {
        assert!(!painted(&circle, x, y), "outside ellipse at ({x},{y})");
    }
}

/// Table 180 defines /IC's colour space by component count.
#[test]
fn square_interior_ic_count_selects_gray_rgb_cmyk_or_transparency() {
    let with = |ic: &str, border: &str| {
        render(&format!(
            "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 \
             /IC {ic} {border} >>"
        ))
    };

    let gray = with("[0.25]", "/BS << /W 0 >>");
    let gray_pixel = colour(&gray, 50, 50);
    assert!(
        gray_pixel.0.abs_diff(gray_pixel.1) <= 2
            && gray_pixel.1.abs_diff(gray_pixel.2) <= 2
            && (20..=235).contains(&gray_pixel.0),
        "one component is DeviceGray and fills the interior: {gray_pixel:?}"
    );
    let rgb = with("[1 0 0]", "/BS << /W 0 >>");
    assert_eq!(
        colour(&rgb, 50, 50),
        (255, 0, 0),
        "three components are RGB"
    );

    let cmyk = with("[1 0 0 0]", "/BS << /W 0 >>");
    let cyan = colour(&cmyk, 50, 50);
    assert!(
        cyan.1 > cyan.0.saturating_add(80) && cyan.2 > cyan.0.saturating_add(80),
        "four components are DeviceCMYK cyan, not a three-component red: {cyan:?}"
    );

    let transparent = with("[]", "/BS << /W 0 >>");
    assert_eq!(
        alpha(&transparent, 50, 50),
        0,
        "zero components mean no fill"
    );
    assert!(
        !painted(&transparent, 50, 50),
        "no border was stated either"
    );
}
