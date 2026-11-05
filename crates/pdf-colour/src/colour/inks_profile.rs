//! An ICC output profile that states this renderer's assumed inks (ADR 2700).
//!
//! A `DeviceCMYK` colour that no profile characterises is drawn through the ink cube of
//! `super::device_cmyk::CMYK_CORNERS` (ADRs 0009, 0263): §10.3.2 lets a processor establish its
//! own CIE-based definition of a device space, and those sixteen corners are this processor's.
//! An archive that has to license such colour through a CMYK output intent (ISO 19005-2 and
//! ISO 19005-4 section 6.2.4.3) must name *some* ICC profile, and a real press's profile moves
//! every colour this renderer drew. This module writes the profile of the press the renderer
//! already assumed, so that the archive draws as its source did — the owner's answer A780.
//!
//! # What it is
//!
//! An ICC.1:1998-09 output profile (class `prtr`, data `CMYK`, connection space `XYZ `),
//! version 2.2.0, which ISO 19005-2 section 6.2.4.2 admits and ISO 32000-2 §8.6.5.5 admits for
//! part 4. That text's section 6.3.3.2 (Table 29) requires ten tags of a colour output profile
//! and all ten are written: `desc`, `cprt`, `wtpt`, `gamt`, and the three `A2B` and three `B2A`
//! intents. The same section permits one table to serve every intent ("It is permissible to
//! reference the same tag for all of these intents"), and the ink cube states no perceptual or
//! saturation rendering of its own, so the three `A2B` entries share one `lut16Type` table and
//! the three `B2A` entries another.
//!
//! - **`A2B`** is the cube sampled at [`A2B_SIDE`] points per ink, as D50 XYZ. A reader
//!   interpolates the table in XYZ, and the cube is multilinear in *encoded* sRGB, so between the
//!   nodes the two part; [`A2B_SIDE`] is the smallest side measured to keep that under one level
//!   of 255 on every one-ink ramp, at the 17⁴ grid the renderer samples a press on
//!   (`press::PRESS_SIDE`), and at the middle of each of that grid's cells (ADR 2700 section 1).
//! - **`B2A`** is this renderer's own conversion *into* the press, `super::ink_search::rgb_to_ink`
//!   (ADR 0263's right inverse), sampled at [`B2A_SIDE`] points per sRGB component. ICC.1
//!   section 6.5.6's matrix — which that text permits exactly where the input is XYZ — takes the
//!   connection space to linear sRGB, and the input tables encode it, so the grid stands on the
//!   components the ink cube is multilinear in, as `Press::table` does; a grid over L\*a\*b\*
//!   was measured and missed the round trip by up to 25 levels of 255 (ADR 2700 section 2).
//! - **`gamt`** marks a node out of gamut where the inks `rgb_to_ink` chose do not reproduce it
//!   within [`IN_GAMUT_LEVELS`] of 255 in every sRGB channel — a choice, recorded as one: ICC.1
//!   section 6.4.16 says only that zero is in gamut and that a larger number is further out.
//!
//! # Determinism
//!
//! The bytes are the same on every run and every platform: no clock is read (the header's date
//! is [`CREATED`], a fixed choice), every `powf` is taken in `f64` so that a platform's
//! last-place difference cannot reach a 16-bit sample, and the stages taken from the renderer in
//! `f32` are its matrices and its ink search, which use only IEEE-exact operations. The profile
//! is checked in as `data/icc/quorra-assumed-inks.icc` and a test regenerates it and compares, so
//! nothing here runs at launch or in a conversion (`CLAUDE.md` principle 2).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "layout and grid arithmetic on this module's own constants — ten tags, grid sides of \
              at most 33, curves of 4096 entries and strings it declares — none of which comes \
              near a usize's range"
)]

use pdf_render::Color;

use super::cie::{linear_srgb_to_xyz_d50, xyz_d50_to_linear_srgb};
use super::device_cmyk::cmyk;
use super::ink_search::rgb_to_ink;
use super::press::assumed_press;

/// Grid points per ink of the `A2B` table (see the module): a 1.6 MB profile, where 17 missed
/// the one-ink ramps by 1.16 levels and 25 doubles the size for a third of a level.
pub const A2B_SIDE: usize = 21;

/// Grid points per sRGB component of the `B2A` table.
pub const B2A_SIDE: usize = 33;

/// Grid points per sRGB component of the `gamt` table.
const GAMUT_SIDE: usize = 33;

/// Entries of each input table that encodes linear sRGB: ICC.1 section 6.5.6's maximum.
const CURVE_ENTRIES: usize = 4096;

/// How far, in levels of 255, the inks may miss a colour before it is out of gamut.
const IN_GAMUT_LEVELS: f32 = 1.0;

/// The header's creation date: year, month, day, hours, minutes, seconds (ICC.1 `dateTimeNumber`).
///
/// A fixed date rather than the clock, so that one tree writes one profile: the day ADR 2700
/// was decided.
const CREATED: [u16; 6] = [2026, 11, 5, 0, 0, 0];

/// The profile's description, as its `desc` tag states it.
pub const DESCRIPTION: &str = "quorra assumed process inks: DeviceCMYK as this renderer draws \
     it, not a physical press";

/// The `cprt` tag's text.
const COPYRIGHT: &str = "No copyright. Generated by pdf-colour from the ink cube this renderer \
     assumes; free to copy, embed, alter and distribute.";

/// ICC.1:1998-09 section 6.1.3's version field: 2.2.0, the version that text describes.
const VERSION: [u8; 4] = [0x02, 0x20, 0x00, 0x00];

/// The connection space's illuminant, D50, as ICC.1 Annex A.1 states it.
const D50: [f64; 3] = [0.9642, 1.0, 0.8249];

/// ICC.1 Annex A's 16-bit XYZ: 1.0 is `8000h`.
const XYZ_ONE: f64 = 32768.0 / 65535.0;

/// Writes the profile. Pure: the same bytes on every call.
#[must_use]
pub fn assumed_inks_profile() -> Vec<u8> {
    build(A2B_SIDE, B2A_SIDE)
}

/// [`assumed_inks_profile`] with the two grids chosen, for the measurement of ADR 2700.
pub(crate) fn build(a2b_side: usize, b2a_side: usize) -> Vec<u8> {
    let encoding = encoding_curve();
    let a2b = lut16(&Lut {
        inputs: 4,
        outputs: 3,
        side: a2b_side,
        matrix: identity(),
        input_curve: &[0, 0xffff],
        output_curves: &[IDENTITY.to_vec(), IDENTITY.to_vec(), IDENTITY.to_vec()],
        samples: &a2b_samples(a2b_side),
    });
    let b2a = lut16(&Lut {
        inputs: 3,
        outputs: 4,
        side: b2a_side,
        matrix: to_linear_srgb(),
        input_curve: &encoding,
        output_curves: &[
            IDENTITY.to_vec(),
            IDENTITY.to_vec(),
            IDENTITY.to_vec(),
            IDENTITY.to_vec(),
        ],
        samples: &b2a_samples(b2a_side),
    });
    let gamut = lut16(&Lut {
        inputs: 3,
        outputs: 1,
        side: GAMUT_SIDE,
        matrix: to_linear_srgb(),
        input_curve: &encoding,
        output_curves: &[IDENTITY.to_vec()],
        samples: &gamut_samples(GAMUT_SIDE),
    });
    let tags: [(&[u8; 4], usize); 10] = [
        (b"desc", 0),
        (b"cprt", 1),
        (b"wtpt", 2),
        (b"A2B0", 3),
        (b"A2B1", 3),
        (b"A2B2", 3),
        (b"B2A0", 4),
        (b"B2A1", 4),
        (b"B2A2", 4),
        (b"gamt", 5),
    ];
    // The paper is sRGB white, which the renderer's matrix takes to D50 itself: the media is the
    // connection space's white, so the relative and the absolute intents agree.
    let data = [
        description(DESCRIPTION),
        text(COPYRIGHT),
        xyz(D50),
        a2b,
        b2a,
        gamut,
    ];
    assemble(&tags, &data)
}

/// The header and tag table, then each distinct tag's data on a four-byte boundary (ICC.1
/// section 6.2.4); entries naming the same data point at one copy of it.
fn assemble(tags: &[(&[u8; 4], usize)], data: &[Vec<u8>]) -> Vec<u8> {
    let table_end = 128 + 4 + 12 * tags.len();
    let mut offsets = Vec::with_capacity(data.len());
    let mut at = table_end;
    for block in data {
        at = at.next_multiple_of(4);
        offsets.push(at);
        at += block.len();
    }
    let size = at;

    let mut out = Vec::with_capacity(size);
    push_u32(&mut out, size);
    out.extend_from_slice(&[0; 4]); // no preferred CMM
    out.extend_from_slice(&VERSION);
    out.extend_from_slice(b"prtr");
    out.extend_from_slice(b"CMYK");
    out.extend_from_slice(b"XYZ ");
    for field in CREATED {
        out.extend_from_slice(&field.to_be_bytes());
    }
    out.extend_from_slice(b"acsp");
    out.extend_from_slice(&[0; 4]); // no primary platform
    out.extend_from_slice(&[0; 4]); // flags: not embedded, usable independently
    out.extend_from_slice(&[0; 4]); // device manufacturer
    out.extend_from_slice(&[0; 4]); // device model
    out.extend_from_slice(&[0; 8]); // attributes: reflective, glossy
    out.extend_from_slice(&1_u32.to_be_bytes()); // relative colorimetric, PDF's default intent
    for value in D50 {
        out.extend_from_slice(&s15_fixed16(value));
    }
    out.extend_from_slice(&[0; 4]); // creator
    out.resize(128, 0);

    push_u32(&mut out, tags.len());
    for (signature, index) in tags {
        out.extend_from_slice(*signature);
        push_u32(&mut out, offsets[*index]);
        push_u32(&mut out, data[*index].len());
    }
    for (block, offset) in data.iter().zip(&offsets) {
        out.resize(*offset, 0);
        out.extend_from_slice(block);
    }
    out
}

/// Appends a length or an offset as ICC's `uInt32Number`.
fn push_u32(out: &mut Vec<u8>, value: usize) {
    // A profile of this module's own fixed shape is about a megabyte, so every length fits.
    let value = u32::try_from(value).unwrap_or(u32::MAX);
    out.extend_from_slice(&value.to_be_bytes());
}

/// ICC.1 section 5.3.3's `s15Fixed16Number`.
fn s15_fixed16(value: f64) -> [u8; 4] {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "every value written is a white point or a matrix entry below 4 in magnitude"
    )]
    let fixed = (value * 65536.0).round() as i32;
    fixed.to_be_bytes()
}

/// ICC.1 section 6.5.16's `textDescriptionType`, with no Unicode or `ScriptCode` description.
fn description(ascii: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + ascii.len() + 1 + 8 + 3 + 67);
    out.extend_from_slice(b"desc");
    out.extend_from_slice(&[0; 4]);
    push_u32(&mut out, ascii.len() + 1);
    out.extend_from_slice(ascii.as_bytes());
    out.push(0);
    out.extend_from_slice(&[0; 8]); // Unicode language code and count: none
    out.extend_from_slice(&[0; 3]); // ScriptCode code and count: none
    out.extend_from_slice(&[0; 67]);
    out
}

/// ICC.1 section 6.5.17's `textType`.
fn text(ascii: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + ascii.len() + 1);
    out.extend_from_slice(b"text");
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(ascii.as_bytes());
    out.push(0);
    out
}

/// ICC.1 section 6.5.25's `XYZType`, one value.
fn xyz(value: [f64; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(20);
    out.extend_from_slice(b"XYZ ");
    out.extend_from_slice(&[0; 4]);
    for component in value {
        out.extend_from_slice(&s15_fixed16(component));
    }
    out
}

/// One `lut16Type` table, before it is laid out.
struct Lut<'a> {
    /// Input channels.
    inputs: u8,
    /// Output channels.
    outputs: u8,
    /// Grid points per input.
    side: usize,
    /// The 3×3 matrix, row by row.
    matrix: [f64; 9],
    /// The input table every input channel shares.
    input_curve: &'a [u16],
    /// One output table per output channel, each of the same length.
    output_curves: &'a [Vec<u16>],
    /// The table, already 16-bit, the last input varying fastest.
    samples: &'a [u16],
}

/// ICC.1 section 6.5.6's `lut16Type`.
fn lut16(lut: &Lut<'_>) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        52 + 2 * (lut.samples.len() + lut.input_curve.len() * usize::from(lut.inputs) + 8),
    );
    out.extend_from_slice(b"mft2");
    out.extend_from_slice(&[0; 4]);
    out.push(lut.inputs);
    out.push(lut.outputs);
    // A grid side this module chooses, at most 33.
    out.push(u8::try_from(lut.side).unwrap_or(u8::MAX));
    out.push(0);
    for entry in lut.matrix {
        out.extend_from_slice(&s15_fixed16(entry));
    }
    // At most `CURVE_ENTRIES`, which ICC.1 section 6.5.6 caps at 4096.
    let entries = u16::try_from(lut.input_curve.len()).unwrap_or(u16::MAX);
    out.extend_from_slice(&entries.to_be_bytes());
    let output_entries = lut.output_curves.first().map_or(2, Vec::len);
    out.extend_from_slice(
        &u16::try_from(output_entries)
            .unwrap_or(u16::MAX)
            .to_be_bytes(),
    );
    for _ in 0..lut.inputs {
        for entry in lut.input_curve {
            out.extend_from_slice(&entry.to_be_bytes());
        }
    }
    for sample in lut.samples {
        out.extend_from_slice(&sample.to_be_bytes());
    }
    for curve in lut.output_curves {
        for entry in curve {
            out.extend_from_slice(&entry.to_be_bytes());
        }
    }
    out
}

/// A two-entry table that changes nothing.
const IDENTITY: [u16; 2] = [0, 0xffff];

/// The identity, which ICC.1 section 6.5.6 mandates where a table's input is not XYZ.
fn identity() -> [f64; 9] {
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
}

/// The renderer's own D50 XYZ to linear sRGB matrix, `cie::xyz_d50_to_linear_srgb`, read off by
/// its columns so that it is stated once.
///
/// The table's XYZ and its linear sRGB share ICC.1 Annex A's encoding (1.0 at `8000h`), so the
/// matrix carries over unscaled.
fn to_linear_srgb() -> [f64; 9] {
    let columns = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(xyz_d50_to_linear_srgb);
    let mut matrix = [0.0; 9];
    for (column, values) in columns.iter().enumerate() {
        for (row, value) in values.iter().enumerate() {
            matrix[row * 3 + column] = f64::from(*value);
        }
    }
    matrix
}

/// The input table that takes linear sRGB, in ICC.1 Annex A's XYZ encoding, to encoded sRGB:
/// IEC 61966-2-1's transfer function, flat at white past 1.0.
fn encoding_curve() -> Vec<u16> {
    (0..CURVE_ENTRIES)
        .map(|index| sample(gamma((node(index, CURVE_ENTRIES) / XYZ_ONE).min(1.0))))
        .collect()
}

/// The position of grid node `index` of `side` on the unit interval.
fn node(index: usize, side: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid indices and sides below 4097, exact in f64"
    )]
    let at = index as f64 / (side - 1) as f64;
    at
}

/// A value on `0.0..=1.0` as a 16-bit sample.
fn sample(value: f64) -> u16 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=65535 first"
    )]
    let quantised = (value.clamp(0.0, 1.0) * 65535.0).round() as u16;
    quantised
}

/// IEC 61966-2-1's decoding of an sRGB component, in `f64` (see the module on determinism).
fn degamma(value: f64) -> f64 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// IEC 61966-2-1's encoding of a linear sRGB component, in `f64`.
fn gamma(value: f64) -> f64 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

/// What the renderer draws four inks as, in sRGB.
fn ink_colour(inks: [f32; 4]) -> Color {
    cmyk(inks[0], inks[1], inks[2], inks[3])
}

/// The ink cube sampled at `side` points per ink, cyan varying slowest (ICC.1 section 6.5.6), as
/// D50 XYZ through the inverse of the renderer's matrix.
fn a2b_samples(side: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(side.pow(4) * 3);
    for c in 0..side {
        for m in 0..side {
            for y in 0..side {
                for k in 0..side {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "grid positions on 0..=1, which the renderer takes as f32"
                    )]
                    let inks = [c, m, y, k].map(|index| node(index, side) as f32);
                    let colour = ink_colour(inks);
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "linear sRGB on 0..=1, which the renderer's matrix takes as f32"
                    )]
                    let linear = [colour.r, colour.g, colour.b]
                        .map(|value| degamma(f64::from(value)) as f32);
                    let xyz = linear_srgb_to_xyz_d50(linear);
                    out.extend(xyz.map(|value| sample(f64::from(value) * XYZ_ONE)));
                }
            }
        }
    }
    out
}

/// Every node of an sRGB grid of `side`, red slowest, as the renderer's colour.
fn srgb_nodes(side: usize) -> impl Iterator<Item = Color> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "grid positions on 0..=1, which the renderer takes as f32"
    )]
    let at = move |index: usize| node(index, side) as f32;
    (0..side).flat_map(move |r| {
        (0..side).flat_map(move |g| (0..side).map(move |b| Color::rgb(at(r), at(g), at(b))))
    })
}

/// `rgb_to_ink` sampled over sRGB, red slowest.
fn b2a_samples(side: usize) -> Vec<u16> {
    let press = assumed_press();
    let mut out = Vec::with_capacity(side.pow(3) * 4);
    for colour in srgb_nodes(side) {
        let inks = rgb_to_ink(&press, colour);
        out.extend(inks.map(|ink| sample(f64::from(ink))));
    }
    out
}

/// Zero where the inks reproduce a node, and the miss as a fraction of the full scale where they
/// do not.
fn gamut_samples(side: usize) -> Vec<u16> {
    let press = assumed_press();
    srgb_nodes(side)
        .map(|colour| {
            let got = ink_colour(rgb_to_ink(&press, colour));
            let miss = [(colour.r, got.r), (colour.g, got.g), (colour.b, got.b)]
                .iter()
                .map(|(want, got)| (want - got).abs() * 255.0)
                .fold(0.0, f32::max);
            if miss <= IN_GAMUT_LEVELS {
                0
            } else {
                sample(f64::from(miss) / 255.0).max(1)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icc::{Profile, Rendering};

    /// The profile as `data/icc/quorra-assumed-inks.icc` holds it, which `pdf-transform` embeds.
    const CHECKED_IN: &[u8] = include_bytes!("../../../../data/icc/quorra-assumed-inks.icc");

    /// The largest channel difference, in levels of 255, between the profile's drawing of `inks`
    /// and the ink cube's.
    fn levels(profile: &Profile, inks: [f32; 4]) -> f32 {
        let got = profile.to_rgb_with(&inks, Rendering::compensating());
        let want = ink_colour(inks);
        [(got.r, want.r), (got.g, want.g), (got.b, want.b)]
            .iter()
            .map(|(a, b)| (a - b).abs() * 255.0)
            .fold(0.0, f32::max)
    }

    /// The 17⁴ grid of the renderer's own press sampling, as inks.
    fn press_nodes() -> impl Iterator<Item = [f32; 4]> {
        (0..17u32.pow(4)).map(|index| {
            #[expect(clippy::cast_precision_loss, reason = "indices below 17, exact in f32")]
            let at = |value: u32| value as f32 / 16.0;
            [
                at(index % 17),
                at(index / 17 % 17),
                at(index / 289 % 17),
                at(index / 4913),
            ]
        })
    }

    #[test]
    fn the_checked_in_profile_is_what_the_generator_writes_every_time() {
        let written = assumed_inks_profile();
        assert_eq!(written, assumed_inks_profile(), "two runs, two profiles");
        assert!(
            written == CHECKED_IN,
            "data/icc/quorra-assumed-inks.icc is stale: regenerate it from assumed_inks_profile()"
        );
    }

    #[test]
    fn it_is_an_output_profile_over_cmyk_with_every_tag_icc_1_requires_of_one() {
        // ICC.1:1998-09 section 6.3.3.2, Table 29: a colour output profile's ten required tags.
        let profile = assumed_inks_profile();
        let stated = crate::icc::Identification::read(&profile).expect("a header");
        assert_eq!((&stated.class, &stated.space), (b"prtr", b"CMYK"));
        assert_eq!(stated.description.as_deref(), Some(DESCRIPTION));
        assert!(DESCRIPTION.contains("not a physical press"));
        for tag in [
            b"desc", b"cprt", b"wtpt", b"gamt", b"A2B0", b"A2B1", b"A2B2", b"B2A0", b"B2A1",
            b"B2A2",
        ] {
            assert!(
                crate::icc::has_tag(&profile, *tag),
                "{}",
                String::from_utf8_lossy(tag)
            );
        }
        let parsed = Profile::parse(&profile).expect("this crate reads it");
        assert!(parsed.is_bidirectional(), "a B2A this crate can evaluate");
    }

    #[test]
    fn at_the_renderers_own_press_grid_the_profile_is_the_ink_cube_within_a_level() {
        // The renderer samples a CMYK press's profile at these points and interpolates between
        // them in sRGB as the cube itself does, so a composite drawn in the press moves by no more
        // than this (measured 0.791; 0.256 where these points are the table's own nodes, at 17).
        let profile = Profile::parse(&assumed_inks_profile()).expect("it parses");
        let worst = press_nodes()
            .map(|inks| levels(&profile, inks))
            .fold(0.0, f32::max);
        assert!(worst < 0.85, "{worst} levels at a press grid point");
    }

    #[test]
    fn a_colour_painted_through_the_profile_is_the_ink_cube_within_a_level() {
        // A `DeviceCMYK` fill under the intent is converted through the profile itself, which
        // interpolates the table in the connection space rather than in sRGB. Over the four ramps
        // of one ink and the middle of every cell of the 17⁴ grid that costs at most 0.88 levels
        // (measured 0.880 and 0.777), against 1.16 and 1.30 at 17 nodes an ink.
        let profile = Profile::parse(&assumed_inks_profile()).expect("it parses");
        let mut worst = 0.0f32;
        for ink in 0..4 {
            for step in 0..=255u8 {
                let mut inks = [0.0f32; 4];
                inks[ink] = f32::from(step) / 255.0;
                worst = worst.max(levels(&profile, inks));
            }
        }
        for inks in press_nodes() {
            if inks.iter().all(|ink| *ink < 1.0) {
                worst = worst.max(levels(&profile, inks.map(|ink| ink + 1.0 / 32.0)));
            }
        }
        assert!(worst < 0.95, "{worst} levels between the nodes");
    }

    #[test]
    fn the_inverse_round_trips_every_colour_of_the_cube_within_four_levels() {
        // A colour the cube draws, taken into the press through `B2A` and drawn again. Measured:
        // at most 3.62 levels, and 3 740 of the 83 521 colours past one level, every one of them
        // at the gamut's edge or in its darkest quarter, where `rgb_to_ink`'s black changes rung.
        let profile = Profile::parse(&assumed_inks_profile()).expect("it parses");
        let mut worst = 0.0f32;
        let mut past_one = 0usize;
        for inks in press_nodes() {
            let colour = ink_colour(inks);
            let xyz = super::super::cie::srgb_to_xyz_d50(colour);
            let back = profile
                .to_device(xyz, Rendering::compensating())
                .expect("a B2A");
            let again = ink_colour([back[0], back[1], back[2], back[3]]);
            let missed = [
                (colour.r, again.r),
                (colour.g, again.g),
                (colour.b, again.b),
            ]
            .iter()
            .map(|(a, b)| (a - b).abs() * 255.0)
            .fold(0.0, f32::max);
            worst = worst.max(missed);
            past_one += usize::from(missed > 1.0);
        }
        assert!(worst < 4.0, "{worst} levels");
        assert!(past_one <= 3_800, "{past_one} colours past one level");
    }
}
