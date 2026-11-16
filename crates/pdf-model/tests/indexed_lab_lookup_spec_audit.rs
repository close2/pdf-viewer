//! Public rendering regression for ISO 32000-2 §8.6.6.3's byte-to-base-range mapping.
//!
//! Source: effective ISO+EC3, printed pp. 199–200 (physical pp. 214–215), footer
//! ISO 32000-2:2020(E). The source requires each palette byte to scale from [0,255] into
//! the corresponding base component range. Lab's L* range is [0,100], so a high L* byte
//! must remain a high lightness rather than become a fraction near zero.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact fixture; invalid structure or pixel coordinates must fail loudly"
)]

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

const GENEROUS: u64 = 1 << 30;

fn pdf(content: &str, resources: &str) -> Vec<u8> {
    let extra = "5 0 obj\n[/Indexed [/Lab <</WhitePoint [0.9642 1 0.8249] /Range [-100 100 -100 100]>>] 0 <FF8080>]\nendobj\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
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

fn centre(bytes: Vec<u8>) -> (u8, u8, u8) {
    let document = Document::open(bytes).expect("exact-xref fixture opens");
    assert!(!document.was_recovered(), "exact xref required");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture must draw completely: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS).expect("small target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("rasterizes");
    let at = ((10 * raster.width) + 10) as usize * 4;
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

#[test]
fn indexed_palette_byte_scales_into_lab_range_and_matches_direct_lab_control() {
    let indexed = centre(pdf(
        "/Idx cs 0 scn 0 0 20 20 re f",
        "/ColorSpace << /Idx 5 0 R >>",
    ));
    // Same color named directly: byte FF -> L*=100, byte 80 -> a*=b*=approximately 0.392.
    let direct = centre(pdf(
        "/Direct cs 100 0.39215687 0.39215687 scn 0 0 20 20 re f",
        "/ColorSpace << /Direct [/Lab <</WhitePoint [0.9642 1 0.8249] /Range [-100 100 -100 100]>>] >>",
    ));
    assert!(
        indexed.0 >= 248 && indexed.1 >= 248 && indexed.2 >= 248,
        "FF in the L* palette component scales to 100 (near white), got {indexed:?}"
    );
    for (actual, control) in [
        (indexed.0, direct.0),
        (indexed.1, direct.1),
        (indexed.2, direct.2),
    ] {
        assert!(
            actual.abs_diff(control) <= 1,
            "palette color {indexed:?} should match the independently stated direct Lab color {direct:?}"
        );
    }
}
