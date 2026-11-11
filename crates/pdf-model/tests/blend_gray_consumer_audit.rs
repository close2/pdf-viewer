//! Scratch PDF-consumer tests for §11.3.5.3 Gray conversion and colour clipping.
//!
//! Promote under `crates/pdf-model/tests/` only after parent review. The Gray outputs are
//! intentionally an observable consequence rather than proof of the internal Gray→RGB→Gray
//! route: nonseparable modes on neutral Gray values reduce by Table 135 to the backdrop (Hue,
//! Saturation, Color) or source (Luminosity). That identity makes the internal conversion
//! unobservable for these operands, but gives an independent end-to-end expected result.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "complete tiny fixture PDFs and a 100x100 raster are test preconditions"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn fixture(group_space: &str, blend: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS {group_space} >> \
         /Resources << /ExtGState << /GS << /BM /{blend} >> >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    assemble(&body)
}

fn assemble(body: &str) -> Vec<u8> {
    let mut output = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(output.len());
        output.push_str(object);
    }
    let xref_at = output.len();
    let size = offsets.len() + 1;
    let _ = writeln!(output, "xref\n0 {size}");
    output.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    output.into_bytes()
}

fn rendered_pixel(bytes: Vec<u8>, x: u32, y: u32) -> [u8; 4] {
    let document = Document::open(bytes).expect("complete fixture PDF");
    assert!(!document.was_recovered(), "exact source xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one fixture page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture is fully supported: {:?}",
        interpretation.unsupported
    );
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100x100 page target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("fixture rasterizes");
    let offset = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

fn assert_rgb(actual: [u8; 4], expected: [u8; 3], reason: &str) {
    for (got, want) in actual[..3].iter().zip(expected) {
        assert!(
            (i16::from(*got) - i16::from(want)).abs() <= 2,
            "{reason}: got {actual:?}, expected {expected:?} ±2"
        );
    }
    assert_eq!(actual[3], 255, "opaque fills must stay opaque");
}

#[test]
fn nonseparable_modes_on_a_devicegray_page_group_follow_gray_table135_results() {
    let content = "0.2 g 10 10 80 80 re f /GS gs 0.8 g 10 10 80 80 re f";
    for (mode, expected, reason) in [
        (
            "Hue",
            [51, 51, 51],
            "Hue preserves neutral backdrop luminosity",
        ),
        (
            "Saturation",
            [51, 51, 51],
            "Saturation over a neutral backdrop leaves it unchanged",
        ),
        (
            "Color",
            [51, 51, 51],
            "Color preserves neutral backdrop luminosity",
        ),
        (
            "Luminosity",
            [204, 204, 204],
            "Luminosity takes the neutral source luminosity",
        ),
    ] {
        let painted = rendered_pixel(fixture("/DeviceGray", mode, content), 50, 50);
        assert_rgb(painted, expected, reason);
    }

    let normal = rendered_pixel(fixture("/DeviceGray", "Normal", content), 50, 50);
    assert_rgb(normal, [204, 204, 204], "Normal source-color control");
}

#[test]
fn hue_consumer_clips_the_red_blue_result_to_the_table135_value() {
    // Table 135 gives Hue(blue backdrop, red source) = SetLum(SetSat(red, 1), .11).
    // SetLum first yields (.81,−.19,−.19); ClipColor's n<0 branch makes red
    // .11 + ((.81−.11)*.11)/(.11−(−.19)) = .3666667, about byte 94.
    // The wrong pre-ClipColor result is .81 (207), so this is a discriminating consumer check.
    let content = "0 0 1 rg 10 10 80 80 re f /GS gs 1 0 0 rg 10 10 80 80 re f";
    let hue = rendered_pixel(fixture("/DeviceRGB", "Hue", content), 50, 50);
    assert_rgb(
        hue,
        [94, 0, 0],
        "RGB Hue applies helper arithmetic and ClipColor before output",
    );

    let normal = rendered_pixel(fixture("/DeviceRGB", "Normal", content), 50, 50);
    assert_ne!(
        hue[..3],
        normal[..3],
        "normal source control differs from Hue"
    );
}
