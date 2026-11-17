//! Public-consumer control for ISO 32000-2 §8.6.4.3 `DeviceRGB` state reset.
//! Source: printed p.183 / physical p.198, footer ISO 32000-2:2020(E): selecting the
//! `DeviceRGB` current colour space initializes its three components to zero.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture; malformed PDF or out-of-range pixel must fail loudly"
)]
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

fn pdf(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        if !object.trim().is_empty() {
            offsets.push(out.len());
            out.push_str(object);
        }
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
fn centre(bytes: Vec<u8>) -> [u8; 3] {
    let document = Document::open(bytes).expect("valid fixture");
    assert!(!document.was_recovered());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 30).expect("target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("raster");
    let at = ((10 * raster.width) + 10) as usize * 4;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}
#[test]
fn selecting_device_rgb_resets_nonstroking_components_to_zero() {
    let after_red = centre(pdf("1 0 0 rg /DeviceRGB cs 0 0 20 20 re f"));
    let black_control = centre(pdf("0 0 0 rg 0 0 20 20 re f"));
    let red_control = centre(pdf("1 0 0 rg 0 0 20 20 re f"));
    assert_eq!(
        after_red,
        [0, 0, 0],
        "CS selection resets RGB components per §8.6.4.3"
    );
    assert_eq!(after_red, black_control);
    assert_eq!(
        red_control,
        [255, 0, 0],
        "the control proves the preceding state was nonzero"
    );
}
