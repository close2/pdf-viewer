//! A balanced BX/EX section ignores a future/unknown operator and its operands
//! without changing the visible path or the normal page content around it.
//!
//! Source: ISO 32000-2:2020 §7.8.2 physical PDF p.126 / printed p.111 and Table 33 physical
//! p.127 / printed p.112. This does not assign recovery behavior to unmatched BX or EX.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small standalone PDF fixture must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn page(content: &str) -> Vec<u8> {
    let stream_length = content.len();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {stream_length} >>\nstream\n{content}\nendstream\nendobj\n"
    );
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object.as_bytes());
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn pixels(content: &str) -> Vec<u8> {
    let document = Document::open(page(content)).expect("strict-xref PDF opens");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid compatibility section is quiet: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("10x10 raster fits");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("simple path renders")
        .data
        .clone()
}

#[test]
fn unknown_operator_and_its_operands_inside_bx_ex_leave_the_path_like_the_control() {
    let with_unknown = pixels("0 g 0 0 10 10 re BX 1 2 3 futureOp EX f");
    let control = pixels("0 g 0 0 10 10 re f");
    assert_eq!(
        with_unknown, control,
        "ignored operator leaves path untouched"
    );
    let center = 220_usize;
    assert_eq!(
        &with_unknown[center..center + 4],
        &[0, 0, 0, 255],
        "known fill after the compatibility section is still drawn"
    );
}
