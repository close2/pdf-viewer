//! Public raster regression to complement the shared-helper `CalRGB` identity oracle.
//!
//! The `CalRGB` matrix uses rounded D65 sRGB primaries, expressed as Table 63 columns.
//! Gamma 2.2 decodes the input components. Literal expected bytes use the independent
//! sRGB transfer equation, with one byte tolerance for the rounded matrix and conversion.
//! Source: ICC, <https://registry.color.org/rgb-registry/files/sRGB.pdf>, section A.8.
//! No production conversion helper computes expected values. `DeviceRGB` is a control.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture and in-bounds samples fail loudly"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const CONTENT: &str = "/C cs 0.2 0.1 0.05 sc 0 0 20 20 re f \
                       0.5 0.25 0.125 sc 20 0 20 20 re f \
                       /DeviceRGB cs 0.2 0.1 0.05 sc 40 0 20 20 re f";
const CALRGB: &str = "[/CalRGB << /WhitePoint [0.95047 1 1.08883] \
    /BlackPoint [0 0 0] /Gamma [2.2 2.2 2.2] \
    /Matrix [0.412456 0.212673 0.019334 0.357576 0.715152 0.119192 \
             0.180437 0.072175 0.950304] >>]";

fn fixture() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 20] \
          /Resources << /ColorSpace << /C 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{CONTENT}\nendstream",
            CONTENT.len() + 1
        ),
        CALRGB.to_owned(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    out
}

fn raster() -> pdf_render::Raster {
    let document = Document::open(fixture()).expect("valid PDF fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "supported CalRGB and DeviceRGB paints: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("small page target");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports these colours")
}

fn rgba(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let index = usize::try_from((y * raster.width + x) * 4).expect("small raster index");
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

fn assert_near(actual: [u8; 4], expected: [u8; 4], label: &str) {
    for (channel, (got, want)) in actual.iter().zip(expected).enumerate() {
        assert!(
            got.abs_diff(want) <= 1,
            "{label} channel {channel}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn calrgb_samples_match_independent_gamma_and_srgb_transfer_values() {
    let raster = raster();
    // At the center of each rectangle; values are rounded IEC sRGB encoding of c^2.2.
    assert_near(
        rgba(&raster, 10, 10),
        [48, 19, 5, 255],
        "first CalRGB sample",
    );
    assert_near(
        rgba(&raster, 30, 10),
        [128, 61, 26, 255],
        "second CalRGB sample",
    );
    // DeviceRGB is the stated-value control, with no CalRGB decoding gamma.
    assert_eq!(rgba(&raster, 50, 10), [51, 26, 13, 255]);
}
