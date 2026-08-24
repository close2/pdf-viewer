//! The twentieth sweep: a page-list note quoting a figure the gate contradicts.
//!
//! # The shape it exists for
//!
//! [`crate::overtaken`] asks whether a note has read the decisions taken since it was written.
//! This one asks the narrower question that sweep could not: **does the note's arithmetic still
//! agree with the gate's?** A note is the prose a round reads before deciding whether a page is
//! our defect, and the numbers in it are most of the case it makes — so a figure that moved when
//! the tree moved is a note arguing from a measurement nobody can reproduce.
//!
//! Two rounds found one of these by hand, one apiece, and both said the same thing afterwards:
//! nothing links a note to the gate figures it quotes. Trap 1 states the by-hand tell exactly —
//! *a group note quoting a number the gate also prints, where the two disagree* — and this is
//! that tell, mechanised.
//!
//! # The population, which is larger than the count that preceded it said
//!
//! That count looked for two tokens, `ssim` and `worst tile`, and its answer was read as *there
//! is little to anchor a figure to*. The gate prints **four** measures and this tree writes each
//! of them several ways: `ssim`, `similarity` and `structural similarity` are one measure, and
//! the differing fraction is quoted as a percentage in either word order. Counted over the whole
//! vocabulary the population is about a quarter of the tree's page-list notes and over a hundred
//! figures — and it is the quarter a round actually opens, the contradicted groups. That is what
//! made a program worth writing rather than a line in a reading list.
//!
//! # The discriminator
//!
//! > A figure quoted in the gate's own vocabulary, for a page the note itself holds, that **no
//! > page of that note carries** in the gate's current output.
//!
//! Both sides are this project's own output. The left is the note; the right is the gate's
//! per-page line, which prints all four measures for every page it does not call agreement —
//! and a page list is ratcheted to exactly the pages that are not agreement, so **every page a
//! note holds appears in that output by construction**. Nothing is rasterised here and nothing
//! is re-measured: the sweep reads a report the round has already run.
//!
//! # Precision is the gate's, and it is what tells one instrument from another
//!
//! The gate prints mean, worst tile and the differing percentage to **two** decimals and the
//! structural similarity to **four**. A comparison is therefore made at the coarser of the two
//! precisions — the note's and the gate's — by formatting both and comparing the text, so no
//! float is ever tested for equality. A figure written *finer* than the gate prints came off
//! some other instrument in this tree (a resolution ladder, a swatch grid, a mean per column),
//! and it says so by its own digits. It is still compared, one rung down, because the archetype
//! this sweep was built from — a note giving ours as `ssim 0.98591` where the gate prints
//! `0.9879` — was written at five.
//!
//! # The rungs — read the first one first
//!
//! 1. **[`Rung::Line`] — the figure is contradicted and a figure beside it is confirmed.** The
//!    note and the gate are demonstrably about the same page's line: the gate agrees about one
//!    measure and disagrees about this one. There is no reading left to do.
//! 2. **[`Rung::Gate`] — contradicted, written to exactly the precision the gate prints, and
//!    nothing beside it confirmed.** The shape of the number says it came off a gate line; which
//!    line is for the reader to say.
//! 3. **[`Rung::Finer`] — contradicted, and written finer than the gate prints.** Most are
//!    another instrument's number and are noise; the archetype was one of them.
//!
//! # The noise, classified rather than filtered
//!
//! - **A note narrates its own corrections**, and the superseded figure stays in the sentence
//!   that supersedes it: *"[u]ntil the four-hundred-and-sixth session this page's line read mean
//!   27.02 …"*. That figure is contradicted by construction and the prose is correct. It is the
//!   largest single source of hits on rungs 2 and 3.
//! - **Another instrument's table.** A ladder at eight times the resolution, a mean per column,
//!   a mean over eighty swatches: the words are the gate's and the measurement is not. Rung 3
//!   collects most of them, by their digits.
//! - **A range** — *"mean at most 1.64"*, *"5.4 to 6.4"* — is one endpoint to this sweep and a
//!   span to a reader. It reads the first number and cannot know that.
//! - **A bound is quoted as often as a distance**, and both are read: the gate prints both, so
//!   `mean 8.45 of 5.00` is two confirmable figures rather than one.
//!
//! # Why it is not a build failure
//!
//! Three of the four entries above are correct prose, and every one of them is a sentence a
//! person has to read. Same argument as the nineteenth sweep's and the same shape: it ranks, it
//! prints what the gate says instead, and it exits zero on a finding.

use std::collections::{BTreeMap, BTreeSet};

use crate::overtaken::Note;

/// The file whose page-list notes the oracle's report is about.
///
/// The other gates in this tree keep notes in the same shape and print their figures in their
/// own words, so a report from one of them is not a right-hand side for a note in another's
/// file. Scoping the population to the file that matches the report is what keeps this sweep
/// from inventing a disagreement out of two gates' vocabularies.
pub const GATE_NOTES: &str = "crates/pdf-model/tests/oracle.rs";

/// One of the four figures the oracle prints for every page it does not call agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Measure {
    /// Mean absolute difference, in levels of 255.
    Mean,
    /// The worst tile's mean absolute difference.
    WorstTile,
    /// The share of channels differing by more than the gate's step, as a percentage.
    Differing,
    /// Structural similarity.
    Ssim,
}

impl Measure {
    /// Every measure, in the order the gate prints them.
    pub const ALL: [Self; 4] = [Self::Mean, Self::WorstTile, Self::Differing, Self::Ssim];

    /// How this report names the measure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mean => "mean",
            Self::WorstTile => "worst tile",
            Self::Differing => "differing",
            Self::Ssim => "ssim",
        }
    }

    /// How many decimals the gate prints this measure to.
    ///
    /// `{:.2}` for the three that are levels of 255 or a percentage and `{:.4}` for the
    /// similarity — the format strings the gate's own line is built from.
    #[must_use]
    pub const fn decimals(self) -> usize {
        match self {
            Self::Mean | Self::WorstTile | Self::Differing => 2,
            Self::Ssim => 4,
        }
    }

    /// The words a note writes this measure with, longest first.
    ///
    /// Longest first because `structural similarity` contains `similarity`; both find the same
    /// number, and a quotation is keyed by where its *number* sits, so the overlap costs a
    /// duplicate key rather than a second finding.
    ///
    /// Public because [`crate::unpriced`] asks the mirror question of the same vocabulary —
    /// whether a note names a measure at all rather than whether the figure beside it is
    /// current — and two sweeps disagreeing about how this tree spells a measure would be two
    /// answers to one question.
    #[must_use]
    pub const fn words(self) -> &'static [&'static str] {
        match self {
            Self::Mean => &["mean"],
            Self::WorstTile => &["worst tile"],
            Self::Differing => &["differing"],
            Self::Ssim => &["structural similarity", "similarity", "ssim"],
        }
    }
}

/// Which half of the gate's line a figure came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    /// `ours at worst …` — this tree's distance from the reference the verdict rests on.
    Ours,
    /// `bound …` — what that page's own reference consensus allows.
    Bound,
}

impl Side {
    /// How this report names the half.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ours => "ours",
            Self::Bound => "bound",
        }
    }
}

/// One figure the gate printed, for one page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Printed {
    /// Which measure it is.
    pub measure: Measure,
    /// Which half of the line it came from.
    pub side: Side,
    /// The value, as the report wrote it.
    pub value: f64,
}

impl Printed {
    /// The value written to `decimals` places, which is how it is compared and printed.
    #[must_use]
    pub fn shown(&self, decimals: usize) -> String {
        format!("{:.decimals$}", self.value)
    }
}

/// One figure a note quotes.
#[derive(Debug, Clone, PartialEq)]
pub struct Quotation {
    /// Which measure it is.
    pub measure: Measure,
    /// The value, parsed from the note's own digits.
    pub value: f64,
    /// How many decimals the note wrote it to.
    pub decimals: usize,
    /// The 1-based line of the source file it sits on.
    pub line: usize,
    /// The words around it, as the finding prints them back.
    pub text: String,
}

impl Quotation {
    /// The precision this quotation is compared at: the coarser of its own and the gate's.
    #[must_use]
    pub fn compared_at(&self) -> usize {
        self.decimals.min(self.measure.decimals())
    }

    /// The value as it is compared, which is also how the finding prints it.
    #[must_use]
    pub fn shown(&self) -> String {
        let decimals = self.compared_at();
        format!("{:.decimals$}", self.value)
    }

    /// Whether one of the gate's figures says the same thing.
    #[must_use]
    pub fn confirmed_by(&self, printed: &[Printed]) -> bool {
        let decimals = self.compared_at();
        let shown = self.shown();
        printed
            .iter()
            .any(|figure| figure.measure == self.measure && figure.shown(decimals) == shown)
    }
}

/// How sharply the gate contradicts a quoted figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// Contradicted, with a figure beside it on the same line of the note that the gate confirms.
    Line,
    /// Contradicted, and written to exactly the precision the gate prints.
    Gate,
    /// Contradicted, and written finer than the gate prints — another instrument's number.
    Finer,
}

impl Rung {
    /// One line saying what this rung means, printed above every hit on it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Line => "the gate contradicts this figure and confirms one beside it",
            Self::Gate => "the gate contradicts this figure, written as the gate writes it",
            Self::Finer => "the gate contradicts this figure, written finer than the gate prints",
        }
    }
}

/// One quoted figure the gate contradicts, with what the gate says instead.
#[derive(Debug, Clone)]
pub struct Contradicted {
    /// The quotation.
    pub quotation: Quotation,
    /// How sharply.
    pub rung: Rung,
    /// What the gate prints for this measure over the note's own pages, nearest value first.
    ///
    /// Nearest first because a figure that drifted is still the same page's, so the nearest
    /// value is usually the correction — which is the whole point of printing it: a note is
    /// corrected off the gate's own output rather than by reasoning about it.
    pub instead: Vec<(String, Printed)>,
}

/// One note, with every figure in it the gate contradicts.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The note.
    pub note: Note,
    /// The contradicted figures, closest rung first and earliest line first within a rung.
    pub contradicted: Vec<Contradicted>,
}

impl Finding {
    /// The closest rung any of the contradicted figures sits on.
    ///
    /// [`Rung::Finer`] where there are none, which [`sweep`] never constructs: a finding with no
    /// contradicted figure is not a finding.
    #[must_use]
    pub fn rung(&self) -> Rung {
        self.contradicted
            .iter()
            .map(|entry| entry.rung)
            .min()
            .unwrap_or(Rung::Finer)
    }
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The hits, closest rung first.
    pub findings: Vec<Finding>,
    /// How many page-list notes were read.
    pub notes: usize,
    /// How many of those quote at least one figure in the gate's vocabulary.
    pub quoting: usize,
    /// How many figures were read in all.
    pub quotations: usize,
    /// How many of those the gate confirms.
    pub confirmed: usize,
    /// How many sit in a note none of whose pages the report prints, so nothing can judge them.
    pub unanchored: usize,
    /// How many pages the report printed figures for.
    pub pages: usize,
}

impl Report {
    /// How many contradicted figures sit on one rung.
    #[must_use]
    pub fn on(&self, rung: Rung) -> usize {
        self.findings
            .iter()
            .flat_map(|finding| finding.contradicted.iter())
            .filter(|entry| entry.rung == rung)
            .count()
    }

    /// How many figures the gate contradicts, over every rung.
    #[must_use]
    pub fn contradicted(&self) -> usize {
        self.findings
            .iter()
            .map(|finding| finding.contradicted.len())
            .sum()
    }
}

/// How many lines either side of a quotation count as beside it.
///
/// A doc comment wraps at the column limit, so one sentence quoting a gate line lands on one
/// line of source or spills onto the next. Wider than this and every figure in a note would sit
/// beside a confirmed one, which is the whole distinction rung 1 rests on.
const NEIGHBOURS: usize = 1;

/// Runs the sweep.
///
/// `printed` is the gate's own output, keyed by the page name it prints; [`report`] parses it.
#[must_use]
pub fn sweep(notes: &[Note], printed: &BTreeMap<String, Vec<Printed>>) -> Report {
    let mut report = Report {
        notes: notes.len(),
        pages: printed.len(),
        ..Report::default()
    };

    for note in notes {
        let quotations = quotations_in(note);
        if quotations.is_empty() {
            continue;
        }
        report.quoting = report.quoting.saturating_add(1);
        report.quotations = report.quotations.saturating_add(quotations.len());

        let figures = figures_of(note, printed);
        if figures.is_empty() {
            report.unanchored = report.unanchored.saturating_add(quotations.len());
            continue;
        }
        let flat: Vec<Printed> = figures.values().flatten().copied().collect();

        let mut confirmed_lines = BTreeSet::new();
        let mut open = Vec::new();
        for quotation in quotations {
            if quotation.confirmed_by(&flat) {
                report.confirmed = report.confirmed.saturating_add(1);
                confirmed_lines.insert(quotation.line);
            } else {
                open.push(quotation);
            }
        }

        let contradicted: Vec<Contradicted> = open
            .into_iter()
            .map(|quotation| {
                let beside = confirmed_lines
                    .iter()
                    .any(|line| line.abs_diff(quotation.line) <= NEIGHBOURS);
                let rung = if beside {
                    Rung::Line
                } else if quotation.decimals == quotation.measure.decimals() {
                    Rung::Gate
                } else {
                    Rung::Finer
                };
                Contradicted {
                    instead: nearest(&quotation, &figures),
                    quotation,
                    rung,
                }
            })
            .collect();
        if contradicted.is_empty() {
            continue;
        }
        let mut contradicted = contradicted;
        contradicted.sort_by_key(|entry| (entry.rung, entry.quotation.line));
        report.findings.push(Finding {
            note: note.clone(),
            contradicted,
        });
    }

    report
        .findings
        .sort_by_key(|finding| (finding.rung(), finding.note.line));
    report
}

/// The gate's figures for every page this note is about.
///
/// **The list is not the whole of what a note is about, and the plant that proved this sweep
/// found that out.** A page can be moved to another group while the note that diagnosed it keeps
/// the diagnosis — `CONTRADICTED_UNEXPLAINED`'s list is empty and its note still carries four
/// paragraphs and six figures about the page it lost — so a sweep anchored to list membership
/// alone is blind to exactly the notes whose figures nothing else is pointing at either. So the
/// anchor is the list's pages **and** every page of a document the note's prose argues, which is
/// the same widening [`crate::overtaken`] makes for the same reason.
fn figures_of(
    note: &Note,
    printed: &BTreeMap<String, Vec<Printed>>,
) -> BTreeMap<String, Vec<Printed>> {
    let mut found: BTreeMap<String, Vec<Printed>> = note
        .pages
        .iter()
        .filter_map(|page| {
            printed
                .get(page)
                .map(|figures| (page.clone(), figures.clone()))
        })
        .collect();
    for document in &note.prose {
        let prefix = format!("{document} page ");
        for (page, figures) in printed.range(prefix.clone()..) {
            if !page.starts_with(&prefix) {
                break;
            }
            found.entry(page.clone()).or_insert_with(|| figures.clone());
        }
    }
    found
}

/// What the gate prints for this measure over the note's pages, nearest the quoted value first.
fn nearest(
    quotation: &Quotation,
    figures: &BTreeMap<String, Vec<Printed>>,
) -> Vec<(String, Printed)> {
    let mut candidates: Vec<(String, Printed)> = figures
        .iter()
        .flat_map(|(page, printed)| printed.iter().map(move |figure| (page.clone(), *figure)))
        .filter(|(_, figure)| figure.measure == quotation.measure)
        .collect();
    candidates.sort_by(|(left_page, left), (right_page, right)| {
        let left_distance = (left.value - quotation.value).abs();
        let right_distance = (right.value - quotation.value).abs();
        left_distance
            .partial_cmp(&right_distance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left_page.cmp(right_page))
    });
    candidates
}

/// Every figure in the gate's vocabulary that one note quotes.
///
/// Keyed by where the *number* sits rather than by which word found it, so the three spellings
/// of the similarity contribute one quotation apiece rather than two.
#[must_use]
pub fn quotations_in(note: &Note) -> Vec<Quotation> {
    let mut found: BTreeMap<(usize, usize), Quotation> = BTreeMap::new();
    for (line, text) in &note.body {
        for (offset, quotation) in figures_quoted(text, *line) {
            found.insert((*line, offset), quotation);
        }
    }
    found.into_values().collect()
}

/// Every figure one line of a note quotes, with the byte the number starts at.
fn figures_quoted(text: &str, line: usize) -> Vec<(usize, Quotation)> {
    let mut found = Vec::new();
    for measure in Measure::ALL {
        for word in measure.words() {
            let mut from = 0usize;
            while let Some(at) = text.get(from..).and_then(|rest| rest.find(word)) {
                let start = from.saturating_add(at);
                let after = start.saturating_add(word.len());
                if is_word(text, start, after)
                    && let Some((offset, digits)) = figure_of(text, start, after, measure)
                    && let Ok(value) = digits.parse::<f64>()
                {
                    found.push((
                        offset,
                        Quotation {
                            measure,
                            value,
                            decimals: decimals_of(&digits),
                            line,
                            text: fragment(text, start.min(offset), offset, digits.len()),
                        },
                    ));
                }
                from = after;
            }
        }
    }
    found
}

/// Whether the word found at `[start, end)` stands on its own.
///
/// `mean` is inside `meaning` and `means`, and both are ordinary English in these notes. A
/// letter on either side is what tells the measure from the verb.
fn is_word(text: &str, start: usize, end: usize) -> bool {
    let before = text.get(..start).and_then(|head| head.chars().next_back());
    let after = text.get(end..).and_then(|tail| tail.chars().next());
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

/// The figure belonging to the word at `[start, after)`.
///
/// Forwards first, because that is the order the gate writes and the order most of these notes
/// write. The differing fraction is the one this tree also writes the other way round — *"0.54%
/// of pixels differing"* — so it, and only it, is also looked for behind the word: the per-cent
/// sign makes a backward search unambiguous where a bare decimal would not be.
fn figure_of(text: &str, start: usize, after: usize, measure: Measure) -> Option<(usize, String)> {
    figure_after(text, after, measure).or_else(|| {
        (measure == Measure::Differing)
            .then(|| percentage_before(text, start))
            .flatten()
    })
}

/// The first decimal number within reach of `from`, and where it starts.
///
/// The scan stops at a full stop followed by a space — a figure in the next sentence is not this
/// word's — and at another measure's word, so `the mean and the worst tile 14.16` gives the tile
/// its number and the mean none.
fn figure_after(text: &str, from: usize, measure: Measure) -> Option<(usize, String)> {
    let window = text.get(from..)?;
    let mut limit = window.find(". ").unwrap_or(window.len());
    for other in Measure::ALL {
        if other == measure {
            continue;
        }
        for word in other.words() {
            if let Some(at) = window.get(..limit).and_then(|head| head.find(word)) {
                limit = at;
            }
        }
    }
    let window = window.get(..limit)?;

    for (offset, token) in words(window).take(STRIDE.saturating_add(1)) {
        let Some((at, digits)) = decimal_in(token) else {
            continue;
        };
        let start = offset.saturating_add(at);
        // The differing fraction is a percentage wherever it is quoted, and the sign is what
        // tells it from a mean that happens to follow the word.
        if measure == Measure::Differing && !percentage(window, start.saturating_add(digits.len()))
        {
            continue;
        }
        return Some((from.saturating_add(start), digits));
    }
    None
}

/// The nearest `N.N%` before `at`, and where it starts.
fn percentage_before(text: &str, at: usize) -> Option<(usize, String)> {
    let window = text.get(..at)?;
    let behind: Vec<(usize, &str)> = words(window).collect();
    for (offset, token) in behind.into_iter().rev().take(STRIDE.saturating_add(1)) {
        let Some((relative, digits)) = decimal_in(token) else {
            continue;
        };
        let start = offset.saturating_add(relative);
        if percentage(window, start.saturating_add(digits.len())) {
            return Some((start, digits));
        }
    }
    None
}

/// The whitespace-separated words of a string, each with the byte it starts at.
fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_whitespace()
        .map(move |word| (offset_of(text, word), word))
}

/// Where a word taken out of `text` by [`words`] sits in it.
///
/// The subslice is a view into `text`, so its address is its offset. That is what
/// `split_whitespace` promises and there is no other way to ask it.
fn offset_of(text: &str, word: &str) -> usize {
    (word.as_ptr() as usize).saturating_sub(text.as_ptr() as usize)
}

/// The first `N.N` in a string, and where it starts.
///
/// An integer is not one of these measures: the gate prints every one of them with a decimal
/// point, and a bare `4` in a note is a count, a page or a session.
fn decimal_in(window: &str) -> Option<(usize, String)> {
    let bytes = window.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if !bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index = index.saturating_add(1);
            continue;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index = index.saturating_add(1);
        }
        if bytes.get(index) == Some(&b'.')
            && bytes
                .get(index.saturating_add(1))
                .is_some_and(u8::is_ascii_digit)
        {
            index = index.saturating_add(1);
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index = index.saturating_add(1);
            }
            return window.get(start..index).map(|d| (start, d.to_owned()));
        }
    }
    None
}

/// How many words may stand between a measure's name and its figure.
///
/// **Words rather than characters, and this is the one place the sweep's precision was bought
/// with a measurement.** A reach in characters cannot separate `mean absolute difference — 5.4`,
/// which is a figure, from `it does not mean item 4 is paid by 0.75`, which is the verb: both
/// put a decimal about twenty characters past the word. Counted in words the first is three and
/// the second five, and every quotation of a gate line in this tree is nought to three —
/// `mean 1.38`, `mean of 1.11`, `structural similarity at worst 0.9906`,
/// `0.54% of pixels differing`.
///
/// What three costs is stated rather than hidden: a figure further from its word than that is
/// not read at all, and `worst tile the only figure that moves, to 13.86` — seven words — is the
/// widest instance this sweep therefore cannot see.
const STRIDE: usize = 3;

/// Whether a per-cent sign follows the digits, allowing the space this tree sometimes writes.
fn percentage(window: &str, after: usize) -> bool {
    window
        .get(after..)
        .is_some_and(|tail| tail.trim_start().starts_with('%'))
}

/// The nearest character boundary at or below `index`, so a note's em-dashes cannot cut a slice.
fn boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index = index.saturating_sub(1);
    }
    index
}

/// How many digits a number has after its point.
fn decimals_of(digits: &str) -> usize {
    digits.split_once('.').map_or(0, |(_, tail)| tail.len())
}

/// The words around a quoted figure, for the finding to print back.
fn fragment(text: &str, from: usize, figure_at: usize, length: usize) -> String {
    let end = boundary(text, figure_at.saturating_add(length));
    text.get(from..end).unwrap_or_default().trim().to_owned()
}

/// Reads the gate's per-page figures out of the run's own output.
///
/// The line is the one the oracle prints for every page it does not call agreement:
///
/// ```text
///   issue7891_bc1.pdf page 1: contradicted — ours at worst mean 0.16 worst tile 6.73 differing
///   0.54% ssim 0.9995; bound mean 5.00 worst tile 6.04 differing 5.00% ssim 0.9000
/// ```
///
/// Everything before the first `: ` is the page's name. A page the gate could not measure prints
/// `nothing measured` in place of the first half and contributes its bound alone.
#[must_use]
pub fn report(text: &str) -> BTreeMap<String, Vec<Printed>> {
    let mut found: BTreeMap<String, Vec<Printed>> = BTreeMap::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(": ") else {
            continue;
        };
        let name = name.trim();
        if !name.contains(".pdf page ") {
            continue;
        }
        let mut figures = Vec::new();
        if let Some((_, ours)) = rest.split_once(OURS) {
            let head = ours.split_once("; ").map_or(ours, |(head, _)| head);
            figures.extend(figures_in(head, Side::Ours));
        }
        if let Some((_, bound)) = rest.rsplit_once(BOUND) {
            figures.extend(figures_in(bound, Side::Bound));
        }
        if !figures.is_empty() {
            found.entry(name.to_owned()).or_default().extend(figures);
        }
    }
    found
}

/// What the gate writes before this tree's own four figures.
const OURS: &str = "ours at worst ";
/// What it writes before the four the page's own consensus allows.
const BOUND: &str = "bound ";

/// The four figures in one half of a gate line.
fn figures_in(text: &str, side: Side) -> Vec<Printed> {
    let mut found = Vec::new();
    for measure in Measure::ALL {
        for word in measure.words() {
            let Some(at) = text.find(word) else { continue };
            let after = at.saturating_add(word.len());
            if let Some((_, digits)) = figure_after(text, after, measure)
                && let Ok(value) = digits.parse::<f64>()
            {
                found.push(Printed {
                    measure,
                    side,
                    value,
                });
                break;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "  issue7891_bc1.pdf page 1: contradicted — ours at worst mean 0.17 \
                        worst tile 6.73 differing 0.54% ssim 0.9995; bound mean 5.00 worst \
                        tile 6.04 differing 5.00% ssim 0.9000";

    const PAGE: &str = "issue7891_bc1.pdf page 1";

    fn note(body: &[&str], pages: &[&str]) -> Note {
        Note {
            name: "A_LIST_OF_PAGES".to_owned(),
            file: GATE_NOTES.to_owned(),
            line: 1,
            cited: BTreeSet::new(),
            prose: BTreeSet::new(),
            members: BTreeSet::new(),
            body: body
                .iter()
                .enumerate()
                .map(|(offset, text)| (offset.saturating_add(1), (*text).to_owned()))
                .collect(),
            pages: pages.iter().map(|page| (*page).to_owned()).collect(),
        }
    }

    #[test]
    fn a_gate_line_yields_both_halves_of_all_four_measures() {
        let printed = report(LINE);
        let figures = &printed[PAGE];
        assert_eq!(figures.len(), 8);
        assert!(figures.contains(&Printed {
            measure: Measure::WorstTile,
            side: Side::Ours,
            value: 6.73
        }));
        assert!(figures.contains(&Printed {
            measure: Measure::Ssim,
            side: Side::Bound,
            value: 0.9
        }));
    }

    #[test]
    fn a_line_that_is_not_a_page_is_not_read() {
        assert!(report("  processor time: 42s ours, 7s in three renderers").is_empty());
    }

    #[test]
    fn the_verb_is_not_the_measure() {
        let quoted = quotations_in(&note(
            &["It does not mean item 4 is paid by 0.75 of it"],
            &[],
        ));
        assert!(
            quoted.is_empty(),
            "`mean` in a sentence about meaning is not a measurement: {quoted:?}"
        );
    }

    #[test]
    fn the_differing_fraction_is_a_percentage_or_it_is_nothing() {
        assert_eq!(
            quotations_in(&note(&["differing 0.54% of it"], &[])).len(),
            1
        );
        assert!(quotations_in(&note(&["differing 0.54 of it"], &[])).is_empty());
    }

    /// This tree writes the differing fraction in front of its word as often as behind it.
    #[test]
    fn the_percentage_is_found_on_either_side_of_the_word() {
        let quoted = quotations_in(&note(&["0.54% of pixels differing on it"], &[]));
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].measure, Measure::Differing);
        assert_eq!(quoted[0].shown(), "0.54");
    }

    #[test]
    fn a_neighbouring_word_does_not_take_this_ones_number() {
        let quoted = quotations_in(&note(&["mean and the worst tile 14.16"], &[]));
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].measure, Measure::WorstTile);
    }

    #[test]
    fn the_three_spellings_of_the_similarity_are_one_figure() {
        let quoted = quotations_in(&note(&["structural similarity 0.9906"], &[]));
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].measure, Measure::Ssim);
        assert_eq!(quoted[0].decimals, 4);
    }

    /// Trap 13's plant, in the small: the archetype is a note giving `mean 0.22` for a page whose
    /// line reads `mean 0.17`. It is named, the gate's own value is offered as the correction,
    /// and the corrected sentence is not named.
    #[test]
    fn a_figure_the_gate_contradicts_is_the_finding() {
        let printed = report(LINE);
        let stale = note(
            &["the page gives mean 0.22 with 0.54% of pixels differing"],
            &[PAGE],
        );
        let found = sweep(std::slice::from_ref(&stale), &printed);
        assert_eq!(found.contradicted(), 1);
        let hit = &found.findings[0].contradicted[0];
        assert_eq!(hit.rung, Rung::Line);
        assert_eq!(hit.quotation.shown(), "0.22");
        assert_eq!(hit.instead[0].1.shown(2), "0.17");

        let current = note(
            &["the page gives mean 0.17 with 0.54% of pixels differing"],
            &[PAGE],
        );
        let clean = sweep(std::slice::from_ref(&current), &printed);
        assert_eq!(clean.contradicted(), 0);
        assert_eq!(clean.confirmed, 2);
    }

    #[test]
    fn a_figure_finer_than_the_gate_prints_is_compared_at_the_gates_precision() {
        let printed = report(LINE);
        // 0.99954 prints as `0.9995`, which is what the gate's line says.
        let agreeing = note(&["ours at ssim 0.99954"], &[PAGE]);
        assert_eq!(
            sweep(std::slice::from_ref(&agreeing), &printed).contradicted(),
            0
        );

        let stale = note(&["ours at ssim 0.98591"], &[PAGE]);
        let found = sweep(std::slice::from_ref(&stale), &printed);
        assert_eq!(found.contradicted(), 1);
        assert_eq!(found.findings[0].contradicted[0].rung, Rung::Finer);
    }

    #[test]
    fn a_note_the_report_does_not_name_has_nothing_to_be_judged_against() {
        let printed = report(LINE);
        let elsewhere = note(&["mean 0.22"], &["some_other.pdf page 3"]);
        let found = sweep(std::slice::from_ref(&elsewhere), &printed);
        assert!(found.findings.is_empty());
        assert_eq!(found.unanchored, 1);
    }
}
