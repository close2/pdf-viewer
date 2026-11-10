//! The fitting written into a face whose outlines are Compact Font Format charstrings inside an
//! OpenType wrapper, as the faces of this machine's URW collection are (`doc/adr/2789`).
//!
//! The charstrings are stated again by [`crate::cff::with_outlines_scaled`], on the 16.16 grid a
//! charstring states numbers on. The horizontal metrics that are x coordinates follow, as
//! [`super::sfnt_outlines`] makes them follow in a `glyf` face: each left side bearing (ISO/IEC
//! 14496-22's `hmtx`, the glyph's left edge, which a positive scale about the origin multiplies),
//! `head`'s box and `hhea`'s three extremes, the last two from the outlines read back. The
//! advances, and every other table, are copied byte for byte.
//!
//! **A bare CFF program is refused.** The reader this tree draws one with, `read-fonts`'
//! `CffFontRef::draw` at no pixel size, states each coordinate as its whole part, so a point on
//! the 16.16 grid is drawn at its floor and a short segment can vanish; whole units, the
//! alternative, make two points a unit apart one point at any scale below one. The OpenType
//! reader keeps the fraction, and every fitted face the censuses reached is OpenType.

use skrifa::outline::OutlinePen;

use super::{Extent, HHEA_EXTREMES, NUMBER_OF_H_METRICS, PROVED_WITHIN, StretchError, put, round};
use crate::sfnt::{be16, horizontal_metric, sfnt_tables, with_tables};

/// The em a charstring's coordinates are read against where the `FontMatrix` is the default one
/// Technical Note #5176 gives, `[0.001 0 0 0.001 0 0]`.
const DEFAULT_EM: f32 = 1000.0;

/// `program`, an OpenType face whose outlines are a `CFF ` table, with every outline's x
/// multiplied by `scale` about the glyph's origin.
///
/// # Errors
///
/// [`StretchError`]: a scale this reader never draws at; a program that is not such a face,
/// states a matrix other than the default one, or whose tables do not read; a charstring that
/// cannot be stated again; or a read-back that disagrees.
pub fn cff_outlines(program: &[u8], scale: f32) -> Result<Vec<u8>, StretchError> {
    if !(0.5..1.0).contains(&scale) {
        return Err(StretchError::Scale(scale));
    }
    // An sfnt states its version first (ISO/IEC 14496-22's table directory); `OTTO` is the one
    // whose outlines are CFF.
    if program.get(..4) != Some(b"OTTO".as_slice()) {
        return Err(StretchError::Unreadable(
            "it is not an OpenType face with a `CFF ` table",
        ));
    }
    let tables = sfnt_tables(program).ok_or(StretchError::Unreadable("no table directory"))?;
    let table = |tag: &[u8; 4]| {
        let (at, length) = *tables
            .get(tag.as_slice())
            .ok_or(StretchError::Unreadable("a table it needs is absent"))?;
        program
            .get(at..at.saturating_add(length))
            .ok_or(StretchError::Unreadable("a table runs past the program"))
    };
    let (cff, head, hhea, hmtx) = (
        table(b"CFF ")?,
        table(b"head")?,
        table(b"hhea")?,
        table(b"hmtx")?,
    );
    let em = crate::cff::units_per_em(cff).map_err(|_| StretchError::Unreadable("CFF"))?;
    if em.to_bits() != DEFAULT_EM.to_bits() {
        return Err(StretchError::Unreadable(
            "its FontMatrix is not the default one, so a charstring unit is not the reader's",
        ));
    }
    let scaled_cff =
        crate::cff::with_outlines_scaled(cff, f64::from(scale)).map_err(|error| match error {
            crate::cff::CffError::AdvanceNotRestatable { glyph, .. } => {
                StretchError::NotRestated { glyph }
            }
            _ => StretchError::Unreadable("CFF"),
        })?;
    let outlines = with_tables(program, &[(b"CFF ", scaled_cff.clone())]).ok_or(
        StretchError::Unreadable("the tables could not be laid out again"),
    )?;
    let glyphs = crate::cff::glyph_count(cff)
        .ok()
        .and_then(|count| u16::try_from(count).ok())
        .ok_or(StretchError::Unreadable("CFF"))?;
    proved(program, &outlines, scale, glyphs)?;
    let pairs =
        usize::from(be16(hhea, NUMBER_OF_H_METRICS).ok_or(StretchError::Unreadable("hhea"))?);
    let mut new_hmtx = Vec::with_capacity(hmtx.len());
    let mut extent = Extent::default();
    for glyph in 0..glyphs {
        let index = usize::from(glyph);
        let (advance, bearing) = horizontal_metric(hmtx, pairs, index);
        let bearing =
            i16::try_from(round(f64::from(bearing) * f64::from(scale))).map_err(|_| {
                StretchError::OutOfRange {
                    glyph,
                    field: "left side bearing",
                }
            })?;
        if index < pairs {
            new_hmtx.extend_from_slice(&advance.to_be_bytes());
        }
        new_hmtx.extend_from_slice(&bearing.to_be_bytes());
        if let Some((min, max)) = drawn(&outlines, glyph).and_then(|drawn| drawn.horizontal()) {
            extent.include(advance, bearing, (min, max));
        }
    }
    let mut new_hhea = hhea.to_vec();
    let mut new_head = head.to_vec();
    if let (Some((min_left, min_right, max_extent)), Some((x_min, x_max))) =
        (extent.stated(), extent.horizontal())
    {
        for (at, value) in [(0, min_left), (2, min_right), (4, max_extent)] {
            put(
                &mut new_hhea,
                HHEA_EXTREMES.saturating_add(at),
                &value.to_be_bytes(),
            )?;
        }
        put(&mut new_head, super::HEAD_X_MIN, &x_min.to_be_bytes())?;
        put(
            &mut new_head,
            super::HEAD_X_MIN.saturating_add(4),
            &x_max.to_be_bytes(),
        )?;
    }
    with_tables(
        program,
        &[
            (b"CFF ", scaled_cff),
            (b"hmtx", new_hmtx),
            (b"head", new_head),
            (b"hhea", new_hhea),
        ],
    )
    .ok_or(StretchError::Unreadable(
        "the tables could not be laid out again",
    ))
}

/// The rewritten face read back through the OpenType reader, every glyph against the scaled
/// original (ADR 0988: prove before writing): the same commands, each y the same, each x within
/// [`PROVED_WITHIN`] of the original's times the scale — a point stands within 2⁻¹⁷ of a unit,
/// and an accent's offset as near — and every advance and charstring width the one it was.
fn proved(original: &[u8], scaled: &[u8], scale: f32, glyphs: u16) -> Result<(), StretchError> {
    let widths = |program: &[u8]| {
        let tables = sfnt_tables(program)?;
        let (at, length) = *tables.get(b"CFF ".as_slice())?;
        let cff = program.get(at..at.checked_add(length)?)?;
        crate::cff::charstring_widths(cff, &(0..glyphs).collect::<Vec<_>>()).ok()
    };
    if widths(original) != widths(scaled) {
        return Err(StretchError::NotProved { glyph: 0 });
    }
    for glyph in 0..glyphs {
        let agrees = match (drawn(original, glyph), drawn(scaled, glyph)) {
            (Some(before), Some(after)) => drawn_alike(&before, &after, scale),
            (None, None) => true,
            _ => false,
        };
        if !agrees {
            return Err(StretchError::NotProved { glyph });
        }
    }
    Ok(())
}

/// Whether `after` draws `before` at `scale`: command for command, each y the same and each x
/// within [`PROVED_WITHIN`] of `x × scale`, except that a line of `before` whose scaled length is
/// within that distance and whose y does not move may be missing — the writer does not state a
/// line whose two ends round to one point (`crate::cff::with_outlines_scaled`).
fn drawn_alike(before: &Commands, after: &Commands, scale: f32) -> bool {
    let mut stated = after.0.iter().peekable();
    let mut previous: Option<&(u8, f32, f32)> = None;
    for was in &before.0 {
        let matches = stated.peek().is_some_and(|now| {
            was.0 == now.0
                && (was.1 * scale - now.1).abs() <= PROVED_WITHIN
                && was.2.to_bits() == now.2.to_bits()
        });
        if matches {
            stated.next();
        } else {
            let vanished = was.0 == LINE
                && previous.is_some_and(|last| {
                    ((was.1 - last.1) * scale).abs() <= PROVED_WITHIN
                        && was.2.to_bits() == last.2.to_bits()
                });
            if !vanished {
                return false;
            }
        }
        previous = Some(was);
    }
    stated.next().is_none()
}

/// One glyph as the OpenType reader draws it, unhinted and in the face's units, or `None` where it
/// does not draw.
fn drawn(program: &[u8], glyph: u16) -> Option<Commands> {
    use skrifa::MetadataProvider as _;
    let font = skrifa::FontRef::new(program).ok()?;
    let mut commands = Commands::default();
    font.outline_glyphs()
        .get(skrifa::GlyphId::from(glyph))?
        .draw(skrifa::instance::Size::unscaled(), &mut commands)
        .ok()?;
    Some(commands)
}

/// Every point a glyph's outline states, each with the kind of command that states it — a move,
/// a line, or one of a curve's three points — so that a read-back is held to the same commands
/// and not only the same points.
#[derive(Default)]
struct Commands(Vec<(u8, f32, f32)>);

impl Commands {
    /// The outline's leftmost and rightmost x, control points included, outward to whole units.
    fn horizontal(&self) -> Option<(i16, i16)> {
        let mut xs = self.0.iter().map(|(_, x, _)| *x);
        let first = xs.next()?;
        let (min, max) = xs.fold((first, first), |(min, max), x| (min.min(x), max.max(x)));
        let whole = |value: f32| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a whole number from a 16.16 coordinate; the i16 conversion checks range"
            )]
            i16::try_from(value as i32).ok()
        };
        Some((whole(min.floor())?, whole(max.ceil())?))
    }
}

/// [`Commands`]' kinds.
const MOVE: u8 = 0;
/// See [`MOVE`].
const LINE: u8 = 1;
/// See [`MOVE`]; a quadratic is a kind of its own, though a charstring never states one.
const QUAD: u8 = 2;
/// See [`MOVE`].
const CURVE: u8 = 3;
/// See [`MOVE`].
const CLOSE: u8 = 4;

impl OutlinePen for Commands {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((MOVE, x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((LINE, x, y));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.extend([(QUAD, cx, cy), (QUAD, x, y)]);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .extend([(CURVE, cx0, cy0), (CURVE, cx1, cy1), (CURVE, x, y)]);
    }
    fn close(&mut self) {
        self.0.push((CLOSE, 0.0, 0.0));
    }
}

#[cfg(test)]
#[path = "stretch_cff_tests.rs"]
mod tests;
