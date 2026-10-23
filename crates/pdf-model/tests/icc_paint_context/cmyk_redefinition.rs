//! Independent controls for §11.7.2's redefinition of `DeviceCMYK` at the paint, the
//! obligation §11.7.5.3's first bullet puts on an inherited colour: a `DeviceCMYK` colour
//! that §14.11.5's output intent resolved outside an `ICCBased` CMYK group is, inside it, a
//! colour in the group's own space.
//!
//! ISO 32000-2 §11.7.2: "If an isolated transparency group or page has an ICCBased 'CMYK'
//! colour space , DeviceCMYK shall be redefined within the transparency group to be the same
//! as the blending colour space". The redefinition is a property of where a colour is
//! painted, so `0 1 0 1 k` selected on the page and painted inside such a group is the
//! group's `0 1 0 1`, and the expected pixel is the group profile's own corner: each fixture
//! profile is a two-point `mft2` table over a multiplicative press, so `0 1 0 1` is a grid
//! point and its connection-space value is the corner's encoded knot. §11.3.3 composites the
//! half-alpha paint over the isolated group's transparent backdrop and the page's
//! `DeviceRGB` group takes it over white. A colour selected cold inside the same group is the
//! same colour by the same sentence and is checked beside it, not as the oracle.
#![expect(
    clippy::doc_markdown,
    reason = "the module comment quotes §11.7.2 verbatim, and a quotation is not marked up"
)]
use super::{content_stream, decoded_xyz_to_srgb, pixel, render_bytes, stream};
use std::fmt::Write as _;

/// The colour every control paints: full magenta and full black, a grid point of every
/// fixture profile, and one whose black ink the presses below absorb differently.
const COLOUR: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

/// The output intent's press, the group's, and a nested group's: the same four inks with the
/// black absorbing 0.80 × (1 − weaker), so each is a different profile with a different
/// answer at [`COLOUR`].
const INTENT: f32 = 0.30;
const GROUP: f32 = 0.0;
const NESTED: f32 = 0.50;

fn press_xyz(weaker: f32, inks: [f32; 4]) -> [f32; 3] {
    let mut xyz = [0.964_2f32, 1.0, 0.824_9];
    let black = 0.80 * (1.0 - weaker);
    let absorb = [
        [0.60f32, 0.10, 0.10, black],
        [0.10, 0.60, 0.20, black],
        [0.10, 0.20, 0.70, black],
    ];
    for (axis, row) in absorb.iter().enumerate() {
        for (ink, factor) in inks.iter().zip(row) {
            xyz[axis] *= 1.0 - factor * ink;
        }
    }
    xyz
}

/// `u1Fixed15`, the encoding of XYZ in a lookup table: 0x8000 is 1.0.
fn encoded(value: f32) -> u16 {
    (value * 32768.0).clamp(0.0, 65535.0) as u16
}

/// A v2 CMYK output profile whose one tag is an `A2B1` `mft2` over [`press_xyz`]'s sixteen
/// corners, the last input varying fastest (ICC's order).
fn cmyk_profile(weaker: f32) -> Vec<u8> {
    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[4, 3, 2, 0]);
    for value in [1i32, 0, 0, 0, 1, 0, 0, 0, 1] {
        tag.extend_from_slice(&(value * 65536).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes());
    tag.extend_from_slice(&2u16.to_be_bytes());
    for _ in 0..4 {
        tag.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
    }
    for corner in 0..16usize {
        let at = |axis: usize| {
            if (corner >> (3 - axis)) & 1 == 1 {
                1.0
            } else {
                0.0
            }
        };
        for value in press_xyz(weaker, [at(0), at(1), at(2), at(3)]) {
            tag.extend_from_slice(&encoded(value).to_be_bytes());
        }
    }
    for _ in 0..3 {
        tag.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
    }
    let mut out = vec![0u8; 128];
    out[8] = 2;
    out[12..16].copy_from_slice(b"prtr");
    out[16..20].copy_from_slice(b"CMYK");
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&144u32.to_be_bytes());
    out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
    out.extend_from_slice(&tag);
    out
}

/// [`COLOUR`] through a press's encoded knot and the published D50-adapted sRGB matrix.
fn corner_srgb(weaker: f32) -> [f64; 3] {
    decoded_xyz_to_srgb(press_xyz(weaker, COLOUR).map(|v| f64::from(encoded(v)) / 32768.0))
}

fn opaque(srgb: [f64; 3]) -> [u8; 3] {
    srgb.map(|v| v.round() as u8)
}

/// A half-alpha paint over white, through tiny-skia's eight-bit premultiplied storage.
fn half_over_white(srgb: [f64; 3]) -> [u8; 3] {
    srgb.map(|v| ((v * 0.5).round() as u16 + 255 - 128) as u8)
}

/// Records `what` in `misses` unless `actual` is within `tolerance` levels of `wanted` on
/// every channel, so that one run names every pixel a control gets wrong.
fn within(misses: &mut Vec<String>, actual: [u8; 3], wanted: [u8; 3], tolerance: u8, what: &str) {
    if actual
        .into_iter()
        .zip(wanted)
        .any(|(a, b)| a.abs_diff(b) > tolerance)
    {
        misses.push(format!("{what}: actual {actual:?}, expected {wanted:?}"));
    }
}

fn none_missed(misses: &[String]) {
    assert!(misses.is_empty(), "{}", misses.join("\n"));
}

const PAINT: &str = "/Half gs 0 0 60 80 re f";

/// The page: `content`, after §8.6.5.9's compensation is turned off so that a press's black
/// point does not move its corners, on a `DeviceRGB` page group under an output intent whose profile is
/// [`INTENT`]'s press (object 5), with `/In` an isolated group in [`GROUP`]'s profile
/// inheriting the page's colour, `/Cold` the same group selecting `cold` itself, `/Outer` a
/// group in [`GROUP`]'s profile selecting `cold` and painting `/Inner` (an isolated group in
/// [`NESTED`]'s profile inheriting it), and `/OuterCold` the same pair with the selection made
/// inside `/Inner`. `default` is the page's extra `/ColorSpace` entries.
fn render(content: &str, cold: &str, default: &str) -> Vec<u8> {
    let half = "/ExtGState << /Half << /ca 0.5 /CA 0.5 >> /Off << /UseBlackPtComp /OFF >> >>";
    let group = |profile: usize, extra: &str| {
        format!(
            "/Type /XObject /Subtype /Form /BBox [0 0 80 80] \
             /Group << /S /Transparency /I true /CS [/ICCBased {profile} 0 R] >> \
             /Resources << {half} /ColorSpace << {SPACES} >> {TILE} {extra} >>"
        )
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX \
         /OutputConditionIdentifier (fixture) /DestOutputProfile 5 0 R >>] >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 100] \
             /Group << /S /Transparency /CS /DeviceRGB >> \
             /Resources << {half} /ColorSpace << {SPACES} {default} >> {TILE} \
             /XObject << /In 6 0 R /Cold 7 0 R /Outer 10 0 R /OuterCold 12 0 R >> >> \
             /Contents 4 0 R >>"
        ),
        content_stream("", &format!("/Off gs {content}")),
        stream("/N 4", &cmyk_profile(INTENT)),
        content_stream(&group(8, ""), PAINT),
        content_stream(&group(8, ""), &format!("{cold} {PAINT}")),
        stream("/N 4", &cmyk_profile(GROUP)),
        stream("/N 4", &cmyk_profile(NESTED)),
        content_stream(
            &group(8, "/XObject << /Inner 11 0 R >>"),
            &format!("{cold} /Inner Do"),
        ),
        content_stream(&group(9, ""), PAINT),
        content_stream(
            &group(8, "/XObject << /InnerCold 13 0 R >>"),
            "/InnerCold Do",
        ),
        content_stream(&group(9, ""), &format!("{cold} {PAINT}")),
        content_stream(
            "/Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 10 10] \
             /XStep 10 /YStep 10 /Resources << >>",
            "0 0 10 10 re f",
        ),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{object}\nendobj", index + 1).expect("write object");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).expect("xref");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("offset");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("trailer");
    render_bytes(out.into_bytes())
}

/// `Indexed` over `DeviceCMYK` with entry 1 [`COLOUR`], and a `Separation` whose alternate is
/// `DeviceCMYK` and whose full tint is [`COLOUR`]: both name the family §11.7.2 redefines.
const SPACES: &str = "/IK [/Indexed /DeviceCMYK 1 <00000000 00FF00FF>] \
/SK [/Separation /Spot /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 0 1] /N 1 >>] \
/PK [/Pattern /DeviceCMYK] ";

/// An uncoloured tiling pattern (object 14) whose one cell fills its step, so a mark painted
/// with it is its tint wherever a pixel is sampled.
const TILE: &str = "/Pattern << /Tile 14 0 R >>";

/// The selections of [`COLOUR`] each control makes: `k`, `cs` and `sc`, the `Indexed` entry
/// and the `Separation`'s full tint.
const SELECTIONS: [&str; 5] = [
    "0 1 0 1 k",
    "/DeviceCMYK cs 0 1 0 1 sc",
    "/IK cs 1 sc",
    "/SK cs 1 scn",
    "/PK cs 0 1 0 1 /Tile scn",
];

/// The tolerance on a pixel that leaves a CMYK group: its raster reaches the page through a
/// sampled `ColourCube` (`content/transparency/into_parent.rs`), whose interpolation between
/// grid points is the conversion's, not the clause's. The presses differ by far more than this
/// at [`COLOUR`], which `the_three_presses_are_told_apart` holds.
const OUT_OF_A_GROUP: u8 = 3;

#[test]
fn the_three_presses_are_told_apart() {
    let [intent, group, nested] = [INTENT, GROUP, NESTED].map(|w| half_over_white(corner_srgb(w)));
    for (a, b) in [(intent, group), (group, nested), (intent, nested)] {
        assert!(
            a.into_iter()
                .zip(b)
                .any(|(x, y)| x.abs_diff(y) > 4 * OUT_OF_A_GROUP),
            "{a:?} and {b:?} are too close to tell the presses apart"
        );
    }
}

#[test]
fn a_cmyk_colour_selected_under_the_output_intent_is_the_groups_own_inside_it() {
    let mut misses = Vec::new();
    for selection in SELECTIONS {
        let actual = render(
            &format!(
                "{selection} 0 10 80 80 re f q 1 0 0 1 100 10 cm /In Do Q \
                 q 1 0 0 1 200 10 cm /Cold Do Q"
            ),
            selection,
            "",
        );
        // Outside every group the output intent is what `DeviceCMYK` means (§14.11.5).
        within(
            &mut misses,
            pixel(&actual, 40, 50),
            opaque(corner_srgb(INTENT)),
            1,
            &format!("{selection} on the page"),
        );
        let wanted = half_over_white(corner_srgb(GROUP));
        within(
            &mut misses,
            pixel(&actual, 130, 50),
            wanted,
            OUT_OF_A_GROUP,
            &format!("{selection} inherited into the group"),
        );
        within(
            &mut misses,
            pixel(&actual, 230, 50),
            wanted,
            OUT_OF_A_GROUP,
            &format!("{selection} selected inside the group"),
        );
    }
    none_missed(&misses);
}

#[test]
fn a_cmyk_colour_selected_in_one_group_is_the_nested_groups_own_inside_it() {
    let mut misses = Vec::new();
    for selection in SELECTIONS {
        let actual = render(
            "q 1 0 0 1 100 10 cm /Outer Do Q q 1 0 0 1 200 10 cm /OuterCold Do Q",
            selection,
            "",
        );
        let wanted = half_over_white(corner_srgb(NESTED));
        within(
            &mut misses,
            pixel(&actual, 130, 50),
            wanted,
            OUT_OF_A_GROUP,
            &format!("{selection} selected in the outer group, painted in the inner"),
        );
        within(
            &mut misses,
            pixel(&actual, 230, 50),
            wanted,
            OUT_OF_A_GROUP,
            &format!("{selection} selected in the inner group"),
        );
    }
    none_missed(&misses);
}

#[test]
fn a_default_cmyk_selected_outside_stays_the_default_inside_the_group() {
    let mut misses = Vec::new();
    // §8.6.5.6 decides a default when the space is *selected*, in the current resource
    // dictionary, and §11.7.2 names "default CIE-based colour spaces" as what takes a device
    // colour out of the device rules. The default here is the output intent's own profile
    // stream, so the inherited colour is that press's and differs from the group's, which the
    // group's own resources (no default) give a cold `k`.
    let actual = render(
        "0 1 0 1 k 0 10 80 80 re f q 1 0 0 1 100 10 cm /In Do Q q 1 0 0 1 200 10 cm /Cold Do Q",
        "0 1 0 1 k",
        "/DefaultCMYK [/ICCBased 5 0 R]",
    );
    within(
        &mut misses,
        pixel(&actual, 40, 50),
        opaque(corner_srgb(INTENT)),
        1,
        "the default on the page",
    );
    within(
        &mut misses,
        pixel(&actual, 130, 50),
        half_over_white(corner_srgb(INTENT)),
        OUT_OF_A_GROUP,
        "the default inherited into the group",
    );
    within(
        &mut misses,
        pixel(&actual, 230, 50),
        half_over_white(corner_srgb(GROUP)),
        OUT_OF_A_GROUP,
        "a cold `k` in the group, whose resources state no default",
    );
    none_missed(&misses);
}
