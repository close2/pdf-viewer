//! What a document says about the size of its glyphs.
//!
//! Advances first, in ISO 32000-2's own order of authority: a simple font's `/Widths`
//! (§9.6.2.1) or a composite font's `/W` (§9.7.4.3), then — for the standard 14, which Table
//! 109 lets state neither — §9.6.2.2's published metrics, and last the substitute program's
//! own charstrings. Whatever shapes the page ends up drawn in, these are the numbers it is
//! laid out by, which is why they are resolved once at load time rather than taken from
//! whichever face happened to answer.
//!
//! Then the two quantities that are not advances: Table 120's `/Ascent` and `/Descent`, which
//! a selection highlight needs and which a descriptor is believed about only within a band
//! (ADR 0216), and §9.7.4.3's vertical displacement and position vector for a font shown in
//! writing mode 1.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object};
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::cff;
use crate::glyph_names::GlyphNames;
use crate::loading::CodeMapping;
use crate::program::Program;
use crate::standard_metrics;
use crate::substitute;

/// Width of a simple font's code that neither `/Widths` nor `/MissingWidth` covers.
///
/// ISO 32000-2 §9.8.1's Table 120 gives `/MissingWidth` the default value 0, and §9.6.2's
/// Table 109 sends every code outside the declared range to it:
///
/// > For character codes outside the range FirstChar to LastChar , the value of
/// > MissingWidth from the FontDescriptor entry for this font shall be used.
///
/// It used to be half an em here, on the reasoning that spacing degrades more gracefully
/// than it does collapsing to zero. That is a plausible thing to want and not a reading of
/// anything: the clause states the default, and a producer wanting half an em can write
/// it. `issue7439.pdf` is what the difference costs — its one line of text shows code 2
/// six times against a `/FirstChar` of 3 and a descriptor with no `/MissingWidth`, so half
/// an em of invented space opened six times between `Issue` and `7439`. The page was
/// contradicted by the reference consensus and now agrees.
const DEFAULT_WIDTH: f32 = 0.0;

/// Narrows a PDF number to `f32`.
pub(crate) fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a glyph advance outside f32's range is not an advance"
    )]
    {
        value as f32
    }
}

/// What [`simple_widths`] needs about the font beyond the document's own statements.
#[derive(Clone, Copy)]
pub(crate) struct SimpleMetrics<'a> {
    /// The substitution request, when the font is a stand-in.
    pub(crate) substituted: Option<substitute::Request>,
    /// The glyph name each code selects, when the font has names.
    pub(crate) names: Option<&'a GlyphNames>,
    /// The font program.
    pub(crate) data: &'a [u8],
    /// Which reader parses `data`, since the advance a program states is read differently
    /// from an sfnt's `hmtx` and from a CFF charstring's leading width operand.
    pub(crate) program: Program,
    pub(crate) mapping: &'a CodeMapping,
    pub(crate) units_per_em: f32,
}

/// Builds a simple font's advance table, in thousandths of an em.
///
/// Three sources, in descending order of authority. `/Widths` is the document's own
/// statement and always wins. Failing that — which only the standard 14 are allowed to do
/// — the specification's published metrics for the face apply, which keeps the layout a
/// property of the document rather than of this machine. Anything still unanswered falls
/// to the substitute's own advances, which in practice means glyphs outside the standard
/// character set.
///
/// Resolved once here rather than at each lookup, so [`crate::LoadedFont::advance`] stays free of
/// work and of allocation on the per-character path.
pub(crate) fn simple_widths(
    document: &Document,
    dict: &Dictionary,
    font: SimpleMetrics<'_>,
) -> BTreeMap<u32, f32> {
    // `/Widths` is indexed from `/FirstChar`.
    let first = document
        .get_key(dict, "FirstChar")
        .as_integer()
        .unwrap_or(0);
    let mut widths = BTreeMap::new();
    if let Some(items) = document.get_key(dict, "Widths").as_array() {
        for (offset, item) in items.iter().enumerate() {
            let Some(width) = document.resolve(item).as_number() else {
                continue;
            };
            let Ok(offset) = i64::try_from(offset) else {
                continue;
            };
            let Ok(code) = u32::try_from(first.saturating_add(offset)) else {
                continue;
            };
            widths.insert(code, narrow(width));
        }
    }

    let Some(request) = font.substituted.filter(|_| widths.is_empty()) else {
        return widths;
    };

    let standard = standard_metrics::StandardFont::for_request(request);
    if let Some(names) = font.names {
        for (code, glyph_name) in names.iter().enumerate() {
            let Ok(code) = u32::try_from(code) else {
                continue;
            };
            if let Some(width) = standard.width(glyph_name.as_ref()) {
                widths.insert(code, width);
            }
        }
    }
    for (code, width) in program_widths(font, |code| !widths.contains_key(&code)) {
        widths.entry(code).or_insert(width);
    }
    widths
}

/// Fills a width table from the font program's own advances, for the codes `wanted` accepts.
///
/// Used only when the document states no widths, which the standard 14 are allowed to do,
/// and asked only about the codes §9.6.2.2's published metrics did not answer — which is
/// what makes it cheap enough to evaluate a bare CFF's charstrings at load time.
///
/// **Ten of the fourteen compiled-in standard faces are bare CFF programs, and until the
/// four-hundred-and-fifth session this function could not read one**: it went through
/// `skrifa`'s `FontRef`, which parses an sfnt container and refuses a bare CFF, so every
/// serif, fixed-pitch and symbolic standard-14 substitution answered nothing here and every
/// unanswered code fell to `/MissingWidth`'s default of 0 (Table 120). `issue4304.pdf` is
/// what that cost — `/Differences [32 /.notdef …]` over a non-embedded `/Times-Roman`, so
/// §9.2.4's horizontal displacement for the code the page uses as a space came from nowhere
/// and the page drew *Wordsthatshouldhavespacesbetweenthem.*
fn program_widths(font: SimpleMetrics<'_>, wanted: impl Fn(u32) -> bool) -> BTreeMap<u32, f32> {
    let mut widths = BTreeMap::new();
    let CodeMapping::Named(table) = font.mapping else {
        return widths;
    };
    let asked: Vec<(u32, u16)> = table
        .iter()
        .enumerate()
        .filter_map(|(code, glyph)| Some((u32::try_from(code).ok()?, (*glyph)?)))
        .filter(|(code, _)| wanted(*code))
        .collect();

    let advances: Vec<Option<f32>> = match font.program {
        Program::Sfnt => {
            let Ok(program) = FontRef::new(font.data) else {
                return widths;
            };
            let metrics = program.glyph_metrics(Size::unscaled(), LocationRef::default());
            asked
                .iter()
                .map(|(_, glyph)| metrics.advance_width(GlyphId::from(*glyph)))
                .collect()
        }
        Program::BareCff => {
            let glyphs: Vec<u16> = asked.iter().map(|(_, glyph)| *glyph).collect();
            let Ok(advances) = cff::advances(font.data, &glyphs) else {
                return widths;
            };
            advances
        }
        // A substitute is never a bare Type 1 program: `substitute::Format` offers an sfnt
        // or a bare CFF and nothing else, and this function runs only for a substituted
        // font. An *embedded* Type 1 reaches here with `substituted` unset and returns above.
        Program::Type1 => return widths,
    };

    for ((code, _), advance) in asked.iter().zip(advances) {
        let Some(advance) = advance else { continue };
        widths.insert(*code, advance / font.units_per_em * 1000.0);
    }
    widths
}

/// Collects `/W` widths for a composite font.
///
/// The array mixes two forms: `c [w1 w2 ...]` gives consecutive codes, and `c1 c2 w` gives
/// one width for a whole range.
///
/// ISO 32000-2 §9.7.4.3 on a CID that appears twice: "specifying a given CID value more than
/// once should not be done. In the case where it is done, the first specification is the one
/// that shall be used." So an entry that already exists is kept, rather than overwritten.
pub(crate) fn composite_widths(document: &Document, descendant: &Dictionary) -> BTreeMap<u32, f32> {
    /// Ranges are bounded so a hostile `/W` cannot ask for four billion entries.
    const MAX_RANGE: u32 = 1 << 16;

    let mut widths = BTreeMap::new();
    let array = document.get_key(descendant, "W");
    let Some(items) = array.as_array() else {
        return widths;
    };

    let resolved: Vec<Object> = items.iter().map(|item| document.resolve(item)).collect();
    let mut index = 0usize;

    while index < resolved.len() {
        let Some(first) = resolved.get(index).and_then(Object::as_integer) else {
            break;
        };
        let Ok(first) = u32::try_from(first) else {
            break;
        };

        match resolved.get(index.saturating_add(1)) {
            Some(Object::Array(list)) => {
                for (offset, item) in list.iter().enumerate() {
                    let Some(width) = document.resolve(item).as_number() else {
                        continue;
                    };
                    let Ok(offset) = u32::try_from(offset) else {
                        continue;
                    };
                    widths
                        .entry(first.saturating_add(offset))
                        .or_insert(narrow(width));
                }
                index = index.saturating_add(2);
            }
            Some(second) => {
                let Some(last) = second
                    .as_integer()
                    .and_then(|value| u32::try_from(value).ok())
                else {
                    break;
                };
                let Some(width) = resolved
                    .get(index.saturating_add(2))
                    .and_then(Object::as_number)
                else {
                    break;
                };
                let span = last.saturating_sub(first).min(MAX_RANGE);
                for offset in 0..=span {
                    widths
                        .entry(first.saturating_add(offset))
                        .or_insert(narrow(width));
                }
                index = index.saturating_add(3);
            }
            None => break,
        }
    }

    widths
}

/// The width of every code the font dictionary's `/Widths` does not cover.
///
/// §9.8.3's Table 120 names `/MissingWidth` for exactly that, on the *font descriptor* this
/// function is handed, and gives it a default; see [`DEFAULT_WIDTH`], which is the whole
/// subject. (**This sentence said "§9.6.2's Table 109" until the four-hundred-and-thirteenth
/// session**, and Table 109 is the Type 1 font dictionary — `/Widths`, `/FirstChar`,
/// `/LastChar` and no `/MissingWidth` anywhere in it. `doc/todo/01`'s ninth sweep, and the
/// code below had the right answer all along: it reads the entry off `descriptor`.)
pub(crate) fn missing_width(document: &Document, descriptor: Option<&Dictionary>) -> f32 {
    descriptor
        .map(|descriptor| document.get_key(descriptor, "MissingWidth"))
        .and_then(|value| value.as_number())
        .map_or(DEFAULT_WIDTH, narrow)
}

/// The em, in the glyph-space units a font descriptor's dimensions are stated in.
///
/// ISO 32000-2 §9.8.1 puts every entry of Table 120 in that space —
///
/// > All dimensional values shall be units in glyph space.
///
/// — and §9.2.4 says how big one of them is:
///
/// > For all font types except Type 3, the units of glyph space are one-thousandth of a unit of
/// > text space
///
/// A text space unit is the font size (§9.4.4), so a thousand glyph-space units is one em at
/// whatever size the text is set. That is what makes a *band* on these entries derivable rather
/// than invented: they are not free numbers, they are multiples of a fixed quantity.
const EM: f32 = 1000.0;

/// The height of one line of text, as the standard's own multiple of the font size.
///
/// §14.8.5.4.4's Table 380 defines a `/LineHeight` of `Normal` by leaving the value to the
/// processor and then saying what a processor should choose:
///
/// > The meaning of the term "reasonable value" is left to the PDF processor to determine. It
/// > should be approximately 1.2 times the font size, but this value may vary depending on the
/// > export format.
///
/// The same entry's NOTE 1 says where that height comes from when nothing states it, and it is
/// this crate's two entries:
///
/// > In the absence of a numeric value for LineHeight or an explicit value for the font size, a
/// > reasonable method of calculating the line height from the information in a tagged PDF file
/// > is to find the difference between the associated font's Ascent and Descent values (see 9.8,
/// > "Font descriptors"), map it from glyph space to default user space (see 9.4.4, "Text space
/// > details"), and use the maximum resulting value for any character in the line.
///
/// So the standard both states the arithmetic `/Ascent` − `/Descent` and states what its answer
/// is worth in font sizes. This is the anchor [`measured_extent`] measures a stated pair against.
///
/// **The standard states the same quantity a second time and at a different number**, which is
/// why the band around this one is a band rather than a tolerance. §9.2.2, on the size a font
/// defines its glyphs at:
///
/// > A font defines the glyphs at one standard size. This standard is arranged so that the
/// > nominal height of tightly spaced lines of text is 1 unit.
///
/// One unit of text space is one em, so that is the same height *tightly* spaced where Table 380
/// is a line with room around it. Both readings are inside the band below, and neither was
/// chosen because a corpus wanted it: 1.0 is what the em-box fallback in [`vertical_extent`]
/// already was, on other grounds, before this constant existed.
const REASONABLE_LINE: f32 = 1.2 * EM;

/// How far from [`REASONABLE_LINE`] a stated pair may fall and still be believed.
///
/// **A choice, and the one number here that is not printed in the standard.** A factor of two
/// each way accepts every line from 0.6 em to 2.4 em, which is a wider spread of faces than
/// exists; what it rejects is a statement no measurement of a face could produce. The size of
/// the factor is decided by the asymmetry of the two mistakes rather than by any corpus:
/// disbelieving a true pair costs a highlight bounded by the em box, one the person can still
/// see and still click, while believing a false one costs the highlight altogether or puts it on
/// a line the person did not select. So the band is set wide enough that only a statement well
/// outside every face falls out of it — and the cost of that width is that a pair which is wrong
/// by less than a factor of two is still believed.
const TOLERANCE: f32 = 2.0;

/// The line a descriptor's `/Ascent` and `/Descent` measure, in ems, where they measure one.
///
/// Both arguments in glyph space, which is the unit §9.8.1 states Table 120's dimensions in; see
/// [`EM`]. `None` means the pair cannot be a measurement of any face, and the caller answers
/// with the em box instead.
///
/// Table 120's own definitions are what make this answerable at all, because each entry is
/// defined as a *measurement of the font* rather than as a free parameter. `/Ascent`:
///
/// > The maximum height above the baseline reached by glyphs in this font. The height of glyphs
/// > for accented characters shall be excluded.
///
/// and `/Descent`:
///
/// > The maximum depth below the baseline reached by glyphs in this font. The value shall be a
/// > negative number.
///
/// Three conditions follow, in order:
///
/// - **the ascent is above the baseline.** A "maximum height above the baseline reached by
///   glyphs" that is zero or negative describes a font whose glyphs reach nothing above the
///   baseline, which is not a font anybody sets text in;
/// - **the descent is not above it.** Zero is accepted as the statement that no glyph goes below
///   the baseline, which is a thing a face can be;
/// - **the line the two describe is within [`TOLERANCE`] of [`REASONABLE_LINE`]**, which is the
///   height the standard itself calls reasonable for a line, computed the way the standard
///   itself computes it.
///
/// **What that rejects is mostly a number in the wrong unit.** `/Ascent 8 /Descent -2` and
/// `/Ascent 4000 /Descent -1140` are both in the corpus, and neither is a lie about a face so
/// much as a face measured in a glyph space that is not §9.2.4's; the em box is the answer to
/// both, because it is the one quantity that is defined whatever the file says.
///
/// # The one repair
///
/// A positive `/Descent` is read as the depth it states, with Table 120's sign convention put
/// back. That is a **choice**, and the argument for it is that the entry's definition and its
/// sign are two different sentences: "the maximum depth below the baseline reached by glyphs in
/// this font" defines a depth, which is a magnitude, and "[t]he value shall be a negative
/// number" is the convention for writing it down. A file that writes `905` and `211` for a face
/// whose real metrics are 905 and −211 has broken the convention and stated the measurement, and
/// this is by a distance the commonest malformed shape the corpus holds — **42 font dictionaries
/// of 1629**, against 40 the band rejects outright. Refusing it would cost those fonts a
/// highlight that stops at the baseline and misses every descender's tail; the cost of accepting
/// it is that a file which meant something else by a positive descent gets a box the band already
/// tolerates for anybody. Nothing here reads the *rendering* of a page, so a mistaken repair
/// cannot move a glyph.
///
/// Public because two other things ask the same question the interpreter does: the gate over
/// `pdf-model`'s selection geometry, and `font_metric_census`, which counts what the corpus
/// states — and a census measuring a *copy* of the rule would measure something that can drift
/// from it.
#[must_use]
pub fn measured_extent(ascent: f32, descent: f32) -> Option<(f32, f32)> {
    let descent = if descent > 0.0 { -descent } else { descent };
    let line = ascent - descent;
    let band = REASONABLE_LINE / TOLERANCE..=REASONABLE_LINE * TOLERANCE;
    let measured = ascent > 0.0 && band.contains(&line);
    measured.then(|| (ascent / EM, descent / EM))
}

/// How far a font's glyphs reach above and below the baseline, in ems.
///
/// ISO 32000-2 §9.8.1's Table 120 requires both `/Ascent` and `/Descent` of every font
/// descriptor, in glyph space, so both are divided by [`EM`].
///
/// **This is not used to place anything.** It answers a question the standard does not ask —
/// how tall is a line of this text — which a viewer needs to lay a selection highlight over a
/// run of glyphs. Where a font states neither entry (the standard 14, which need no descriptor,
/// and every malformed file) the answer is the em box, 1 above the baseline and 0 below: that is
/// a *defined* quantity rather than a guess, and §9.2.2 is where it is defined —
///
/// > A font defines the glyphs at one standard size. This standard is arranged so that the
/// > nominal height of tightly spaced lines of text is 1 unit.
///
/// — so the fallback is the standard's own nominal line, and it is the one place a fallback here
/// could invent a number.
///
/// **A stated pair is believed only where it could be a measurement**, which is
/// [`measured_extent`] and which the em box is also the answer to. The guard used to be
/// `ascent > descent` — an ordering rather than a plausibility — and it accepted three shapes a
/// scanned page's OCR layer routinely states, each of which puts a selection highlight where the
/// text is not (ADR 0216).
pub(crate) fn vertical_extent(document: &Document, descriptor: Option<&Dictionary>) -> (f32, f32) {
    let entry = |key: &str| {
        descriptor
            .map(|descriptor| document.get_key(descriptor, key))
            .and_then(|value| value.as_number())
            .map(narrow)
            .filter(|value| value.is_finite())
    };
    match (entry("Ascent"), entry("Descent")) {
        (Some(ascent), Some(descent)) => measured_extent(ascent, descent).unwrap_or((1.0, 0.0)),
        _ => (1.0, 0.0),
    }
}

/// ISO 32000-2 §9.7.4.3's vertical metrics: `/DW2` and `/W2`, in glyph space.
///
/// > Glyphs from a CIDFont may be shown in vertical writing mode. This is selected by the
/// > WMode entry in the associated CMap dictionary … To be used in this way, the CIDFont
/// > shall define the vertical displacement for each glyph and the position vector that
/// > relates the horizontal and vertical writing origins.
///
/// Two vectors per glyph, and only three of their four components are ever stated: the
/// displacement `w1` has a horizontal component the clause fixes at 0, and the position
/// vector `v`'s horizontal component defaults to "half the glyph width" — which is why this
/// carries the *horizontal* width in as an argument rather than duplicating it.
#[derive(Debug)]
pub(crate) struct Vertical {
    /// `/DW2`'s two numbers: the vertical component of `v`, then that of `w1`.
    ///
    /// Table 115's default is `[880 -1000]`, and the sign is the clause's own NOTE: "a
    /// negative value for the vertical component places the origin of the next glyph below
    /// the current glyph because vertical coordinates in a standard coordinate system
    /// increase from bottom to top".
    default: [f32; 2],
    /// `/W2`, by CID: the vertical displacement `w1y`, then `v`'s two components.
    metrics: BTreeMap<u32, [f32; 3]>,
}

impl Vertical {
    /// Reads `/DW2` and `/W2` from a `CIDFont` dictionary.
    pub(crate) fn read(document: &Document, descendant: &Dictionary) -> Self {
        let default = document
            .get_key(descendant, "DW2")
            .as_array()
            .and_then(|items| {
                let read = |at: usize| {
                    items
                        .get(at)
                        .map(|item| document.resolve(item))
                        .and_then(|item| item.as_number())
                        .map(narrow)
                };
                Some([read(0)?, read(1)?])
            })
            .unwrap_or([880.0, -1000.0]);

        Self {
            default,
            metrics: composite_vertical_metrics(document, descendant),
        }
    }

    /// The displacement `w1` and position vector `v` for one CID, in glyph space.
    ///
    /// `width` is the same glyph's horizontal displacement `w0`, because the clause defines
    /// `v`'s horizontal component as half of it whenever `/W2` does not state one.
    pub(crate) fn metrics(&self, cid: u32, width: f32) -> ([f32; 2], [f32; 2]) {
        match self.metrics.get(&cid) {
            Some([w1y, vx, vy]) => ([0.0, *w1y], [*vx, *vy]),
            None => ([0.0, self.default[1]], [width / 2.0, self.default[0]]),
        }
    }
}

/// Reads §9.7.4.3's `/W2` array, whose two formats mirror `/W`'s with three numbers per CID.
///
/// > The elements of the array shall be organised in groups of two or five … In the first
/// > format, c is a starting CID and shall be followed by an array containing numbers
/// > interpreted in groups of three.
fn composite_vertical_metrics(
    document: &Document,
    descendant: &Dictionary,
) -> BTreeMap<u32, [f32; 3]> {
    /// The same bound `/W` takes, and for the same reason.
    const MAX_RANGE: u32 = 1 << 16;

    let mut metrics = BTreeMap::new();
    let array = document.get_key(descendant, "W2");
    let Some(items) = array.as_array() else {
        return metrics;
    };
    let resolved: Vec<Object> = items.iter().map(|item| document.resolve(item)).collect();
    let number = |at: Option<&Object>| at.and_then(Object::as_number).map(narrow);

    let mut index = 0usize;
    while index < resolved.len() {
        let Some(first) = resolved
            .get(index)
            .and_then(Object::as_integer)
            .and_then(|value| u32::try_from(value).ok())
        else {
            break;
        };
        match resolved.get(index.saturating_add(1)) {
            Some(Object::Array(list)) => {
                let values: Vec<f32> = list
                    .iter()
                    .map(|item| document.resolve(item))
                    .map_while(|item| item.as_number().map(narrow))
                    .collect();
                for (offset, group) in values.chunks_exact(3).enumerate() {
                    let (Ok(offset), [w1y, vx, vy]) = (u32::try_from(offset), group) else {
                        continue;
                    };
                    // "Specifying a given CID value more than once should not be done. In the
                    // case where it is done, the first specification is the one that shall be
                    // used" — §9.7.4.3, of `/W`, and `/W2` is the same array one field wider.
                    metrics
                        .entry(first.saturating_add(offset))
                        .or_insert([*w1y, *vx, *vy]);
                }
                index = index.saturating_add(2);
            }
            Some(second) => {
                let Some(last) = second
                    .as_integer()
                    .and_then(|value| u32::try_from(value).ok())
                else {
                    break;
                };
                let group = [
                    number(resolved.get(index.saturating_add(2))),
                    number(resolved.get(index.saturating_add(3))),
                    number(resolved.get(index.saturating_add(4))),
                ];
                let (Some(w1y), Some(vx), Some(vy)) = (group[0], group[1], group[2]) else {
                    break;
                };
                let end = last.min(first.saturating_add(MAX_RANGE));
                for cid in first..=end {
                    metrics.entry(cid).or_insert([w1y, vx, vy]);
                }
                index = index.saturating_add(5);
            }
            None => break,
        }
    }

    metrics
}

/// ISO 32000-2 §9.8.1's Table 120, on the one entry of it that moves a glyph.
///
/// A width is not a picture, so no rendering test can be pointed at this: what it changes
/// is where the *next* glyph goes, and only on a code the document's own `/Widths` does
/// not cover. `issue7439.pdf` is the corpus's one witness and the oracle holds it by name;
/// these state the rule itself, which is what survives that file being deleted.
#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "both sides are an integer written in the fixture or a constant, so the \
              comparison is exact by construction"
)]
mod missing_width_tests {
    use crate::fixture::font_dictionary;

    use super::missing_width;

    /// `/MissingWidth` is read from the descriptor when it is there.
    #[test]
    fn a_stated_missing_width_is_the_width() {
        let (document, dict) = font_dictionary("/FontDescriptor << /MissingWidth 321 >>");
        let descriptor = document.get_key(&dict, "FontDescriptor");

        assert_eq!(missing_width(&document, descriptor.as_dict()), 321.0);
    }

    /// Table 120's default value is 0, not a guess at an average glyph.
    #[test]
    fn an_absent_missing_width_is_zero() {
        let (document, dict) = font_dictionary("/FontDescriptor << /Flags 32 >>");
        let descriptor = document.get_key(&dict, "FontDescriptor");

        assert_eq!(missing_width(&document, descriptor.as_dict()), 0.0);
        assert_eq!(missing_width(&document, None), 0.0);
    }
}

/// The third source of a simple font's advances: the program this processor actually draws.
///
/// ISO 32000-2 Table 109 lets a standard-14 dictionary omit `/Widths` altogether, so a
/// processor that means to draw the page has to supply the advances itself — and the metrics
/// and the substitution face are the two things it can supply them from. (This paragraph rested
/// on §9.6.2.1's closing `shall` and on §9.6.2.2's "their font metrics and suitable substitution
/// fonts" until the four-hundred-and-eighteenth session; Errata Collection 3 struck both, and
/// [`crate::standard`] carries the reading that replaces them.) Adobe's published metrics name
/// only the standard character set, so a `/Differences` reaching any other glyph — `.notdef` is the
/// one every Type 1 program is required to have (§9.6.5.2) — is answered by the program.
///
/// `issue4304.pdf` is the corpus's witness and the oracle held it by name for a hundred and
/// eighty sessions under the wrong diagnosis; these state the rule, which is what survives
/// that file being deleted.
#[cfg(test)]
mod substituted_width_tests {
    use crate::fixture::font_dictionary;
    use crate::{Code, LoadedFont};

    /// A code the published metrics do not name takes the substitute program's own advance.
    ///
    /// `/Times-Roman` with no `/Widths` resolves to a compiled-in bare CFF face, whose
    /// `.notdef` charstring states 250 — the same number every metric clone of that face
    /// carries, because each of them gives `.notdef` its own `space` width.
    #[test]
    fn a_differences_entry_naming_notdef_takes_the_programs_advance() {
        let (document, dict) =
            font_dictionary("/BaseFont /Times-Roman /Encoding << /Differences [32 /.notdef] >>");
        let font = LoadedFont::load(&document, &dict, "F1").expect("a standard 14 name loads");

        assert!((font.advance(Code::single_byte(32)) - 0.250).abs() < 1e-6);
        // The rest of the encoding is untouched: `A` is Adobe's published 722.
        assert!((font.advance(Code::single_byte(b'A')) - 0.722).abs() < 1e-6);
    }

    /// Without the `/Differences`, code 32 is the base encoding's `space` and is 250 too.
    ///
    /// The pair is what says the fix did not simply hard-code a number: the two routes to
    /// 250 are the published metrics for `space` and the program's charstring for `.notdef`.
    #[test]
    fn an_unmodified_code_thirty_two_is_the_published_space_width() {
        let (document, dict) = font_dictionary("/BaseFont /Times-Roman");
        let font = LoadedFont::load(&document, &dict, "F1").expect("a standard 14 name loads");

        assert!((font.advance(Code::single_byte(32)) - 0.250).abs() < 1e-6);
    }
}

/// The band on a descriptor's line, as arithmetic (ADR 0216).
///
/// Stated here rather than only through a fixture because the rule is arithmetic on two numbers:
/// `crates/pdf-model/tests/selection_geometry.rs` checks that the rule reaches a quadrilateral,
/// and these check the rule.
#[cfg(test)]
mod measured_extent_tests {
    use super::measured_extent;

    /// A face's real measurements are believed to the number, and the corpus's malformed shapes
    /// are not.
    ///
    /// Every rejected row is a pair the corpus actually states — `font_metric_census` names the
    /// document beside each — and every one of them passed the `ascent > descent` guard this
    /// replaced.
    #[test]
    fn a_pair_is_believed_where_it_could_be_a_measurement() {
        assert_eq!(measured_extent(718.0, -207.0), Some((0.718, -0.207)));
        // Table 120 permits a face no glyph of which goes below the baseline.
        assert_eq!(measured_extent(1000.0, 0.0), Some((1.0, 0.0)));

        assert_eq!(measured_extent(0.0, -205.0), None, "zero_descent.pdf");
        assert_eq!(measured_extent(8.0, -2.0), None, "bug868745.pdf");
        assert_eq!(
            measured_extent(4000.0, -1140.0),
            None,
            "PDFJS-9279-reduced.pdf"
        );
        assert_eq!(measured_extent(3116.0, -2463.0), None, "issue13193.pdf");
        assert_eq!(measured_extent(282.0, 0.0), None, "pr4922.pdf");
        assert_eq!(measured_extent(f32::NAN, -200.0), None, "not a number");
    }

    /// The band's own edges, which are §14.8.5.4.4's 1.2 em either multiplied or divided by two.
    #[test]
    fn the_band_is_closed_at_six_tenths_of_an_em_and_at_two_and_two_fifths() {
        assert!(measured_extent(600.0, 0.0).is_some(), "0.6 em is inside");
        assert!(
            measured_extent(599.0, 0.0).is_none(),
            "and just under is not"
        );
        assert!(measured_extent(2400.0, 0.0).is_some(), "2.4 em is inside");
        assert!(
            measured_extent(2401.0, 0.0).is_none(),
            "and just over is not"
        );
    }

    /// A `/Descent` written without Table 120's sign is the depth it states.
    ///
    /// 42 of the corpus's 1629 font dictionaries do this, and `905 211` is Arial's real metrics
    /// with the sign dropped.
    #[test]
    fn a_positive_descent_is_read_as_a_depth() {
        assert_eq!(measured_extent(905.0, 211.0), Some((0.905, -0.211)));
        assert_eq!(measured_extent(891.0, 216.0), Some((0.891, -0.216)));
        // The repair is not a licence: the band still applies to what it produces.
        assert_eq!(measured_extent(100.0, 50.0), None, "still a sliver");
    }
}
