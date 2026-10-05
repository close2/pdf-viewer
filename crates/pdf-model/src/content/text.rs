//! Showing text: §9.4's positioning, Table 104's rendering modes, and the readback.
//!
//! One pass places the glyphs and extracts the page's text from the same code-to-glyph
//! decisions, which is what makes the readback evidence about the rendering. §9.3.8's text
//! knockout and §11.7.4.4's implicit per-glyph group are judged at `ET`, where the finished
//! object can be seen whole.

use std::rc::Rc;
use std::sync::Arc;

use pdf_font::Code;
use pdf_render::display_list::Clip;
use pdf_render::{BlendMode, ClipId, Command, FillRule, Path, Point, Rect, Transform};

use super::font::Font;
use super::overprint::{FirstBullet, implicit_group_owed};
use super::pattern::{PatternPaint, Tiled};
use super::report::{Placed, Unsupported};
use super::transparency::{AlphaSourcesSeen, Painted, implicit_knockout_group, outline_bounds};
use super::{GraphicsState, Interpreter};

/// The share of a space a gap along the line has to exceed before it reads as a word break.
///
/// **A choice, and the standard is why one is needed**: §9.4.4 states where each glyph goes and
/// nothing about words, and §14.8.2.6.2 names what is left to an untagged page's reader as
/// "heuristics based on information such as glyph positioning on the page". Half is the point at
/// which a gap is nearer a space than no gap at all. Over the readback of 2808 pages (ADR 1502),
/// the steps between show operations in fonts that state a space fall into two clusters, kerning
/// within a tenth of a space and word gaps from about three quarters of one up, with a flat trough
/// from 0.3 to 0.75 between them; half sits in that trough, the fewest steps lie within a quarter
/// of the threshold either side of it, and the threshold falls inside more fonts' own widest gap
/// between the two clusters than 0.6 or 0.7 does. It is the threshold of every font a page shows
/// too few steps of to decide by, and the ceiling of the one [`own_threshold`] reads off a font's
/// own steps (ADR 1515).
const WORD_GAP_SHARE: f32 = 0.5;

/// The space, in ems, read for a font that states none at code 32.
///
/// **A choice**, stood in where §9.3.3's single-byte code 32 has no width: a subset carrying no
/// space is common, and its producer's word gaps are then `TJ` adjustments. A quarter em sits
/// between the lower quartile and the median of the spaces the corpus's fonts do state (0.226 and
/// 0.278 em), and under [`WORD_GAP_SHARE`] it makes the threshold an eighth of an em, inside the
/// trough between kerning and word gaps that such fonts' own steps show (ADR 1502).
const NOMINAL_SPACE_EM: f32 = 0.25;

/// The least share of a space a font's own threshold may take, and so the shortest gap
/// [`Interpreter::separate_text`] keeps a provisional space for.
///
/// **A choice** (ADR 1515): the floor of the trough ADR 1502 measured between the kerning and the
/// word-gap clusters — 0.3 of a stated space, 0.065 em of a font stating none, which is 0.26 of the
/// quarter em [`NOMINAL_SPACE_EM`] reads it with. No font's threshold goes below it, however its
/// own steps fall.
const LEAST_WORD_GAP_SHARE: f32 = 0.25;

/// How many steps along a line a font has to show on a page before its own steps decide its
/// threshold; below it, [`WORD_GAP_SHARE`] does (ADR 1515).
const STEPS_TO_DECIDE: usize = 40;

/// How wide, as the ratio of its two edges, the empty interval below a font's word gaps has to be
/// before it is read as the font's own threshold (ADR 1515): twice, so that a page's kerning and
/// its word gaps are a factor apart rather than neighbours.
const GAP_TO_DECIDE: f32 = 2.0;

/// The steps along a line one page showed, per font, kept until the page is finished.
///
/// §9.4.4 places glyphs and states no quantity that separates words, so where a word ends is this
/// program's choice (ADR 1502). ADR 1515 makes it per font: a font's steps between show operations
/// fall into a kerning cluster and a word-gap cluster, and where the empty interval between the
/// two lies differs from font to font — so the threshold is read off the page's own steps once
/// the walk has seen them all. The walk keeps a provisional space for every step a threshold
/// could call a word gap, and [`Interpreter::settle_word_gaps`] takes back the ones the font's own
/// threshold does not.
#[derive(Debug, Clone, Default)]
pub(super) struct WordGaps {
    /// The fonts the steps were shown in, each once, held so that its identity outlives the walk.
    fonts: Vec<Font>,
    /// Every step along a line, in the order the walk measured them.
    steps: Vec<Step>,
}

/// One step [`Interpreter::separate_text`] measured along a line, in text space.
#[derive(Debug, Clone, Copy)]
struct Step {
    /// Which of [`WordGaps::fonts`] the show operation after the step was in.
    font: usize,
    /// How far the pen moved forward between the two show operations.
    along: f32,
    /// The space the gap is measured against: the font's own, or [`NOMINAL_SPACE_EM`].
    space: f32,
    /// How many codes the show operation after the step holds.
    run: usize,
    /// The provisional space the walk put in for this step, if it put one in.
    provisional: Provisional,
}

/// Whether a step left a space in the readback for [`Interpreter::settle_word_gaps`] to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provisional {
    /// Too short for any threshold: no space.
    None,
    /// A space at this byte of the readback.
    At(usize),
    /// A space that was put in and then discarded with the text around it — an `/ActualText`
    /// replaced it (§14.9.4) — still counted in [`Interpreter::inferred_separators`].
    Discarded,
}

impl WordGaps {
    /// The index of `font` in [`Self::fonts`], adding it the first time it is seen.
    fn font_index(&mut self, font: &Font) -> usize {
        // A line is usually one font, so the step before is asked first.
        if let Some(last) = self.steps.last()
            && self
                .fonts
                .get(last.font)
                .is_some_and(|held| same_font(held, font))
        {
            return last.font;
        }
        let found = self.fonts.iter().rposition(|held| same_font(held, font));
        found.unwrap_or_else(|| {
            self.fonts.push(font.clone());
            self.fonts.len().saturating_sub(1)
        })
    }

    /// Forgets the provisional spaces at or after `len`, which the readback has just been cut to.
    ///
    /// Each one is still a step the page showed, so it stays in its font's steps; only its place
    /// in the text is gone.
    pub(super) fn discard_from(&mut self, len: usize) {
        for step in self.steps.iter_mut().rev() {
            match step.provisional {
                Provisional::At(at) if at >= len => step.provisional = Provisional::Discarded,
                Provisional::At(_) => break,
                Provisional::None | Provisional::Discarded => {}
            }
        }
    }

    /// How many steps are held, for a rewind to come back to.
    pub(super) fn len(&self) -> usize {
        self.steps.len()
    }

    /// Takes back every step measured after the first `len`.
    pub(super) fn truncate(&mut self, len: usize) {
        self.steps.truncate(len);
    }

    /// Each font's threshold, as a share of the space its steps were measured against.
    fn thresholds(&self) -> Vec<f32> {
        let mut shares: Vec<Vec<(f32, usize)>> = vec![Vec::new(); self.fonts.len()];
        for step in &self.steps {
            let share = step.along / step.space;
            if let Some(held) = shares.get_mut(step.font)
                && share.is_finite()
            {
                held.push((share, step.run));
            }
        }
        shares
            .into_iter()
            .map(|mut held| own_threshold(&mut held))
            .collect()
    }
}

/// Whether two loaded fonts are one: the page's font cache hands out one `Arc` per font object.
fn same_font(a: &Font, b: &Font) -> bool {
    match (a, b) {
        (Font::Program(a), Font::Program(b)) => Arc::ptr_eq(a, b),
        (Font::Type3(a), Font::Type3(b)) => Arc::ptr_eq(a, b),
        (Font::Program(_), Font::Type3(_)) | (Font::Type3(_), Font::Program(_)) => false,
    }
}

/// One font's threshold, read off the shares of a space its steps on this page moved, each with
/// the number of codes the show operation after it held.
///
/// ADR 1515's rule, which only ever **lowers** [`WORD_GAP_SHARE`]: a font whose producer set its
/// word gaps nearer its kerning than its own space is the one the constant reads as unbroken text
/// ("Linktopage1."), and the reverse case had no gain on the corpus that was not paid for by a
/// page number joined to its leader. Where the font shows at least [`STEPS_TO_DECIDE`] steps, the
/// widest empty interval between neighbouring steps — the ratio of its edges, since the clusters are
/// a factor apart rather than a distance — that reaches into
/// [`LEAST_WORD_GAP_SHARE`]..[`WORD_GAP_SHARE`] is the font's gap below its word gaps, provided it
/// is at least [`GAP_TO_DECIDE`] wide; its geometric middle, held inside that window, is the
/// threshold. And the steps it reads differently from the constant must, more often than not,
/// lead into a show operation of more than one code — a word — because a step before a single
/// code is a letter set apart, and a threshold lowered on letter-spaced type would spell
/// "Ta b l e". Otherwise the constant stands.
fn own_threshold(shares: &mut [(f32, usize)]) -> f32 {
    if shares.len() < STEPS_TO_DECIDE {
        return WORD_GAP_SHARE;
    }
    shares.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let mut widest: Option<(f32, f32, f32)> = None;
    for pair in shares.windows(2) {
        let [(low, _), (high, _)] = pair else {
            continue;
        };
        if *low <= 0.0 || *high <= *low || *high < LEAST_WORD_GAP_SHARE {
            continue;
        }
        if *low > WORD_GAP_SHARE {
            break;
        }
        let ratio = high / low;
        if widest.is_none_or(|(_, _, best)| ratio > best) {
            widest = Some((*low, *high, ratio));
        }
    }
    let Some((low, high, ratio)) = widest else {
        return WORD_GAP_SHARE;
    };
    if ratio < GAP_TO_DECIDE {
        return WORD_GAP_SHARE;
    }
    let threshold = (low * high)
        .sqrt()
        .clamp(LEAST_WORD_GAP_SHARE, WORD_GAP_SHARE);
    let (moved, words) = shares
        .iter()
        .filter(|(share, _)| *share > threshold && *share <= WORD_GAP_SHARE)
        .fold((0_usize, 0_usize), |(moved, words), (_, run)| {
            (
                moved.saturating_add(1),
                words.saturating_add(usize::from(*run > 1)),
            )
        });
    if words.saturating_mul(2) < moved {
        return WORD_GAP_SHARE;
    }
    threshold
}

/// What a text object owns, as against what the graphics state does.
///
/// ISO 32000-2 §9.4.1 draws the line:
///
/// > In addition, three parameters may be specified only within a text object and shall not
/// > persist from one text object to the next
///
/// Two of the three are fields here. The third, `Trm`, "is actually just an intermediate
/// result" and is recomputed for each glyph in [`Interpreter::show_text`] rather than
/// stored. The accumulated clipping path joins them because it has exactly the same scope —
/// §9.3.6 starts it at `BT` and consumes it at `ET` — and because keeping it out of
/// [`GraphicsState`] is what stops `q`/`Q` from saving and restoring something the
/// specification never puts in the graphics state.
///
/// A `BT` resets the whole struct, which is Table 105's requirement for the two matrices
/// and §9.3.6's for the third field, in one line that cannot get one of them wrong.
#[derive(Debug, Default)]
pub(super) struct TextObject {
    /// `Tm`, the text matrix.
    pub(super) matrix: Transform,
    /// `Tlm`, the text line matrix: `Tm` as it was at the start of the current line.
    pub(super) line: Transform,
    /// Glyph outlines accumulated by rendering modes 4 to 7, already in page space.
    ///
    /// Empty means no clipping mode has shown a glyph with an outline, which §9.3.6 makes a
    /// meaningful state of its own rather than an empty clip — see
    /// [`Interpreter::end_text_object`].
    pub(super) clip: Path,
    /// Where this object's glyphs have marked the page under a paint that composites.
    ///
    /// `None` for a Type 3 glyph, whose ink is a content stream this does not run twice to
    /// find out. Accumulated rather than reported per glyph because knockout is a property
    /// of the *text object*: one glyph cannot overlap itself, so the difference §9.3.8
    /// describes needs two — see [`Interpreter::end_text_object`].
    pub(super) composited: Vec<Option<Rect>>,
    /// Whether two of those glyphs were found to overlap, which is what `Tk` would change.
    pub(super) knockout_owed: bool,
    /// Command ranges holding one glyph's fill and stroke, for §11.7.4.4's implicit group.
    ///
    /// A glyph shown in rendering mode 2 or 6 is filled *and* stroked, and the clause makes
    /// that pair one object rather than two — the same requirement §11.6.2 places on `B`. The
    /// ranges are collected rather than wrapped as they are drawn because §9.3.8's own group
    /// may turn out to enclose the whole object, and a knockout group inside a knockout group
    /// is not something either backend can state; which of the two is built is therefore one
    /// decision, taken at `ET` in [`Interpreter::end_text_object`].
    ///
    /// Each range carries the readings of §11.6.4.3's `/AIS` its two parts were painted under:
    /// the one in force at the showing operator, and whatever a tiling cell among them ran
    /// under — the pair's own, as a `B`'s is (ADR 1306).
    pub(super) combined: Vec<(usize, usize, AlphaSourcesSeen)>,
    /// How many commands the display list held at this object's `BT`.
    ///
    /// §9.3.8 makes a text object with `Tk` true "equivalent to treating the entire text
    /// object as if it were a non-isolated knockout transparency group", so what the group
    /// contains is everything drawn between `BT` and `ET` — which is this mark to the end.
    pub(super) start: usize,
    /// The enclosing content's record of §11.6.4.3's readings, held while this object keeps
    /// its own: §9.3.8's group is the object's glyphs, so the readings it asks about are the
    /// ones they were shown under, from the reading in force at `BT` on
    /// (`Interpreter::open_reading_scope`, ADR 1306). `None` outside a text object.
    pub(super) enclosing_reading: Option<super::transparency::Readings>,
}

/// What one glyph is to have done to it, decided once per show string rather than per glyph.
///
/// §9.3.6's Table 104 is three independent operations — fill, stroke, add to the clipping path
/// — rather than eight cases, and the two knockout questions are answers about the *paint*
/// rather than about the glyph, so all five are constant across a `Tj`.
#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent yes-or-no answers about one glyph, three of them Table 104's \
              own decomposition of the rendering mode; a state machine would have to \
              enumerate the product of five bits that the clause deliberately keeps separate"
)]
struct GlyphPainting {
    /// Modes 0, 2, 4 and 6.
    fills: bool,
    /// Modes 1, 2, 5 and 6.
    strokes: bool,
    /// Modes 4 to 7.
    clipping: bool,
    /// Whether §9.3.8's text knockout could change a pixel of this object.
    knockout_can_show: bool,
    /// Whether it could by a tiling cell's own marks alone, which only painting the glyph
    /// answers, as [`Self::tiled_pair`] is for §11.7.4.4's.
    knockout_by_cells: bool,
    /// Whether §11.7.4.4's implicit group could change a pixel of this glyph.
    combining: bool,
    /// Whether it could by a tiling cell's own marks alone, which only painting the glyph
    /// answers ([`Interpreter::tile`]).
    tiled_pair: bool,
    /// Which of §11.7.4's implicit groups this glyph's parts are wrapped in once painted.
    implicit: Implicit,
    /// The blend mode the fill is painted under, §11.7.4.3's special one included.
    fill_blend: BlendMode,
    /// The blend mode the stroke is painted under.
    stroke_blend: BlendMode,
}

/// Which of ISO 32000-2 §11.7.4's two implicit non-isolated, non-knockout groups a glyph's
/// parts are wrapped in once they are painted.
///
/// The two differ in where §11.6.4.4's alpha constants sit, which is why they are two:
/// §11.7.4.3's last paragraph moves the blend mode alone, and §11.7.4.4's first bullet paints
/// the parts "with an alpha value of 1.0" and composites the group at the stated one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Implicit {
    /// Neither clause asks for one.
    None,
    /// §11.7.4.3's last paragraph, around an object §11.7.4.4 does not address.
    Special,
    /// §11.7.4.4's first bullet, around a glyph filled and stroked by mode 2 or 6.
    FirstBullet,
}

impl GlyphPainting {
    /// Reads Table 104's mode and the two clauses that ask about the paint behind it.
    ///
    /// A hidden optional-content layer suppresses the two operations that mark the page and
    /// *not* the clip: §8.11.3.1 lists clipping among the "graphics state operations" that
    /// "shall still be applied", and requires that "graphics state parameters that persist
    /// past the end of a marked-content section shall be the same whether the optional content
    /// is visible or not". The clip a text object leaves behind is one of those, since it
    /// outlives the `ET` that built it.
    fn read(mode: i64, hidden: bool, state: &GraphicsState, parts: Parts) -> Self {
        let fills = matches!(mode, 0 | 2 | 4 | 6) && !hidden;
        let strokes = matches!(mode, 1 | 2 | 5 | 6) && !hidden;
        // §11.7.4.4 applies to "the painting of glyphs with text rendering mode 2 or 6",
        // which is `fills && strokes`, and its NOTE 1 says the rule "is independent of the
        // text knockout parameter in the graphics state" — so this is a different condition
        // from `knockout_can_show` below, not a special case of it. The other two halves are
        // §11.6.2's, for the same reason they are there: the paint has to composite at all,
        // and both parts have to mark the page.
        // A part that keeps a component of the backdrop composites with what is under it
        // however opaque it is, which is one more way for the portions of an object to be
        // composited with one another; and §11.7.4.4's *first* bullet, where it applies, is a
        // group of its own rather than §11.4.6's, so a pair under it is not recorded here.
        // `Interpreter::combined_overprint` decides that.
        let pair = fills
            && strokes
            && parts.first_bullet == FirstBullet::No
            && state.fill_marks()
            && state.stroke_marks();
        let combining = pair && (state.paint_composites() || parts.overprints());
        // §11.6.7 makes what a tiling cell evaluates to the part's own opacity, so a part
        // painted through one may composite where the state does not (ADR 1306).
        let tiled_pair = pair
            && !combining
            && [&state.fill_pattern, &state.stroke_pattern]
                .into_iter()
                .any(|pattern| matches!(pattern, Some(PatternPaint::Tiling(_))));
        Self {
            fill_blend: parts.fill,
            stroke_blend: parts.stroke,
            fills,
            strokes,
            clipping: matches!(mode, 4..=7),
            // §9.3.8: with `Tk` true — its initial value — the whole text object behaves as a
            // non-isolated knockout group, so "later glyphs shall overwrite ('knock out')
            // earlier ones in the area of overlap". We composite each glyph against what is
            // already on the page, which is exactly the `Tk` false behaviour. Two conditions
            // have to hold before the models can differ, and both are checked rather than
            // assumed: the paint has to composite at all — an opaque glyph under the Normal
            // blend mode overwrites what it covers either way — and two glyphs of the object
            // have to overlap, which most text never does and which only `ET` can know.
            knockout_can_show: (fills || strokes)
                && state.text.knockout
                && state.paint_composites(),
            // §11.6.7 makes what a cell evaluates to the glyph's own opacity (ADR 1306), so a
            // glyph whose paint is a tiling pattern may composite where the state does not.
            knockout_by_cells: (fills || strokes)
                && state.text.knockout
                && !state.paint_composites()
                && [
                    fills.then_some(&state.fill_pattern),
                    strokes.then_some(&state.stroke_pattern),
                ]
                .into_iter()
                .flatten()
                .any(|pattern| matches!(pattern, Some(PatternPaint::Tiling(_)))),
            combining,
            tiled_pair,
            // §11.7.4.3's last paragraph asks for a group around "the object being painted"
            // wherever the special mode is invoked under a mode other than Normal, and
            // §11.7.4.4's first bullet asks for one around a pair. A pair the second bullet
            // takes is neither: that bullet states the construction for such an object
            // itself, down to where the prevailing blend mode goes, so there is no third
            // group around it (ADR 1170).
            //
            // A glyph wrapped this way cannot then be an element of §9.3.8's own group for
            // the text object, because a non-isolated group may not be one (§11.4.6 NOTE 6,
            // `implicit_knockout_group`). Where both are owed the glyph's group is built and
            // the text object's is named instead — §11.7.4.4's NOTE 1 makes this rule
            // independent of the text knockout parameter, and §9.3.8's needs two glyphs to
            // overlap before it can change a pixel at all.
            implicit: match parts.first_bullet {
                FirstBullet::Group => Implicit::FirstBullet,
                FirstBullet::No | FirstBullet::AsPainted
                    if !combining
                        && implicit_group_owed(
                            state,
                            [
                                if fills { parts.fill } else { BlendMode::Normal },
                                if strokes {
                                    parts.stroke
                                } else {
                                    BlendMode::Normal
                                },
                            ],
                        ) =>
                {
                    Implicit::Special
                }
                FirstBullet::No | FirstBullet::AsPainted => Implicit::None,
            },
        }
    }
}

/// What §11.7.4's reading of the graphics state answered for one glyph's two parts.
///
/// Three values that always travel together, for the reason `ImagePlacement` is a struct: a
/// call site that would otherwise be a row of unlabelled booleans.
#[derive(Debug, Clone, Copy)]
struct Parts {
    /// The blend mode the fill paints under (`Interpreter::overprint_blend`).
    fill: BlendMode,
    /// The blend mode the stroke paints under.
    stroke: BlendMode,
    /// Which of §11.7.4.4's first bullet's three shapes the pair takes.
    first_bullet: FirstBullet,
}

impl Parts {
    /// Whether either part carries §11.7.4.3's special overprinting blend mode.
    fn overprints(self) -> bool {
        matches!(self.fill, BlendMode::Overprint(_))
            || matches!(self.stroke, BlendMode::Overprint(_))
    }
}

impl TextObject {
    /// Records where a glyph marked the page, and whether §9.3.8 could show on this object.
    ///
    /// `bounds` is `None` where the ink is not known — a Type 3 glyph — and an unknown box is
    /// taken to overlap everything, which is the safe direction for a *report*: it may say a
    /// text object could differ where it does not, and never the reverse.
    fn note_knockout(&mut self, bounds: Option<Rect>) {
        let overlaps = self.composited.iter().any(|other| match (other, bounds) {
            (Some(first), Some(second)) => {
                first.min.x < second.max.x
                    && second.min.x < first.max.x
                    && first.min.y < second.max.y
                    && second.min.y < first.max.y
            }
            _ => true,
        });
        self.knockout_owed |= overlaps;
        self.composited.push(bounds);
    }
}

/// What a page's codes got out of one font, tallied while they are shown.
#[derive(Debug, Clone, Default)]
pub(super) struct Coverage {
    /// Codes that reached an outline.
    pub(super) drawn: u32,
    /// Codes that did not.
    pub(super) empty: u32,
    /// How many of `empty` were §9.10.2's uncovered characters, which decides which of the
    /// two reports a silent font gets.
    pub(super) uncovered: u32,
    /// How many of `empty` reached a glyph the program's repair could not draw — a CID-keyed
    /// CFF read against an empty Private DICT, whose glyph calls a local subroutine (ADR 0808).
    pub(super) lost: u32,
    /// The repair's own sentence, taken from the font the first time `lost` is raised, so that
    /// the page's report can say what was done to the program and not only what it cost.
    pub(super) shortfall: Option<String>,
    /// A `glyf` composite one of the empty codes reached that includes itself, which describes no
    /// outline at all: the program's fault, named in the report (ADR 1411).
    pub(super) cycle: Option<u16>,
    /// Whether the font's descriptor sets both Table 121's Symbolic and Nonsymbolic flags, which
    /// §9.8.2 says "shall not both be set": the reading that chose the empty glyphs is the one the
    /// clause's "should always check the Symbolic flag" gives such a file (ADR 1411).
    pub(super) both_flags: bool,
    /// Whether the program describes no outline for any glyph it holds, asked once at the first
    /// empty code. Such a font draws nothing by §9.7.6.3's own route, and the page does not report
    /// it (ADR 1411).
    pub(super) outline_free: Option<bool>,
}

/// What one code contributed to the page's readback.
///
/// Three states rather than a string, because the difference between the last two decides
/// whether a code that reached no outline is a mark the reader lost. A code that reads back as
/// a space is *meant* to have no outline; a code §9.10.2 could not name says nothing either
/// way, and taking the second for the first is a wrong answer that reports nothing.
///
/// **They are separate states because a test on the buffer cannot tell them apart** (ADR
/// 0311): `self.text[start..].chars().all(char::is_whitespace)` is satisfied vacuously by an
/// empty slice, so a font that named none of its codes would read as a page of spaces, and
/// inside §14.8.2.5.3's reversal the readback is collected per code and appended after the
/// string, so *every* code's slice is empty there. Asking the font what it said, rather than
/// asking the buffer what arrived, answers both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readback {
    /// Text, at least one character of which is not whitespace.
    Characters,
    /// Text, all of it whitespace.
    Whitespace,
    /// Nothing at all: every one of §9.10.2's methods, its closing permission and §9.3.3's
    /// naming of code 32 declined, or the producer's own mapping is the empty string.
    Nothing,
}

impl Readback {
    /// Classifies what [`Font::text`] appended for one code.
    fn of(named: bool, text: &str) -> Self {
        if !named || text.is_empty() {
            Self::Nothing
        } else if text.chars().all(char::is_whitespace) {
            Self::Whitespace
        } else {
            Self::Characters
        }
    }

    /// Whether this readback says a mark was owed.
    ///
    /// Only characters do. A space is *meant* to have no outline, and a code §9.10.2 could not
    /// name says nothing about what the page owed — the clause's own words are "there is no way
    /// to determine what the character code represents", which is not evidence in either
    /// direction and must not be read as either.
    ///
    /// **It is a question about the reader and not about the picture** (ADR 0520).
    /// Whether the *program* answered a code is decided by the glyph the code reached, and
    /// needs no character: a route that ends at no glyph at all ended without an answer
    /// whatever §9.10.2 could or could not say about the code. So this decides which readback
    /// lost something and never whether a mark did.
    fn names_a_mark(self) -> bool {
        self == Self::Characters
    }
}

/// A glyph's box, mapped by §9.4.4's text rendering matrix.
///
/// The corners in glyph space are (0, descent), (advance, descent), (advance, ascent) and
/// (0, ascent) — the advance along the baseline and the font's own reach above and below it —
/// and they go round the quadrilateral in that order so that a consumer can draw it as a
/// polygon without sorting.
fn glyph_quad(advance: f32, extent: (f32, f32), transform: Transform) -> [f32; 8] {
    let (ascent, descent) = extent;
    let corner = |x: f32, y: f32| {
        let point = transform.apply(Point::new(x, y));
        (point.x, point.y)
    };
    let (a, b, c, d) = (
        corner(0.0, descent),
        corner(advance, descent),
        corner(advance, ascent),
        corner(0.0, ascent),
    );
    [a.0, a.1, b.0, b.1, c.0, c.1, d.0, d.1]
}

impl Interpreter<'_> {
    /// Adds one show string's worth of coverage to a font's tally.
    ///
    /// Per *string* rather than per glyph, which is not a style choice: the map is keyed by the
    /// resource name and a lookup per glyph costs **2%** of interpretation on the specification's
    /// own page, measured by `callgrind_interpret`. The font cannot
    /// change inside a show string — only `Tf` changes it — so the counts are accumulated in
    /// three integers and applied once.
    fn tally_glyph(&mut self, name: &str, counted: Coverage) {
        // `entry` would take the resource name by value, which is an allocation per show
        // string whether or not the font is already in the map — **2.2% of interpretation**
        // on the specification's own page, measured by stubbing this function out. A page
        // names two or three fonts and shows thousands of strings through them, so the
        // lookup that allocates is the one that almost never has to.
        if let Some(entry) = self.glyph_coverage.get_mut(name) {
            entry.drawn = entry.drawn.saturating_add(counted.drawn);
            entry.empty = entry.empty.saturating_add(counted.empty);
            entry.uncovered = entry.uncovered.saturating_add(counted.uncovered);
            entry.lost = entry.lost.saturating_add(counted.lost);
            if entry.shortfall.is_none() {
                entry.shortfall = counted.shortfall;
            }
        } else {
            self.glyph_coverage.insert(name.to_owned(), counted);
        }
    }

    /// Draws a string, advancing the text matrix.
    ///
    /// # The positioning arithmetic
    ///
    /// Each glyph is placed by the text rendering matrix, which is the font size and
    /// horizontal scaling, times the text matrix, times the current transform. The advance
    /// after each glyph is `(w0 * size + char_spacing + word_spacing) * horizontal_scale`,
    /// where `w0` is the glyph's width in em units and word spacing applies only to a
    /// single-byte code 32.
    ///
    /// Getting the order wrong produces text that is present but misplaced, which looks
    /// like a font bug and is really an arithmetic one.
    ///
    /// # The rendering mode
    ///
    /// §9.3.6 Table 104's eight modes are three independent operations — fill, stroke, add
    /// to the clipping path — rather than eight cases, and they are read that way below.
    /// The clause makes each behave as it would for a path: "Stroking, filling, and clipping
    /// shall have the same effects for a text object as they do for a path object … although
    /// they are specified in an entirely different way."
    #[expect(
        clippy::too_many_lines,
        reason = "one pass over a show string's codes, and every step of it is a clause: the \
                  readback, the text layer's geometry, the two kinds of glyph and §9.3's four \
                  spacing parameters. Splitting it would need nine parameters to carry the \
                  loop's state into the piece that was moved"
    )]
    pub(super) fn show_text(&mut self, bytes: &[u8], state: &GraphicsState, text: &mut TextObject) {
        let Some(font) = state.text.font.clone() else {
            // Text we cannot draw is counted so the page says it is incomplete — unless the
            // layer it belongs to is off, in which case not drawing it is correct.
            if !self.is_hidden() {
                self.text_operations = self.text_operations.saturating_add(1);
            }
            return;
        };

        if font.is_metrics_only() {
            self.advance_through_a_refused_font(&font, bytes, state, text);
            return;
        }

        // The three operations of Table 104, and the two clauses that ask about the paint
        // behind them; see `GlyphPainting::read`. Mode 3 does none of the three and mode 7
        // only the last, which is what an OCR layer under a scanned image uses; either way
        // the text matrix still advances and the extracted text still accumulates, because
        // §9.3.6 requires it — "The e and f components of Tm shall be updated for each glyph
        // drawn when using text rendering mode 3 or 7 in exactly the same way as would be
        // done for other text rendering modes."
        // ISO 32000-2 §11.7.4.3's special overprinting blend mode, once per show-text operator
        // rather than once per glyph: it is a function of the graphics state, which no glyph of
        // one operator changes. `state.blend` on every page that does not overprint inside a
        // four-component blending colour space.
        let mut parts = Parts {
            fill: self.overprint_blend(state, false),
            stroke: self.overprint_blend(state, true),
            first_bullet: FirstBullet::No,
        };
        if matches!(state.text.render_mode, 2 | 6) && !self.is_hidden() {
            parts.first_bullet = Interpreter::combined_overprint(state, [parts.fill, parts.stroke]);
        }
        let painting = GlyphPainting::read(state.text.render_mode, self.is_hidden(), state, parts);
        // Once per show-text operator, for the parts its glyphs paint (ADR 1311).
        if painting.fills {
            self.note_colourants_without_a_plane(&state.fill_space);
        }
        if painting.strokes {
            self.note_colourants_without_a_plane(&state.stroke_space);
        }
        // Inside §11.7.4.4's first-bullet group "the fill and stroke shall be performed with an
        // alpha value of 1.0", so the two constants are lifted off the parts and on to the group
        // `show_program_glyph` wraps them in. Once per show-text operator rather than once per
        // glyph, because it is a function of the graphics state and no glyph of one operator
        // changes that; only the *paint* is built from it, since §11.7.5.2's opacity conditions
        // are about the object as it composites and `state` is what says that.
        let lifted =
            (parts.first_bullet == FirstBullet::Group).then(|| state.with_opaque_constants());
        let paints = lifted.as_ref().unwrap_or(state);
        let GlyphPainting {
            fills,
            strokes,
            clipping,
            ..
        } = painting;
        let size = state.text.size;
        let scale = state.text.horizontal_scale;

        let space = Self::space(&font, size);
        let vertical = font.is_vertical();

        // §14.8.2.5.3: inside a `ReversedChars` sequence, "the sequence of the characters as
        // found in the show string operator shall be reversed before using them. If the
        // sequence encompasses multiple show strings, only the individual characters within
        // each string shall be reversed." So the readback of *this* string is collected per
        // code and appended backwards, and the reversal is per code rather than per `char`:
        // what the clause reverses are the characters the show string states, and one code
        // may map to several — a ligature's `/ToUnicode` says `fi`, which reversing by `char`
        // would spell `if`.
        //
        // The inferred word breaks `separate_text` adds are suppressed inside the string for
        // the same clause: such a block "may have a SPACE (U+0020) character or other
        // whitespace characters at the beginning or end to indicate a word break … but shall
        // not contain interior SPACE characters", so a break is something the file states
        // rather than something a gap implies — and the glyphs of a reversed string run
        // against the writing direction, where a gap means nothing.
        let reversing = self.reversed_chars > 0;
        let mut pieces: Vec<String> = Vec::new();
        // The quadrilaterals of a reversed string, in the order the glyphs were *placed*, so
        // that they can be paired with their pieces when those are appended backwards.
        let mut reversed_quads: Vec<[f32; 8]> = Vec::new();
        // One show string's worth of glyph coverage, applied to the font's tally once at the
        // end: see `tally_glyph` for why it is not applied per code.
        let mut coverage = Coverage::default();

        // Table 120's `/Ascent` and `/Descent`, which say how tall this font's line is. Read
        // once per show operation: they are a property of the font. Table 120 requires neither
        // of a Type 3 font, so its box is the em box.
        let extent = match &font {
            Font::Program(program) => program.extent(),
            Font::Type3(_) => (1.0, 0.0),
        };

        // One separation decision per show string, taken before its first glyph, because
        // §9.4.4 leaves nothing inside one to infer from. The clause's combined displacement
        // is `tx = ((w0 − Tj/1000) × Tfs + Tc + Tw) × Th`, and between two codes of one
        // string the `Tj` term is absent: what separates them is the first glyph's own width
        // plus `Tc`, which applies to every pair alike and is tracking rather than a word
        // break, plus `Tw`, which §9.3.3 applies to the single-byte code 32 alone. So the only
        // word gap a show string can state is that code, and `Font::text` reads it as the
        // character §9.3.3 names rather than as a distance. The separation *between* show
        // operations still has a position to read, which is where `Tj`'s adjustment and every
        // `Td`, `T*` and `Tm` land.
        let codes = font.decode(bytes);
        if !codes.is_empty() {
            self.separate_text(&font, text.matrix, size, scale, space, codes.len());
        }

        for code in codes {
            let advance_em = font.advance(code);
            // §9.7.4.3's second set of metrics, which decide where the glyph is drawn
            // relative to the current text position and where that position goes next.
            let program_metrics = match &font {
                Font::Program(program) => program.vertical_metrics(code),
                Font::Type3(_) => ([0.0, 0.0], [0.0, 0.0]),
            };

            let start = self.text.len();
            let read = self.read_back(&font, code, reversing.then_some(&mut pieces));
            if read == Some(Readback::Nothing)
                && let Some(gap) = font.naming_gap(code)
            {
                // §9.10.2 exhausted on a code the page *showed*. Counted rather than reported,
                // for ADR 0152's reason one column over — a report would cost the oracle a
                // judged page (trap 11) for a shortfall in the readback and not in the picture
                // — but counted rather than nothing at all, because a refusal that says nothing
                // is indistinguishable from a page with no text on it.
                //
                // Counted *by cause*, because the total cannot say whether the clause has no
                // answer or this program did not take one it states; `UnnamedCodes` has the
                // argument. `PDFVIEWER_TRACE_UNNAMED_CODE=1` names each one on stderr, the same
                // idiom the missing-glyph trace below uses, and it is what shows the glyph name
                // behind an `UnlistedName` — the one variant where what the name *is* decides
                // whose gap it is.
                if std::env::var_os("PDFVIEWER_TRACE_UNNAMED_CODE").is_some() {
                    eprintln!(
                        "UNNAMED font=/{} code={} gap={gap:?}",
                        state.text.font_name,
                        code.value()
                    );
                }
                self.codes_without_a_character.count(&gap);
            }

            // Glyph space to text space: scale by the font size, apply horizontal scaling and
            // rise, then the text matrix and the current transform. §9.4.4 calls this the text
            // rendering matrix, and both kinds of glyph are placed by it — the difference is
            // only what is placed.
            //
            // Computed here rather than inside the branch below because the *text layer* wants
            // it for every code, including the ones rendering modes 3 and 7 draw nothing for:
            // an OCR layer under a scanned page is invisible text that a person still selects.
            let glyph_to_text =
                Self::glyph_to_text(size, scale, state.text.rise, program_metrics.1);
            let glyph_to_user = glyph_to_text.then(text.matrix);
            let transform = glyph_to_user.then(state.transform);
            let quad = glyph_quad(advance_em, extent, transform);
            if reversing {
                reversed_quads.push(quad);
            } else {
                let span = start..self.text.len();
                self.text_layer.push(Placed { span, quad });
            }

            if (fills || strokes || clipping) && size != 0.0 {
                let glyph_fill_clip = self.paint_clip(state, true);
                match &font {
                    Font::Program(program) => {
                        if let Some(outline) = program.outline(code) {
                            self.show_program_glyph(
                                &outline,
                                [transform, glyph_to_user],
                                (state, paints, glyph_fill_clip),
                                text,
                                painting,
                            );
                            coverage.drawn = coverage.drawn.saturating_add(1);
                            // A mark was made, and the question this asks is whether it is the
                            // *shape* the producer chose. §9.7.5.1's NOTE: "the horizontal and
                            // vertical variants of a CMap specify different CIDs for a given
                            // character code", so a vertical CID names a form — and a
                            // substituted face, reached by character alone (§9.7.4.2), draws
                            // whatever it has unless it states that form. Counted where it does
                            // not, and counted here rather than beside the two arms below
                            // because this is the arm where something was drawn: the other two
                            // are a mark missed and a mark the font meant not to make, and this
                            // is a mark made in the wrong shape. A report would cost the oracle
                            // a judged page for a statement about a face (ADRs 0152, 0763), so
                            // this is a number and `Shortfall` is where it goes (ADR 0764).
                            if let Some(character) = program.unsupplied_vertical_form(code) {
                                self.codes_without_a_vertical_form =
                                    self.codes_without_a_vertical_form.saturating_add(1);
                                // The same idiom as the two traces below, and it prints the
                                // character because that is what says which form was wanted.
                                if std::env::var_os("PDFVIEWER_TRACE_VERTICAL_FORM").is_some() {
                                    eprintln!(
                                        "UPRIGHT font=/{} code={} char={character:?}",
                                        state.text.font_name,
                                        code.value()
                                    );
                                }
                            }
                        } else if program.uncovered_character(code).is_some() {
                            // §9.10.2 gave this code a character and the substitute face has
                            // no glyph for it, so a mark the document states is not made.
                            // Tallied rather than reported here: see `glyph_coverage`.
                            coverage.empty = coverage.empty.saturating_add(1);
                            coverage.uncovered = coverage.uncovered.saturating_add(1);
                        } else if read.is_some_and(|read| read != Readback::Whitespace) {
                            // The program made no mark, and the code did not read back as a
                            // **space** — which is the one readback that answers this by
                            // itself, because a space is *meant* to have no outline. Measured
                            // rather than assumed: counting one took the corpus's incomplete
                            // documents from 79 to 109, and twenty-two of the thirty new
                            // reports named a single code (trap 11 — print what a condition
                            // matched before trusting it).
                            //
                            // Everything else divides on the *glyph* rather than on the
                            // reading (ADR 0520). §9.6.5.4 and §9.7.4.2 state the routes
                            // from a code to a glyph, and this asks which of two things
                            // happened at the end of one. A code that reached a glyph the
                            // program contains has been answered: what that glyph draws is the
                            // program's own statement, and a glyph with no contours states a
                            // mark of nothing — which is how every sfnt in existence stores a
                            // space. A code that reached no glyph, or reached `.notdef`, was
                            // not answered: §9.6.5.2 makes `.notdef` what is shown when "an
                            // encoding maps to a character name that does not exist in the
                            // Type 1 font program", and §9.7.6.3 makes CID 0 what is
                            // substituted when "no glyph exists for that CID", so glyph 0 is
                            // the program saying it has none.
                            let blank = program
                                .glyph_index(code)
                                .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH);
                            // What the report at the end of the page says about *why* nothing
                            // was drawn, asked here where the code and the program are both at
                            // hand, and each asked once per font (ADR 1411).
                            if coverage.cycle.is_none() {
                                coverage.cycle = program.composite_cycle(code);
                            }
                            coverage.both_flags |= program.states_both_symbolic_flags();
                            if coverage.outline_free.is_none() {
                                coverage.outline_free = Some(program.holds_no_outline());
                            }
                            // Asked before the two arms below, because a glyph the program's
                            // repair could not supply has an index and no contours *drawn* —
                            // which the first arm would read as the program describing it
                            // empty. It is the one empty outline here with a known reason, and
                            // the page says it at the end (`Interpreter::glyph_coverage`);
                            // a well-formed program never reaches this.
                            if program.glyph_lost_to_repair(code) {
                                coverage.empty = coverage.empty.saturating_add(1);
                                coverage.lost = coverage.lost.saturating_add(1);
                                if coverage.shortfall.is_none() {
                                    coverage.shortfall = program.repair_shortfall();
                                }
                            } else if blank {
                                // A glyph the program contains and describes as empty is the
                                // program answering, so nothing was missed here. What a
                                // *reader* lost is the readback's question, and §9.10.2's own
                                // answer for a code it cannot name is "there is no way to
                                // determine what the character code represents" — not evidence
                                // in either direction. So this half stays gated on a named
                                // character; the rest is in `codes_without_a_character`.
                                if read.is_some_and(Readback::names_a_mark) {
                                    coverage.empty = coverage.empty.saturating_add(1);
                                    self.codes_reaching_a_blank_glyph =
                                        self.codes_reaching_a_blank_glyph.saturating_add(1);
                                }
                            } else {
                                // The route ended without an answer, which is a fact about the
                                // *program* and needs no character to establish. Gating it on
                                // the readback made `issue17333.pdf` — one `Tj`, one code, an
                                // embedded subset, and §9.6.5.4's algorithm terminating with
                                // nothing — draw an entirely blank page while every counter
                                // that measures the picture read zero and the report below
                                // could not fire, because its `coverage.empty` was never
                                // raised. One of these is still not news on its own — a
                                // producer's deliberate `.notdef` is one — but a font every
                                // one of whose codes comes back empty has drawn nothing the
                                // document asked for, which is exactly what that report says.
                                coverage.empty = coverage.empty.saturating_add(1);
                                self.codes_without_a_glyph =
                                    self.codes_without_a_glyph.saturating_add(1);
                            }
                            // `PDFVIEWER_TRACE_MISSING_GLYPH=1` names each one on stderr, the
                            // same idiom `tests/corpus.rs` uses for a document that never
                            // returns. It fires for the whole arm rather than for what was
                            // counted, because the codes the two branches divide are what a
                            // person reading it is trying to tell apart. The readback is there
                            // because the count alone cannot tell a mark that is missing from
                            // a *space* whose font reads it back as something else, and the
                            // glyph index because that is what the two arms above are decided
                            // by.
                            if std::env::var_os("PDFVIEWER_TRACE_MISSING_GLYPH").is_some() {
                                eprintln!(
                                    "MISSING {} font=/{} code={} glyph={:?} read={:?}",
                                    if blank { "blank" } else { "absent" },
                                    state.text.font_name,
                                    code.value(),
                                    program.glyph_index(code),
                                    self.text.get(start..)
                                );
                            }
                        } else {
                            // Neither a mark made nor a mark missed, for one of two reasons,
                            // which [`Readback`] tells apart (ADR 0311).
                            //
                            // A code that reads back as a **space** is *meant* to have no
                            // outline, and the arm above says what that cost when it was
                            // measured.
                            //
                            // `None` — a code inside a Type 3 glyph description — is here for a
                            // second reason: what such a code is, and whether it drew, are
                            // §9.6.4's questions about the glyph rather than this page's. The
                            // test used to be `self.text[start..]` all whitespace, which an
                            // empty slice satisfies vacuously, so a code with no readback at
                            // all sat here too *and was blind inside §14.8.2.5.3's reversal*,
                            // where a code's readback never lands in that slice at all.
                        }
                    }
                    Font::Type3(type3) => {
                        // §9.3.6 on a Type 3 font: the glyph description is run for every
                        // mode but 3 and 7 — which is exactly `fills || strokes`, since the
                        // description does its own painting and the mode's choice between
                        // filling and stroking has nothing to apply to — and "If text
                        // rendering mode is set to a value of 4, 5, 6 or 7, nothing shall be
                        // added to the clipping path."
                        if fills || strokes {
                            self.glyphs = self.glyphs.saturating_add(1);
                            self.draw_type3_glyph(type3, code.value(), state, transform);
                            if painting.knockout_can_show {
                                // A Type 3 glyph's ink is whatever its description painted,
                                // which is not knowable without running it again.
                                text.note_knockout(None);
                            }
                        }
                    }
                }
            }

            // Word spacing applies only to the single-byte code 32 (§9.3.3), which is a rule
            // about the code's encoded length rather than its value — see
            // `pdf_font::Code::takes_word_spacing`. A Type 3 font's codes are all one byte,
            // Table 110 giving it `/FirstChar` and `/LastChar`, so the same test serves both
            // kinds of font.
            let word = if code.takes_word_spacing() {
                state.text.word_spacing
            } else {
                0.0
            };
            let displacement = if vertical {
                program_metrics.0[1]
            } else {
                advance_em
            };
            text.matrix = Self::advance_step(
                displacement,
                size,
                state.text.char_spacing + word,
                scale,
                vertical,
            )
            .then(text.matrix);
            self.text_cursor = Some((text.matrix.e, text.matrix.f));
        }

        if coverage.drawn > 0 || coverage.empty > 0 {
            self.tally_glyph(&state.text.font_name, coverage);
        }
        self.append_reversed(&pieces, reversed_quads);
    }

    /// Moves the pen across a string whose font has no program, drawing and reading nothing.
    ///
    /// ISO 32000-2 §9.4.4 makes the update owed by every code a show string states, whatever
    /// became of the glyph — "[a]fter the glyph is painted, the text matrix shall be updated
    /// according to the glyph displacement and any spacing parameters that apply" — and §9.2.4
    /// puts the displacement in the *font dictionary* as well as in the program, so a font
    /// `pdf-font` refused still states it. What the refusal costs is the mark; what it does not
    /// cost is the position of everything after it, which on a line that continues in a second
    /// font is that font's glyphs. `issue6127.pdf` page 1 is the witness and ADR 1094 the
    /// argument: `/C2_14 1 Tf 5.737 0 Td <0003>Tj /C2_2 1 Tf [<0003>187<000b>…]TJ`, where
    /// `/C2_14` is an `/Identity-H` over a `CIDFont` with no program (§9.7.5.2 forbids the
    /// combination outright) whose descendant's `/W` states 250 for CID 3, and `/C2_2` is a font
    /// this tree loads — so the `TJ` began 3.0158 pt short of where both references put it.
    ///
    /// Three things the ordinary path does are deliberately not done here, and they are one
    /// sentence: this font marked no part of the page. There is no glyph to paint, no
    /// quadrilateral for the text layer to place over blank paper, and no §9.10.2 readback — a
    /// code named here would be selectable text with nothing under it, while the page's own
    /// `Unsupported::Font` already says the program was refused. The string is still counted as
    /// text the page could not draw, exactly as it was before the metrics were kept.
    fn advance_through_a_refused_font(
        &mut self,
        font: &Font,
        bytes: &[u8],
        state: &GraphicsState,
        text: &mut TextObject,
    ) {
        if !self.is_hidden() {
            self.text_operations = self.text_operations.saturating_add(1);
        }
        let vertical = font.is_vertical();
        for code in font.decode(bytes) {
            // §9.3.3's word spacing, on the same test the drawing path uses: the rule is about
            // the code's encoded length rather than its value.
            let word = if code.takes_word_spacing() {
                state.text.word_spacing
            } else {
                0.0
            };
            let displacement = match font {
                // §9.7.4.3's `w1`, whose horizontal component is 0 and whose vertical component
                // `/W2` and `/DW2` state — read from the dictionary like every other metric here.
                Font::Program(program) if vertical => program.vertical_metrics(code).0[1],
                Font::Program(_) | Font::Type3(_) => font.advance(code),
            };
            text.matrix = Self::advance_step(
                displacement,
                state.text.size,
                state.text.char_spacing + word,
                state.text.horizontal_scale,
                vertical,
            )
            .then(text.matrix);
            self.text_cursor = Some((text.matrix.e, text.matrix.f));
        }
    }

    /// §14.8.2.5.3's reversal: one show string's readback, appended backwards.
    ///
    /// Nothing about the *drawing* changed — the glyphs were placed where their positions put
    /// them, and what the clause reverses is what a reader extracts or hears — so each piece
    /// keeps the quadrilateral of the glyph that produced it and only their order changes.
    fn append_reversed(&mut self, pieces: &[String], quads: Vec<[f32; 8]>) {
        for (piece, quad) in pieces.iter().zip(quads).rev() {
            let start = self.text.len();
            self.text.push_str(piece);
            self.text_layer.push(Placed {
                span: start..self.text.len(),
                quad,
            });
        }
    }

    /// Appends one code's text to the readback, or to the string being reversed.
    ///
    /// The two destinations are §14.8.2.5.3's whole difference, and the reversal is per *code*
    /// rather than per `char` because what the clause reverses are the characters "as found in
    /// the show string operator" — one code may map to several, and a ligature's `/ToUnicode`
    /// saying `fi` would come back as `if` from a reversal that worked on characters.
    ///
    /// Returns what the code contributed, or `None` where it contributed nothing *because it is
    /// not the page's text* — a code inside a Type 3 glyph description, below. That is a
    /// different thing from [`Readback::Nothing`], which is a code the page showed and §9.10.2
    /// could not name, and the caller counts only the second.
    fn read_back(
        &mut self,
        font: &Font,
        code: Code,
        reversed: Option<&mut Vec<String>>,
    ) -> Option<Readback> {
        // **Not from inside a Type 3 glyph description.** §9.6.4 makes a glyph description a
        // way of *painting* one glyph — "a glyph in a Type 3 font shall be defined by a
        // content stream that contains the operators that paint the glyph" — so the text
        // operators inside it are the glyph's implementation and not text of the page. What
        // the page showed is the code that invoked it, and §9.10.2 is what says what *that*
        // means.
        //
        // `pr4922.pdf` is the case, and it is why this is here: its Type 3 glyphs are drawn
        // by showing a character of another font, so before this line the page read back
        // "pp2200--4400::" — every character twice, once from the outer code and once from
        // the description that draws it.
        if self.glyph_depth > 0 {
            return None;
        }
        Some(if let Some(pieces) = reversed {
            let mut piece = String::new();
            let named = font.text(code, &mut piece);
            let read = Readback::of(named, &piece);
            pieces.push(piece);
            read
        } else {
            let start = self.text.len();
            let named = font.text(code, &mut self.text);
            Readback::of(named, self.text.get(start..).unwrap_or_default())
        })
    }

    /// The space a gap is measured against before it means a word break rather than kerning.
    ///
    /// The font's own space, because that is what a word break is made of.
    /// A fixed fraction of the font size cannot work: a title set with loose tracking moves
    /// each glyph further than a body-text space, and judging it by size alone spells
    /// "Clarification" as "Clar if ic at ion".
    ///
    /// Taken from the magnitude of the size because §9.3.1's NOTE says "Negative text font
    /// size is permitted", and a negative threshold is below every gap there is — which would
    /// have put a space between every pair of glyphs in the extracted text.
    ///
    /// **Both numbers are choices** (ADRs 1502, 1515): §9.3 and §9.4.4 state no quantity that
    /// separates words, and §14.8.2.6.2 calls any such reading a heuristic. [`NOMINAL_SPACE_EM`] is
    /// the space a font stating none is read with, and the share of it a gap has to exceed is the
    /// font's own, decided by [`Self::settle_word_gaps`].
    fn space(font: &Font, size: f32) -> f32 {
        let stated = font.advance(Code::single_byte(32));
        let space_em = if stated > 0.0 {
            stated
        } else {
            NOMINAL_SPACE_EM
        };
        space_em * size.abs()
    }

    /// Adds a space or a newline to the readback where the glyphs' positions imply one.
    ///
    /// A content stream has no notion of words or lines; it has positions. A glyph placed
    /// against the writing direction, or well off the line, began a new line, and one placed
    /// a noticeable gap along it began a new word. These are the only two separators
    /// reconstructed, because anything more is layout analysis and belongs to a consumer of
    /// this text rather than to the drawing pass. `pdftotext` does do that analysis, which is
    /// why the comparison normalises whitespace away.
    ///
    /// The two axes swap in writing mode 1, where a column advances downward and a new column
    /// is a new line.
    ///
    /// **A heuristic the standard names as one.** §14.8.2.6.2 requires a *tagged* producer to
    /// state its word breaks — "any white-space characters that would be present to separate
    /// words in a pure text representation shall be present in the tagged PDF representation
    /// of the text" — and says what that spares a reader: "the PDF processor can determine
    /// word breaks without having to rely on heuristics based on information such as glyph
    /// positioning on the page, font changes, or glyph sizes". An untagged page leaves exactly
    /// that reliance, so what is below is a **choice** rather than a clause obeyed, and the
    /// standard's own sentence is what says which kind of thing it is.
    ///
    /// **It is called once per show operation and not once per code**, because §9.4.4 leaves
    /// nothing inside one show string to read: see the comment at the call site for the
    /// decomposition, and `Font::text` for the one gap a show string *can* state.
    ///
    /// **The gap is read in text space, where the advance is stated** (ADR 1490). ISO 32000-2
    /// §9.4.4:
    ///
    /// > Both the glyph's shape and its displacement (horizontal or vertical) shall be
    /// > interpreted in text space.
    ///
    /// so the distance a show string moved from where the last glyph left the pen is compared
    /// with `space` — a displacement — in the space that displacement is in:
    /// [`Self::text_space_step`] takes it back through the text matrix's linear part and out of
    /// `Th`, and the direction a glyph advances there is the sign of `Tfs`. Read along user-space
    /// x instead, a mirroring `Tm` turns every `TJ` adjustment that closes a gap into one that
    /// opens it, and a `Tm` that scales measures a gap in different units from the threshold.
    ///
    /// **A gap along the line is decided when the page is finished** (ADR 1515). Its threshold is
    /// a share of `space` that the font's own steps on the page choose, so here every gap wider
    /// than [`LEAST_WORD_GAP_SHARE`] of a space — the least that share can be — leaves a
    /// provisional space, and every gap is recorded in [`Self::word_gaps`] for
    /// [`Self::settle_word_gaps`] to keep or take back.
    fn separate_text(
        &mut self,
        font: &Font,
        matrix: Transform,
        size: f32,
        scale: f32,
        space: f32,
        run: usize,
    ) {
        let vertical = font.is_vertical();
        // The text-space origin under the matrix is simply its translation.
        let here = (matrix.e, matrix.f);
        let Some((last_x, last_y)) = self.text_cursor else {
            return;
        };
        let (x, y) = Self::text_space_step(matrix, scale, (here.0 - last_x, here.1 - last_y));
        // §9.4.4's `tx` and `ty` both carry `Tfs` as a factor, so a negative size — which §9.3.1's
        // NOTE permits — advances the other way, and "along" is measured in that direction. In
        // vertical writing §9.7.4.3's `w1` is negative for a column running down the page, which
        // is the direction the reading already takes as forward.
        let forward = if size < 0.0 { -1.0 } else { 1.0 };
        let (along, across) = if vertical {
            (-y * forward, x)
        } else {
            (x * forward, y)
        };
        if across.abs() > size.abs() * 0.5 {
            self.text.push('\n');
            self.inferred_separators = self.inferred_separators.saturating_add(1);
        } else if along > 0.0 {
            let provisional = if along > space * LEAST_WORD_GAP_SHARE {
                let at = self.text.len();
                self.text.push(' ');
                self.inferred_separators = self.inferred_separators.saturating_add(1);
                Provisional::At(at)
            } else {
                Provisional::None
            };
            let font = self.word_gaps.font_index(font);
            self.word_gaps.steps.push(Step {
                font,
                along,
                space,
                run,
                provisional,
            });
        }
    }

    /// Cuts the readback to `len` bytes, and forgets the provisional spaces cut with it.
    ///
    /// The one way the readback is shortened, so that no provisional space of [`WordGaps`] can
    /// name a byte that something else has since been written to.
    pub(super) fn truncate_readback(&mut self, len: usize) {
        self.text.truncate(len);
        self.word_gaps.discard_from(len);
    }

    /// Writes §14.9.4's replacement where the enclosed operators' readback was cut away, and gives
    /// each code they showed the whole of it.
    ///
    /// §14.9.4 makes `/ActualText` a replacement for the content rather than a description of it,
    /// so a glyph inside the sequence reads back as the replacement and nothing narrower: its span
    /// covers the replacement, rather than whatever bytes of it its own readback once occupied —
    /// which would make what a selection or a caret finds there depend on how many spaces the cut
    /// text happened to hold (ADR 1515).
    pub(super) fn push_replacement(&mut self, replacement: &str) {
        let from = self.text.len();
        self.text.push_str(replacement);
        let to = self.text.len();
        for placed in self.text_layer.iter_mut().rev() {
            if placed.span.start < from {
                break;
            }
            placed.span = from..to;
        }
    }

    /// Keeps or takes back each provisional space by its font's own threshold (ADR 1515).
    ///
    /// Run once, when the page's content and its annotations have all been walked, so that each
    /// font's threshold is read off every step it showed. A space taken back is removed from the
    /// readback and every range over it — the text layer's spans, §14.9's and §14.8.2.2's, the
    /// marked-content sequences, the associated files and the structural annotations — is moved to
    /// match; where every provisional space is kept, nothing is touched.
    pub(super) fn settle_word_gaps(&mut self) {
        let thresholds = self.word_gaps.thresholds();
        let mut removed: Vec<usize> = Vec::new();
        for step in &self.word_gaps.steps {
            let share = thresholds.get(step.font).copied().unwrap_or(WORD_GAP_SHARE);
            if step.along > step.space * share {
                continue;
            }
            match step.provisional {
                Provisional::None => {}
                Provisional::At(at) => {
                    removed.push(at);
                    self.inferred_separators = self.inferred_separators.saturating_sub(1);
                }
                Provisional::Discarded => {
                    self.inferred_separators = self.inferred_separators.saturating_sub(1);
                }
            }
        }
        self.word_gaps = WordGaps::default();
        if removed.is_empty() {
            return;
        }
        // The provisional spaces' bytes rise strictly through the walk — a cut forgets every one
        // after it — so a position moves back by the number of removed bytes before it.
        let mut kept = String::with_capacity(self.text.len().saturating_sub(removed.len()));
        let mut from = 0;
        for &at in &removed {
            kept.push_str(self.text.get(from..at).unwrap_or_default());
            from = at.saturating_add(1);
        }
        kept.push_str(self.text.get(from..).unwrap_or_default());
        self.text = kept;
        let moved =
            |position: usize| position.saturating_sub(removed.partition_point(|&at| at < position));
        let span = |range: &mut std::ops::Range<usize>| {
            *range = moved(range.start)..moved(range.end);
        };
        for placed in &mut self.text_layer {
            span(&mut placed.span);
        }
        for described in &mut self.described {
            span(&mut described.range);
        }
        for artifact in &mut self.artifacts {
            span(&mut artifact.range);
        }
        for marked in &mut self.marked {
            span(&mut marked.range);
        }
        for (range, _) in &mut self.associated {
            span(range);
        }
        for range in &mut self.structural_annotations {
            span(range);
        }
    }

    /// A step between two text positions, from the space the text matrix's translation is in
    /// back into text space with `Th` taken out: the space `space` and the font size measure.
    ///
    /// The translation is not part of a *step*, so only the matrix's linear part is undone. `Th`
    /// multiplies §9.4.4's `tx` alone, so it is divided out of the horizontal component alone,
    /// sign and all: a negative horizontal scaling mirrors the line and is undone with it. A
    /// matrix or a scaling that collapses text space has nothing to undo, and the step is read
    /// as it stands rather than divided by zero.
    fn text_space_step(matrix: Transform, scale: f32, step: (f32, f32)) -> (f32, f32) {
        let linear = Transform::new(matrix.a, matrix.b, matrix.c, matrix.d, 0.0, 0.0);
        let Some(inverse) = linear.invert() else {
            return step;
        };
        let back = inverse.apply(Point::new(step.0, step.1));
        let x = if scale.abs() > f32::EPSILON {
            back.x / scale
        } else {
            back.x
        };
        (x, back.y)
    }

    /// Glyph space to text space: the font size, the horizontal scaling, and the rise.
    ///
    /// §9.2.4 adds one term in writing mode 1: "the glyph position shall be described by a
    /// position vector from the origin used for horizontal writing (origin 0) to the origin
    /// used for vertical writing (origin 1)". The outline is stated relative to origin 0 and
    /// the text position *is* origin 1, so the glyph moves back by `v`, which is zero for
    /// every font in writing mode 0.
    fn glyph_to_text(size: f32, scale: f32, rise: f32, position: [f32; 2]) -> Transform {
        Transform::new(
            size * scale,
            0.0,
            0.0,
            size,
            -position[0] * size * scale,
            (-position[1]).mul_add(size, rise),
        )
    }

    /// §9.4.4's combined displacement, as the translation it applies to the text matrix.
    ///
    /// The clause computes `tx` in horizontal writing mode and `ty` in vertical, "the
    /// variable corresponding to the other writing mode shall be set to 0", and the two
    /// differ in one term: the horizontal scaling multiplies `tx` alone, because `Th` scales
    /// the *width* of a line rather than the advance along it. Character and word spacing are
    /// added to whichever component applies.
    fn advance_step(
        displacement: f32,
        size: f32,
        spacing: f32,
        scale: f32,
        vertical: bool,
    ) -> Transform {
        if vertical {
            Transform::translate(0.0, displacement.mul_add(size, spacing))
        } else {
            Transform::translate(displacement.mul_add(size, spacing) * scale, 0.0)
        }
    }

    /// Fills one glyph outline, which a pattern makes more than a `Fill` command.
    ///
    /// §9.2.3 lets a glyph be painted "in any colour", and §8.7.2 makes a pattern one: "All
    /// patterns shall be treated as colours". A *tiling* pattern is not a paint, though — it
    /// is a cell replayed across an area — so a glyph filled with one is its outline tiled,
    /// exactly as a path is. The transform is the *glyph's* rather than the text object's,
    /// because the outline is in glyph space.
    ///
    /// Answers whether a tiling cell the glyph was filled through composites by its own marks.
    fn fill_glyph(
        &mut self,
        outline: &Arc<Path>,
        transform: Transform,
        painted: (&GraphicsState, &GraphicsState),
        clip: Option<ClipId>,
        blend: BlendMode,
    ) -> bool {
        // The state the glyph is painted under, and the one its *colour* is built from: the
        // two differ only inside §11.7.4.4's first-bullet group, where the parts paint at an
        // alpha constant of 1.0 (`Interpreter::show_program_glyph`).
        let (state, paints) = painted;
        // Borrowed rather than cloned: this runs once per glyph, and cloning the whole
        // `Option<PatternPaint>` would bump a shading's refcount on every glyph of a page whose
        // text is painted with one.
        if let Some(PatternPaint::Tiling(tiling)) = &state.fill_pattern {
            let tiling = Rc::clone(tiling);
            // `paints` rather than `state`, for the same reason the colour is built from it:
            // the cell's own marks are the part §11.7.4.4's first bullet paints at an alpha
            // constant of 1.0, and its group applies the stated one once.
            return self.tile(
                outline,
                transform,
                Tiled::Fill(FillRule::NonZero),
                &tiling,
                paints,
            );
        }
        let transfer = self.mark_transfer(state, Painted::of(state, false));
        let paint = self.fill_paint(paints);
        self.draw_mark(
            Command::Fill {
                // The font hands out shared outlines and the display list keeps them shared: a
                // page of text is the same few dozen glyphs over and over, so this is a refcount
                // rather than a copy of the segments.
                path: Arc::clone(outline),
                transform,
                // Glyph outlines are non-zero filled; even-odd would hollow out counters that
                // overlap, such as in a bold 'B'.
                fill_rule: FillRule::NonZero,
                paint,
                clip,
                mask: state.soft_mask,
                blend,
            },
            transfer,
        );
        false
    }

    /// Strokes one glyph outline, ISO 32000-2 §9.3.6 rendering modes 1, 2, 5 and 6.
    ///
    /// `glyph_to_user` maps the outline from glyph space to the *user* space in effect,
    /// which is the whole reason this is not two lines beside the fill. The clause puts the
    /// stroke's parameters in that space:
    ///
    /// > The graphics state parameters affecting those operations, such as line width, shall
    /// > be interpreted in user space rather than in text space.
    ///
    /// A [`Command::Stroke`]'s width and dash lengths are in its path's own space, so
    /// leaving the outline in em units would have divided the width by the font size and
    /// stretched it by the horizontal scaling — an 11-point glyph would have been outlined
    /// about eleven times too thickly, and a horizontally scaled one anisotropically. Moving
    /// the geometry instead is exact for any text matrix, including one that shears; the
    /// cost is a copy of the outline per stroked glyph, which is paid only by the modes that
    /// stroke and never on the ordinary fill path.
    ///
    /// # A glyph stroked in a tiling pattern is tiled
    ///
    /// §8.7.2's "All patterns shall be treated as colours" makes a glyph's stroke colour no
    /// different from a path's, so a `Tr 1` glyph whose `SCN` names a tiling pattern is never
    /// outlined in whatever solid colour was last set — the silent fallback principle 3 forbids —
    /// but drawn by the same [`Interpreter::tile`] the fill route takes and over the same outline
    /// this function has already moved into user space. ADR 0735.
    ///
    /// Answers whether a tiling cell the glyph was stroked through composites by its own marks.
    fn stroke_glyph(
        &mut self,
        outline: &Arc<Path>,
        glyph_to_user: Transform,
        painted: (&GraphicsState, &GraphicsState),
        blend: BlendMode,
    ) -> bool {
        // As [`Interpreter::fill_glyph`]: the state painted under, and the one the colour is
        // built from.
        let (state, paints) = painted;
        let mut in_user_space = Path::new();
        in_user_space.extend_transformed(outline, glyph_to_user);
        let in_user_space = Arc::new(in_user_space);
        if let Some(PatternPaint::Tiling(tiling)) = &state.stroke_pattern {
            let tiling = Rc::clone(tiling);
            // `paints`, as [`Interpreter::fill_glyph`].
            return self.tile(
                &in_user_space,
                state.transform,
                Tiled::Stroke(&state.stroke),
                &tiling,
                paints,
            );
        }
        let glyph_stroke_clip = self.paint_clip(state, false);
        let transfer = self.mark_transfer(state, Painted::of(state, true));
        let paint = self.stroke_paint(paints);
        self.draw_mark(
            Command::Stroke {
                path: in_user_space,
                transform: state.transform,
                stroke: state.stroke.clone(),
                paint,
                clip: glyph_stroke_clip,
                mask: state.soft_mask,
                blend,
            },
            transfer,
        );
        false
    }

    /// Turns the glyph outlines a text object accumulated into a clip, at its `ET`.
    ///
    /// ISO 32000-2 §9.3.6:
    ///
    /// > At the end of the text object identified by the ET operator the accumulated glyph
    /// > outlines, if any, shall be combined into a single path, treating the individual
    /// > outlines as subpaths of that path and applying the non-zero winding number rule
    /// > (see 8.5.3.3.2, "Non-zero winding number rule"). The current clipping path in the
    /// > graphics state shall be set to the intersection of this path with the previous
    /// > clipping path.
    ///
    /// Intersection is what the display list's `parent` chain already means, so the new clip
    /// is a child of the one in effect. It is set on the live graphics state rather than on a
    /// saved copy because the clause continues: "It remains in effect until a previous
    /// clipping path is restored by an invocation of the Q operator" — so it outlives the
    /// text object, and `Q` is the only thing that ends it.
    ///
    /// # An empty accumulator is not an empty clip
    ///
    /// > If no glyphs are shown or if the only glyphs shown have no outlines (for example,
    /// > if they are ASCII SPACE characters (20h)), no clipping shall occur.
    ///
    /// Clipping to an empty path would hide everything drawn after the text object, which is
    /// the opposite of what the clause says and would be invisible to every metric this tree
    /// owns except pixels somebody else produced. A text object in mode 7 showing one space
    /// is not a hypothetical: it is what a producer emits when a line of OCR text happens to
    /// be blank.
    pub(super) fn end_text_object(&mut self, text: &mut TextObject, state: &mut GraphicsState) {
        // §9.3.8's knockout is a property of the finished object, so this is where it can be
        // judged: two or more glyphs marked the page under a paint that composites, and `Tk`
        // asked for them to knock one another out instead.
        //
        // The condition is deliberately narrow. Treating every text object drawn while `Tk`
        // is true as a group would wrap almost every page in the world, since true is the
        // initial value, and would say nothing: with opaque glyphs and the Normal blend mode
        // the two models produce identical pixels.
        //
        // The clause states the construction exactly, and it is the one §11.4.6's knockout
        // group already has: "the behaviour shall be equivalent to treating the entire
        // text object as if it were a non-isolated knockout transparency group … where each
        // glyph is an individual element in that group's transparency stack", after which
        // "the group results shall be composited with the backdrop, using the Normal blend
        // mode and alpha and soft mask values of 1.0" — which is this command's four other
        // fields. The graphics state is *not* reset for the elements, unlike §11.6.6's group
        // XObject, and it is not: each glyph command already carries the alpha, mask and
        // blend mode in force when it was shown.
        //
        // §11.7.4.4's implicit group is decided here too, and it has to be: a glyph shown in
        // mode 2 or 6 owes a knockout group of its own fill and stroke, and where the object
        // above is built that group is *inside* it. It does not have to be stated, because it
        // computes the same picture flat: in a knockout group every element composites with
        // the initial backdrop, so at each point the topmost element wins, and nesting cannot
        // change which element that is. So the whole-object group subsumes every glyph's, and
        // the per-glyph groups are built only where there is no whole-object group to be
        // inside — `push_combined_glyphs` builds them on the other branch.
        let knockout_owed = text.knockout_owed;
        // The readings the object's glyphs were shown under, and the enclosing record back.
        let (reading, in_force) = match text.enclosing_reading.take() {
            Some(outer) => self.close_reading_scope(outer),
            None => (self.settled_readings(), state.alpha_is_shape),
        };
        if knockout_owed || !text.combined.is_empty() {
            let glyphs = text.composited.len();
            let elements = self.list.split_off_commands(text.start);
            let stated = knockout_owed
                .then(|| {
                    implicit_knockout_group(
                        &elements,
                        reading,
                        self.enclosing_knockout,
                        self.image_masks.shape_masks(),
                    )
                })
                .flatten();
            if let Some(group) = stated {
                self.draw(Command::Group {
                    commands: group.elements,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: group.blend,
                    isolated: group.isolated,
                    knockout: true,
                    // Stated rather than asked: this group carries no clip of its own, and
                    // §8.5.4's intersection at the blit is the only thing the flag decides.
                    // A round that gives it one owes the question — which §11.4.6's
                    // knockout no longer answers by itself (ADR 0554).
                    alpha_is_shape: false,
                    blending: None,
                });
            } else {
                if knockout_owed {
                    self.note(Unsupported::TextKnockout { glyphs });
                }
                self.push_combined_glyphs(elements, text);
            }
        }
        text.knockout_owed = false;
        text.combined.clear();
        text.composited.clear();
        // Whether as §9.3.8's group or glyph by glyph, the glyphs are elements of the
        // enclosing content now, so the reading they were shown under joins its record — and
        // the reading in force at `ET` stays in force, since §9.3.8 lets "[c]hanges made to
        // graphics state parameters within the text object ... persist beyond the end of the
        // text object" (ADR 1319).
        self.absorb_reading(reading, text.start);
        self.note_alpha_source(in_force);

        let path = std::mem::take(&mut text.clip);
        if path.is_empty() {
            return;
        }
        let clip = Clip {
            path,
            // The outlines were mapped into page space as they were collected, because one
            // path cannot carry one transform per glyph.
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: state.clip,
        };
        match self.list.add_clip(clip) {
            Ok(id) => state.clip = Some(id),
            Err(_) => self.note(Unsupported::LimitReached { limit: "max_clips" }),
        }
    }

    /// Draws one glyph of an outline font, in whichever of §9.3.6's three operations apply.
    ///
    /// `places` is the glyph's two transforms: into page space, and into user space — the
    /// second is what a stroke needs, since §9.3.6 makes the stroke's width a user-space
    /// quantity like any other path's.
    fn show_program_glyph(
        &mut self,
        outline: &Arc<Path>,
        places: [Transform; 2],
        painted: (&GraphicsState, &GraphicsState, Option<ClipId>),
        text: &mut TextObject,
        painting: GlyphPainting,
    ) {
        let [transform, glyph_to_user] = places;
        // The state the glyph is painted under, the one its *colour* is built from — the two
        // differ only inside §11.7.4.4's first-bullet group — and the clip its fill takes.
        let (state, paints, fill_clip) = painted;
        if painting.fills || painting.strokes {
            // Marked the page; see `Interpretation::glyphs`. An empty outline — a space in a
            // font that has one — is a glyph the font drew and is counted, because the
            // question this answers is what *kind* of page this is.
            self.glyphs = self.glyphs.saturating_add(1);
        }
        let parts_at = self.list.command_count();
        // §11.7.4.4's pair is read under the readings its own parts are painted under, which
        // only a glyph that may combine asks about.
        let enclosing_reading = (painting.combining || painting.tiled_pair)
            .then(|| self.open_knockout_reading_scope(state.alpha_is_shape));
        let mut cells_composite = false;
        if painting.fills {
            cells_composite |= self.fill_glyph(
                outline,
                transform,
                (state, paints),
                fill_clip,
                painting.fill_blend,
            );
        }
        if painting.strokes {
            cells_composite |= self.stroke_glyph(
                outline,
                glyph_to_user,
                (state, paints),
                painting.stroke_blend,
            );
        }
        if painting.implicit != Implicit::None {
            self.wrap_in_the_implicit_group(state, parts_at, painting.implicit);
        }
        // §11.7.4.4 makes this glyph's fill and stroke one object; the range is recorded and
        // `ET` decides what to build from it. Fewer than two commands is a glyph that marked
        // the page once — an empty outline, or a fill a tiling pattern drew nothing for — and
        // there is nothing for it to composite with.
        if let Some(outer) = enclosing_reading {
            let (reading, _) = self.close_reading_scope(outer);
            self.absorb_reading(reading, parts_at);
            if (painting.combining || cells_composite)
                && self.list.command_count() > parts_at.saturating_add(1)
            {
                text.combined
                    .push((parts_at, self.list.command_count(), reading));
            }
        }
        if painting.clipping {
            // §9.3.6 wants "a single path, treating the individual outlines as subpaths of
            // that path", and the glyphs of one text object have as many transforms as there
            // are glyphs — so the transform is baked in here and the clip carries none. Note
            // that a hidden layer still reaches this line.
            text.clip.extend_transformed(outline, transform);
        }
        // §9.3.8 asks whether the glyphs composite, and a glyph painted through a cell whose
        // marks do composites whatever the state at the `Tj` says.
        if painting.knockout_can_show || (painting.knockout_by_cells && cells_composite) {
            text.note_knockout(outline_bounds(outline, transform));
        }
    }

    /// §11.7.4's implicit group around the commands the glyph starting at `mark` left behind.
    ///
    /// Out of line because the test above it is what every glyph of every page pays and the
    /// answer is [`Implicit::None`] on all but a handful. Measured under callgrind on page 101
    /// of ISO 32000-2 — the tree's densest text page — interpreted fifty times, against the same
    /// build with §11.7.4's decisions and constructions planted away:
    ///
    /// | what stands in `show_text`'s glyph loop | instructions | of one interpretation |
    /// |---|---|---|
    /// | nothing: the rest of §11.7.4 only | +513 947 | 0.041% |
    /// | this call, guarded and out of line | **+2 788 934** | **0.223%** |
    /// | the three-armed match inlined instead | +3 163 973 | 0.253% |
    ///
    /// So the split is worth 375 039 instructions and the wrap *point* is worth the rest: a
    /// call in a loop this large costs the registers spilled around it, and the only way to
    /// stop paying that is not to build the clause's group. It is a page's glyphs times two
    /// instructions or so, and it buys §11.7.4.3's last paragraph and §11.7.4.4's first bullet
    /// on every glyph that owes one. `CLAUDE.md` asks for the number beside the technique;
    /// [`Interpreter::overprint_blend`] carries the same split one clause over.
    #[inline(never)]
    fn wrap_in_the_implicit_group(
        &mut self,
        state: &GraphicsState,
        mark: usize,
        implicit: Implicit,
    ) {
        match implicit {
            Implicit::Special => self.implicit_overprint_group(state, mark),
            Implicit::FirstBullet => self.first_bullet_group(state, mark),
            Implicit::None => {}
        }
    }

    /// Pushes a text object's commands back, wrapping §11.7.4.4's fill-and-stroke pairs.
    ///
    /// ISO 32000-2 §11.7.4.4, of a combined fill and stroke — which "include the B , B\* , b ,
    /// and b\* operators … and the painting of glyphs with text rendering mode 2 or 6":
    ///
    /// > In all other cases, a non-isolated knockout group shall be established. Within the
    /// > group, the fill and stroke shall be performed with their respective prevailing alpha
    /// > constants and the prevailing blend mode. The group results shall then be composited
    /// > with the backdrop, using an alpha value of 1.0 and the Normal blend mode.
    ///
    /// The pairs that reach here are the ones §11.7.4.4 sends to its second bullet: the first
    /// needs overprinting enabled for both operations and the two alpha constants equal, and
    /// [`Interpreter::combined_overprint`] has already built its group or found it to be the
    /// two commands as they stand. The construction is identical to the one the `B` operator
    /// gets in [`Interpreter::end_path`], and NOTE 2 says what it is for — "to avoid having
    /// a non-opaque stroke composite with the result of the fill in the region of overlap,
    /// which would produce a double border effect".
    ///
    /// A pair the backends cannot draw as a knockout — one carrying a soft mask, or a fill a
    /// tiling pattern turned into a group — is pushed flat and named once for the whole text
    /// object, because a report per glyph would name the same gap a hundred times on one line.
    fn push_combined_glyphs(&mut self, elements: Vec<Command>, text: &TextObject) {
        let mut owed = false;
        let mut pairs = text.combined.iter().peekable();
        let mut index = text.start;
        let mut rest = elements.into_iter();
        while let Some(command) = rest.next() {
            let pair = pairs.next_if(|(from, _, _)| *from == index).copied();
            let Some((from, to, reading)) = pair else {
                self.draw(command);
                index = index.saturating_add(1);
                continue;
            };
            let mut parts = vec![command];
            parts.extend(
                rest.by_ref()
                    .take(to.saturating_sub(from).saturating_sub(1)),
            );
            index = to;
            if let Some(group) = implicit_knockout_group(
                &parts,
                reading,
                self.enclosing_knockout,
                self.image_masks.shape_masks(),
            ) {
                self.draw(Command::Group {
                    commands: group.elements,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: group.blend,
                    isolated: group.isolated,
                    knockout: true,
                    // §11.4.6's accumulation is not §11.4.4's union, so this group's raster is not
                    // asked to carry Table 139's shape — see `group_alpha_is_shape`.
                    alpha_is_shape: false,
                    blending: None,
                });
            } else {
                owed = true;
                for part in parts {
                    self.draw(part);
                }
            }
        }
        if owed {
            self.note(Unsupported::CompositedInParts {
                detail: "a glyph filled and stroked by text rendering mode 2 or 6",
            });
        }
    }

    /// Runs one Type 3 glyph description, ISO 32000-2 §9.6.4.
    ///
    /// `text_rendering` is §9.4.4's text rendering matrix — everything the glyph is placed by
    /// except the font's own matrix, which is applied here because it is the font's business
    /// rather than the text object's.
    ///
    /// The steps §9.6.4 lays out for each character code are all here or in
    /// [`crate::type3::Type3Font`]: the encoding and `/CharProcs` lookups are the font's, and
    /// this does the rest — save the state, set the CTM, run the description, restore.
    fn draw_type3_glyph(
        &mut self,
        font: &crate::type3::Type3Font,
        code: u32,
        state: &GraphicsState,
        text_rendering: Transform,
    ) {
        // §9.6.4 b): "If the name is not present as a key in CharProcs, no glyph shall be
        // painted." Neither that nor a code the encoding does not name is a failure — both
        // are defined outcomes — so neither is reported, and both still advance the text
        // position, which the caller does whatever happens here.
        let Some(glyph) = font.glyph(self.document, code) else {
            return;
        };

        // A glyph description may show text in another Type 3 font, which is a recursion a
        // file can build a cycle out of — `ContentStreamCycleType3insideType3.pdf` in the
        // corpus is exactly that. It shares the bound with form XObjects because it is the
        // same danger and the same cost: a nested content stream, and `Interpreter::run` is
        // where the bound is asked (`MAX_FORM_DEPTH`).
        //
        // §9.6.4 says so itself since Errata Collection 3 (Issue #111), which inserts a
        // paragraph below NOTE 1: "Implementations also need to avoid potential infinite
        // recursion if a Type 3 glyph description refers to itself directly or indirectly. The
        // result in all such cases is implementation-dependent." The bound was written from
        // principle 3's budgets rather than from the clause, and the clause now states it —
        // which leaves only *which* implementation-dependent result to produce, and this one
        // is reported rather than silent.

        // Table 110's `/CharProcs`: each value "shall be a content stream that constructs and
        // paints the glyph for that character. The stream shall include as its first operator
        // either d0 or d1 , followed by operators describing one or more graphics objects." So
        // §7.8.2's prefix rule reaches a glyph description, and this clause makes the prefix
        // *faithful* in a way the general argument does not: `d0`/`d1` is required to be first,
        // so any prefix carrying a mark carries the glyph's own declaration ahead of it, and
        // Table 110's `/Widths` — not the description — supplies the advance, so what the
        // damage costs is marks inside this glyph and never the position of the next one.
        // Named by the glyph rather than by the code, because §9.6.4 step b) keys `/CharProcs`
        // that way and two codes may reach one description. `glyph` above returned `Some`, so
        // the encoding does name this code; the fallback is unreachable and is written rather
        // than unwrapped because nothing in the type system says so.
        let name = font.glyph_name(code).map_or_else(
            || "?".to_owned(),
            |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
        );
        let Some(data) = self.content_stream(
            &glyph,
            &format!("a Type 3 glyph description /{name} (§9.6.4)"),
        ) else {
            self.note(Unsupported::Font {
                detail: format!("Type 3 glyph for code {code} could not be decoded"),
            });
            return;
        };

        // §9.6.4: "When the glyph description begins execution, the current transformation
        // matrix (CTM) shall be the concatenation of the font matrix (FontMatrix in the
        // current font dictionary) and the text space that was in effect at the time the
        // text-showing operator was invoked". Everything else is inherited: "Aside from the
        // CTM, the graphics state shall be inherited from the graphics state at the point of
        // invocation of the text-showing operator" — which is what cloning it does, and the
        // clone is also step c)'s save and restore, since nothing the description changes can
        // reach the caller's copy.
        let mut inner = state.clone();
        inner.transform = font.font_matrix().then(text_rendering);

        // §7.8.3's first step for a glyph description, which Errata Collection 3 put in front
        // of §9.6.4's own rule (Issue #128): "the stream dictionary of that glyph description
        // content stream". Resolved here rather than in `Type3Font` because the font holds the
        // `/CharProcs` dictionary and not the decoded streams — a glyph is read when it is
        // drawn — and cloned only where the stream states one, which is the rare case.
        let stated = self
            .document
            .get_key(&glyph.dict, "Resources")
            .as_dict()
            .cloned();

        let saved_uncoloured = self.uncoloured;
        self.glyph_depth = self.glyph_depth.saturating_add(1);
        self.enter_ledger_frame(
            super::ledger::Route::Type3Glyph,
            font.glyph_reference(self.document, code),
        );
        // §7.8.3's search for a glyph description's resources ends at the page: the glyph
        // stream's own dictionary, then the Type 3 font dictionary that held `/CharProcs`, then
        // the page and what §7.7.3.4 gave it — Errata Collection 3's Issue #128, stated in full
        // on [`crate::type3::Type3Font::resources`], whose last parameter is the page's for that
        // reason. **Not the stream whose text-showing operator reached this glyph**, which is a
        // different dictionary whenever that stream is a form with `/Resources` of its own, and
        // which is what this call site passed until ADR 1059.
        // The page's dictionary is reached through an `Arc` rather than borrowed, because
        // `run` takes `&mut self`: one pointer clone per glyph description, against a `Dictionary`
        // that has to outlive the call. [`Interpreter::page_resources`] says why it is owned.
        let page_resources = Arc::clone(&self.page_resources);
        self.run(
            &data,
            font.resources(stated.as_ref(), &page_resources),
            &inner,
        );
        self.leave_ledger_frame();
        self.glyph_depth = self.glyph_depth.saturating_sub(1);
        // `d1` inside the description raised this; the description is over. Restoring rather
        // than clearing is what lets an uncoloured glyph invoke another one without the
        // inner one's end re-enabling colour for the rest of the outer.
        self.uncoloured = saved_uncoloured;
    }
}

#[cfg(test)]
mod tests {
    //! `separate_text`'s gap, read in §9.4.4's text space (ADR 1490).
    //!
    //! Each page is one `TJ` in Helvetica at 20 units, whose `a` and `b` are 556 thousandths wide
    //! and whose space is 278, so — with fewer steps than a font's own threshold is read off (ADR
    //! 1515) — the threshold is 278 × 20 / 1000 × 0.5 = 2.78 text-space units at `20 Tf` and 0.139
    //! at `1 Tf`. Every expected readback follows from §9.4.4's `tx` and the `Tm` written beside it.

    #![expect(
        clippy::arithmetic_side_effects,
        reason = "test code: the fixture's offsets are computed from strings this module wrote"
    )]

    use std::fmt::Write as _;

    use pdf_render::Transform;

    use super::super::Interpreter;

    /// The readback of a one-page document whose content stream is `content`, in Helvetica.
    fn readback(content: &str) -> String {
        readback_in(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            content,
        )
    }

    /// The readback of a one-page document whose content stream is `content`, in `font`.
    fn readback_in(font: &str, content: &str) -> String {
        interpretation_in(font, content).text
    }

    /// The interpretation of a one-page document whose content stream is `content`, in `font`.
    fn interpretation_in(font: &str, content: &str) -> crate::Interpretation {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
             5 0 obj\n{font}\nendobj\n",
            content.len() + 1,
        );
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        let mut cursor = out.len();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(cursor);
            cursor += object.len();
        }
        out.push_str(&body);
        let xref_at = out.len();
        let size = offsets.len() + 1;
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let document =
            pdf_syntax::Document::open(out.into_bytes()).expect("the fixture is a valid file");
        let pages = crate::Pages::new(&document);
        let page = pages.get(0).expect("one page");
        crate::interpret(&document, &page)
    }

    /// A `TJ` of `words` words, each `(ab) -10 (ab)` and then `gap` before the next.
    fn words_set_apart(gap: i32, words: usize) -> String {
        let mut array = String::new();
        for _ in 0..words {
            let _ = write!(array, "(ab) -10 (ab) {gap} ");
        }
        format!("BT /F1 20 Tf 10 100 Td [{array}] TJ ET")
    }

    /// A `TJ` written in reading order under a mirroring `Tm`: each adjustment of 1000 moves the
    /// pen back by 20 in text space, twice the 11.12 the glyph advanced, so there is no gap to
    /// read — and along user-space x the mirror made each one a step of 8.88 rightwards, over the
    /// 2.78 threshold.
    #[test]
    fn a_tj_under_a_mirroring_tm_reads_back_with_no_gaps() {
        let mirrored =
            readback("BT /F1 20 Tf -1 0 0 1 200 0 Tm 90 100 Td [(b) 1000 (a) 1000 (b)] TJ ET");
        assert_eq!(mirrored, "bab");
        // And without the mirror, the same string is the same word.
        let upright = readback("BT /F1 20 Tf 90 100 Td [(b) 1000 (a) 1000 (b)] TJ ET");
        assert_eq!(upright, "bab");
    }

    /// The sign is the advance's, not the page's: a gap that opens in text space is a word gap
    /// whichever way the mirror turns it. `-300` adds 6 to the pen, over 2.78.
    #[test]
    fn a_gap_opened_under_a_mirroring_tm_is_still_a_word_gap() {
        assert_eq!(
            readback("BT /F1 20 Tf -1 0 0 1 200 0 Tm 50 100 Td [(a) -300 (b)] TJ ET"),
            "a b"
        );
        assert_eq!(
            readback("BT /F1 20 Tf 50 100 Td [(a) -300 (b)] TJ ET"),
            "a b"
        );
        // A negative horizontal scaling mirrors the line as `Tm` does, and is undone with it.
        assert_eq!(
            readback("BT /F1 20 Tf -100 Tz 150 100 Td [(a) -300 (b) 1000 (a)] TJ ET"),
            "a ba"
        );
    }

    /// A `Tm` that scales: `1 Tf` under `20 0 0 20 Tm` is the same line as `20 Tf` under the
    /// identity, and its gaps are measured in the units its threshold is. `-100` is 0.1 of a text
    /// unit, under 0.139, so it is tracking; `-300` is 0.3, over it, so it is a word gap.
    #[test]
    fn a_scaling_tm_measures_its_gap_in_text_space() {
        assert_eq!(
            readback("BT /F1 1 Tf 20 0 0 20 50 100 Tm [(a) -100 (b) -300 (a)] TJ ET"),
            "ab a"
        );
        assert_eq!(
            readback("BT /F1 20 Tf 50 100 Td [(a) -100 (b) -300 (a)] TJ ET"),
            "ab a"
        );
    }

    /// The step undone: through the linear part and out of `Th`, and read as it stands where
    /// either collapses text space.
    #[test]
    fn a_step_is_taken_back_into_text_space() {
        let mirror = Transform::new(-1.0, 0.0, 0.0, 1.0, 200.0, 0.0);
        assert_eq!(
            Interpreter::text_space_step(mirror, 1.0, (8.0, 2.0)),
            (-8.0, 2.0)
        );
        let scaled = Transform::new(20.0, 0.0, 0.0, 20.0, 50.0, 100.0);
        assert_eq!(
            Interpreter::text_space_step(scaled, 0.5, (4.0, 10.0)),
            (0.4, 0.5)
        );
        let collapsed = Transform::new(0.0, 0.0, 0.0, 0.0, 50.0, 100.0);
        assert_eq!(
            Interpreter::text_space_step(collapsed, 1.0, (4.0, 10.0)),
            (4.0, 10.0)
        );
    }

    /// A font stating no space is given a quarter em for one, and the same half of it: a `TJ`
    /// adjustment of `-250` — a quarter em, the word gap a producer writes for such a subset —
    /// is a word gap, where a threshold of the whole quarter em read it as none.
    #[test]
    fn a_font_with_no_space_still_has_a_word_gap() {
        let boxes = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
                     /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 97 /LastChar 98 \
                     /Widths [500 500] /Encoding << /Differences [97 /a /b] >> \
                     /CharProcs << /a 6 0 R /b 6 0 R >> /Resources << >> >>\nendobj\n\
                     6 0 obj\n<< /Length 25 >>\nstream\n500 0 d0 0 0 450 700 re f\nendstream";
        assert_eq!(
            readback_in(boxes, "BT /F1 20 Tf 50 100 Td [(a) -250 (b) -50 (a)] TJ ET"),
            "a ba"
        );
    }

    /// `issue1453.pdf`'s title in miniature: a display face stating no space whose producer set
    /// its word gaps at `-169` and `-140` and kerned one pair at `-31`. At `20 Tf` the threshold
    /// is half a quarter em, 2.5 units; `-140` moves the pen 2.8 and is a word gap, `-31` moves it
    /// 0.62 and is not (ADR 1502).
    #[test]
    fn a_display_face_with_no_space_breaks_at_an_eighth_of_an_em() {
        let boxes = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
                     /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 97 /LastChar 98 \
                     /Widths [500 500] /Encoding << /Differences [97 /a /b] >> \
                     /CharProcs << /a 6 0 R /b 6 0 R >> /Resources << >> >>\nendobj\n\
                     6 0 obj\n<< /Length 25 >>\nstream\n500 0 d0 0 0 450 700 re f\nendstream";
        assert_eq!(
            readback_in(
                boxes,
                "BT /F1 20 Tf 50 100 Td [(a) -169 (b) -140 (a) -31 (b)] TJ ET"
            ),
            "a b ab"
        );
    }

    /// A font whose producer set its word gaps nearer its kerning than its own space: Helvetica,
    /// kerned at 0.01 em (0.036 of its space) and its words set 0.09 em apart (0.32 of it), over
    /// forty steps. Under the constant half a space every gap is kerning and the line reads as one
    /// word; the font's own steps put the empty interval between 0.036 and 0.32, and a threshold
    /// inside it reads each pair of strings as a word (ADR 1515).
    #[test]
    fn a_font_whose_word_gaps_sit_below_half_its_space_breaks_at_its_own_gap() {
        assert_eq!(readback(&words_set_apart(-90, 21)), ["abab"; 21].join(" "));
    }

    /// The same steps with every string a single code are a letter-spaced line rather than words,
    /// and the font's own gap does not decide: the constant reads it as one word (ADR 1515).
    #[test]
    fn letter_spaced_single_codes_keep_the_constant() {
        let mut array = String::new();
        for _ in 0..21 {
            array.push_str("(a) -10 (b) -90 ");
        }
        let content = format!("BT /F1 20 Tf 10 100 Td [{array}] TJ ET");
        assert_eq!(readback(&content), "ab".repeat(21));
    }

    /// A provisional space the constant takes back leaves every range over the readback where its
    /// characters are: 0.1 em in Helvetica is 0.36 of its space, so `[(ab) -100 (cd)]` reads "abcd",
    /// each code's span is its own character, and the `/MCID` sequence around it covers all four.
    #[test]
    fn a_space_taken_back_moves_the_ranges_over_the_text() {
        let read = interpretation_in(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            "/P << /MCID 0 >> BDC BT /F1 20 Tf 10 100 Td [(ab) -100 (cd)] TJ ET EMC",
        );
        assert_eq!(read.text, "abcd");
        let spans: Vec<_> = read
            .text_layer
            .iter()
            .map(|placed| placed.span.clone())
            .collect();
        assert_eq!(spans, [0..1, 1..2, 2..3, 3..4]);
        assert_eq!(read.inferred_separators, 0);
        let marked: Vec<(usize, usize)> = read
            .marked
            .iter()
            .map(|span| (span.range.start, span.range.end))
            .collect();
        assert_eq!(marked, [(0, 4)]);
    }

    /// §14.9.4's replacement stands for every code it encloses: each of the four codes shown
    /// inside the sequence reads back as "xy", whatever space the cut readback held between them.
    #[test]
    fn a_code_inside_a_replacement_reads_back_as_all_of_it() {
        let read = interpretation_in(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            "/Span << /ActualText (xy) >> BDC BT /F1 20 Tf 10 100 Td [(ab) -300 (cd)] TJ ET EMC",
        );
        assert_eq!(read.text, "xy");
        let spans: Vec<_> = read
            .text_layer
            .iter()
            .map(|placed| placed.span.clone())
            .collect();
        assert_eq!(spans, [0..2, 0..2, 0..2, 0..2]);
    }

    /// A gap of 0.15 em in Helvetica is 0.54 of its 278-thousandth space: nearer a space than no
    /// gap, so a word break; 0.12 em is 0.43 of one, and is not.
    #[test]
    fn a_gap_nearer_a_space_than_none_is_a_word_break() {
        assert_eq!(
            readback("BT /F1 20 Tf 50 100 Td [(a) -150 (b) -120 (a)] TJ ET"),
            "a ba"
        );
    }
}
