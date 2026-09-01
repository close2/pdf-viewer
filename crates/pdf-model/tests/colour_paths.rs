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

/// Renders a fixture and returns every pixel of it.
fn raster_of(bytes: Vec<u8>) -> Vec<u8> {
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
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported")
        .data
        .clone()
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

/// A fourth route to the same colour: a `DCTDecode` image whose codestream is CMYK.
///
/// # Why this is a colour test and not an image test
///
/// `zune-jpeg` converts a four-component codestream to RGB by default, with a formula of its
/// own — `(1 − C)(1 − K)` over samples it takes to be stored inverted, which is the convention
/// a *standalone* Adobe CMYK JPEG follows. That is a second place a colour becomes a pixel,
/// which this file exists to forbid, and it is not what a PDF says: §8.9.5.2's `/Decode` array
/// states a sample's meaning, its Table 88 default for `DeviceCMYK` is the identity, and
/// §7.4.8 defers to Adobe Technical Note #5116 for which *markers* to honour and for the
/// YCbCr/YCCK transform — not for the polarity of a sample.
///
/// `cmykjpeg.pdf` is the corpus witness. Its image carries the Adobe APP14 marker with
/// transform 0, no `/Decode`, and ordinary CMYK samples; read as inverted, its sky comes out
/// black, which is what this reader drew until the hundred-and-seventy-eighth session while
/// all four references drew a photograph. The oracle could not fail on it — the references
/// disagree among themselves about `DeviceCMYK` (ADR 0048) so the verdict is `ambiguous`
/// either way — and it was the ambiguous ranking that named it.
///
/// The sample asserted on is the codestream's first, read out of the file by two decoders
/// that are not ours: `(122, 55, 14, 1)`, which `ImageMagick` and PIL both report as the
/// complement `(133, 200, 241, 254)` because both apply the standalone-JPEG inversion. The
/// expected pixel is what `ColourSpace::to_rgb` makes of it — the same function every other
/// route in this file goes through — and the inverted reading is nowhere near it.
///
/// The image raster is taken from the display list rather than from a rendered page, so that
/// nothing here depends on how a 200x150 image is resampled onto a letter page.
#[test]
fn a_cmyk_jpegs_samples_are_the_colour_spaces_and_not_the_decoders() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    if !root.is_dir() {
        println!("skipped: the pdf.js corpus submodule is not checked out");
        return;
    }
    let path = root.join("cmykjpeg.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        panic!("the corpus is present but {} is missing", path.display());
    };
    let document = Document::open(bytes).expect("cmykjpeg.pdf opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "cmykjpeg.pdf page one draws completely: {:?}",
        interpretation.unsupported
    );

    let image = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Image { image, .. } => Some(image),
            _ => None,
        })
        .expect("the page draws one image");
    // The samples as the file states them: this image carries no mask, so the identity
    // placement asks for nothing the decode did not already produce.
    let image = image.at(pdf_render::Transform::IDENTITY);
    let top_left = (image.data[0], image.data[1], image.data[2]);

    let stored = [122.0 / 255.0, 55.0 / 255.0, 14.0 / 255.0, 1.0 / 255.0];
    let expected = pdf_model::colour::ColourSpace::Cmyk.to_rgb(&stored);
    let expected = (
        (expected.r * 255.0).round() as u8,
        (expected.g * 255.0).round() as u8,
        (expected.b * 255.0).round() as u8,
    );
    for (a, b) in [
        (top_left.0, expected.0),
        (top_left.1, expected.1),
        (top_left.2, expected.2),
    ] {
        assert!(
            a.abs_diff(b) <= 1,
            "a CMYK JPEG's first sample must reach the raster through ColourSpace::to_rgb: \
             got {top_left:?}, the clause's reading is {expected:?}, and the inverted one is \
             black"
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

/// An `ICCBased` image is converted through its profile, exactly as a fill in it is.
///
/// ISO 32000-2 §8.6.5.5, and the whole of what this test is about: the profile is the
/// document's statement of what its numbers mean, so the same four numbers must produce the
/// same colour whether they arrive as an `scn` operand or as an image sample. Until the
/// twenty-fifth session they did not — `image.rs` reduced an `ICCBased` space to a device
/// space by its `/N` and unpacked it as one, so the profile applied to fills and not to
/// images.
///
/// It is the same defect this whole file exists to catch, one level up: three `DeviceCMYK`
/// conversions once disagreed, and when that was fixed the shape survived where an *image*
/// was on one side of it. The measurement that made it worth fixing is in ADR 0034 — 31
/// corpus documents carry 1037 `ICCBased` images and on 15 the two answers differ by 18
/// levels out of 255 for a mid grey — and none of them could fail a gate, because a
/// difference that big on an image nobody compares is still `ambiguous`.
///
/// [`green_cyan_profile`] answers "pure cyan is sRGB's green primary", which no press makes
/// and no fallback table produces, so a passing assertion cannot be a coincidence.
#[test]
fn an_icc_image_is_converted_through_the_same_profile_as_a_fill() {
    let mut hex = String::new();
    for byte in green_cyan_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    // Object order is object number here: the fixture builder numbers what it finds.
    let objects = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Filter /ASCIIHexDecode \
         /Length 9 >>\nstream\nFF000000>\nendstream\nendobj\n\
         6 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream\n\
         endobj\n",
        hex.len().saturating_add(1)
    );

    let by_image = centre_colour(pdf_with(
        &objects,
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    let by_fill = centre_colour(pdf_with(
        &objects,
        "/XObject << /Im0 5 0 R >> /ColorSpace << /CS0 [/ICCBased 6 0 R] >>",
        "/CS0 cs 1 0 0 0 scn 0 0 20 20 re f",
    ));

    assert_eq!(
        by_image, by_fill,
        "the same colour through the same profile, as a sample and as an operand"
    );
    let (r, g, b) = by_image;
    assert!(
        r <= 1 && g >= 254 && b <= 1,
        "the profile says pure cyan is green; got {by_image:?}"
    );
}

/// §8.6.5.6's remapping reaches an image's own `/ColorSpace`, not only the graphics state's.
///
/// > A colour space is selected for painting each graphics object. This is either the current
/// > colour space parameter in the graphics state or a colour space given as an entry in an
/// > image XObject, inline image, or shading dictionary. Regardless of how the colour space
/// > is specified, it shall be subject to remapping as described below.
///
/// The second sentence is the one that was missing. An image's space was parsed against an
/// *empty* resource dictionary — defensible on the reading that an image states its space in
/// full, and wrong, because "in full" is about resolving a *name* and this clause is about
/// replacing a *device space*.
///
/// **This comment said "[o]ne corpus document names a default at all", and the count was wrong**:
/// nine of the 974 do — eight a `/DefaultRGB` and `bug886717.pdf` a `/DefaultCMYK` — one of them
/// stating it inside an object stream, where a byte search over the file sees nothing. That does
/// not make the fixture unnecessary, because a default's effect on an image is only visible where
/// the image names a *device* space rather than its own, and this fixture is that shape by
/// construction. It makes the reason for the fixture an argument rather than a count. ADR 0405.
#[test]
fn a_default_colour_space_replaces_an_images_device_space() {
    let objects = "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
                   /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /ASCIIHexDecode \
                   /Length 9 >>\nstream\nFF000000>\nendstream\nendobj\n\
                   6 0 obj\n[/DeviceN [/C /M /Y /K] /DeviceRGB 7 0 R]\nendobj\n\
                   7 0 obj\n<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] /C0 [0 1 0] \
                   /C1 [0 1 0] /N 1 >>\nendobj\n";

    let with_default = centre_colour(pdf_with(
        objects,
        "/XObject << /Im0 5 0 R >> /ColorSpace << /DefaultCMYK 6 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_eq!(
        with_default,
        (0, 255, 0),
        "an image's DeviceCMYK must be remapped through /DefaultCMYK too"
    );

    let without = centre_colour(pdf_with(
        objects,
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_eq!(
        without,
        (0, 173, 239),
        "and without the entry it is the process cyan a fill gives"
    );
}

/// A content stream that cannot be decoded must be reported, not silently dropped.
///
/// This is the failure mode the project's third principle exists to prevent, in its purest
/// form: the page renders, it is simply not the page the document describes, and nothing
/// anywhere says so. Before this was reported, a page whose content stream used a filter
/// this reader did not implement drew nothing and returned `unsupported: []` —
/// indistinguishable from a page the producer meant to leave empty.
///
/// **The filter this test names had to change in the twenty-seventh session, and the reason
/// is worth keeping.** It was `/LZWDecode`, chosen deliberately real rather than invented,
/// with a note saying that implementing the filter should make the test fail and that would
/// be the moment to revisit it. That moment came: with `LZWDecode` written, **there is no
/// standard filter left that this reader does not implement**, so no name can stand in for
/// "a filter we do not have". What is left is a filter that is real, is implemented, and is
/// *not a content stream codec*: `/JPXDecode` produces an image raster, and `filter.rs`
/// deliberately answers `None` for the image codecs so that a stream expecting bytes is
/// visibly unsupported rather than silently empty.
#[test]
fn a_content_stream_that_will_not_decode_is_reported() {
    let bytes = pdf_with_content_filter("/JPXDecode");
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("JPXDecode"),
        "an undecodable content stream must name its filter: {reported}"
    );
}

/// A one-page PDF whose content stream declares a filter it does not honour.
fn pdf_with_content_filter(filter: &str) -> Vec<u8> {
    let objects = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_owned(),
        "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n".to_owned(),
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R >>\nendobj\n"
            .to_owned(),
        format!("4 0 obj\n<< /Length 8 /Filter {filter} >>\nstream\nnot data\nendstream\nendobj\n"),
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
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

/// A `Separation` space naming `colourant`, whose tint transform paints pure red.
///
/// Red is chosen because it is what the transform would produce if it were consulted, and
/// neither `/All` nor `/None` may produce it: §8.6.6.4 says a processor "shall ignore the
/// alternateSpace and tintTransform parameters" for both names. A transform that agreed with
/// the right answer would leave the test unable to tell whether it had been ignored.
fn special_separation(colourant: &str) -> String {
    format!(
        "5 0 obj\n[/Separation /{colourant} /DeviceRGB 6 0 R]\nendobj\n\
         6 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >>\nendobj\n"
    )
}

/// The `/None` colourant paints nothing at all.
///
/// ISO 32000-2 §8.6.6.4: "The special colourant name None shall not produce any visible
/// output. Painting operations in a Separation space with this colourant name shall have no
/// effect on the current page." The fixture paints a green background and then covers it
/// entirely with a `/None` fill; the background is what must survive. Before the nineteenth
/// session the colourant name was not read at all, so the tint transform ran and the page
/// came out red.
#[test]
fn the_none_colourant_marks_nothing() {
    let colour = centre_colour(pdf_with(
        &special_separation("None"),
        "/ColorSpace << /Sep 5 0 R >>",
        "0 1 0 rg 0 0 20 20 re f /Sep cs 1 scn 0 0 20 20 re f",
    ));
    assert_eq!(colour, (0, 255, 0), "a /None fill covered the background");
}

/// A `DeviceN` space whose colourants are all `/None` paints nothing either.
///
/// §8.6.6.5 requires a `DeviceN` space "whose component colourant names are all None" to
/// "always discard its output, just the same as a Separation colour space for None; it shall
/// never revert to the alternate colour space". The two-component space is deliberate — the
/// repetition of `None` is the one case that clause's names array permits.
#[test]
fn a_devicen_of_only_none_colourants_marks_nothing() {
    let space = "5 0 obj\n[/DeviceN [/None /None] /DeviceRGB 6 0 R]\nendobj\n\
                 6 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >>\n\
                 endobj\n";
    let colour = centre_colour(pdf_with(
        space,
        "/ColorSpace << /Sep 5 0 R >>",
        "0 1 0 rg 0 0 20 20 re f /Sep cs 1 1 scn 0 0 20 20 re f",
    ));
    assert_eq!(
        colour,
        (0, 255, 0),
        "an all-/None fill covered the background"
    );
}

/// A `DeviceN` reverting to its alternate passes every component, `/None` included.
///
/// ISO 32000-2 §8.6.6.5 states the two halves as a pair, and only the second can be reached on
/// a screen:
///
/// > When a DeviceN colour space is painting the named device colourants directly, colour
/// > components corresponding to None colourants shall be discarded. However, when the
/// > DeviceN colour space reverts to its alternate colour space, those components shall be
/// > passed to the tint transformation function, which may use them as desired.
///
/// This device has none of the named colourants, so it always reverts; discarding a `/None`
/// component here would be the first sentence applied where the clause states the second.
/// The tint transform is `red = first, green = second, blue = 0`, so the `/None` component
/// carries the whole of the green channel and a discarded one is black rather than a shade.
///
/// The second assertion is the other sentence of the same clause — "[o]perand values supplied
/// to SCN or scn shall be interpreted as colour component values in the order in which the
/// colours are given in the names array" — which the same transform pins by being asymmetric
/// in its two inputs.
#[test]
fn a_devicen_passes_its_none_components_to_the_tint_transform() {
    // The two inputs are already on the stack when the program starts (§7.10.5), so pushing
    // one zero leaves exactly the three values the `/Range` asks for. A two-input function has
    // to be a stream: Table 38's exponential and stitching types take one input each.
    let space = "5 0 obj\n[/DeviceN [/Spot /None] /DeviceRGB 6 0 R]\nendobj\n\
                 6 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] \
                 /Length 5 >>\nstream\n{ 0 }\nendstream\nendobj\n";
    let draw = |tints: &str| {
        centre_colour(pdf_with(
            space,
            "/ColorSpace << /Sep 5 0 R >>",
            &format!("/Sep cs {tints} scn 0 0 20 20 re f"),
        ))
    };
    assert_eq!(
        draw("0 1"),
        (0, 255, 0),
        "the /None component reached the tint transform"
    );
    assert_eq!(
        draw("1 0"),
        (255, 0, 0),
        "the operands are in the order the names array gives"
    );
}

/// The `/All` colourant is the tint complemented, applied to every colourant.
///
/// §8.6.6.4: "When outputting to an additive device, such as a computer monitor, the
/// subtractive tint values of the All colourant shall be complemented by subtracting from 1
/// before applying to all available colourants." A tint of 0.25 is therefore 0.75 in each of
/// this device's three, which is 191 of 255. Two further tints pin the direction: full ink is
/// black and none is white, which is the opposite of what a `DeviceGray` operand would mean
/// and the reason the clause states the complement at all.
#[test]
fn the_all_colourant_is_a_complemented_tint() {
    let draw = |tint: &str| {
        centre_colour(pdf_with(
            &special_separation("All"),
            "/ColorSpace << /Sep 5 0 R >>",
            &format!("/Sep cs {tint} scn 0 0 20 20 re f"),
        ))
    };
    assert_eq!(draw("0.25"), (191, 191, 191));
    assert_eq!(draw("1"), (0, 0, 0), "full ink in every colourant is black");
    assert_eq!(draw("0"), (255, 255, 255), "no ink at all is white");
}

/// `/All` and `/None` survive a tint transform that cannot be read.
///
/// §8.6.6.4 requires the alternate space and the tint transform to be ignored for these two
/// names "although valid values shall still be provided" — so a file that fails to provide
/// them has still named a colourant this processor is required to support "on all devices,
/// even if the devices are not capable of supporting any others". The names are therefore
/// decided before either parameter is parsed; reading the transform first would have refused
/// the space and reported it.
#[test]
fn a_special_colourant_does_not_need_a_readable_tint_transform() {
    let space = "5 0 obj\n[/Separation /All /Bogus null]\nendobj\n";
    let colour = centre_colour(pdf_with(
        space,
        "/ColorSpace << /Sep 5 0 R >>",
        "/Sep cs 0.25 scn 0 0 20 20 re f",
    ));
    assert_eq!(colour, (191, 191, 191));
}

/// Overprinting changes no pixel on a device with three process colourants and no spot ones.
///
/// This is a *derivation*, not an omission, and the test is here to keep it honest — see ADR
/// 0028 and the ledger's §8.6.7 and §11.7.4 rows. Table 146's blend function is the source
/// colour `Cs` for every row this device can reach: its group colour space has three process
/// components and no spot colourants, so every "spot colourant" row has no component to
/// affect, and the one row whose `OPM 1` cell differs requires the *group* space to be
/// `DeviceCMYK` (§11.7.4.3), which §11.6.6 reports as a departure when a document asks for it.
/// `Cs` is the Normal blend function, which is what these pixels composite through.
///
/// The fixture is the case overprinting is written for — `DeviceCMYK` with a zero component,
/// under `/OP true /OPM 1`, painted over a backdrop that component would otherwise erase. If
/// a later session implements the special blend mode without reading those rows, this is what
/// fails.
#[test]
fn overprinting_changes_nothing_on_a_three_component_device() {
    let content = |gs: &str| format!("0 0 1 1 k 0 0 20 20 re f {gs} 0 0.9 0.9 0 k 0 0 20 20 re f");
    let plain = centre_colour(pdf_with("", "", &content("")));
    let overprinting = centre_colour(pdf_with(
        "",
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>",
        &content("/GS gs"),
    ));
    assert_eq!(
        plain, overprinting,
        "overprinting must not change a page composited in three additive components"
    );
}

/// `/SA` reaches the stroke, and only the strokes drawn while it is in force.
///
/// ISO 32000-2 §10.7.5 by way of Table 57's `/SA`. What the parameter *does* is
/// `Stroke::device_width`'s and is tested in pixels by `render-cpu/tests/stroke_width.rs`;
/// what this pins is that the key is read at all and that `q`/`Q` restores it, which is the
/// half a graphics state parameter carried on the stroke could get wrong.
#[test]
fn stroke_adjustment_is_read_and_restored() {
    let document = Document::open(pdf_with(
        "",
        "/ExtGState << /GS << /SA true >> >>",
        "0 0 m 10 10 l S q /GS gs 0 0 m 10 10 l S Q 0 0 m 10 10 l S",
    ))
    .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let adjusted: Vec<bool> = pdf_model::interpret(&document, &page)
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            pdf_render::Command::Stroke { stroke, .. } => Some(stroke.adjust),
            _ => None,
        })
        .collect();
    assert_eq!(adjusted, vec![false, true, false]);
}

/// ISO 32000-2 §10.7.2: flatness is a tolerance a processor is permitted to ignore.
///
/// > PDF processors may choose to ignore any flatness tolerance specified within a PDF file.
///
/// The permission is taken, and taking it is a decision like any other: a page drawn with `i`
/// and with an `/ExtGState` `/FL` must be the page drawn without them, to the pixel. A reader
/// that half-honoured the value — flattening curves more coarsely under a large tolerance —
/// would make a curve worse for no gain, which is what the clause's NOTE 2 warns of by calling
/// a large value's result "unpredictable".
///
/// Written against a *curve*, because that is the only geometry flatness can reach: a page of
/// straight lines would pass under any reading at all. And compared over the whole raster
/// rather than at one point, because a coarser flattening moves a curve's *edge* and leaves
/// its interior exactly where it was.
#[test]
fn flatness_changes_nothing_because_the_clause_permits_ignoring_it() {
    let curve = "0 0 1 rg 2 2 m 2 18 18 18 18 2 c f 0 0 0 RG 5 w 1 10 m 10 19 19 10 c S";
    let plain = raster_of(pdf_with("", "", curve));
    let tolerant = raster_of(pdf_with(
        "",
        "/ExtGState << /GS << /FL 100 >> >>",
        &format!("100 i /GS gs {curve}"),
    ));
    assert_eq!(
        plain, tolerant,
        "a flatness tolerance, by either of the clause's two routes, must change no pixel"
    );
}
