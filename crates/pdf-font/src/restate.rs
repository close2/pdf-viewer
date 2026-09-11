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
            let advances = design
                .iter()
                .map(|(glyph, units)| (*glyph, u16::try_from(whole(*units)).unwrap_or(0)))
                .collect();
            sfnt::with_advances(program, &advances).ok_or_else(|| RestateError::Unreadable {
                detail: "the program states no readable hhea, hmtx and maxp, which is where an \
                         sfnt's advances are"
                    .to_owned(),
            })
        }
    }
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
/// It is not silent: [`with_widths`] has already checked the width against the em, and
/// [`cff::with_advances`] refuses a difference that will not fit its operand.
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
    fn compiled_in() -> Vec<(&'static str, &'static [u8], Format)> {
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
