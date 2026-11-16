//! Table 65 requires `ICCBased` range clipping before unsupported-profile Alternate use.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "fixed tiny exact-xref fixture and integer raster coordinates"
)]
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;
const ICC2_V5_LAB: &[u8] = include_bytes!("fixtures/icc2-lab/icc2_v5_lab_mpe.icc");
const LAB: &str = "[/Lab <</WhitePoint [0.9642 1 0.8249] /Range [-200 200 -200 200]>>]";
fn pdf(extra: &str, resources: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
      2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
      3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
      4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
        content.len().saturating_add(1)
    );
    let mut out = String::from("%PDF-2.0\n");
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
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102030405060708090A0B0C0D0E0F10> <0102030405060708090A0B0C0D0E0F10>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        let _ = write!(s, "{b:02X}");
    }
    s.push('>');
    s
}
fn icc_stream() -> String {
    let encoded = hex(ICC2_V5_LAB);
    format!(
        "5 0 obj\n<< /N 3 /Range [0 100 -128 127 -128 127] /Alternate {LAB} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{encoded}\nendstream\nendobj\n",
        encoded.len()
    )
}
fn centre(bytes: Vec<u8>) -> [u8; 3] {
    let doc = Document::open(bytes).expect("fixture opens");
    assert!(!doc.was_recovered());
    let page = pdf_model::Pages::new(&doc).get(0).expect("one page");
    let parsed = pdf_model::interpret(&doc, &page);
    assert!(parsed.is_complete(), "{:?}", parsed.unsupported);
    let target = TargetSpec::for_page(&parsed.display_list, 1.0, 1 << 30).expect("small page");
    let raster = CpuRasterizer::new()
        .rasterize(&parsed.display_list, target)
        .expect("rasterizes");
    let at = ((10 * raster.width) + 10) as usize * 4;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}
#[test]
fn unsupported_icc2_profile_clamps_to_icc_range_before_wider_alternate() {
    let resources =
        "/ColorSpace << /FromProfile [/ICCBased 5 0 R] /DirectLab ".to_owned() + LAB + " >>";
    let from_profile = centre(pdf(
        &icc_stream(),
        &resources,
        "/FromProfile cs 50 150 0 scn 0 0 20 20 re f",
    ));
    let clamped_control = centre(pdf(
        &icc_stream(),
        &resources,
        "/DirectLab cs 50 127 0 scn 0 0 20 20 re f",
    ));
    let unclamped_control = centre(pdf(
        &icc_stream(),
        &resources,
        "/DirectLab cs 50 150 0 scn 0 0 20 20 re f",
    ));
    assert_ne!(
        clamped_control, unclamped_control,
        "the controls must make skipping the first ICC /Range clamp observable"
    );
    assert_eq!(
        from_profile, clamped_control,
        "Table 65 constrains source a*=150 to ICC Lab's maximum 127 before applying the wider alternate"
    );
}
