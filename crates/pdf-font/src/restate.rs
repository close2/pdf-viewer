//! A font program with its advances restated to the widths a font dictionary already states.
//!
//! # The one thing a converter may change about a font, and why it is that one
//!
//! ISO 32000-2 §9.2.4 says where a glyph's advance is read from, and says it of the *dictionary*:
//!
//! > Storing this information in the font dictionary, although redundant, enables a PDF processor
//! > to determine glyph positioning without having to look inside the font program.
//!
//! So `/Widths` and `/W` are what put a glyph on the page, and the number inside the program is
//! not consulted for positioning at all. ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section
//! 6.2.10.5 all the same require the two to agree, which leaves a file whose producer let them
//! drift with exactly two futures: restate the dictionary, which moves every glyph after the one
//! that changed, or restate the program, which moves nothing.
//! `doc/pdf-a-conversion-limits.md` section 4.9 calls the first of those **never** and the second
//! the mechanical route, and this module is the second.
//!
//! **Nothing here touches an outline.** For an sfnt the rewrite is `hmtx` and the one `hhea`
//! field that bounds it; for a CFF it is the leading width operand of the named glyphs'
//! charstrings, defined by Adobe Technical Note #5177 section 3.1, with every other byte of the
//! charstring copied. [`tests::an_outline_survives_its_advance_being_restated`] draws every glyph
//! of every compiled-in face before and after and compares the two, because "the outline is
//! untouched" is a claim worth checking rather than asserting.
//!
//! # The same restatement going down the page, and why it is safer still
//!
//! ISO 19005-4 section 6.2.10.5 asks the question a second time of a composite font set in
//! writing mode 1: where the program states vertical metrics, they shall agree with the
//! §9.7.4.3 `/DW2` and `/W2` entries of the `CIDFont` dictionary. [`with_vertical_advances`] is
//! that restatement, and it runs in the same direction as the horizontal one — the program is
//! rewritten, never the dictionary — on an argument the standard makes outright rather than on
//! §9.2.4's inference. §9.9.1:
//!
//! > The "vhea" and "vmtx" tables that specify vertical metrics shall never be used by a PDF
//! > processor. The only way to specify vertical metrics in PDF shall be by means of the DW2
//! > and W2 entries in a CIDFont dictionary.
//!
//! So the side that may move is not a judgement about which statement is authoritative: a
//! conforming processor is *forbidden* to read the one this rewrites, and rewriting the other
//! would move every glyph on a vertical line on the authority of a table no reader may consult.
//! What the PDF/A requirement is for follows from the same sentence — an archive internally
//! consistent for a reader that is not a PDF processor, which is `/CIDSet`'s and `/CharSet`'s
//! motive too.
//!
//! # What a caller supplies, and in what units
//!
//! Widths in the scale [`crate::LoadedFont::advance`] answers in — thousandths of a text space
//! unit, as a fraction of the em — so that a caller comparing the dictionary against
//! [`crate::LoadedFont::program_advance`] can hand this the same numbers it compared. The
//! program's own units per em is read here and the rounding checked against the tolerance both
//! parts state, because a program whose em is coarse enough that no integer advance lands within
//! it is one to decline rather than to restate approximately.

use std::collections::BTreeMap;

use crate::substitute::Format;
use crate::{cff, sfnt};

/// The tolerance both parts allow between a dictionary's width and a program's, in ems.
///
/// ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 state the same number: a
/// thousandth of a text space unit. It appears here because the *rounding* this module does has
/// to land inside it — an advance is an integer in the program's own units, and a font whose em
/// is coarse cannot state every width a dictionary can.
const CONSISTENT: f32 = 0.001;

/// Offset of `numOfLongVerMetrics` within `vhea`, which is that table's last field.
///
/// ISO/IEC 14496-22 lays `vhea` out field for field as `hhea`, so this is also `hhea`'s
/// `numberOfHMetrics` — [`sfnt`] reads it there under its own name.
const NUMBER_OF_LONG_METRICS: usize = 34;

/// Offset of `numGlyphs` within `maxp`.
const NUM_GLYPHS: usize = 4;

/// Offset of `checkSumAdjustment` within `head`.
const CHECKSUM_ADJUSTMENT: usize = 8;

/// Why a font program's advances could not be restated.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RestateError {
    /// The program could not be read far enough to find where its advances are stated.
    #[error("the font program's advances could not be located: {detail}")]
    Unreadable {
        /// What the reader reported.
        detail: String,
    },
    /// A glyph's advance is stated somewhere this does not rewrite.
    #[error("{0}")]
    NotRestatable(#[from] cff::CffError),
    /// The program's units per em is too coarse for the width asked for.
    ///
    /// An advance is an integer in the program's own units, so the widths a program can state
    /// are spaced one unit apart. A font whose em is a thousand units states any width a
    /// `/Widths` array can to within half a thousandth; one whose em is sixteen cannot, and
    /// restating it would leave the two statements as far apart as they began.
    #[error(
        "the width {width} of glyph {glyph} cannot be stated in a program of {units} units per \
         em: the nearest it holds is {nearest}, which is further from it than ISO 19005 allows \
         the two statements to be"
    )]
    TooCoarse {
        /// Which glyph.
        glyph: u16,
        /// The width the dictionary states, in ems.
        width: f32,
        /// The program's units per em.
        units: f32,
        /// The nearest width the program can state, in ems.
        nearest: f32,
    },
    /// An advance that does not fit the field an sfnt states it in.
    ///
    /// ISO/IEC 14496-22 gives both `hmtx` and `vmtx` a `uint16` advance in design units, so a
    /// dictionary asking for more than 65 535 of them is asking for a number the program has
    /// nowhere to put. It is refused rather than clamped: a clamped advance is a wrong number
    /// written silently into a program, which is the one outcome this module exists against.
    #[error(
        "the advance of {units} design units asked for glyph {glyph} does not fit the uint16 \
         field an sfnt states an advance in"
    )]
    OutOfRange {
        /// Which glyph.
        glyph: u16,
        /// The advance asked for, in the program's own units.
        units: i64,
    },
    /// The program states no vertical metrics, so there is nothing to restate.
    ///
    /// ISO 19005-4 section 6.2.10.5's vertical requirement is conditional on the program
    /// stating them at all, and a face never meant to be set vertically carries no `vhea` and
    /// no `vmtx` — which is most faces. A bare CFF program reaches this too: vertical metrics
    /// live in the sfnt wrapper around a `CFF ` table and not in the table.
    #[error(
        "the font program states no vhea and vmtx, which is where an sfnt's vertical \
             advances are"
    )]
    NoVerticalMetrics,
    /// A glyph whose vertical advance the program states only by inheritance.
    ///
    /// ISO/IEC 14496-22 gives `vmtx` `numOfLongVerMetrics` advance-and-bearing pairs followed
    /// by top side bearings alone, the last stated advance applying to every glyph past the
    /// pairs. So a glyph in that tail has no advance of its own to overwrite, and stating one
    /// for it means lengthening the table — which would restate the advance of every other
    /// glyph in the tail at the same time, and none of those was asked for.
    #[error(
        "glyph {glyph} takes its advance height from the last of the program's {pairs} long \
         vertical metrics, so restating it alone would mean lengthening the vmtx table and \
         restating every glyph past the pairs with it"
    )]
    InheritedVerticalAdvance {
        /// Which glyph.
        glyph: u16,
        /// How many long vertical metrics the program states.
        pairs: u16,
    },
}

/// The program with the advance of each named glyph restated as the width the caller states.
///
/// `widths` is keyed by glyph index in the program's own numbering and valued in ems, which is
/// the scale [`crate::LoadedFont::advance`] and [`crate::LoadedFont::program_advance`] both
/// answer in. A glyph the map does not name keeps the advance the program gave it.
///
/// # Errors
///
/// [`RestateError`]: a program whose advances cannot be located, a charstring whose width is
/// stated somewhere this does not follow, or an em too coarse to hold the width asked for.
pub fn with_widths(
    program: &[u8],
    format: Format,
    widths: &BTreeMap<u16, f32>,
) -> Result<Vec<u8>, RestateError> {
    let units = units_per_em(program, format)?;
    let mut design = BTreeMap::new();
    for (glyph, width) in widths {
        let rounded = (width * units).round();
        let nearest = rounded / units;
        if (nearest - width).abs() > CONSISTENT {
            return Err(RestateError::TooCoarse {
                glyph: *glyph,
                width: *width,
                units,
                nearest,
            });
        }
        design.insert(*glyph, rounded);
    }
    match format {
        Format::BareCff => {
            let advances = design
                .iter()
                .map(|(glyph, units)| (*glyph, whole(*units)))
                .collect();
            Ok(cff::with_advances(program, &advances)?)
        }
        Format::Sfnt => {
            let mut advances: BTreeMap<u16, u16> = BTreeMap::new();
            for (glyph, units) in &design {
                advances.insert(*glyph, sfnt_advance(*glyph, *units)?);
            }
            sfnt::with_advances(program, &advances).ok_or_else(|| RestateError::Unreadable {
                detail: "the program states no readable hhea, hmtx and maxp, which is where an \
                         sfnt's advances are"
                    .to_owned(),
            })
        }
    }
}

/// The program with the vertical advance of each named glyph restated as the caller states it.
///
/// ISO 19005-4 section 6.2.10.5's third paragraph is the requirement, [`with_widths`] is the
/// same act across the page, and this module's header is why the program is the side that moves
/// in both directions — for the vertical one, because §9.9.1 forbids a PDF processor from
/// reading the table this rewrites at all.
///
/// `displacements` is keyed by glyph index in the program's own numbering and valued in the
/// quantity [`crate::LoadedFont::program_vertical_advance`] and
/// [`crate::LoadedFont::vertical_metrics`] both answer in: ems, **negative going down the
/// page**, which is §9.7.4.3's sign for `w1` rather than `vmtx`'s. A caller comparing the
/// dictionary's number against the program's therefore hands this the same number it compared,
/// with no sign to get backwards. A glyph the map does not name keeps the advance the program
/// gave it.
///
/// # Errors
///
/// [`RestateError`]: a program with no vertical metrics at all (which is most programs, and
/// which makes the requirement vacuous rather than failed), a glyph whose advance the program
/// states only by inheritance, an em too coarse to hold the height asked for, and an advance
/// past what `vmtx`'s `uint16` holds.
pub fn with_vertical_advances(
    program: &[u8],
    format: Format,
    displacements: &BTreeMap<u16, f32>,
) -> Result<Vec<u8>, RestateError> {
    if format != Format::Sfnt {
        return Err(RestateError::NoVerticalMetrics);
    }
    let units = units_per_em(program, format)?;
    let vertical = VerticalMetrics::of(program)?;
    let mut edits: BTreeMap<usize, u16> = BTreeMap::new();
    for (glyph, displacement) in displacements {
        // §9.7.4.3's `w1` runs down the page and `vmtx` states a distance, so the sign is the
        // caller's convention rather than the table's and is taken off here, once.
        let height = -displacement;
        let rounded = (height * units).round();
        let nearest = rounded / units;
        if (nearest - height).abs() > CONSISTENT {
            return Err(RestateError::TooCoarse {
                glyph: *glyph,
                width: height,
                units,
                nearest,
            });
        }
        edits.insert(vertical.advance_at(*glyph)?, sfnt_advance(*glyph, rounded)?);
    }
    overwritten(program, *b"vmtx", &edits).ok_or_else(|| RestateError::Unreadable {
        detail: "the program's vmtx record could not be rewritten where its directory says it is"
            .to_owned(),
    })
}

/// One glyph's vertical advance as the program states it, in ems and negative going down.
///
/// The program-side counterpart of [`advance`], and the same quantity
/// [`crate::LoadedFont::program_vertical_advance`] answers — asked of a program on its own, so
/// that a caller holding nothing but bytes can check that a restatement landed where it said it
/// would.
///
/// `None` for a program that is not an sfnt, for one stating no `vhea` and `vmtx`, and for a
/// glyph the program does not hold.
#[must_use]
pub fn vertical_advance(program: &[u8], format: Format, glyph: u16) -> Option<f32> {
    if format != Format::Sfnt {
        return None;
    }
    let units = units_per_em(program, format).ok()?;
    let vertical = VerticalMetrics::of(program).ok()?;
    if glyph >= vertical.glyphs {
        return None;
    }
    // ISO/IEC 14496-22: the last stated advance applies to every glyph past the pairs, which is
    // how a face whose glyphs are all one em tall states one advance for thousands of them.
    let paired = glyph.min(vertical.pairs.saturating_sub(1));
    let at = vertical
        .vmtx
        .checked_add(usize::from(paired).checked_mul(4)?)?;
    Some(-f32::from(be16(program, at)?) / units)
}

/// Where an sfnt states its vertical metrics, and how many of them it states.
///
/// `vhea`'s layout is `hhea`'s field for field (ISO/IEC 14496-22), which is why one offset
/// serves both: the count of long metrics is the table's last field.
struct VerticalMetrics {
    /// Where `vmtx` begins in the program.
    vmtx: usize,
    /// How long the `vmtx` record is.
    length: usize,
    /// `numOfLongVerMetrics`, which is how many glyphs state an advance of their own.
    pairs: u16,
    /// `maxp`'s `numGlyphs`.
    glyphs: u16,
}

impl VerticalMetrics {
    /// Reads the three numbers from a program, or says the program states no vertical metrics.
    fn of(program: &[u8]) -> Result<Self, RestateError> {
        let tables = sfnt::sfnt_tables(program).ok_or_else(|| RestateError::Unreadable {
            detail: "the program states no readable table directory".to_owned(),
        })?;
        let (vhea, vhea_length) = *tables
            .get(b"vhea".as_slice())
            .ok_or(RestateError::NoVerticalMetrics)?;
        let (vmtx, length) = *tables
            .get(b"vmtx".as_slice())
            .ok_or(RestateError::NoVerticalMetrics)?;
        let (maxp, _) =
            *tables
                .get(b"maxp".as_slice())
                .ok_or_else(|| RestateError::Unreadable {
                    detail: "the program states no maxp, which is where its glyph count is"
                        .to_owned(),
                })?;
        if vhea_length < NUMBER_OF_LONG_METRICS.saturating_add(2) {
            return Err(RestateError::NoVerticalMetrics);
        }
        let pairs = vhea
            .checked_add(NUMBER_OF_LONG_METRICS)
            .and_then(|at| be16(program, at))
            .ok_or(RestateError::NoVerticalMetrics)?;
        let glyphs = maxp
            .checked_add(NUM_GLYPHS)
            .and_then(|at| be16(program, at))
            .ok_or_else(|| RestateError::Unreadable {
                detail: "the program's maxp is too short to state a glyph count".to_owned(),
            })?;
        if pairs == 0 {
            return Err(RestateError::NoVerticalMetrics);
        }
        Ok(Self {
            vmtx,
            length,
            pairs,
            glyphs,
        })
    }

    /// Where one glyph's own advance height sits in the program, for a glyph that has one.
    ///
    /// [`RestateError::InheritedVerticalAdvance`] for a glyph in the tail past the long
    /// metrics: it has no field of its own, and giving it one means lengthening the table.
    fn advance_at(&self, glyph: u16) -> Result<usize, RestateError> {
        if glyph >= self.pairs {
            return Err(RestateError::InheritedVerticalAdvance {
                glyph,
                pairs: self.pairs,
            });
        }
        let at = usize::from(glyph)
            .checked_mul(4)
            .filter(|at| at.saturating_add(2) <= self.length)
            .ok_or_else(|| RestateError::Unreadable {
                detail: format!(
                    "the program's vmtx record is {} bytes, too short for the {} long vertical \
                     metrics its vhea states",
                    self.length, self.pairs
                ),
            })?;
        self.vmtx
            .checked_add(at)
            .ok_or_else(|| RestateError::Unreadable {
                detail: "the program's vmtx record does not lie inside it".to_owned(),
            })
    }
}

/// The program with 16-bit fields of one table overwritten in place, checksums carried along.
///
/// `edits` is keyed by offset **in the program** rather than in the table, because that is what
/// [`VerticalMetrics::advance_at`] answers and what bounds each edit to the table `table` names;
/// this function adjusts that record's checksum and does not check the offsets against it.
///
/// # Why this rewrites in place where [`sfnt::with_advances`] appends a new table
///
/// The horizontal restatement has to lengthen `hmtx` — it gives every glyph an advance of its
/// own, because a font that stated one advance for its whole tail cannot state a different one
/// for a single glyph of it — so the table moves to the end of the file and the directory
/// record follows it. Nothing of the sort is needed here: every advance this writes already has
/// a field, [`VerticalMetrics::advance_at`] having refused the glyphs that do not, so the edit
/// is a handful of bytes where they already are and every other byte of the program — every
/// offset, every length, every outline — is the producer's.
///
/// # The two checksums, adjusted rather than recomputed
///
/// ISO/IEC 14496-22 states both: a directory record's `checkSum` is the sum of its table's
/// 32-bit words, and `head`'s `checkSumAdjustment` is `0xB1B0AFBA` less the same sum taken
/// over the whole file with that field zero. Overwriting a `uint16` changes exactly one word of
/// one table, so both sums move by an amount this can compute exactly — the table's by the
/// word's own difference, and the file's by twice it, because the record's `checkSum` field is
/// itself a word of the file.
///
/// Adjusting rather than recomputing is deliberate and is `doc/adr/0947`'s first rule read down
/// to the byte: a program whose producer's checksums were already wrong keeps exactly the error
/// it arrived with, because its checksums are not what any requirement asked to be changed. For
/// a program whose checksums were right, adjusting and recomputing give the same file.
fn overwritten(program: &[u8], table: [u8; 4], edits: &BTreeMap<usize, u16>) -> Option<Vec<u8>> {
    let mut out = program.to_vec();
    let entry = directory_entry(program, table)?;
    let mut delta = 0u32;
    for (at, value) in edits {
        // A `uint16` at an even offset in a table that itself begins on a four-byte boundary
        // lies inside one word of the file, so one read before and one after states the whole
        // difference the edit makes to both sums.
        let word = at.checked_sub(at % 4)?;
        let before = sfnt::be32(&out, word)?;
        out.get_mut(*at..at.checked_add(2)?)?
            .copy_from_slice(&value.to_be_bytes());
        let after = sfnt::be32(&out, word)?;
        delta = delta.wrapping_add(after.wrapping_sub(before));
    }
    let checksum = entry.checked_add(4)?;
    let stated = sfnt::be32(&out, checksum)?.wrapping_add(delta);
    out.get_mut(checksum..checksum.checked_add(4)?)?
        .copy_from_slice(&stated.to_be_bytes());
    let record = directory_entry(program, *b"head")?;
    let head_at = usize::try_from(sfnt::be32(&out, record.checked_add(8)?)?).ok()?;
    let adjustment = head_at.checked_add(CHECKSUM_ADJUSTMENT)?;
    let stated_adjustment = sfnt::be32(&out, adjustment)?.wrapping_sub(delta.wrapping_mul(2));
    out.get_mut(adjustment..adjustment.checked_add(4)?)?
        .copy_from_slice(&stated_adjustment.to_be_bytes());
    Some(out)
}

/// Where one table's directory record begins, found by tag.
///
/// By tag rather than by position: the directory is sorted by tag and nothing here has any
/// business assuming where a record sits.
fn directory_entry(program: &[u8], table: [u8; 4]) -> Option<usize> {
    let count = usize::from(be16(program, 4)?);
    (0..count)
        .map(|index| 12usize.saturating_add(index.saturating_mul(16)))
        .find(|at| {
            at.checked_add(4)
                .and_then(|end| program.get(*at..end))
                .is_some_and(|tag| tag == table.as_slice())
        })
}

/// A rounded design-unit advance as the `uint16` an sfnt states it in.
///
/// The upper bound is the format's: ISO/IEC 14496-22 gives `hmtx` and `vmtx` a `uint16`
/// advance, so a number past it is refused by name rather than clamped into a wrong one.
fn sfnt_advance(glyph: u16, units: f32) -> Result<u16, RestateError> {
    let whole = whole(units);
    u16::try_from(whole).map_err(|_| RestateError::OutOfRange {
        glyph,
        units: whole,
    })
}

/// A table's leading two bytes read as a big-endian `u16`.
fn be16(data: &[u8], at: usize) -> Option<u16> {
    let pair = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([*pair.first()?, *pair.get(1)?]))
}

/// One glyph's advance as the program states it, in ems.
///
/// The same quantity [`crate::LoadedFont::program_advance`] answers, asked of a program on its
/// own rather than of a loaded font: a converter choosing between
/// `doc/pdf-a-conversion-limits.md` section 4.9's two metric routes has a face in hand and no
/// document to load it through, and route 1 is precisely the question "does this face already
/// state the width the file states".
///
/// `None` for a glyph the program does not hold, and for a program whose advances cannot be
/// read at all.
#[must_use]
pub fn advance(program: &[u8], format: Format, glyph: u16) -> Option<f32> {
    let units = units_per_em(program, format).ok()?;
    let stated = match format {
        Format::BareCff => cff::advances(program, &[glyph])
            .ok()?
            .first()
            .copied()
            .flatten()?,
        Format::Sfnt => sfnt::advance(program, glyph)?,
    };
    Some(stated / units)
}

/// A rounded design-unit advance as a whole number, clamped to what a font can state.
///
/// An advance the program can hold is a non-negative integer — `hmtx` states it as a `uint16`
/// and a CFF charstring's width operand as a difference from `nominalWidthX` — so a value
/// outside that is one the caller's own widths put there, and clamping it here would be silent.
/// It is not silent: [`with_widths`] has already checked the width against the em,
/// [`sfnt_advance`] refuses one past the `uint16` an sfnt states it in, and
/// [`cff::with_advances`] refuses a difference that will not fit its operand. **The upper bound
/// used to be silent**, clamped with an `unwrap_or(0)` on the sfnt path that turned a width past
/// 65 535 design units into an advance of zero; `RestateError::OutOfRange` is that bound named.
fn whole(units: f32) -> i64 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value has been rounded, and a width outside i64 is refused by the caller \
                  that states it"
    )]
    let value = units as i64;
    value.max(0)
}

/// The program's units per em, which is the scale its stated advances are in.
fn units_per_em(program: &[u8], format: Format) -> Result<f32, RestateError> {
    let units = match format {
        Format::BareCff => {
            cff::units_per_em(program).map_err(|error| RestateError::Unreadable {
                detail: error.to_string(),
            })?
        }
        Format::Sfnt => sfnt::units_per_em(program).ok_or_else(|| RestateError::Unreadable {
            detail: "the program states no readable head table, which is where an sfnt's \
                         units per em is"
                .to_owned(),
        })?,
    };
    if units > 0.0 {
        Ok(units)
    } else {
        Err(RestateError::Unreadable {
            detail: format!("the program states {units} units per em"),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use skrifa::outline::OutlinePen;

    use super::{RestateError, with_widths};
    use crate::standard::face;
    use crate::substitute::{Family, Format, Request};

    /// One glyph's outline as a list of numbers, so that two can be compared exactly.
    ///
    /// The coordinates are quantised to a thousandth of a unit before comparison, because the
    /// question is whether the *path* is the one the program drew and not whether two `f32`
    /// additions came out bit-identical.
    #[derive(Default, PartialEq, Eq, Debug)]
    struct Traced(Vec<i64>);

    impl Traced {
        /// One segment: its kind, then its coordinates.
        fn segment(&mut self, kind: i64, points: &[f32]) {
            self.0.push(kind);
            for point in points {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a glyph coordinate is bounded by the em square many times over"
                )]
                self.0.push((f64::from(*point) * 1000.0).round() as i64);
            }
        }
    }

    impl OutlinePen for Traced {
        fn move_to(&mut self, x: f32, y: f32) {
            self.segment(1, &[x, y]);
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.segment(2, &[x, y]);
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.segment(3, &[cx, cy, x, y]);
        }
        fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
            self.segment(4, &[ax, ay, bx, by, x, y]);
        }
        fn close(&mut self) {
            self.segment(5, &[]);
        }
    }

    /// Every compiled-in face, with the reader its bytes belong to.
    pub(super) fn compiled_in() -> Vec<(&'static str, &'static [u8], Format)> {
        let mut faces = Vec::new();
        for (name, family) in [
            ("sans", Family::SansSerif),
            ("serif", Family::Serif),
            ("monospace", Family::Monospace),
            ("symbol", Family::Symbol),
            ("dingbats", Family::ZapfDingbats),
        ] {
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let (bytes, format) = face(Request {
                    family,
                    bold,
                    italic,
                    standard: true,
                });
                faces.push((
                    name,
                    bytes,
                    match format {
                        Format::Sfnt => Format::Sfnt,
                        Format::BareCff => Format::BareCff,
                    },
                ));
            }
        }
        faces
    }

    /// One glyph's outline, drawn through whichever reader the program belongs to.
    fn outline(program: &[u8], format: Format, glyph: u16) -> Option<Traced> {
        let mut traced = Traced::default();
        match format {
            Format::BareCff => crate::cff::draw(program, glyph, &mut traced).ok()?,
            Format::Sfnt => {
                use skrifa::MetadataProvider as _;
                let font = skrifa::raw::FontRef::new(program).ok()?;
                font.outline_glyphs()
                    .get(skrifa::GlyphId::from(glyph))?
                    .draw(skrifa::instance::Size::unscaled(), &mut traced)
                    .ok()?;
            }
        }
        Some(traced)
    }

    /// Restating a glyph's advance leaves the glyph it draws exactly as it was.
    ///
    /// **The property the whole of `doc/pdf-a-conversion-limits.md` section 4.9's second metric
    /// route rests on**, checked over every glyph of all twenty compiled-in faces rather than
    /// asserted in a doc comment: the CFF path rewrites one operand of a charstring and splices
    /// subroutines into its head to reach it, and a splice that dropped a byte would draw a
    /// different letter on a page nobody looks at.
    #[test]
    fn an_outline_survives_its_advance_being_restated() {
        let mut checked = 0usize;
        for (name, program, format) in compiled_in() {
            // A width the face is unlikely to state already, so every glyph named is rewritten.
            let widths: BTreeMap<u16, f32> = (0..u16::try_from(glyph_count(program, format))
                .unwrap_or(0))
                .map(|glyph| (glyph, 0.317))
                .collect();
            let mut restated = 0usize;
            for (glyph, width) in &widths {
                let one = BTreeMap::from([(*glyph, *width)]);
                let Ok(rewritten) = with_widths(program, format, &one) else {
                    continue;
                };
                assert_eq!(
                    outline(program, format, *glyph),
                    outline(&rewritten, format, *glyph),
                    "{name}: glyph {glyph} draws differently after its advance was restated"
                );
                restated += 1;
            }
            assert!(
                restated * 10 > widths.len() * 9,
                "{name}: only {restated} of {} glyphs could be restated",
                widths.len()
            );
            checked += 1;
        }
        assert_eq!(checked, 20, "the compiled-in faces");
    }

    /// The advance a restated program states is the width that was asked for.
    #[test]
    fn a_restated_program_states_the_width_it_was_given() {
        for (name, program, format) in compiled_in() {
            let glyphs = glyph_count(program, format);
            let widths: BTreeMap<u16, f32> = (1..u16::try_from(glyphs).unwrap_or(0).min(40))
                .map(|glyph| (glyph, 0.25 + f32::from(glyph) / 1000.0))
                .collect();
            let restatable: BTreeMap<u16, f32> = widths
                .iter()
                .filter(|(glyph, width)| {
                    with_widths(program, format, &BTreeMap::from([(**glyph, **width)])).is_ok()
                })
                .map(|(glyph, width)| (*glyph, *width))
                .collect();
            let rewritten =
                with_widths(program, format, &restatable).expect("every glyph named restates");
            for (glyph, width) in &restatable {
                let stated = advance(&rewritten, format, *glyph).expect("a restated advance");
                assert!(
                    (stated - width).abs() <= super::CONSISTENT,
                    "{name}: glyph {glyph} states {stated} where {width} was asked for"
                );
            }
        }
    }

    /// Every width a `/Widths` array can state is restatable in the ems this program ships.
    ///
    /// The complement of [`RestateError::TooCoarse`], which exists for a program whose em is
    /// coarser than the tolerance: a width is an integer thousandth, so a thousand-unit em
    /// states each one exactly and a 2048-unit one to within a quarter of a thousandth. The
    /// guard is therefore never reached by a face this program ships, and this is what says so
    /// — a face added later with a coarse em would fail here rather than be rounded silently.
    #[test]
    fn every_width_a_widths_array_can_state_fits_the_ems_this_program_ships() {
        for (name, program, format) in compiled_in() {
            let units = super::units_per_em(program, format).expect("a compiled-in face's em");
            assert!(
                units >= 1000.0,
                "{name} states {units} units per em, which cannot hold every integer thousandth"
            );
            for thousandths in [1u16, 250, 317, 723, 1000, 2000] {
                let width = f32::from(thousandths) / 1000.0;
                let restated = with_widths(program, format, &BTreeMap::from([(1u16, width)]));
                assert!(
                    !matches!(restated, Err(RestateError::TooCoarse { .. })),
                    "{name}: {width} was called too coarse for a {units}-unit em"
                );
            }
        }
    }

    /// How many glyphs a program holds.
    fn glyph_count(program: &[u8], format: Format) -> usize {
        match format {
            Format::BareCff => (0..u16::MAX)
                .take_while(|glyph| {
                    crate::cff::advances(program, &[*glyph]).is_ok_and(|found| {
                        found.first().copied().flatten().is_some() || *glyph == 0
                    })
                })
                .count(),
            Format::Sfnt => skrifa::raw::FontRef::new(program)
                .ok()
                .and_then(|font| {
                    use skrifa::raw::TableProvider as _;
                    font.maxp().ok().map(|maxp| usize::from(maxp.num_glyphs()))
                })
                .unwrap_or(0),
        }
    }

    /// One glyph's advance as the program states it, in ems.
    fn advance(program: &[u8], format: Format, glyph: u16) -> Option<f32> {
        let units = super::units_per_em(program, format).ok()?;
        let raw = match format {
            Format::BareCff => crate::cff::advances(program, &[glyph])
                .ok()?
                .first()
                .copied()
                .flatten()?,
            Format::Sfnt => {
                use skrifa::MetadataProvider as _;
                let font = skrifa::raw::FontRef::new(program).ok()?;
                font.glyph_metrics(
                    skrifa::instance::Size::unscaled(),
                    skrifa::instance::LocationRef::default(),
                )
                .advance_width(skrifa::GlyphId::from(glyph))?
            }
        };
        Some(raw / units)
    }
}

#[cfg(test)]
mod vertical_tests {
    use std::collections::BTreeMap;

    use super::tests::compiled_in;
    use super::{RestateError, vertical_advance, with_vertical_advances};
    use crate::substitute::Format;

    /// The constant a font's whole-file checksum is subtracted from (ISO/IEC 14496-22, `head`).
    const CHECKSUM_MAGIC: u32 = 0xb1b0_afba;

    /// The units per em every fixture below states.
    const EM: f32 = 1000.0;

    /// An sfnt stating a `head`, a `maxp`, a `vhea` and a `vmtx`, with true checksums.
    ///
    /// Four tables and no outlines, because vertical metrics are all this module reads or
    /// writes: a fixture carrying `glyf` and `loca` too would be testing the fixture. The
    /// advances are stated in design units of a thousand-unit em, and `pairs` is `vhea`'s
    /// `numOfLongVerMetrics` — the tail past it takes the last pair's advance, which is the
    /// shape [`a_glyph_past_the_long_vertical_metrics_is_refused`] is about.
    fn vertical_fixture(glyphs: u16, pairs: u16, advances: &[u16]) -> Vec<u8> {
        assert_eq!(
            advances.len(),
            usize::from(pairs),
            "one advance per long vertical metric"
        );
        let mut head = vec![0u8; 54];
        head.get_mut(18..20)
            .expect("unitsPerEm")
            .copy_from_slice(&1000u16.to_be_bytes());
        let mut maxp = vec![0u8; 6];
        maxp.get_mut(0..4)
            .expect("a version")
            .copy_from_slice(&0x0001_0000u32.to_be_bytes());
        maxp.get_mut(4..6)
            .expect("numGlyphs")
            .copy_from_slice(&glyphs.to_be_bytes());
        let mut vhea = vec![0u8; 36];
        vhea.get_mut(0..4)
            .expect("a version")
            .copy_from_slice(&0x0001_1000u32.to_be_bytes());
        vhea.get_mut(34..36)
            .expect("numOfLongVerMetrics")
            .copy_from_slice(&pairs.to_be_bytes());
        let mut vmtx = Vec::new();
        for advance in advances {
            vmtx.extend_from_slice(&advance.to_be_bytes());
            // A top side bearing, left alone by every rewrite here and therefore worth having.
            vmtx.extend_from_slice(&30i16.to_be_bytes());
        }
        for _ in pairs..glyphs {
            vmtx.extend_from_slice(&30i16.to_be_bytes());
        }
        checksummed(assembled(&[
            (*b"head", head),
            (*b"maxp", maxp),
            (*b"vhea", vhea),
            (*b"vmtx", vmtx),
        ]))
    }

    /// An sfnt file assembled from tables, in the directory order given.
    fn assembled(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(
            &u16::try_from(tables.len())
                .expect("a few tables")
                .to_be_bytes(),
        );
        // The binary-search hints, which nothing here reads and a well-formed file still states.
        for _ in 0..3 {
            out.extend_from_slice(&0u16.to_be_bytes());
        }
        let directory = 12usize.saturating_add(16usize.saturating_mul(tables.len()));
        let mut offset = directory;
        let mut body = Vec::new();
        for (tag, data) in tables {
            out.extend_from_slice(tag);
            out.extend_from_slice(&0u32.to_be_bytes());
            out.extend_from_slice(&u32::try_from(offset).expect("a small file").to_be_bytes());
            out.extend_from_slice(
                &u32::try_from(data.len())
                    .expect("a small table")
                    .to_be_bytes(),
            );
            body.extend_from_slice(data);
            // Every table in an sfnt begins on a four-byte boundary.
            while !body.len().is_multiple_of(4) {
                body.push(0);
            }
            offset = directory.saturating_add(body.len());
        }
        out.extend_from_slice(&body);
        out
    }

    /// Every 32-bit word of a byte range, summed as ISO/IEC 14496-22 states a checksum.
    ///
    /// The independent recomputation [`checksums_hold_across_a_vertical_restatement`] compares
    /// the incremental adjustment against: the module adjusts two sums by a computed delta, and
    /// a test that adjusted them the same way would prove only that the arithmetic was copied.
    fn sum(data: &[u8], at: usize, length: usize) -> u32 {
        let table = data.get(at..at.saturating_add(length)).expect("a table");
        let mut total = 0u32;
        for chunk in table.chunks(4) {
            let mut word = [0u8; 4];
            word.get_mut(..chunk.len())
                .expect("a chunk fits a word")
                .copy_from_slice(chunk);
            total = total.wrapping_add(u32::from_be_bytes(word));
        }
        total
    }

    /// Every directory record's stated checksum, and `head`'s `checkSumAdjustment`, recomputed.
    ///
    /// Returns the file with all of them written afresh, so that a font whose checksums are
    /// already true comes back byte for byte identical.
    fn checksummed(mut data: Vec<u8>) -> Vec<u8> {
        let count = usize::from(u16::from_be_bytes([
            *data.get(4).expect("a directory"),
            *data.get(5).expect("a directory"),
        ]));
        let mut head_at = None;
        for index in 0..count {
            let entry = 12usize.saturating_add(index.saturating_mul(16));
            let tag = data
                .get(entry..entry.saturating_add(4))
                .expect("a record")
                .to_vec();
            let word = |at: usize| {
                u32::from_be_bytes(
                    data.get(at..at.saturating_add(4))
                        .expect("a field")
                        .try_into()
                        .expect("four bytes"),
                )
            };
            let offset = usize::try_from(word(entry.saturating_add(8))).expect("an offset");
            let length = usize::try_from(word(entry.saturating_add(12))).expect("a length");
            if tag == b"head" {
                head_at = Some(offset);
                data.get_mut(offset.saturating_add(8)..offset.saturating_add(12))
                    .expect("checkSumAdjustment")
                    .copy_from_slice(&0u32.to_be_bytes());
            }
            let stated = sum(&data, offset, length);
            data.get_mut(entry.saturating_add(4)..entry.saturating_add(8))
                .expect("a checkSum field")
                .copy_from_slice(&stated.to_be_bytes());
        }
        let head_at = head_at.expect("a head table");
        let whole = sum(&data, 0, data.len());
        data.get_mut(head_at.saturating_add(8)..head_at.saturating_add(12))
            .expect("checkSumAdjustment")
            .copy_from_slice(&CHECKSUM_MAGIC.wrapping_sub(whole).to_be_bytes());
        data
    }

    /// A restated program states the vertical advance it was given, read back two ways.
    ///
    /// Two readers rather than one: this module's own, and `skrifa`'s `vmtx` — which is what
    /// [`crate::LoadedFont::program_vertical_advance`] reads and therefore what
    /// `pdf_archive`'s ISO 19005-4 section 6.2.10.5 rule will ask after the conversion. A
    /// rewrite the writer's own reader agreed with and the validator's did not would convert a
    /// document into one that still fails the requirement it was converted for.
    #[test]
    fn a_restated_program_states_the_vertical_advance_it_was_given() {
        use skrifa::raw::TableProvider as _;

        let program = vertical_fixture(6, 6, &[1000; 6]);
        // §9.7.4.3's sign: `w1` runs down the page, so a displacement is negative.
        let asked = BTreeMap::from([(1u16, -0.880), (4u16, -0.500)]);
        let restated =
            with_vertical_advances(&program, Format::Sfnt, &asked).expect("a restatable fixture");
        let font = skrifa::raw::FontRef::new(&restated).expect("the fixture re-reads");
        for (glyph, displacement) in &asked {
            assert_eq!(
                vertical_advance(&restated, Format::Sfnt, *glyph),
                Some(*displacement),
                "glyph {glyph} does not state what it was given"
            );
            let through_skrifa = font
                .vmtx()
                .expect("a vmtx")
                .advance(skrifa::GlyphId::from(*glyph))
                .expect("an advance");
            assert!(
                (-f32::from(through_skrifa) / EM - displacement).abs() <= super::CONSISTENT,
                "glyph {glyph} reads back differently through the reader the validator uses"
            );
        }
        // A glyph nobody named keeps the number the program gave it.
        assert_eq!(
            vertical_advance(&restated, Format::Sfnt, 2),
            Some(-1.0),
            "a glyph the caller did not name was restated anyway"
        );
    }

    /// Nothing but the advances named and the two checksums changes.
    ///
    /// The vertical restatement is an in-place edit, which is the claim this makes exact: the
    /// file is the same length, every offset and length in its directory is the producer's, and
    /// the bytes that differ are the ones the caller asked for plus the two sums those
    /// invalidate.
    #[test]
    fn nothing_but_the_named_advances_and_the_two_checksums_moves() {
        let program = vertical_fixture(6, 6, &[1000; 6]);
        let restated =
            with_vertical_advances(&program, Format::Sfnt, &BTreeMap::from([(3u16, -0.750)]))
                .expect("a restatable fixture");
        assert_eq!(
            program.len(),
            restated.len(),
            "an in-place restatement changed the program's length"
        );
        let differing: Vec<usize> = (0..program.len())
            .filter(|at| program.get(*at) != restated.get(*at))
            .collect();
        // The advance is two bytes; a `checkSum` field and a `checkSumAdjustment` are four each,
        // and neither is obliged to differ in every byte.
        assert!(
            differing.len() <= 10,
            "{} bytes changed, which is more than one advance and two checksums",
            differing.len()
        );
    }

    /// The adjusted checksums are the ones a full recomputation would have written.
    ///
    /// The module adjusts both sums by a delta rather than re-summing the file; this is what
    /// says the two constructions agree, over a fixture whose checksums were true to begin with.
    #[test]
    fn checksums_hold_across_a_vertical_restatement() {
        let program = vertical_fixture(6, 4, &[1000, 1000, 900, 900]);
        assert_eq!(
            checksummed(program.clone()),
            program,
            "the fixture's own checksums are not true, so this test would prove nothing"
        );
        let restated = with_vertical_advances(
            &program,
            Format::Sfnt,
            &BTreeMap::from([(0u16, -0.333), (2u16, -0.667)]),
        )
        .expect("a restatable fixture");
        assert_eq!(
            checksummed(restated.clone()),
            restated,
            "the adjusted checksums are not the ones a recomputation writes"
        );
    }

    /// A glyph whose advance the program states only by inheritance is refused by name.
    ///
    /// ISO/IEC 14496-22's `vmtx` tail shares the last long metric's advance, so glyph 5 of a
    /// four-pair table has no field of its own — and giving it one would restate glyph 4's too.
    #[test]
    fn a_glyph_past_the_long_vertical_metrics_is_refused() {
        let program = vertical_fixture(6, 4, &[1000; 4]);
        assert_eq!(
            with_vertical_advances(&program, Format::Sfnt, &BTreeMap::from([(5u16, -0.5)])),
            Err(RestateError::InheritedVerticalAdvance { glyph: 5, pairs: 4 }),
            "a glyph in the vmtx tail was restated rather than refused"
        );
    }

    /// A program stating no vertical metrics is refused, and so is one that is not an sfnt.
    ///
    /// ISO 19005-4 section 6.2.10.5's vertical requirement is conditional on the program
    /// stating them, so this is the common answer rather than a failure: a face never meant to
    /// be set vertically carries no `vhea`, and neither does a bare CFF program.
    #[test]
    fn a_program_stating_no_vertical_metrics_is_refused() {
        for (name, program, format) in compiled_in() {
            assert_eq!(
                with_vertical_advances(program, format, &BTreeMap::from([(1u16, -0.5)])),
                Err(RestateError::NoVerticalMetrics),
                "{name} was treated as though it stated vertical metrics"
            );
            assert_eq!(
                vertical_advance(program, format, 1),
                None,
                "{name} answered a vertical advance it does not state"
            );
        }
    }

    /// An advance past what a `uint16` holds is refused rather than clamped.
    #[test]
    fn an_advance_an_sfnt_cannot_state_is_refused() {
        let program = vertical_fixture(6, 6, &[1000; 6]);
        assert_eq!(
            with_vertical_advances(&program, Format::Sfnt, &BTreeMap::from([(1u16, -70.0)])),
            Err(RestateError::OutOfRange {
                glyph: 1,
                units: 70_000,
            }),
            "an advance past the field's range was written rather than refused"
        );
    }
}
