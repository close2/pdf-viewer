//! Public intent-dispatch control using an independently validated ICC.1 v4.4 profile.
//!
//! Profile provenance and independent expected-value scripts live in fixtures/icc1-gray-intents.
//! Exact RGB values pin this viewer's screen conversion, not universal ISO screen colors.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture and in-bounds samples fail loudly"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

const PROFILE: &[u8] = include_bytes!("fixtures/icc1-gray-intents/gray-display-three-intents.icc");

fn pdf_with_intent(intent: &str) -> Vec<u8> {
    let mut encoded = String::with_capacity(PROFILE.len() * 2 + 1);
    for byte in PROFILE {
        write!(encoded, "{byte:02X}").expect("write to String");
    }
    encoded.push('>');
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 64 64] /Resources << /ColorSpace << /GrayIntent [/ICCBased 5 0 R] >> >> /Contents 4 0 R >>".to_owned(),
        {
            let content = format!("/GrayIntent cs 0.5 scn {intent} ri 0 0 64 64 re f");
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len() + 1)
        },
        format!("<< /N 1 /Alternate /DeviceGray /Range [0 1] /Filter /ASCIIHexDecode /Length {} >>\nstream\n{encoded}\nendstream", encoded.len()),
    ];
    let mut pdf = String::from("%PDF-2.0\n%\u{00E2}\u{00E3}\u{00CF}\u{00D3}\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj\n{object}\nendobj", index + 1).expect("write to String");
    }
    let xref = pdf.len();
    writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).expect("write xref");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R /ID [<0102030405060708090A0B0C0D0E0F10><0102030405060708090A0B0C0D0E0F10>] >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("write trailer");
    pdf.into_bytes()
}

fn middle_pixel(intent: &str) -> [u8; 4] {
    let document = Document::open(pdf_with_intent(intent)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "fixture xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "unsupported: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("64x64 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("raster");
    assert_eq!((raster.width, raster.height), (64, 64));
    let offset = (32 * 64 + 32) * 4;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

#[test]
fn complete_gray_display_profile_selects_all_three_intent_tables() {
    // Independent expected values come from the actual quantized mft2 middle knots,
    // the ICC D50 PCS -> sRGB matrix, and the IEC sRGB transfer/rounding script beside the
    // profile fixture. These are this renderer's device-output regression values.
    for (intent, expected) in [
        ("/Perceptual", [253, 57, 60, 255]),
        ("/RelativeColorimetric", [137, 137, 137, 255]),
        ("/Saturation", [0, 167, 255, 255]),
    ] {
        assert_eq!(middle_pixel(intent), expected, "intent {intent}");
    }
}
