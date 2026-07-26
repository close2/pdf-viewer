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
              the fixtures are small enough that no index can overflow, and the ICC \
              fixture's constants are written as the fixed-point values it encodes"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

/// Builds a one-page PDF from a content stream and an optional extra object.
fn pdf_with(extra: &str, resources: &str, content: &str) -> Vec<u8> {
    pdf_with_catalog(extra, "", resources, content)
}

/// The same, with extra entries in the document catalog.
fn pdf_with_catalog(extra: &str, catalog: &str, resources: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n\
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

/// A CMYK profile built to say one thing, so a test can tell whether it was consulted.
///
/// Four input channels and two grid points per channel, which makes the table exactly the
/// sixteen ink corners — the same shape as the fallback it is replacing. Corner eight is
/// pure cyan, and it is set to the XYZ that sRGB renders as full green, a colour no press
/// makes and the fallback table never produces. Full ink is set to zero so the profile
/// reaches true black and needs no compensation, keeping the expected value exact.
fn green_cyan_profile() -> Vec<u8> {
    // XYZ, D50, of sRGB's green primary, in the `u1Fixed15` encoding lookup tables use.
    let green: [u16; 3] = [12620, 23491, 3182];
    let mut clut = vec![0u16; 16 * 3];
    clut[8 * 3..8 * 3 + 3].copy_from_slice(&green);

    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[16..20].copy_from_slice(b"CMYK");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[4, 3, 2, 0]); // four in, three out, two grid points
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes());
    tag.extend_from_slice(&2u16.to_be_bytes());
    for _ in 0..4 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }
    for value in &clut {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }

    let mut out = header;
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&144u32.to_be_bytes());
    out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
    out.extend_from_slice(&tag);
    out
}

/// Objects five and six: an output intent whose profile is [`green_cyan_profile`].
fn output_intent_objects() -> String {
    let mut hex = String::new();
    for byte in green_cyan_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    format!(
        "5 0 obj\n<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier (test) \
         /DestOutputProfile 6 0 R >>\nendobj\n\
         6 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream\n\
         endobj\n",
        hex.len().saturating_add(1)
    )
}

/// An output intent says what the document's device colours mean.
///
/// ISO 32000-2 §14.11.5: an output intent's `/DestOutputProfile` is "an ICC profile stream
/// defining the transformation from the PDF document's source colours to output device
/// colourants", and §8.6.5.7 NOTE 3 names it as the one thing in a PDF that can describe
/// the calibration its device colours were prepared for. A document that carries one has
/// said what its `DeviceCMYK` means, and guessing instead renders it in the wrong colours.
#[test]
fn an_output_intent_says_what_the_documents_device_colours_mean() {
    let colour = centre_colour(pdf_with_catalog(
        &output_intent_objects(),
        "/OutputIntents [5 0 R]",
        "",
        "1 0 0 0 k 0 0 20 20 re f",
    ));
    // Within a level: the green primary's XYZ does not land on exact multiples of
    // 1/32768, so the fixture's own encoding of it is a fraction of a level off. The
    // fallback's process cyan is 173 away in green, so nothing here is a near miss.
    let (r, g, b) = colour;
    assert!(
        r <= 1 && g >= 254 && b <= 1,
        "the output intent's profile must decide what `1 0 0 0 k` looks like, got {colour:?}"
    );

    // Without it, the same content is the process cyan of the assumed press.
    let without = centre_colour(pdf_with("", "", "1 0 0 0 k 0 0 20 20 re f"));
    assert_eq!(without, (0, 173, 239));
}

/// A `/DefaultCMYK` in the page's resources outranks the document's output intent.
///
/// §8.6.5.6 says a `Default` entry "shall be used"; §8.6.5.7 NOTE 3 says an output intent
/// "can suggest" a calibration. One is a requirement about this operation and the other is
/// a statement about the document, so the nearer and stronger of the two wins.
#[test]
fn a_default_space_outranks_the_output_intent() {
    let objects = format!(
        "{}7 0 obj\n[/DeviceN [/C /M /Y /K] /DeviceRGB 8 0 R]\nendobj\n\
         8 0 obj\n<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] /C0 [0 0 1] /C1 [0 0 1] \
         /N 1 >>\nendobj\n",
        output_intent_objects()
    );

    let colour = centre_colour(pdf_with_catalog(
        &objects,
        "/OutputIntents [5 0 R]",
        "/ColorSpace << /DefaultCMYK 7 0 R >>",
        "1 0 0 0 k 0 0 20 20 re f",
    ));
    assert_eq!(
        colour,
        (0, 0, 255),
        "the resources' /DefaultCMYK must win over the document's output intent"
    );
}
