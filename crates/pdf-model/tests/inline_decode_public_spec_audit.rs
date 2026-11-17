//! Public-consumer regression for Table 91's `/D` Decode abbreviation.
//!
//! An unfiltered 8-bit gray inline sample 64 has default output 64; Table 91 `/D [1 0]` must
//! reverse it to 191. `/Decode [1 0]` is the independent spelling control.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the standalone exact-xref fixture has fixed bounds"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const BUDGET: u64 = 1 << 20;

fn fixture(content: &[u8]) -> Vec<u8> {
    let mut objects = Vec::new();
    objects.push(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec());
    objects.push(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec());
    objects.push(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> /Contents 4 0 R >>\nendobj\n".to_vec(),
    );
    let mut content_object =
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_object.extend_from_slice(content);
    content_object.extend_from_slice(b"\nendstream\nendobj\n");
    objects.push(content_object);

    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(&object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut xref = String::new();
    let _ = writeln!(xref, "xref\n0 {size}");
    xref.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());
    out
}

fn center(decode: &str) -> [u8; 4] {
    let mut content =
        format!("40 0 0 40 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /G {decode} ID ").into_bytes();
    content.push(64);
    content.extend_from_slice(b" EI");

    let document = Document::open(fixture(&content)).expect("complete PDF 1.7 fixture");
    assert!(!document.was_recovered(), "fixture xref is exact");
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, BUDGET)
        .expect("40x40 page fits target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("inline image rasterizes");
    let row = raster.height.saturating_sub(1).saturating_sub(20);
    let at = ((row * raster.width + 20) as usize) * 4;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

#[test]
fn inline_decode_abbreviation_reaches_the_image_sample_mapper() {
    assert_eq!(center(""), [64, 64, 64, 255], "default gray control");
    let full_name = center("/Decode [1 0]");
    let abbreviated = center("/D [1 0]");
    assert_eq!(full_name, [191, 191, 191, 255], "full Decode spelling");
    assert_eq!(abbreviated, full_name, "Table 91 /D spelling");
}
