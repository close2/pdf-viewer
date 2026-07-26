//! One colour, drawn every way a PDF can draw it, must come out the same.
//!
//! This is the test that would have caught the defect it was written for. A `DeviceCMYK`
//! colour reaches the page by three routes — the `k` operator, `scn` in a `DeviceCMYK`
//! space, and the samples of a CMYK image — and each had its own conversion. They
//! disagreed: `0.5 0 0 0.5 k` produced a red channel of 0.25 where the identical colour
//! set through `scn` produced 0.0.
//!
//! Nothing about the rendered page reveals that. Each drawing looks like a plausible
//! colour; they are simply not the *same* colour, and which one you get depends on how
//! the producer happened to write the file.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are small enough that no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

/// Builds a one-page PDF from a content stream and an optional extra object.
fn pdf_with(extra: &str, resources: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         {extra}",
        content.len().saturating_add(1)
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        if object.trim().is_empty() {
            continue;
        }
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Renders a fixture and returns the colour at its centre.
fn centre_colour(bytes: Vec<u8>) -> (u8, u8, u8) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let at = ((10 * raster.width) + 10) as usize * 4;
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

/// A colour chosen so that the conversions that used to disagree, disagree loudly.
///
/// With `c + k` at exactly 1.0, the additive formula clips to black while the
/// multiplicative one gives a quarter intensity — the widest gap the two produced.
const C: f32 = 0.5;
const M: f32 = 0.0;
const Y: f32 = 0.0;
const K: f32 = 0.5;

#[test]
fn a_cmyk_colour_is_the_same_however_it_is_drawn() {
    // Route one: the `k` operator, which sets space and colour together.
    let by_operator = centre_colour(pdf_with(
        "",
        "",
        &format!("{C} {M} {Y} {K} k 0 0 20 20 re f"),
    ));

    // Route two: an explicit DeviceCMYK space and `scn`.
    let by_scn = centre_colour(pdf_with(
        "",
        "",
        &format!("/DeviceCMYK cs {C} {M} {Y} {K} scn 0 0 20 20 re f"),
    ));

    // Route three: a one-pixel DeviceCMYK image stretched over the page. Written as
    // ASCII hex so the fixture stays readable.
    let samples = format!(
        "{:02X}{:02X}{:02X}{:02X}>",
        (C * 255.0) as u8,
        (M * 255.0) as u8,
        (Y * 255.0) as u8,
        (K * 255.0) as u8
    );
    let image = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /ASCIIHexDecode \
         /Length {} >>\nstream\n{samples}\nendstream\nendobj\n",
        samples.len()
    );
    let by_image = centre_colour(pdf_with(
        &image,
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));

    assert_eq!(
        by_operator, by_scn,
        "the `k` operator and `scn` in DeviceCMYK must agree"
    );
    // An image's samples are eight-bit, so they cannot express 0.5 exactly; one level of
    // difference is quantisation rather than a second conversion.
    for (a, b) in [
        (by_operator.0, by_image.0),
        (by_operator.1, by_image.1),
        (by_operator.2, by_image.2),
    ] {
        assert!(
            a.abs_diff(b) <= 2,
            "a CMYK image must use the same conversion as a fill: {by_operator:?} against \
             {by_image:?}"
        );
    }
}

/// The same, for grey — which is a pass-through and so should be exact everywhere.
#[test]
fn a_grey_is_the_same_however_it_is_drawn() {
    let by_operator = centre_colour(pdf_with("", "", "0.25 g 0 0 20 20 re f"));
    let by_scn = centre_colour(pdf_with("", "", "/DeviceGray cs 0.25 scn 0 0 20 20 re f"));

    let image = "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
                 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /ASCIIHexDecode \
                 /Length 3 >>\nstream\n40>\nendstream\nendobj\n";
    let by_image = centre_colour(pdf_with(
        image,
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));

    assert_eq!(by_operator, by_scn);
    assert_eq!(by_operator, by_image, "0x40 is exactly 0.25 of 255");
}

/// `/DefaultCMYK` replaces the device space, as the specification requires.
///
/// ISO 32000-2 §8.6.5.6: when a device colour space is selected, the resources'
/// `/ColorSpace` dictionary is checked for the matching `Default` entry, and "if such an
/// entry is present, its value **shall** be used as the colour space for the operation
/// currently being performed". A document uses this to say what press its `DeviceCMYK`
/// means; a reader that ignores it renders those pages in the wrong colours and has no
/// way of knowing.
#[test]
fn a_default_colour_space_replaces_the_device_space() {
    // A `/DefaultCMYK` that maps every ink to a fixed green, through a Separation-style
    // tint transform, so the substitution is unmistakable in the output.
    let space = "5 0 obj\n[/DeviceN [/C /M /Y /K] /DeviceRGB 6 0 R]\nendobj\n\
                 6 0 obj\n<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] \
                 /C0 [0 1 0] /C1 [0 1 0] /N 1 >>\nendobj\n";
    let with_default = centre_colour(pdf_with(
        space,
        "/ColorSpace << /DefaultCMYK 5 0 R >>",
        "1 0 0 0 k 0 0 20 20 re f",
    ));
    assert_eq!(
        with_default,
        (0, 255, 0),
        "the document's /DefaultCMYK must be used in place of DeviceCMYK"
    );

    // Without the entry the same content is process cyan, as before.
    let without = centre_colour(pdf_with("", "", "1 0 0 0 k 0 0 20 20 re f"));
    assert_eq!(
        without,
        (0, 173, 239),
        "and DeviceCMYK is unaffected otherwise"
    );
}

/// An `ICCBased` space uses its `/Alternate` rather than a guess from `/N`.
#[test]
fn an_icc_space_prefers_its_stated_alternate() {
    // Three components, so a guess from /N would say DeviceRGB. The /Alternate says the
    // profile stands in for a Lab space instead, and that is what the producer meant.
    let profile = "5 0 obj\n<< /N 3 /Alternate [/Lab << /Range [-100 100 -100 100] >>] \
                   /Length 0 >>\nstream\n\nendstream\nendobj\n";
    let colour = centre_colour(pdf_with(
        profile,
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >>",
        "/CS0 cs 100 0 0 scn 0 0 20 20 re f",
    ));
    // L* of 100 with no a* or b* is white; read as DeviceRGB it would be pure red.
    assert!(
        colour.0 > 250 && colour.1 > 250 && colour.2 > 250,
        "expected Lab white, got {colour:?} — /Alternate was ignored"
    );
}
