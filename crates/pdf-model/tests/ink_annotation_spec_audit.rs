//! Self-contained pdf-model consumer tests for `InkList` joins and the PDF 2.0 Ink /Path curve.
//! Missing AP deliberately exercises constructed-appearance policy; the PDF 2.0 Path
//! case violates the writer AP requirement and is not a fully conforming-file witness.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixed complete PDF fixtures fail loudly if parsing, rendering, or pixel indexing is wrong"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 30;

fn pdf_with_ink(geometry: &str, version: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Ink /Rect [10 10 90 90] /F 4 /C [0 0 0] \
         /BS << /W 2 >> {geometry} >>\nendobj\n"
    );
    assemble(&body, version)
}

fn assemble(body: &str, version: &str) -> Vec<u8> {
    let mut out = format!("%PDF-{version}\n");
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
    let id = if version == "2.0" {
        " /ID [<00112233445566778899aabbccddeeff> <ffeeddccbbaa99887766554433221100>]"
    } else {
        ""
    };
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R{id} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn raster(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture has an exact xref table"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture is fully supported: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("a bounded target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("the CPU backend supports this path")
}

fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let index = ((row.saturating_mul(raster.width)).saturating_add(x) as usize)
        .saturating_mul(4)
        .saturating_add(3);
    raster.data[index] > 0
}

#[test]
fn an_ink_list_with_three_points_joins_each_consecutive_pair() {
    let image = raster(pdf_with_ink("/InkList [[20 20 80 20 80 80]]", "1.7"));

    assert!(
        painted(&image, 50, 20),
        "the first straight segment is drawn"
    );
    assert!(
        painted(&image, 80, 50),
        "the second straight segment is drawn"
    );
    assert!(
        !painted(&image, 50, 50),
        "the implementation's chosen straight-line connection does not shortcut the corner"
    );
}

#[test]
fn ink_path_six_operands_produce_a_cubic_curve_not_an_endpoint_chord() {
    // Table 185 still requires /InkList even when /Path is present. Include a conflicting
    // straight /InkList line to exercise the implementation's documented Path preference.
    // Move to (20,20), then curve with controls (20,80), (80,80), ending at (80,20).
    // At t=1/2 the cubic is (50,65), well away from the straight endpoint chord y=20.
    let image = raster(pdf_with_ink(
        "/InkList [[20 20 80 20]] /Path [[20 20] [20 80 80 80 80 20]]",
        "2.0",
    ));

    assert!(
        painted(&image, 50, 64),
        "the bow of the specified cubic is stroked"
    );
    assert!(
        !painted(&image, 50, 20),
        "there is no straight chord between the cubic's endpoints"
    );
}
