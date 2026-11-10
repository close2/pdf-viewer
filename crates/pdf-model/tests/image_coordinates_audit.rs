//! End-to-end regression for ISO 32000-2 §8.9.4 image coordinates.
//! The 3x2 sample grid must map to the user-space unit square, even when the source
//! dimensions do not match the destination aspect ratio.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture is fully controlled and failures should be explicit"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;
const CONTENTS: &str = "q 60 0 0 60 20 20 cm /Im Do Q";
// 3 x 2 DeviceRGB samples: red, green, blue / cyan, magenta, yellow.
const IMAGE_ASCII_HEX: &str = "FF000000FF000000FF00FFFFFF00FFFFFF00>\n";

fn fixture(contents: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100]\n\
         /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Image /Width 3 /Height 2\n\
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Interpolate false\n\
         /Filter /ASCIIHexDecode /Length {} >>\nstream\n{}endstream\nendobj\n",
        contents.len().saturating_add(1),
        contents,
        IMAGE_ASCII_HEX.len(),
        IMAGE_ASCII_HEX,
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

fn at_page_point(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn raster(contents: &str) -> pdf_render::Raster {
    let document = Document::open(fixture(contents)).expect("valid complete one-page PDF");
    assert!(!document.was_recovered(), "fixture xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "the image and content operators are supported: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported image rasterization")
}

#[test]
fn non_square_source_grid_maps_row_major_onto_the_transformed_unit_square() {
    let raster = raster(CONTENTS);
    // The image's 3x2 source aspect ratio is intentionally mapped to a 60x60 user-space
    // square. Correct §8.9.4 mapping gives 20x30 destination cells, not a 90x60 image.
    for (x, y, expected, name) in [
        (30, 70, [255, 0, 0, 255], "row 0, column 0"),
        (50, 70, [0, 255, 0, 255], "row 0, column 1"),
        (70, 70, [0, 0, 255, 255], "row 0, column 2"),
        (30, 30, [0, 255, 255, 255], "row 1, column 0"),
        (50, 30, [255, 0, 255, 255], "row 1, column 1"),
        (70, 30, [255, 255, 0, 255], "row 1, column 2"),
        (
            10,
            10,
            [255, 255, 255, 255],
            "outside image on white page medium",
        ),
        (
            85,
            50,
            [255, 255, 255, 255],
            "outside square edge on white page medium",
        ),
    ] {
        assert_eq!(at_page_point(&raster, x, y), expected, "{name}");
    }
}

#[test]
fn a_reflected_image_reverses_columns_without_reversing_rows() {
    let raster = raster("q -60 0 0 60 80 20 cm /Im Do Q");
    for (x, y, expected) in [
        (70, 70, [255, 0, 0, 255]),
        (50, 70, [0, 255, 0, 255]),
        (30, 70, [0, 0, 255, 255]),
        (70, 30, [0, 255, 255, 255]),
        (50, 30, [255, 0, 255, 255]),
        (30, 30, [255, 255, 0, 255]),
        (85, 50, [255, 255, 255, 255]),
    ] {
        assert_eq!(at_page_point(&raster, x, y), expected, "at ({x}, {y})");
    }
}

#[test]
fn image_cells_follow_both_columns_of_a_skewed_ctm() {
    // Unit-square corners are (10,10), (60,20), (80,80), (30,70).
    // Sample cell centers map well inside those four non-axis-aligned edges.
    let raster = raster("q 50 10 20 60 10 10 cm /Im Do Q");
    for (x, y, expected) in [
        (33, 56, [255, 0, 0, 255]),
        (50, 60, [0, 255, 0, 255]),
        (66, 63, [0, 0, 255, 255]),
        (23, 26, [0, 255, 255, 255]),
        (40, 30, [255, 0, 255, 255]),
        (56, 33, [255, 255, 0, 255]),
        (15, 65, [255, 255, 255, 255]),
        (75, 25, [255, 255, 255, 255]),
    ] {
        assert_eq!(at_page_point(&raster, x, y), expected, "at ({x}, {y})");
    }
}
