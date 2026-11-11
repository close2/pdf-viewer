//! Regression for ordinary image `XObject` interpolation plumbing.
//! ISO 32000-2 §8.9.5.3 makes `/Interpolate` a hint a processor may ignore; this test
//! records the current viewer's deliberate policy of honoring true for magnified images.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "controlled valid PDF fixture; failures should be explicit"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;
const IMAGE_HEX: &str = "00FF>\n";

fn fixture(interpolate: Option<bool>, matrix: &str) -> Vec<u8> {
    let contents = format!("q {matrix} cm /Im Do Q");
    let extra = match interpolate {
        Some(true) => "/Interpolate true",
        Some(false) => "/Interpolate false",
        None => "",
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40]\n\
         /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 1\n\
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /ASCIIHexDecode\n\
         {extra} /Length {} >>\nstream\n{}endstream\nendobj\n",
        contents.len().saturating_add(1),
        contents,
        IMAGE_HEX.len(),
        IMAGE_HEX,
    );
    assemble(&body)
}

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

fn render(interpolate: Option<bool>, matrix: &str) -> pdf_render::Raster {
    let document = Document::open(fixture(interpolate, matrix)).expect("valid complete PDF");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "supported image page: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported rasterization")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn mixed_gray(pixel: [u8; 4], what: &str) {
    assert_eq!(pixel[0], pixel[1], "{what}: equal gray channels: {pixel:?}");
    assert_eq!(pixel[1], pixel[2], "{what}: equal gray channels: {pixel:?}");
    assert!(
        (16..=239).contains(&pixel[0]),
        "{what}: bilinear transition should be visibly between black and white, got {pixel:?}"
    );
    assert_eq!(pixel[3], 255, "{what}: opaque image sample");
}

#[test]
fn ordinary_image_interpolation_hint_is_parsed_and_honored_on_magnified_reflected_images() {
    for (name, matrix, left, right) in [
        (
            "normal",
            "20 0 0 20 10 10",
            [0, 0, 0, 255],
            [255, 255, 255, 255],
        ),
        // Reflect horizontally around x=20 while keeping the same 20x20 destination square.
        (
            "reflected",
            "-20 0 0 20 30 10",
            [255, 255, 255, 255],
            [0, 0, 0, 255],
        ),
    ] {
        let absent = render(None, matrix);
        let explicitly_false = render(Some(false), matrix);
        assert_eq!(
            absent.data, explicitly_false.data,
            "{name}: false is the table default"
        );

        // The 2x1 source is magnified tenfold. With the current default-false policy, each
        // boundary pixel takes one source sample; source interiors verify the image/color route.
        assert_eq!(
            pixel(&absent, 14, 20),
            left,
            "{name}: left-side source interior"
        );
        assert_eq!(
            pixel(&absent, 25, 20),
            right,
            "{name}: right-side source interior"
        );
        assert_eq!(
            pixel(&absent, 19, 20),
            left,
            "{name}: nearest sample before seam"
        );
        assert_eq!(
            pixel(&absent, 20, 20),
            right,
            "{name}: nearest sample after seam"
        );

        let requested = render(Some(true), matrix);
        assert_eq!(
            pixel(&requested, 14, 20),
            left,
            "{name}: left interior remains stable"
        );
        assert_eq!(
            pixel(&requested, 25, 20),
            right,
            "{name}: right interior remains stable"
        );
        mixed_gray(
            pixel(&requested, 19, 20),
            &format!("{name}: first seam pixel"),
        );
        mixed_gray(
            pixel(&requested, 20, 20),
            &format!("{name}: second seam pixel"),
        );
    }
}
