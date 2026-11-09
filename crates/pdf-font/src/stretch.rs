//! A `glyf` program with every outline scaled horizontally: the shape this reader draws a
//! substituted face in, written into the face itself.
//!
//! **What it is for** is ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 under
//! the profile that promises the window's own picture (`doc/adr/2769`). This reader draws a font a
//! document names and does not embed from a face it ships, and where the file's `/Widths` say the
//! absent font was narrower than that face it scales each outline's x by one number,
//! `crate::metrics::substitute_stretch` (`doc/adr/0358`), about the glyph's own origin. Embedding
//! the face unmodified removes that scale, which is the loss `substitute-stretch`; embedding the
//! face *with* it loses nothing, because the output then draws what the window drew.
//!
//! **The advances are not touched.** ISO 32000-2 §9.2.4 positions every glyph by the dictionary's
//! widths, and ISO 19005-2 section 6.2.11.5 holds the program's advances to the same numbers;
//! `crate::restate` is what makes them agree, after this, exactly as for a face embedded unscaled.
//! Only the shape moves inside the advance, which is what the window's scale moves.
//!
//! **What is rewritten** is what ISO/IEC 14496-22 makes depend on an x coordinate: each simple
//! glyph's x coordinates and its bounding box; each composite's x offsets, its bounding box, and a
//! two-by-two transform conjugated by the scale so that a component scaled first and transformed
//! after lands where the transform of the unscaled component scaled after would; `loca`, written
//! long because the glyphs' lengths change; `head`'s box; `hmtx`'s left side bearings, so that
//! each glyph's origin stays where its outline puts it; and `hhea`'s three horizontal extremes.
//! The y coordinates, the instructions and every other table are copied byte for byte.
//!
//! **A coordinate is an integer**, so a scaled one is rounded: each point lands within half a
//! unit of the program's em of the window's, and a point of a component within one (its offset
//! is rounded too). [`sfnt_outlines`] reads every glyph back and holds it to that before it
//! returns.

use std::collections::BTreeMap;

use skrifa::outline::OutlinePen;

use crate::sfnt::{be16, horizontal_metric, sfnt_tables, with_tables};

/// Why a program could not be given the scaled outlines.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum StretchError {
    /// The scale is not a narrowing this reader draws: ADR 0358 condenses only, and refuses
    /// anything below one half.
    #[error("the horizontal scale {0} is not one this reader draws a substituted face at")]
    Scale(f32),
    /// The program is not an sfnt with `glyf` outlines, or a table it needs does not read.
    #[error("the font program's outlines could not be scaled: {0}")]
    Unreadable(&'static str),
    /// A scaled number does not fit the field ISO/IEC 14496-22 states it in.
    #[error("glyph {glyph}'s {field} does not fit its field once scaled")]
    OutOfRange {
        /// Which glyph.
        glyph: u16,
        /// Which number.
        field: &'static str,
    },
    /// The rewritten program does not draw the scaled outline of the one it was made from.
    #[error("glyph {glyph} of the rewritten program does not draw the scaled outline")]
    NotProved {
        /// The first glyph that disagreed.
        glyph: u16,
    },
}

/// `numberOfHMetrics` within `hhea`.
const NUMBER_OF_H_METRICS: usize = 34;
/// `minLeftSideBearing`, `minRightSideBearing` and `xMaxExtent` within `hhea`.
const HHEA_EXTREMES: usize = 10;
/// `xMin` within `head`; `xMax` is four bytes on.
const HEAD_X_MIN: usize = 36;
/// `indexToLocFormat` within `head`.
const INDEX_TO_LOC: usize = 50;
/// `numGlyphs` within `maxp`.
const NUM_GLYPHS: usize = 4;

/// Simple-glyph flags: x is one byte, and (with it) its sign or (without it) "same as before".
const X_SHORT: u8 = 0x02;
/// See [`X_SHORT`].
const X_SAME_OR_POSITIVE: u8 = 0x10;
/// The flag byte is followed by a repeat count.
const REPEAT: u8 = 0x08;

/// Composite flags, ISO/IEC 14496-22's `glyf` table.
const ARGS_ARE_WORDS: u16 = 0x0001;
/// See [`ARGS_ARE_WORDS`].
const ARGS_ARE_XY: u16 = 0x0002;
/// See [`ARGS_ARE_WORDS`].
const HAVE_A_SCALE: u16 = 0x0008;
/// See [`ARGS_ARE_WORDS`].
const MORE_COMPONENTS: u16 = 0x0020;
/// See [`ARGS_ARE_WORDS`].
const HAVE_X_AND_Y_SCALE: u16 = 0x0040;
/// See [`ARGS_ARE_WORDS`].
const HAVE_TWO_BY_TWO: u16 = 0x0080;
/// See [`ARGS_ARE_WORDS`].
const HAVE_INSTRUCTIONS: u16 = 0x0100;

/// How far a read-back point may be from the window's, in the program's own units: half a unit
/// for the point's own rounding, half for the offset of the component it belongs to.
const PROVED_WITHIN: f32 = 1.0;

/// `program`, a `glyf` sfnt, with every outline's x multiplied by `scale` about the glyph's origin.
///
/// # Errors
///
/// [`StretchError`]: a scale this reader never draws at, a program that is not a `glyf` sfnt or
/// whose tables do not read, a number that does not fit its field, or a read-back that disagrees.
pub fn sfnt_outlines(program: &[u8], scale: f32) -> Result<Vec<u8>, StretchError> {
    if !(0.5..1.0).contains(&scale) {
        return Err(StretchError::Scale(scale));
    }
    let tables = sfnt_tables(program).ok_or(StretchError::Unreadable("no table directory"))?;
    if tables.contains_key(b"gvar".as_slice()) {
        return Err(StretchError::Unreadable(
            "it is a variable font, whose deltas would need scaling too",
        ));
    }
    let table = |tag: &[u8; 4]| {
        let (at, length) = *tables
            .get(tag.as_slice())
            .ok_or(StretchError::Unreadable("a table it needs is absent"))?;
        program
            .get(at..at.saturating_add(length))
            .ok_or(StretchError::Unreadable("a table runs past the program"))
    };
    let (head, hhea, maxp) = (table(b"head")?, table(b"hhea")?, table(b"maxp")?);
    let (glyf, loca, hmtx) = (table(b"glyf")?, table(b"loca")?, table(b"hmtx")?);
    let count = usize::from(be16(maxp, NUM_GLYPHS).ok_or(StretchError::Unreadable("maxp"))?);
    let pairs =
        usize::from(be16(hhea, NUMBER_OF_H_METRICS).ok_or(StretchError::Unreadable("hhea"))?);
    let long = be16(head, INDEX_TO_LOC).ok_or(StretchError::Unreadable("head"))? != 0;

    let mut scaled_glyf: Vec<u8> = Vec::with_capacity(glyf.len());
    let mut offsets: Vec<u32> = Vec::with_capacity(count.saturating_add(1));
    let mut metrics: Vec<(u16, i16)> = Vec::with_capacity(count);
    let mut extent = Extent::default();
    for index in 0..count {
        let glyph = u16::try_from(index).map_err(|_| StretchError::Unreadable("maxp"))?;
        let (start, end) = located(loca, long, index).ok_or(StretchError::Unreadable("loca"))?;
        let data = glyf
            .get(start..end)
            .ok_or(StretchError::Unreadable("a glyph runs past glyf"))?;
        offsets
            .push(u32::try_from(scaled_glyf.len()).map_err(|_| StretchError::Unreadable("glyf"))?);
        let (advance, bearing) = horizontal_metric(hmtx, pairs, index);
        if data.is_empty() {
            metrics.push((
                advance,
                scaled_i16(bearing, scale, glyph, "left side bearing")?,
            ));
            continue;
        }
        let Scaled {
            bytes: rewritten,
            old_min,
            new_box,
        } = scaled_glyph(data, scale, glyph)?;
        // The origin keeps its distance from the outline's left edge, scaled: ISO/IEC 14496-22's
        // first phantom point is `xMin - lsb`, so the bearing that keeps it at x = 0 times the
        // scale is the new `xMin` less the scaled difference.
        let origin = i32::from(old_min).saturating_sub(i32::from(bearing));
        let bearing =
            i32::from(new_box.0).saturating_sub(round(f64::from(origin) * f64::from(scale)));
        let bearing = i16::try_from(bearing).map_err(|_| StretchError::OutOfRange {
            glyph,
            field: "left side bearing",
        })?;
        extent.include(advance, bearing, new_box);
        metrics.push((advance, bearing));
        scaled_glyf.extend_from_slice(&rewritten);
        while !scaled_glyf.len().is_multiple_of(4) {
            scaled_glyf.push(0);
        }
    }
    offsets.push(u32::try_from(scaled_glyf.len()).map_err(|_| StretchError::Unreadable("glyf"))?);

    let mut new_loca = Vec::with_capacity(offsets.len().saturating_mul(4));
    for offset in &offsets {
        new_loca.extend_from_slice(&offset.to_be_bytes());
    }
    let mut new_hmtx = Vec::with_capacity(count.saturating_mul(4));
    for (advance, bearing) in &metrics {
        new_hmtx.extend_from_slice(&advance.to_be_bytes());
        new_hmtx.extend_from_slice(&bearing.to_be_bytes());
    }
    let glyphs = u16::try_from(count).map_err(|_| StretchError::Unreadable("maxp"))?;
    let (new_head, new_hhea) = summaries(head, hhea, glyphs, &extent)?;

    let scaled = with_tables(
        program,
        &[
            (b"glyf", scaled_glyf),
            (b"loca", new_loca),
            (b"head", new_head),
            (b"hhea", new_hhea),
            (b"hmtx", new_hmtx),
        ],
    )
    .ok_or(StretchError::Unreadable(
        "the tables could not be laid out again",
    ))?;
    proved(program, &scaled, scale, glyphs)?;
    Ok(scaled)
}

/// How far a point of a program [`sfnt_outlines`] wrote may stand from the viewer's, in ems: one
/// unit of the program's em, which is what its read-back held every glyph to, and the rounding a
/// quadratic's elevation to a cubic adds beside it.
#[must_use]
pub fn within_in_ems(program: &[u8]) -> Option<f32> {
    let units = crate::sfnt::units_per_em(program).filter(|units| *units > 0.0)?;
    Some(PROVED_WITHIN / units + f32::EPSILON)
}

/// `head` and `hhea` restated for the scaled glyphs: the box and the three horizontal extremes,
/// one advance per glyph, and `loca` long.
fn summaries(
    head: &[u8],
    hhea: &[u8],
    glyphs: u16,
    extent: &Extent,
) -> Result<(Vec<u8>, Vec<u8>), StretchError> {
    let mut new_hhea = hhea.to_vec();
    put(&mut new_hhea, NUMBER_OF_H_METRICS, &glyphs.to_be_bytes())?;
    if let Some((min_left, min_right, max_extent)) = extent.stated() {
        put(&mut new_hhea, HHEA_EXTREMES, &min_left.to_be_bytes())?;
        put(
            &mut new_hhea,
            HHEA_EXTREMES.saturating_add(2),
            &min_right.to_be_bytes(),
        )?;
        put(
            &mut new_hhea,
            HHEA_EXTREMES.saturating_add(4),
            &max_extent.to_be_bytes(),
        )?;
    }
    let mut new_head = head.to_vec();
    if let Some((x_min, x_max)) = extent.horizontal() {
        put(&mut new_head, HEAD_X_MIN, &x_min.to_be_bytes())?;
        put(
            &mut new_head,
            HEAD_X_MIN.saturating_add(4),
            &x_max.to_be_bytes(),
        )?;
    }
    put(&mut new_head, INDEX_TO_LOC, &1u16.to_be_bytes())?;
    Ok((new_head, new_hhea))
}

/// Where glyph `index`'s data starts and ends in `glyf`.
fn located(loca: &[u8], long: bool, index: usize) -> Option<(usize, usize)> {
    let entry = |at: usize| -> Option<usize> {
        if long {
            usize::try_from(crate::sfnt::be32(loca, at.checked_mul(4)?)?).ok()
        } else {
            usize::from(be16(loca, at.checked_mul(2)?)?).checked_mul(2)
        }
    };
    let (start, end) = (entry(index)?, entry(index.checked_add(1)?)?);
    (start <= end).then_some((start, end))
}

/// One glyph rewritten.
struct Scaled {
    /// Its data.
    bytes: Vec<u8>,
    /// The `xMin` it stated before.
    old_min: i16,
    /// The `xMin` and `xMax` it states now.
    new_box: (i16, i16),
}

/// One glyph rewritten, with its old `xMin` and its new `(xMin, xMax)`.
fn scaled_glyph(data: &[u8], scale: f32, glyph: u16) -> Result<Scaled, StretchError> {
    let unreadable = StretchError::Unreadable("a glyph's data does not read");
    let contours = i16::from_be_bytes([
        *data.first().ok_or(unreadable.clone())?,
        *data.get(1).ok_or(unreadable.clone())?,
    ]);
    let old_min = read_i16(data, 2).ok_or(unreadable.clone())?;
    let old_max = read_i16(data, 6).ok_or(unreadable.clone())?;
    let mut out = data.get(..10).ok_or(unreadable.clone())?.to_vec();
    let new_box = if contours >= 0 {
        simple(
            data,
            &mut out,
            usize::try_from(contours).unwrap_or(0),
            scale,
            glyph,
        )?
    } else {
        composite(data, &mut out, scale, glyph)?;
        (
            scaled_i16(old_min, scale, glyph, "xMin")?,
            scaled_i16(old_max, scale, glyph, "xMax")?,
        )
    };
    out.get_mut(2..4)
        .ok_or(unreadable.clone())?
        .copy_from_slice(&new_box.0.to_be_bytes());
    out.get_mut(6..8)
        .ok_or(unreadable)?
        .copy_from_slice(&new_box.1.to_be_bytes());
    Ok(Scaled {
        bytes: out,
        old_min,
        new_box,
    })
}

/// A simple glyph's body after its header, its x coordinates scaled; its new `(xMin, xMax)`.
///
/// The flags are written one per point, without the repeat count (ISO/IEC 14496-22 makes it an
/// option of the encoder), each with its x bits restated for the new x and every other bit kept;
/// the y coordinates' bytes follow unchanged, because their encoding is decided by the y bits
/// alone.
fn simple(
    data: &[u8],
    out: &mut Vec<u8>,
    contours: usize,
    scale: f32,
    glyph: u16,
) -> Result<(i16, i16), StretchError> {
    let unreadable = || StretchError::Unreadable("a simple glyph's data does not read");
    let ends_at = 10usize
        .checked_add(contours.checked_mul(2).ok_or_else(unreadable)?)
        .ok_or_else(unreadable)?;
    if contours == 0 {
        out.extend_from_slice(data.get(10..).ok_or_else(unreadable)?);
        return Ok((
            read_i16(data, 2).ok_or_else(unreadable)?,
            read_i16(data, 6).ok_or_else(unreadable)?,
        ));
    }
    let points = usize::from(
        be16(data, ends_at.checked_sub(2).ok_or_else(unreadable)?).ok_or_else(unreadable)?,
    )
    .checked_add(1)
    .ok_or_else(unreadable)?;
    let instructions = usize::from(be16(data, ends_at).ok_or_else(unreadable)?);
    let mut at = ends_at
        .checked_add(2)
        .and_then(|at| at.checked_add(instructions))
        .ok_or_else(unreadable)?;
    // End points, instruction length and instructions, unchanged.
    out.extend_from_slice(data.get(10..at).ok_or_else(unreadable)?);
    let mut flags: Vec<u8> = Vec::with_capacity(points);
    while flags.len() < points {
        let flag = *data.get(at).ok_or_else(unreadable)?;
        at = at.checked_add(1).ok_or_else(unreadable)?;
        let mut times = 1usize;
        if flag & REPEAT != 0 {
            times = times.saturating_add(usize::from(*data.get(at).ok_or_else(unreadable)?));
            at = at.checked_add(1).ok_or_else(unreadable)?;
        }
        for _ in 0..times {
            flags.push(flag);
        }
    }
    flags.truncate(points);
    let mut x = 0i32;
    let mut xs: Vec<i32> = Vec::with_capacity(points);
    for flag in &flags {
        let delta = if flag & X_SHORT != 0 {
            let byte = i32::from(*data.get(at).ok_or_else(unreadable)?);
            at = at.checked_add(1).ok_or_else(unreadable)?;
            if flag & X_SAME_OR_POSITIVE != 0 {
                byte
            } else {
                byte.saturating_neg()
            }
        } else if flag & X_SAME_OR_POSITIVE != 0 {
            0
        } else {
            let word = read_i16(data, at).ok_or_else(unreadable)?;
            at = at.checked_add(2).ok_or_else(unreadable)?;
            i32::from(word)
        };
        x = x.saturating_add(delta);
        xs.push(x);
    }
    let ys = data.get(at..).ok_or_else(unreadable)?;
    let scaled: Vec<i32> = xs
        .iter()
        .map(|x| round(f64::from(*x) * f64::from(scale)))
        .collect();
    let mut x_bytes: Vec<u8> = Vec::with_capacity(points.saturating_mul(2));
    let mut previous = 0i32;
    for (flag, x) in flags.iter().zip(&scaled) {
        let delta = x.saturating_sub(previous);
        previous = *x;
        let kept = flag & !(X_SHORT | X_SAME_OR_POSITIVE | REPEAT);
        let restated = if delta == 0 {
            kept | X_SAME_OR_POSITIVE
        } else if let Ok(byte) = u8::try_from(delta.unsigned_abs()) {
            x_bytes.push(byte);
            kept | X_SHORT | if delta > 0 { X_SAME_OR_POSITIVE } else { 0 }
        } else {
            let word = i16::try_from(delta).map_err(|_| StretchError::OutOfRange {
                glyph,
                field: "x coordinate",
            })?;
            x_bytes.extend_from_slice(&word.to_be_bytes());
            kept
        };
        out.push(restated);
    }
    out.extend_from_slice(&x_bytes);
    out.extend_from_slice(ys);
    let min = scaled.iter().copied().min().unwrap_or(0);
    let max = scaled.iter().copied().max().unwrap_or(0);
    let field = |value: i32, field| {
        i16::try_from(value).map_err(|_| StretchError::OutOfRange { glyph, field })
    };
    Ok((field(min, "xMin")?, field(max, "xMax")?))
}

/// A composite glyph's components, each x offset scaled and each two-by-two conjugated.
///
/// A component is drawn as `T·p + o`, its points `p` already scaled by `S` here. The window draws
/// `S·(T·p + o)`, which is `(S·T·S⁻¹)·(S·p) + S·o`: so the offset's x is scaled, and of a
/// two-by-two only the two cross terms move — the one that adds y into x is multiplied by the
/// scale and the one that adds x into y divided by it. A scale alone, or one per axis, commutes
/// with `S` and is kept. Arguments that name points rather than offsets are kept too: they align
/// two points the scale has already moved together.
fn composite(data: &[u8], out: &mut Vec<u8>, scale: f32, glyph: u16) -> Result<(), StretchError> {
    let unreadable = || StretchError::Unreadable("a composite glyph's data does not read");
    let mut at = 10usize;
    let mut instructions = false;
    loop {
        let flags = be16(data, at).ok_or_else(unreadable)?;
        let component = be16(data, at.saturating_add(2)).ok_or_else(unreadable)?;
        at = at.checked_add(4).ok_or_else(unreadable)?;
        let words = flags & ARGS_ARE_WORDS != 0;
        let (first, second) = if words {
            (
                read_i16(data, at).map(i32::from),
                read_i16(data, at.saturating_add(2)).map(i32::from),
            )
        } else {
            (
                data.get(at)
                    .map(|byte| i32::from(i8::from_be_bytes([*byte]))),
                data.get(at.saturating_add(1))
                    .map(|byte| i32::from(i8::from_be_bytes([*byte]))),
            )
        };
        let raw_args = data
            .get(at..at.saturating_add(if words { 4 } else { 2 }))
            .ok_or_else(unreadable)?;
        at = at.checked_add(raw_args.len()).ok_or_else(unreadable)?;
        let transform_length = if flags & HAVE_A_SCALE != 0 {
            2
        } else if flags & HAVE_X_AND_Y_SCALE != 0 {
            4
        } else if flags & HAVE_TWO_BY_TWO != 0 {
            8
        } else {
            0
        };
        let mut transform = data
            .get(at..at.saturating_add(transform_length))
            .ok_or_else(unreadable)?
            .to_vec();
        at = at.checked_add(transform_length).ok_or_else(unreadable)?;
        instructions |= flags & HAVE_INSTRUCTIONS != 0;

        let mut restated_flags = flags;
        let mut args: Vec<u8> = raw_args.to_vec();
        if flags & ARGS_ARE_XY != 0 {
            let (dx, dy) = (
                first.ok_or_else(unreadable)?,
                second.ok_or_else(unreadable)?,
            );
            let dx = round(f64::from(dx) * f64::from(scale));
            args.clear();
            match (i8::try_from(dx), i8::try_from(dy)) {
                (Ok(dx), Ok(dy)) if !words => {
                    args.extend_from_slice(&dx.to_be_bytes());
                    args.extend_from_slice(&dy.to_be_bytes());
                }
                _ => {
                    let field = |value: i32| {
                        i16::try_from(value).map_err(|_| StretchError::OutOfRange {
                            glyph,
                            field: "component offset",
                        })
                    };
                    args.extend_from_slice(&field(dx)?.to_be_bytes());
                    args.extend_from_slice(&field(dy)?.to_be_bytes());
                    restated_flags |= ARGS_ARE_WORDS;
                }
            }
        }
        if flags & HAVE_TWO_BY_TWO != 0 {
            // xscale, scale01 (x into y), scale10 (y into x), yscale, each an F2Dot14.
            let x_into_y = read_i16(&transform, 2).ok_or_else(unreadable)?;
            let y_into_x = read_i16(&transform, 4).ok_or_else(unreadable)?;
            let f2dot14 = |value: f64| {
                i16::try_from(round(value)).map_err(|_| StretchError::OutOfRange {
                    glyph,
                    field: "component transform",
                })
            };
            let x_into_y = f2dot14(f64::from(x_into_y) / f64::from(scale))?;
            let y_into_x = f2dot14(f64::from(y_into_x) * f64::from(scale))?;
            put(&mut transform, 2, &x_into_y.to_be_bytes())?;
            put(&mut transform, 4, &y_into_x.to_be_bytes())?;
        }
        out.extend_from_slice(&restated_flags.to_be_bytes());
        out.extend_from_slice(&component.to_be_bytes());
        out.extend_from_slice(&args);
        out.extend_from_slice(&transform);
        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }
    if instructions {
        let length = usize::from(be16(data, at).ok_or_else(unreadable)?);
        let end = at
            .checked_add(2)
            .and_then(|at| at.checked_add(length))
            .ok_or_else(unreadable)?;
        out.extend_from_slice(data.get(at..end).ok_or_else(unreadable)?);
    }
    Ok(())
}

/// The horizontal extremes over every glyph with an outline, which `head` and `hhea` summarise.
#[derive(Default)]
struct Extent {
    /// `head`'s `xMin` and `xMax`.
    x: Option<(i16, i16)>,
    /// `hhea`'s `minLeftSideBearing`, `minRightSideBearing` and `xMaxExtent`.
    sides: Option<(i16, i16, i16)>,
}

impl Extent {
    fn include(&mut self, advance: u16, bearing: i16, (min, max): (i16, i16)) {
        self.x = Some(
            self.x
                .map_or((min, max), |(low, high)| (low.min(min), high.max(max))),
        );
        let width = i32::from(max).saturating_sub(i32::from(min));
        let extent = i32::from(bearing).saturating_add(width);
        let right = i32::from(advance).saturating_sub(extent);
        let clamp = |value: i32| {
            i16::try_from(value.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0)
        };
        let (right, extent) = (clamp(right), clamp(extent));
        self.sides = Some(
            self.sides
                .map_or((bearing, right, extent), |(left, low_right, high)| {
                    (left.min(bearing), low_right.min(right), high.max(extent))
                }),
        );
    }

    const fn horizontal(&self) -> Option<(i16, i16)> {
        self.x
    }

    const fn stated(&self) -> Option<(i16, i16, i16)> {
        self.sides
    }
}

/// `value` times `scale`, rounded, as an `i16` field named `field` of `glyph`.
fn scaled_i16(
    value: i16,
    scale: f32,
    glyph: u16,
    field: &'static str,
) -> Result<i16, StretchError> {
    i16::try_from(round(f64::from(value) * f64::from(scale)))
        .map_err(|_| StretchError::OutOfRange { glyph, field })
}

/// The nearest integer, halves away from zero; a value past `i32` saturates, which every caller
/// then refuses as a field it does not fit.
fn round(value: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded first; a value outside i32 saturates and the field it is for refuses it"
    )]
    let whole = value.round() as i32;
    whole
}

fn read_i16(data: &[u8], at: usize) -> Option<i16> {
    be16(data, at).map(|value| i16::from_be_bytes(value.to_be_bytes()))
}

fn put(table: &mut [u8], at: usize, bytes: &[u8]) -> Result<(), StretchError> {
    table
        .get_mut(at..at.saturating_add(bytes.len()))
        .ok_or(StretchError::Unreadable(
            "a table is too short for a field it states",
        ))?
        .copy_from_slice(bytes);
    Ok(())
}

/// Every point of a glyph as a reader draws it, unhinted and in the program's own units.
#[derive(Default)]
struct Points(Vec<(f32, f32)>);

impl OutlinePen for Points {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((x, y));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.extend([(cx, cy), (x, y)]);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.extend([(cx0, cy0), (cx1, cy1), (x, y)]);
    }
    fn close(&mut self) {}
}

/// One glyph's points through `read-fonts`' reader, or `None` where it draws nothing.
fn drawn(program: &[u8], glyph: u16) -> Option<Points> {
    use skrifa::MetadataProvider as _;
    let font = skrifa::FontRef::new(program).ok()?;
    let mut points = Points::default();
    font.outline_glyphs()
        .get(skrifa::GlyphId::from(glyph))?
        .draw(skrifa::instance::Size::unscaled(), &mut points)
        .ok()?;
    Some(points)
}

/// The rewritten program read back, every glyph against the scaled original (ADR 0988: prove
/// before writing): the same points, each y the same, each x within [`PROVED_WITHIN`] of the
/// original's times the scale, and every advance the one it was.
fn proved(original: &[u8], scaled: &[u8], scale: f32, glyphs: u16) -> Result<(), StretchError> {
    let advances = |program: &[u8]| -> Option<BTreeMap<u16, f32>> {
        use skrifa::MetadataProvider as _;
        let font = skrifa::FontRef::new(program).ok()?;
        let metrics = font.glyph_metrics(
            skrifa::instance::Size::unscaled(),
            skrifa::instance::LocationRef::default(),
        );
        Some(
            (0..glyphs)
                .filter_map(|glyph| {
                    Some((glyph, metrics.advance_width(skrifa::GlyphId::from(glyph))?))
                })
                .collect(),
        )
    };
    if advances(original) != advances(scaled) {
        return Err(StretchError::NotProved { glyph: 0 });
    }
    for glyph in 0..glyphs {
        let agrees = match (drawn(original, glyph), drawn(scaled, glyph)) {
            (Some(before), Some(after)) => {
                before.0.len() == after.0.len()
                    && before.0.iter().zip(&after.0).all(|((x0, y0), (x1, y1))| {
                        (x0 * scale - x1).abs() <= PROVED_WITHIN && y0.to_bits() == y1.to_bits()
                    })
            }
            (None, None) => true,
            _ => false,
        };
        if !agrees {
            return Err(StretchError::NotProved { glyph });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "stretch_tests.rs"]
mod tests;
