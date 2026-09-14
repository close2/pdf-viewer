//! ISO 32000-2 §8.6.5.8's rendering intent, and the black point §8.6.5.9 makes it decide.
//!
//! Two clauses meet here and this file is one test per sentence of the meeting. §8.6.5.8 gives an
//! object three routes to an intent — the `ri` operator, an `/ExtGState`'s `/RI`, and §8.9.5.1
//! Table 87's `/Intent` on an image dictionary — and §8.6.5.9 states what one of the four names
//! then does:
//!
//! > If the current render intent of an object is AbsColorimetric then the value of
//! > UseBlackPtComp shall be treated as OFF .
//!
//! **The subject of that sentence is an object rather than a parameter**, and until the
//! six-hundred-and-seventh session this tree read it as a parameter: the intent and
//! `/UseBlackPtComp` shared one field of the graphics state, so whichever operator ran last won.
//! Two orderings came out wrong and each has a test below. The third route came out wrong in a
//! different way — an image's `/Intent` was read by nobody, and the state's intent reached no
//! image sample, shading ramp or mesh vertex either, because `crate::image`, `crate::shading` and
//! `crate::mesh` each passed a literal "compensate" to the conversion.
//!
//! Every fixture is an `ICCBased` space, because that is the only family black point compensation
//! moves: `ColourSpace::to_rgb_at` applies it in its `Icc` arm and nowhere else. The profile is
//! [`dark_black_profile`], whose darkest colour is a tenth of its white point — so compensation
//! brings full ink to the display's black, and turning it off leaves it the grey the profile
//! describes. PDF 2.0 Application Note 001 is what defines the first of those.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, the \
              fixtures are small enough that no index can overflow, and the ICC fixture's \
              constants are written as the fixed-point values it encodes"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

/// An ICC profile whose darkest colour is well above the display's black.
///
/// One input channel, three XYZ outputs, two grid points: grid point 0 is the white point and
/// grid point 1 — full ink — is a tenth of it. So a sample of 1.0 converts to that dark grey,
/// and compensation is what pulls it the rest of the way to zero.
///
/// The encoding is ICC's `lut16Type`, assembled by hand for the same reason
/// `tests/colour_paths.rs` assembles one: a fixture whose answer no fallback table can produce
/// is a fixture a passing assertion cannot reach by coincidence.
fn dark_black_profile() -> Vec<u8> {
    // D50's white point and a tenth of it, in the `u1Fixed15` encoding a lookup table uses.
    let white: [u16; 3] = [
        (0.964_2 * 32768.0) as u16,
        32768,
        (0.824_9 * 32768.0) as u16,
    ];
    let dark: [u16; 3] = [white[0] / 10, white[1] / 10, white[2] / 10];

    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[1, 3, 2, 0]); // one in, three out, two grid points
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
    tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
    for value in [0u16, 0xFFFF] {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for value in white.iter().chain(dark.iter()) {
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

/// The paper a press profile states as its medium: a warm white, as D50 XYZ.
const PAPER: [f32; 3] = [0.85, 0.88, 0.65];

/// A one-component press profile whose unprinted medium is `white` and whose `wtpt`, where
/// `medium` is given, states that medium.
///
/// The same two-point table as [`dark_black_profile`] — no ink at grid point 0, a tenth of it
/// at grid point 1 — over whichever white the caller states, and a `prtr` class so that the
/// media white point is the tag's rather than the value ICC.1:2022 clause 9.2.36 fixes for a
/// display. Two profiles built by this are the whole of the absolute-intent test: one states
/// D50 in its table and the paper in its `wtpt`, the other the paper in its table and no `wtpt`,
/// and Equations (4) to (6) of that standard's clause 6.3.2.2 say the first under
/// `AbsoluteColorimetric` is the second under `RelativeColorimetric`.
fn paper_profile(white: [f32; 3], medium: Option<[f32; 3]>) -> Vec<u8> {
    let fixed = |value: f32| ((value * 32768.0) as u16).to_be_bytes();

    let mut table = Vec::new();
    table.extend_from_slice(b"mft2");
    table.extend_from_slice(&[0; 4]);
    table.extend_from_slice(&[1, 3, 2, 0]); // one in, three out, two grid points
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        table.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    table.extend_from_slice(&2u16.to_be_bytes()); // input table entries
    table.extend_from_slice(&2u16.to_be_bytes()); // output table entries
    for value in [0u16, 0xFFFF] {
        table.extend_from_slice(&value.to_be_bytes());
    }
    for point in [white, [white[0] / 10.0, white[1] / 10.0, white[2] / 10.0]] {
        for value in point {
            table.extend_from_slice(&fixed(value));
        }
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            table.extend_from_slice(&value.to_be_bytes());
        }
    }

    let mut tags: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"A2B1", table)];
    if let Some(medium) = medium {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"XYZ ");
        tag.extend_from_slice(&[0; 4]);
        for value in medium {
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tags.push((b"wtpt", tag));
    }

    let mut out = vec![0u8; 128];
    out[8] = 2;
    out[12..16].copy_from_slice(b"prtr");
    out[16..20].copy_from_slice(b"GRAY");
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    out.extend_from_slice(&(tags.len() as u32).to_be_bytes());
    let mut offset = 128 + 4 + 12 * tags.len();
    let mut data = Vec::new();
    for (name, tag) in &tags {
        out.extend_from_slice(*name);
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
        offset += tag.len();
        data.extend_from_slice(tag);
    }
    out.extend_from_slice(&data);
    out
}

/// One `mft2` lookup table of [`three_table_profile`]'s three.
///
/// Three grid points on one input axis. The two ends are the same in all three tables — D50's
/// white at input 0.0 and a tenth of it at 1.0 — so every table describes the same device range
/// and `Profile::detect_black` finds the same black for each; what differs is `mid`, the
/// connection-space colour at input 0.5. That is the whole calibration: a test that probes the
/// mid-tone is reading which table was selected and nothing else, because compensation, the
/// white point and the darkest colour are identical whichever one it was.
fn mid_table(mid: [f32; 3]) -> Vec<u8> {
    let white = [0.964_2f32, 1.0, 0.824_9];
    let fixed = |value: f32| ((value * 32768.0) as u16).to_be_bytes();

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[1, 3, 3, 0]); // one in, three out, three grid points
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
    tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
    for value in [0u16, 0xFFFF] {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for point in [
        white,
        mid,
        [white[0] / 10.0, white[1] / 10.0, white[2] / 10.0],
    ] {
        for value in point {
            tag.extend_from_slice(&fixed(value));
        }
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }
    tag
}

/// An ICC profile that answers a mid-tone in three different colours, one per rendering intent.
///
/// ISO 15076-1:2010 gives `A2B0`, `A2B1` and `A2B2` the device-to-connection transform of one
/// rendering intent each (its tag listing, clause 9.2), and §8.6.5.8 says the names PDF uses
/// "have been chosen to correspond to those defined by the International Color Consortium
/// (ICC)". So this profile is three answers to one question, and which one comes back says which
/// intent was in force:
///
/// - `A2B0`, perceptual: a red mid-tone.
/// - `A2B1`, colorimetric: a neutral mid-tone — the one Table 51 makes the initial value.
/// - `A2B2`, saturation: a blue mid-tone.
///
/// No real profile is built this way; that is the point of a fixture (trap 8). A profile whose
/// tables agreed to within a few levels, as real ones tend to, could pass these tests with the
/// selection unimplemented.
fn three_table_profile() -> Vec<u8> {
    let tables: [(&[u8; 4], [f32; 3]); 3] = [
        (b"A2B0", [0.45, 0.25, 0.05]),
        (b"A2B1", [0.25, 0.25, 0.25]),
        (b"A2B2", [0.12, 0.25, 0.75]),
    ];

    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut out = header;
    out.extend_from_slice(&(tables.len() as u32).to_be_bytes());
    let mut offset = 128 + 4 + 12 * tables.len();
    let mut data = Vec::new();
    for (name, mid) in tables {
        let tag = mid_table(mid);
        out.extend_from_slice(name);
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
        offset += tag.len();
        data.extend_from_slice(&tag);
    }
    out.extend_from_slice(&data);
    out
}

/// A profile as a one-component `ICCBased` stream, as object `number`.
///
/// The number is a parameter because [`pdf_with`] builds its cross-reference table by counting
/// the objects it is given: a fixture that states object six and no object five would have every
/// offset after the fourth pointing at the wrong bytes.
fn icc_object(number: u32, profile: &[u8]) -> String {
    let mut hex = String::new();
    for byte in profile {
        let _ = write!(hex, "{byte:02X}");
    }
    format!(
        "{number} 0 obj\n<< /N 1 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\n\
         endstream\nendobj\n",
        hex.len().saturating_add(1)
    )
}

/// [`dark_black_profile`] as a one-component `ICCBased` stream, as object `number`.
fn profile_object(number: u32) -> String {
    icc_object(number, &dark_black_profile())
}

/// [`three_table_profile`] as a one-component `ICCBased` stream, as object `number`.
fn table_object(number: u32) -> String {
    icc_object(number, &three_table_profile())
}

/// Object five: a one-pixel image of full ink in that space, with whatever `extra` states.
fn ink_image(extra: &str) -> String {
    format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 {extra} /Filter /ASCIIHexDecode \
         /Length 3 >>\nstream\nFF>\nendstream\nendobj\n{}",
        profile_object(6)
    )
}

/// Builds a one-page PDF from a content stream and extra objects numbered from five.
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

/// Asserts that the profile's darkest colour reached the display's black.
fn assert_compensated(colour: (u8, u8, u8), what: &str) {
    let (r, g, b) = colour;
    assert!(
        r <= 1 && g <= 1 && b <= 1,
        "{what}: black point compensation should have brought full ink to black, got {colour:?}"
    );
}

/// Asserts that it stayed the grey the profile describes.
fn assert_uncompensated(colour: (u8, u8, u8), what: &str) {
    let (r, g, b) = colour;
    assert!(
        r > 60 && g > 60 && b > 60,
        "{what}: black point compensation should have been off, got {colour:?}"
    );
}

/// The fixture is only worth anything if the two answers differ, so this pins that first.
#[test]
fn the_profile_distinguishes_compensating_from_not() {
    let plain = centre_colour(pdf_with(
        &ink_image(""),
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_compensated(plain, "an image under the initial graphics state");
}

/// **§8.9.5.1 Table 87**: an image states its own rendering intent, and it is obeyed.
///
/// > The name of a colour rendering intent that shall be used in rendering any image that is not
/// > an image mask (see 8.6.5.8, "Rendering intents"). This value is ignored if ImageMask is true
/// > . Default value: the current rendering intent in the graphics state.
///
/// The entry was read by nothing in this tree until the six-hundred-and-seventh session, and
/// neither was the graphics state's intent on this route: `crate::image` converted every sample
/// with compensation on whatever any of §8.6.5.8's three routes said.
#[test]
fn an_images_own_intent_turns_black_point_compensation_off() {
    let absolute = centre_colour(pdf_with(
        &ink_image("/Intent /AbsoluteColorimetric"),
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_uncompensated(absolute, "an image stating /Intent /AbsoluteColorimetric");
}

/// A name Table 69 does not list leaves the image where §8.6.5.8's default sentence puts it.
///
/// > If a PDF processor does not recognise the specified name, it shall use the
/// > RelativeColorimetric intent by default.
///
/// Which is not absolute, so compensation stays on. `AbsColorimetric` is the interesting case
/// rather than an invented one: §8.6.5.9 spells the intent that way in its own prose while
/// Table 69 defines `AbsoluteColorimetric`, and a file states what Table 69 defines.
#[test]
fn an_unrecognised_intent_name_is_relative_colorimetric() {
    let short = centre_colour(pdf_with(
        &ink_image("/Intent /AbsColorimetric"),
        "/XObject << /Im0 5 0 R >>",
        "q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_compensated(short, "an image stating a name Table 69 does not list");
}

/// The state's own intent reaches an image that states none — Table 87's default value.
#[test]
fn an_image_without_an_intent_takes_the_graphics_states() {
    let inherited = centre_colour(pdf_with(
        &ink_image(""),
        "/XObject << /Im0 5 0 R >>",
        "/AbsoluteColorimetric ri q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_uncompensated(inherited, "an image under `ri /AbsoluteColorimetric`");
}

/// **§8.6.5.9, first ordering**: a later intent must not switch compensation back on.
///
/// The clause makes an absolute intent force the entry off. It says nothing whatever about any
/// other intent, so `/UseBlackPtComp OFF` stands: a `ri` naming `Perceptual` sets a parameter
/// this device does not otherwise act on, and must leave the document's own answer alone.
#[test]
fn a_later_intent_does_not_restore_a_black_point_the_state_turned_off() {
    let kept = centre_colour(pdf_with(
        &profile_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
         /ExtGState << /GS0 << /UseBlackPtComp /OFF >> >>",
        "/GS0 gs /Perceptual ri /CS0 cs 1 scn 0 0 20 20 re f",
    ));
    assert_uncompensated(kept, "`/UseBlackPtComp /OFF` followed by `ri /Perceptual`");
}

/// **§8.6.5.9, second ordering**: an absolute intent still in force overrides a later `ON`.
///
/// > If the current render intent of an object is AbsColorimetric then the value of
/// > UseBlackPtComp shall be treated as OFF .
///
/// The intent of the object being painted is absolute here, whichever operator ran last, so the
/// entry is treated as `OFF` however loudly it says `ON`.
#[test]
fn an_absolute_intent_overrides_a_black_point_entry_set_after_it() {
    let overridden = centre_colour(pdf_with(
        &profile_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
         /ExtGState << /GS0 << /UseBlackPtComp /ON >> >>",
        "/AbsoluteColorimetric ri /GS0 gs /CS0 cs 1 scn 0 0 20 20 re f",
    ));
    assert_uncompensated(
        overridden,
        "`ri /AbsoluteColorimetric` followed by `/UseBlackPtComp /ON`",
    );
}

/// **§8.4.5 Table 57's `/RI`**: the second route sets the same parameter as the first.
///
/// And it now *replaces* an absolute intent rather than only ever adding one, which is what an
/// intent kept as a parameter rather than as a decision buys: the state below ends
/// `RelativeColorimetric`, so the `ON` two operators earlier is the answer.
#[test]
fn an_ext_gstate_intent_replaces_the_one_in_force() {
    let replaced = centre_colour(pdf_with(
        &profile_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
         /ExtGState << /GS0 << /UseBlackPtComp /ON /RI /RelativeColorimetric >> >>",
        "/AbsoluteColorimetric ri /GS0 gs /CS0 cs 1 scn 0 0 20 20 re f",
    ));
    assert_compensated(
        replaced,
        "an /RI naming a relative intent after an absolute ri",
    );
}

/// A shading's ramp is converted under the same parameters as a path's colour (§11.7.5.3).
///
/// > The rendering intent, black-generation, undercolour-removal and black point compensation
/// > parameters control certain colour conversions. In the presence of transparency, they may
/// > need to be applied earlier than the actual rendering of colour onto the page.
///
/// A shading is exactly that "earlier": `crate::shading` samples the colour function into a ramp
/// at build time, so the parameters have to travel with the build rather than be read at the
/// paint. They did not until the six-hundred-and-seventh session.
#[test]
fn a_shadings_ramp_honours_the_rendering_intent() {
    let shading = "/Shading << /Sh0 << /ShadingType 2 /ColorSpace [/ICCBased 5 0 R] \
                   /Coords [0 0 20 0] /Extend [true true] \
                   /Function << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [1] /N 1 >> >> >>";

    let compensated = centre_colour(pdf_with(&profile_object(5), shading, "/Sh0 sh"));
    assert_compensated(compensated, "a shading under the initial graphics state");

    let absolute = centre_colour(pdf_with(
        &profile_object(5),
        shading,
        "/AbsoluteColorimetric ri /Sh0 sh",
    ));
    assert_uncompensated(absolute, "a shading under `ri /AbsoluteColorimetric`");
}

/// A shading **pattern** whose colours the same page's `ri` cannot reach (§11.6.7).
///
/// The test above is `sh`, which paints where it stands: §11.7.5.3 puts an elementary object's
/// intent at "the time of the painting operation", so the `ri` before it is the answer. A pattern
/// is not that. §11.6.7 makes its definition an implicitly enclosed group and says which state
/// that group is evaluated under:
///
/// > The definition shall not inherit the current values of the graphics state parameters at the
/// > time it is evaluated; those parameters shall take effect only when the resulting pattern is
/// > later used to paint an object.
///
/// > Any parameters that are not so specified shall be inherited from the graphics state that was
/// > in effect at the beginning of the content stream in which the shading pattern is set to be
/// > the current colour in the graphics state or in which the sh operator is used.
///
/// So the `ri` below reaches the *fill* and not the pattern, and the ramp compensates although the
/// intent in force at both the `scn` and the `f` says it shall not. **Both of the obvious answers
/// are wrong here** — resolving at the `scn` and resolving at the mark give the same uncompensated
/// grey, and the clause gives neither.
#[test]
fn a_shading_patterns_colours_are_resolved_where_its_content_stream_began() {
    let pattern = "/Pattern << /P0 << /PatternType 2 /Shading \
                   << /ShadingType 2 /ColorSpace [/ICCBased 5 0 R] /Coords [0 0 20 0] \
                   /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] /C0 [1] \
                   /C1 [1] /N 1 >> >> >> >>";

    let unreached = centre_colour(pdf_with(
        &profile_object(5),
        pattern,
        "/AbsoluteColorimetric ri /Pattern cs /P0 scn 0 0 20 20 re f",
    ));
    assert_compensated(
        unreached,
        "a pattern selected after an `ri` its own definition never sees",
    );

    // The mutation, and the only way a content stream can move the parameter for a pattern: a
    // *form* is a new content stream, so an `ri` before the `Do` is in effect at its beginning.
    let form = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] \
         /Resources << {pattern} >> /Length 33 >>\nstream\n\
         /Pattern cs /P0 scn 0 0 20 20 re f\nendstream\nendobj\n{}",
        profile_object(5)
    );
    let inherited = centre_colour(pdf_with(
        &form,
        "/XObject << /Fm0 6 0 R >>",
        "/AbsoluteColorimetric ri /Fm0 Do",
    ));
    assert_uncompensated(
        inherited,
        "a form whose content stream begins under an absolute intent",
    );
}

/// Table 75's `/ExtGState`, which §11.6.7 lets augment the state a pattern is evaluated under.
///
/// > In the case of a shading pattern, the parameter values may be augmented by the contents of
/// > the ExtGState entry in the pattern dictionary (see 8.7.4, "Shading patterns"). Only those
/// > parameters that affect the sh operator, such as the current transformation matrix, black
/// > point compensation and rendering intent, shall be used.
///
/// The entry was read by nothing in this tree until the six-hundred-and-fifty-fifth session, on a
/// ledger claim — "no corpus document writes one" — that had been measured over `doc/pdf.js` and
/// never over the crawl, where 42 documents do. Both of the two names the sentence gives are
/// tested, because they reach the same parameter by §8.6.5.9's override and a fixture proving one
/// proves nothing about the other.
#[test]
fn a_patterns_own_ext_gstate_augments_that_state() {
    let pattern = |extra: &str| {
        format!(
            "/Pattern << /P0 << /PatternType 2 {extra} /Shading \
             << /ShadingType 2 /ColorSpace [/ICCBased 5 0 R] /Coords [0 0 20 0] \
             /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [1] \
             /N 1 >> >> >> >>"
        )
    };
    let content = "/Pattern cs /P0 scn 0 0 20 20 re f";

    let plain = centre_colour(pdf_with(&profile_object(5), &pattern(""), content));
    assert_compensated(plain, "a pattern stating no /ExtGState at all");

    let off = centre_colour(pdf_with(
        &profile_object(5),
        &pattern("/ExtGState << /UseBlackPtComp /OFF >>"),
        content,
    ));
    assert_uncompensated(
        off,
        "a pattern whose /ExtGState states /UseBlackPtComp /OFF",
    );

    let absolute = centre_colour(pdf_with(
        &profile_object(5),
        &pattern("/ExtGState << /RI /AbsoluteColorimetric >>"),
        content,
    ));
    assert_uncompensated(
        absolute,
        "a pattern whose /ExtGState states an absolute intent",
    );
}

/// The rebuild §10.5 needs at the mark does **not** take §11.6.7's parameters with it.
///
/// A shading pattern's colours are built again where it is painted, because §11.7.5.2 puts the
/// transfer function at the topmost object enclosing a point and §11.7.2 puts the compositing
/// space at the mark's group. The trap the six-hundred-and-fifty-fifth session left written down
/// is that such a rebuild trades one departure for another if it reads the *state* for the black
/// point, the intent and the smoothness — which §11.6.7 has already fixed at the beginning of the
/// content stream, augmented by Table 75's `/ExtGState`.
///
/// So the fixture forces the rebuild without changing a colour by any other route: the `gs` at
/// the mark states a transfer function that is §7.10.3's identity written as a function, which
/// Table 57 makes a stated function rather than the `/Identity` name that clears one. The
/// pattern's own `/ExtGState` says `/UseBlackPtComp /OFF`; the state at the mark says nothing and
/// therefore compensates. A rebuild reading the state would compensate and this would be black.
#[test]
fn a_rebuilt_patterns_black_point_is_still_its_definitions() {
    let pattern = "/Pattern << /P0 << /PatternType 2 /ExtGState << /UseBlackPtComp /OFF >> \
                   /Shading << /ShadingType 2 /ColorSpace [/ICCBased 5 0 R] /Coords [0 0 20 0] \
                   /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] /C0 [1] \
                   /C1 [1] /N 1 >> >> >> >> \
                   /ExtGState << /GS << /TR << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] \
                   /N 1 >> >> >>";

    let rebuilt = centre_colour(pdf_with(
        &profile_object(5),
        pattern,
        "/Pattern cs /P0 scn /GS gs 0 0 20 20 re f",
    ));
    assert_uncompensated(
        rebuilt,
        "a pattern rebuilt at a mark that states a transfer function",
    );
}

// ---------------------------------------------------------------------------------------------
// Which of a profile's transforms the intent selects (ADR 1032).
//
// Everything above asks what an intent does to §8.6.5.9's black point, which was the only thing
// an intent could do in this tree until session 1014: `Perceptual` and `Saturation` were read,
// kept apart from `RelativeColorimetric`, carried the length of `crate::colour`, and acted on
// nothing. What makes them act is ISO 32000-2 §10.3.1 — "[c]onversion from a CIE-based source
// colour to a CIE-based destination colour shall be performed based on ISO 15076-1:2010
// (ICC.1:2010)" — read beside §8.6.5.8's own sentence about where the four names came from.
// Every test below fails against the code before that change, which drew all four names through
// `A2B1`; `an_intent_with_no_table_of_its_own_falls_back` and the absolute one are the two that
// pin what did **not** move.

/// Whichever channel dominates the colour at the centre: which of the three tables answered.
///
/// Twenty levels apart is the band, and the fixture's three tables are far wider than that —
/// the probe below prints what each one actually reads, so a band that stopped separating them
/// would say so rather than pass.
fn dominant(colour: (u8, u8, u8), what: &str) -> &'static str {
    let (red, _, blue) = colour;
    let answer = if u16::from(red).abs_diff(u16::from(blue)) <= 20 {
        "neutral"
    } else if red > blue {
        "red"
    } else {
        "blue"
    };
    println!("{what}: {colour:?} reads as {answer}");
    answer
}

/// The fixture is only worth anything if its three tables are three colours.
///
/// Trap 13: this is the probe that says the assertions below can fail. It also pins the default
/// — an object that states no intent takes Table 51's `RelativeColorimetric`, which is `A2B1`,
/// which is the neutral table.
#[test]
fn the_three_tables_answer_a_mid_tone_in_three_colours() {
    let fill = |content: &str| {
        centre_colour(pdf_with(
            &table_object(5),
            "/ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
             /ExtGState << /GP << /RI /Perceptual >> /GS << /RI /Saturation >> >>",
            content,
        ))
    };

    assert_eq!(
        dominant(fill("/CS0 cs 0.5 scn 0 0 20 20 re f"), "no intent stated"),
        "neutral",
        "Table 51's initial intent is RelativeColorimetric, which is the A2B1 table"
    );
    assert_eq!(
        dominant(fill("/GP gs /CS0 cs 0.5 scn 0 0 20 20 re f"), "perceptual"),
        "red",
        "the fixture's A2B0 has to be distinguishable from its A2B1"
    );
    assert_eq!(
        dominant(fill("/GS gs /CS0 cs 0.5 scn 0 0 20 20 re f"), "saturation"),
        "blue",
        "and so does its A2B2"
    );
}

/// **§8.6.5.8, the `ri` operator**: a perceptual intent selects the profile's perceptual
/// transform.
///
/// > Rendering intents shall be specified with the ri operator (see 8.4.4, "Graphics state
/// > operators"), the RI entry in a graphics state parameter dictionary (see 8.4.5, "Graphics
/// > state parameter dictionaries"), or with the Intent entry in image dictionaries (see 8.9.5,
/// > "Image dictionaries").
///
/// The first of the three routes, and the clause's Table 69 says what the name asks for:
/// "[c]olours shall be represented in a manner that provides a pleasing perceptual appearance".
/// What decides what that is, for a colour in an `ICCBased` space, is the profile's own `A2B0` —
/// §10.3.1 hands the conversion to ISO 15076-1 and that standard tabulates one transform per
/// intent.
#[test]
fn a_perceptual_intent_selects_the_profiles_perceptual_transform() {
    let perceptual = centre_colour(pdf_with(
        &table_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >>",
        "/Perceptual ri /CS0 cs 0.5 scn 0 0 20 20 re f",
    ));
    assert_eq!(
        dominant(perceptual, "`ri /Perceptual` on a fill"),
        "red",
        "a perceptual intent must go through the profile's A2B0"
    );
}

/// **§8.6.5.8, Table 57's `/RI`**: the second route selects the transform as well.
///
/// Table 69: "[c]olours shall be represented in a manner that preserves or emphasizes
/// saturation" — the profile's `A2B2`, which nothing in this tree read before session 1014.
#[test]
fn a_saturation_intent_selects_the_profiles_saturation_transform() {
    let saturation = centre_colour(pdf_with(
        &table_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
         /ExtGState << /GS0 << /RI /Saturation >> >>",
        "/GS0 gs /CS0 cs 0.5 scn 0 0 20 20 re f",
    ));
    assert_eq!(
        dominant(saturation, "an /ExtGState stating /RI /Saturation"),
        "blue",
        "a saturation intent must go through the profile's A2B2"
    );
}

/// **§8.9.5.1 Table 87**: an image's own `/Intent` selects the transform its samples go through.
///
/// > The name of a colour rendering intent that shall be used in rendering any image that is not
/// > an image mask (see 8.6.5.8, "Rendering intents"). This value is ignored if ImageMask is true
/// > . Default value: the current rendering intent in the graphics state.
///
/// The third route, and the one this file already tests for the black point. Both halves of the
/// entry now reach a sample: `Interpreter::image_conversion` reads it, and the `Conversion` it
/// builds carries the transform as well as the compensation into `crate::image`.
///
/// The page states `ri /Saturation` before the `Do`, so a reader that ignored Table 87 would draw
/// the blue table and one that ignored the intent altogether the neutral one. Three answers, one
/// assertion.
#[test]
fn an_images_own_intent_selects_the_transform_its_samples_take() {
    let image = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Intent /Perceptual \
         /Filter /ASCIIHexDecode /Length 3 >>\nstream\n80>\nendstream\nendobj\n{}",
        table_object(6)
    );
    let drawn = centre_colour(pdf_with(
        &image,
        "/XObject << /Im0 5 0 R >>",
        "/Saturation ri q 20 0 0 20 0 0 cm /Im0 Do Q",
    ));
    assert_eq!(
        dominant(
            drawn,
            "an image stating /Intent /Perceptual under `ri /Saturation`"
        ),
        "red",
        "Table 87's entry overrides the state's intent for the image's own samples"
    );
}

/// An intent the profile states no table for falls back to the colorimetric one.
///
/// [`dark_black_profile`] carries an `A2B1` and nothing else, which is the shape ISO 15076-1:2010
/// requires of an output profile (its clause 8.5) and all that most profiles carry. §8.6.5.8
/// disposes of an intent a *processor* cannot honour by sending it to `RelativeColorimetric`, and
/// a profile that tabulates no transform for the intent asked leaves a processor in that same
/// position, so the fallback is the table the file does have.
///
/// This one passes before the change as well as after, and says so: it is what keeps the
/// selection from turning a profile with one table into a blank or a panic.
#[test]
fn an_intent_with_no_table_of_its_own_falls_back() {
    let fallback = centre_colour(pdf_with(
        &profile_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >>",
        "/Perceptual ri /CS0 cs 1 scn 0 0 20 20 re f",
    ));
    assert_compensated(
        fallback,
        "a perceptual intent over a profile with only an A2B1",
    );
}

/// The two colorimetric intents share a table.
///
/// ISO 15076-1:2010 tabulates three intents and derives the absolute colorimetric one from the
/// media-relative colorimetric transform (its clause 6.2), so there is no `A2B3` to select and
/// `AbsoluteColorimetric` takes `A2B1` — while §8.6.5.9 still makes it turn compensation off.
/// Both halves in one fixture: the colour is the neutral table's, uncompensated. This profile
/// states no `wtpt`, so the derivation has nothing to scale by and the table is the whole
/// answer; [`an_absolute_intent_draws_unprinted_paper_as_the_paper`] is the profile that does.
#[test]
fn an_absolute_intent_takes_the_colorimetric_transform_without_compensation() {
    let absolute = centre_colour(pdf_with(
        &table_object(5),
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >>",
        "/AbsoluteColorimetric ri /CS0 cs 0.5 scn 0 0 20 20 re f",
    ));
    assert_eq!(
        dominant(absolute, "`ri /AbsoluteColorimetric` on a fill"),
        "neutral",
        "an absolute intent is the colorimetric table, not a fourth one"
    );
    assert_uncompensated(absolute, "`ri /AbsoluteColorimetric` on a fill");
}

/// Table 69's other half of `AbsoluteColorimetric`: "no correction shall be made for the output
/// medium's white point (such as the colour of unprinted paper)". Unprinted paper in a profile
/// whose `wtpt` states a warm white draws as that warm white under the absolute intent, and as
/// the display's white under the relative one — which is the same table's own second row, a
/// medium's white "reproduced on a printer by simply leaving the paper unmarked".
///
/// The expected pixel is derived rather than read off this tree's raster: ICC.1:2022 clause
/// 6.3.2.2's Equations (4) to (6) scale the media-relative XYZ by the media white point over
/// the connection space's, so a D50-white table under the absolute intent *is* a paper-white
/// table under the relative intent, and the two fixtures are compared pixel for pixel.
/// Calibrated by taking the scaling out of `Profile::to_xyz_with`, which draws the first
/// fixture as the display's white and fails both assertions.
#[test]
fn an_absolute_intent_draws_unprinted_paper_as_the_paper() {
    let d50 = [0.964_2f32, 1.0, 0.824_9];
    // Compensation off throughout, so that the relative fixture is the table's own white and
    // not §8.6.5.9's stretch of it — the absolute intent turns it off by itself, and the
    // comparison is between the two intents' white points and nothing else.
    let no_ink = |profile: &[u8], before: &str| {
        centre_colour(pdf_with(
            &icc_object(5, profile),
            "/ColorSpace << /CS0 [/ICCBased 5 0 R] >> \
             /ExtGState << /GS0 << /UseBlackPtComp /OFF >> >>",
            &format!("/GS0 gs {before} /CS0 cs 0 scn 0 0 20 20 re f"),
        ))
    };
    let relative = no_ink(&paper_profile(d50, Some(PAPER)), "");
    assert!(
        relative.0 >= 254 && relative.1 >= 254 && relative.2 >= 254,
        "relative: unprinted paper is the display's white, got {relative:?}"
    );

    let absolute = no_ink(&paper_profile(d50, Some(PAPER)), "/AbsoluteColorimetric ri");
    let paper = no_ink(&paper_profile(PAPER, None), "");
    assert!(
        paper.2 + 10 < paper.0 && paper.0 < 250,
        "the paper the second profile states is a warm white the display can show: {paper:?}"
    );
    for (axis, (got, want)) in [absolute.0, absolute.1, absolute.2]
        .into_iter()
        .zip([paper.0, paper.1, paper.2])
        .enumerate()
    {
        assert!(
            got.abs_diff(want) <= 1,
            "absolute: channel {axis} is {got} where the paper's own table gives {want} \
             ({absolute:?} against {paper:?})"
        );
    }
}
