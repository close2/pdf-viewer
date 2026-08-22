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

/// [`dark_black_profile`] as a one-component `ICCBased` stream, as object `number`.
///
/// The number is a parameter because [`pdf_with`] builds its cross-reference table by counting
/// the objects it is given: a fixture that states object six and no object five would have every
/// offset after the fourth pointing at the wrong bytes.
fn profile_object(number: u32) -> String {
    let mut hex = String::new();
    for byte in dark_black_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    format!(
        "{number} 0 obj\n<< /N 1 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\n\
         endstream\nendobj\n",
        hex.len().saturating_add(1)
    )
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
