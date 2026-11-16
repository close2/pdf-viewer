//! Pixel test for ISO 32000-2 §11.6.6's per-Do group rule.
//! Expected pixels come from source-over arithmetic, not another renderer.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

fn assemble(body: &str) -> Vec<u8> {
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn pixel(page_content: &str) -> [u8; 4] {
    let form_content = "1 0 0 rg 0 0 100 100 re f\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /Half << /ca 0.5 >> >> /XObject << /Group 5 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I true >> /Length {} >>\nstream\n{}endstream\nendobj\n",
        page_content.len() + 1,
        page_content,
        form_content.len(),
        form_content
    );
    let document = Document::open(assemble(&body)).expect("valid fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(
        interpreted.is_complete(),
        "no unsupported content: {:?}",
        interpreted.unsupported
    );
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("raster");
    let row = raster.height - 1 - 50;
    let at = ((row * raster.width + 50) * 4) as usize;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// Section 11.6.6's prose following Table 145 says repeated Do calls are separate groups.
/// One half-opacity opaque red result over white is (255,128,128); two independent
/// invocations source-over that result produce about (255,64,64). A cached or reused
/// already-composited result would fail the two-call pixel while the one-call control
/// proves the fixture paints and the alpha path is live.
#[test]
fn each_do_of_the_same_transparency_group_composites_as_a_new_instance() {
    let once = pixel("/Half gs /Group Do");
    assert_eq!(once[0], 255);
    assert!(
        once[1].abs_diff(128) <= 1,
        "single half-opacity group: {once:?}"
    );
    assert!(
        once[2].abs_diff(128) <= 1,
        "single half-opacity group: {once:?}"
    );
    assert_eq!(once[3], 255);

    let twice = pixel("/Half gs /Group Do /Group Do");
    assert_eq!(twice[0], 255);
    assert!(
        twice[1].abs_diff(64) <= 1,
        "second independent red-over-pink pass: {twice:?}"
    );
    assert!(
        twice[2].abs_diff(64) <= 1,
        "second independent red-over-pink pass: {twice:?}"
    );
    assert_eq!(twice[3], 255);
    assert_ne!(
        twice, once,
        "the second invocation is not a cached final composite"
    );
}
