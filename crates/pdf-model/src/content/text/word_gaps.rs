//! The steps along a line a page showed, kept per font until the walk settles which are word gaps
//! (ADR 1515).

use std::sync::Arc;

use crate::content::font::Font;

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
pub(super) const WORD_GAP_SHARE: f32 = 0.5;

/// The space, in ems, read for a font that states none at code 32.
///
/// **A choice**, stood in where §9.3.3's single-byte code 32 has no width: a subset carrying no
/// space is common, and its producer's word gaps are then `TJ` adjustments. A quarter em sits
/// between the lower quartile and the median of the spaces the corpus's fonts do state (0.226 and
/// 0.278 em), and under [`WORD_GAP_SHARE`] it makes the threshold an eighth of an em, inside the
/// trough between kerning and word gaps that such fonts' own steps show (ADR 1502).
pub(super) const NOMINAL_SPACE_EM: f32 = 0.25;

/// The characters a line may end on to divide a word: U+002D, §14.8.2.3's soft hyphen U+00AD,
/// and Unicode's own U+2010 HYPHEN.
pub(super) const HYPHENS: [char; 3] = ['-', '\u{00ad}', '\u{2010}'];

/// The least share of a space a font's own threshold may take, and so the shortest gap
/// [`Interpreter::separate_text`](crate::content::Interpreter::separate_text) keeps a provisional space for.
///
/// **A choice** (ADR 1515): the floor of the trough ADR 1502 measured between the kerning and the
/// word-gap clusters — 0.3 of a stated space, 0.065 em of a font stating none, which is 0.26 of the
/// quarter em [`NOMINAL_SPACE_EM`] reads it with. No font's threshold goes below it, however its
/// own steps fall.
pub(super) const LEAST_WORD_GAP_SHARE: f32 = 0.25;

/// How many steps along a line a font has to show on a page before its own steps decide its
/// threshold; below it, [`WORD_GAP_SHARE`] does (ADR 1515).
pub(super) const STEPS_TO_DECIDE: usize = 40;

/// How wide, as the ratio of its two edges, the empty interval below a font's word gaps has to be
/// before it is read as the font's own threshold (ADR 1515): twice, so that a page's kerning and
/// its word gaps are a factor apart rather than neighbours.
pub(super) const GAP_TO_DECIDE: f32 = 2.0;

/// The steps along a line one page showed, per font, kept until the page is finished.
///
/// §9.4.4 places glyphs and states no quantity that separates words, so where a word ends is this
/// program's choice (ADR 1502). ADR 1515 makes it per font: a font's steps between show operations
/// fall into a kerning cluster and a word-gap cluster, and where the empty interval between the
/// two lies differs from font to font — so the threshold is read off the page's own steps once
/// the walk has seen them all. The walk keeps a provisional space for every step a threshold
/// could call a word gap, and [`Interpreter::settle_word_gaps`](crate::content::Interpreter::settle_word_gaps) takes back the ones the font's own
/// threshold does not.
#[derive(Debug, Clone, Default)]
pub(in crate::content) struct WordGaps {
    /// The fonts the steps were shown in, each once, held so that its identity outlives the walk.
    fonts: Vec<Font>,
    /// Every step along a line, in the order the walk measured them.
    pub(super) steps: Vec<Step>,
    /// The byte of the readback the last inferred separator — a space or a line break — was
    /// written at, for [`Interpreter::cut_replaced`](crate::content::Interpreter::cut_replaced) to tell it from what a section read back.
    pub(super) last_separator: Option<usize>,
    /// Whether the next line break [`Interpreter::separate_text`](crate::content::Interpreter::separate_text) would infer is withheld, because
    /// the replacement just written removed the hyphen that ended its line (ADR 1948).
    pub(super) joins_next_line: bool,
}

/// One step [`Interpreter::separate_text`](crate::content::Interpreter::separate_text) measured along a line, in text space.
#[derive(Debug, Clone, Copy)]
pub(super) struct Step {
    /// Which of [`WordGaps::fonts`] the show operation after the step was in.
    pub(super) font: usize,
    /// How far the pen moved forward between the two show operations.
    pub(super) along: f32,
    /// The space the gap is measured against: the font's own, or [`NOMINAL_SPACE_EM`].
    pub(super) space: f32,
    /// How many codes the show operation after the step holds.
    pub(super) run: usize,
    /// The provisional space the walk put in for this step, if it put one in.
    pub(super) provisional: Provisional,
}

/// Whether a step left a space in the readback for [`Interpreter::settle_word_gaps`](crate::content::Interpreter::settle_word_gaps) to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Provisional {
    /// Too short for any threshold: no space.
    None,
    /// A space at this byte of the readback.
    At(usize),
    /// A space that was put in and then discarded with the text around it — an `/ActualText`
    /// replaced it (§14.9.4) — still counted in [`Interpreter::inferred_separators`](crate::content::Interpreter::inferred_separators).
    Discarded,
}

impl WordGaps {
    /// The index of `font` in [`Self::fonts`], adding it the first time it is seen.
    pub(super) fn font_index(&mut self, font: &Font) -> usize {
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
    pub(in crate::content) fn discard_from(&mut self, len: usize) {
        if self.last_separator.is_some_and(|at| at >= len) {
            self.last_separator = None;
        }
        for step in self.steps.iter_mut().rev() {
            match step.provisional {
                Provisional::At(at) if at >= len => step.provisional = Provisional::Discarded,
                Provisional::At(_) => break,
                Provisional::None | Provisional::Discarded => {}
            }
        }
    }

    /// How many steps are held, for a rewind to come back to.
    pub(in crate::content) fn len(&self) -> usize {
        self.steps.len()
    }

    /// Takes back every step measured after the first `len`.
    ///
    /// The last separator and a withheld line break are forgotten with them: a rewind cuts the
    /// readback back to a mark this does not know, and a separator remembered past it would name
    /// a byte something else is written to next.
    pub(in crate::content) fn truncate(&mut self, len: usize) {
        self.steps.truncate(len);
        self.last_separator = None;
        self.joins_next_line = false;
    }

    /// Each font's threshold, as a share of the space its steps were measured against.
    pub(super) fn thresholds(&self) -> Vec<f32> {
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
