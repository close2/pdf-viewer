//! The twenty-first sweep: a page-list note that never names the bound the gate fails on.
//!
//! # The shape it exists for
//!
//! [`crate::quoted`] asks whether the figures a note quotes still agree with the gate's. This
//! one asks the question that sweep said it could not: **a figure a note is missing.** Its own
//! closing sentence is the specification for this one — *`--bin quoted` checks a figure a note
//! quotes; it cannot ask for one that is missing* — and five rounds recorded the same debt in
//! the same words: *nothing links a group's note to which bound the gate fails its pages on*
//! (sessions 489, 668, 672, 675 and 680).
//!
//! What that debt costs is stated in `doc/adr/0497`'s sixth criterion, and it is the reason a
//! contradicted page is allowed to stand at all: **a mechanism explained is not a number
//! accounted for.** A contradicted entry is a standing exemption from a *specific failing
//! bound*, so a note that prices its mechanism in ink, in cap rows or in a perimeter — and
//! never says which of the gate's four measures its pages actually fail — has explained the
//! picture and not the verdict. Every one of those thirteen diagnoses began by reading the
//! failing bound off a log by hand.
//!
//! # The discriminator
//!
//! > A measure the gate fails one of a note's own pages on, in a verdict of **contradicted**,
//! > that the note's prose never names.
//!
//! Both sides are this project's own output, as [`crate::quoted`]'s are. The right-hand side is
//! the gate's per-page line, which prints all four measures beside all four bounds, so *which
//! bound fails* is arithmetic on a line the round has already run — no rasterising and no
//! re-measuring. The left-hand side is the note's own words.
//!
//! # Why only `contradicted`
//!
//! Trap 11: a report is only as good as the condition it fires on. The gate prints its four
//! figures for every page it does not call agreement, and on an `ambiguous` page no two
//! references agreed, so the bound beside them decided nothing — asking a note to account for a
//! bound no verdict rests on would manufacture a debt out of a line's shape. `not comparable`,
//! `no render` and the two geometry verdicts are reached without a comparison at all. So the
//! population is the pages the gate calls contradicted and no others, which is also exactly the
//! population the sixth criterion is about.
//!
//! # The rungs — read the first one first
//!
//! 1. **[`Rung::Elsewhere`] — the note names measures, and none of them is one its pages fail.**
//!    The sharpest shape and the one the criterion was written for: a note arguing about a mean
//!    over a page that fails on the differing fraction is explaining a number nobody is holding
//!    it to. `CONTRADICTED_GLYPH_EDGES` stood on that exact sentence for three hundred sessions
//!    — *"\[e\]ach fails **only** on mean absolute difference"* — where all 21 of its pages failed
//!    on the differing fraction and nothing else (ADR 0242).
//! 2. **[`Rung::Silent`] — the note names no measure at all.** The diagnosis is prose, and the
//!    verdict it is an exemption from is nowhere in it.
//! 3. **[`Rung::Partial`] — the note names one failing measure and misses another.** The
//!    commonest and the mildest: a group whose pages do not all fail on the same bound, written
//!    up from the one that was read.
//!
//! # The noise, classified rather than filtered
//!
//! - **A group's pages need not fail on one bound**, so a note can be complete about the page
//!   its author opened and land on rung 3 for a page nobody has. That is a reading list entry
//!   and not an error; the finding prints the page beside the measure for exactly that reason.
//! - **The word `mean` is also a verb**, and the measure and the auxiliary are the same four
//!   letters. `NOT_A_MEASURE` is the stated exclusion — a preceding word that makes it the
//!   verb — and it is the one place this sweep guesses. The guess is calibrated in the tests
//!   and its residue is a *missed* hit rather than an invented one, which is the direction
//!   trap 11 asks a report to err in.
//! - **A note may name the measure while arguing that the failure is not ours**, which is a
//!   good note and is credited: the discriminator is whether the bound is *named*, never
//!   whether the argument persuades. Only a person can judge the second.
//!
//! # Why it is not a build failure
//!
//! Same argument as the nineteenth and twentieth sweeps': every hit is a sentence somebody has
//! to read, and the sixth criterion's own answer on eight groups was that each was defensible.
//! It ranks, it names the page and the measure, and it exits zero on a finding.

use std::collections::{BTreeMap, BTreeSet};

use crate::overtaken::Note;
use crate::quoted::{Measure, Printed, Side};

/// What the gate writes as the verdict this sweep is about.
///
/// `Verdict::label` in `crates/pdf-model/tests/oracle.rs`, verbatim and in its own case: the
/// other six labels are lower case, so an exact match is what tells this verdict from a
/// sentence about it.
const CONTRADICTED: &str = "CONTRADICTED";

/// The words that make `mean` the verb rather than the measure.
///
/// The measure is a noun and takes an article, a preposition or a possessive; the auxiliary
/// takes a subject or a negation. Over the oracle's own notes the verb reading is a handful of
/// occurrences — *would mean*, *cannot mean*, *they mean* — against dozens of the measure, so
/// the list is short by measurement rather than by optimism, and a word missing from it costs a
/// finding this sweep does not print rather than one it invents.
const NOT_A_MEASURE: [&str; 15] = [
    "not", "cannot", "to", "would", "will", "may", "might", "must", "should", "could", "does",
    "did", "do", "they", "that",
];

/// Which of the gate's four measures one contradicted page is outside its bound on.
///
/// The comparison is `pdfref::Tolerance::accepts`' — three measures are ceilings and the
/// structural similarity is a floor — taken at the precision the gate *prints*, because a
/// printed line is all this sweep reads. A page whose figures round to equality therefore
/// contributes an empty set and is counted rather than assumed: see [`Report::rounded`].
#[must_use]
pub fn failing(figures: &[Printed]) -> BTreeSet<Measure> {
    let side = |measure: Measure, side: Side| {
        figures
            .iter()
            .find(|figure| figure.measure == measure && figure.side == side)
            .map(|figure| figure.value)
    };
    let mut failing = BTreeSet::new();
    for measure in Measure::ALL {
        let (Some(ours), Some(bound)) = (side(measure, Side::Ours), side(measure, Side::Bound))
        else {
            continue;
        };
        let outside = if measure == Measure::Ssim {
            ours < bound
        } else {
            ours > bound
        };
        if outside {
            failing.insert(measure);
        }
    }
    failing
}

/// Every page the gate calls contradicted, with the measures it fails on.
///
/// The line is the one [`crate::quoted::report`] parses, with its verdict kept:
///
/// ```text
///   issue7891_bc1.pdf page 1: CONTRADICTED — ours at worst mean 0.16 worst tile 6.73 differing
///   0.54% ssim 0.9995; bound mean 5.00 worst tile 6.04 differing 5.00% ssim 0.9000
/// ```
#[must_use]
pub fn contradicted(
    text: &str,
    printed: &BTreeMap<String, Vec<Printed>>,
) -> BTreeMap<String, BTreeSet<Measure>> {
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(": ") else {
            continue;
        };
        let name = name.trim();
        if !name.contains(".pdf page ") || !rest.starts_with(CONTRADICTED) {
            continue;
        }
        if let Some(figures) = printed.get(name) {
            found.insert(name.to_owned(), failing(figures));
        }
    }
    found
}

/// How sharply a note misses the bound its pages are held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The note names measures and not one of them is a measure its pages fail.
    Elsewhere,
    /// The note names no measure at all.
    Silent,
    /// The note names one failing measure and misses another.
    Partial,
}

impl Rung {
    /// One line saying what this rung means, printed above every hit on it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Elsewhere => {
                "the note names measures, and none of them is one its pages are failing on"
            }
            Self::Silent => "the note names no measure at all",
            Self::Partial => "the note names one failing measure and misses another",
        }
    }
}

/// One note, and the bounds its own pages fail on that it never names.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The note.
    pub note: Note,
    /// How sharply.
    pub rung: Rung,
    /// The measures the note does name, whether or not its pages fail them.
    pub named: BTreeSet<Measure>,
    /// Each unnamed failing measure, with the note's own pages that fail on it.
    pub unpriced: Vec<(Measure, Vec<String>)>,
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The hits, closest rung first.
    pub findings: Vec<Finding>,
    /// How many page-list notes were read.
    pub notes: usize,
    /// How many of those hold at least one page the gate calls contradicted.
    pub holding: usize,
    /// How many contradicted pages the report named.
    pub pages: usize,
    /// How many failing (page, measure) pairs were read in all.
    pub bounds: usize,
    /// How many of those the note holding the page names.
    pub priced: usize,
    /// The contradicted pages that print a line inside every bound once rounded.
    ///
    /// A contradicted verdict is a failure at full precision by construction, so these are
    /// pages whose margin is smaller than the two decimals the gate prints — not pages that
    /// pass. They contribute no bound and are named rather than folded away.
    pub rounded: BTreeSet<String>,
    /// The contradicted pages that sit in no page-list note at all.
    ///
    /// **A count beside a list is not the list** (`doc/todo/02` §6), and this field is the
    /// second half of that rule arriving here: a page nobody has diagnosed is the finding, and
    /// the number 5 becoming 6 is not one anybody can act on.
    pub unheld: BTreeSet<String>,
}

impl Report {
    /// How many findings sit on one rung.
    #[must_use]
    pub fn on(&self, rung: Rung) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rung == rung)
            .count()
    }

    /// How many unnamed failing bounds the run found, over every rung.
    #[must_use]
    pub fn unpriced(&self) -> usize {
        self.findings
            .iter()
            .flat_map(|finding| finding.unpriced.iter())
            .map(|(_, pages)| pages.len())
            .sum()
    }
}

/// Runs the sweep.
///
/// `failing` is [`contradicted`]'s map: every page the gate calls contradicted, with the
/// measures it is outside its bound on.
#[must_use]
pub fn sweep(notes: &[Note], failing: &BTreeMap<String, BTreeSet<Measure>>) -> Report {
    let mut report = Report {
        notes: notes.len(),
        pages: failing.len(),
        rounded: failing
            .iter()
            .filter(|(_, measures)| measures.is_empty())
            .map(|(page, _)| page.clone())
            .collect(),
        ..Report::default()
    };

    let mut held: BTreeSet<&str> = BTreeSet::new();
    for note in notes {
        let mine: Vec<(&String, &BTreeSet<Measure>)> = note
            .pages
            .iter()
            .filter_map(|page| failing.get_key_value(page))
            .collect();
        if mine.is_empty() {
            continue;
        }
        report.holding = report.holding.saturating_add(1);
        for (page, _) in &mine {
            held.insert(page.as_str());
        }

        let named = named_in(note);
        let mut unpriced: BTreeMap<Measure, Vec<String>> = BTreeMap::new();
        for (page, measures) in &mine {
            for measure in *measures {
                report.bounds = report.bounds.saturating_add(1);
                if named.contains(measure) {
                    report.priced = report.priced.saturating_add(1);
                } else {
                    unpriced.entry(*measure).or_default().push((*page).clone());
                }
            }
        }
        if unpriced.is_empty() {
            continue;
        }

        let rung = if named.is_empty() {
            Rung::Silent
        } else if report_named_a_failing_one(&named, &mine) {
            Rung::Partial
        } else {
            Rung::Elsewhere
        };
        report.findings.push(Finding {
            note: note.clone(),
            rung,
            named,
            unpriced: unpriced.into_iter().collect(),
        });
    }

    report.unheld = failing
        .iter()
        .filter(|(page, measures)| !measures.is_empty() && !held.contains(page.as_str()))
        .map(|(page, _)| page.clone())
        .collect();
    report
        .findings
        .sort_by_key(|finding| (finding.rung, finding.note.line));
    report
}

/// Whether any measure the note names is one its own pages fail on.
fn report_named_a_failing_one(
    named: &BTreeSet<Measure>,
    mine: &[(&String, &BTreeSet<Measure>)],
) -> bool {
    mine.iter()
        .any(|(_, measures)| measures.iter().any(|measure| named.contains(measure)))
}

/// Every measure one note's prose names.
///
/// Word presence rather than [`crate::quoted`]'s word-plus-figure, and the difference is the
/// point: a note saying *"all three fail on mean and structural similarity"* has named both
/// bounds and quoted neither, which is exactly what this sweep is looking for.
#[must_use]
pub fn named_in(note: &Note) -> BTreeSet<Measure> {
    let mut named = BTreeSet::new();
    for (_, text) in &note.body {
        for measure in Measure::ALL {
            if named.contains(&measure) {
                continue;
            }
            if names(text, measure) {
                named.insert(measure);
            }
        }
    }
    named
}

/// Whether one line of prose names one measure.
fn names(text: &str, measure: Measure) -> bool {
    measure.words().iter().any(|word| {
        let mut from = 0usize;
        while let Some(at) = text.get(from..).and_then(|rest| rest.find(word)) {
            let start = from.saturating_add(at);
            let end = start.saturating_add(word.len());
            if standalone(text, start, end) && !(measure == Measure::Mean && is_verb(text, start)) {
                return true;
            }
            from = end;
        }
        false
    })
}

/// Whether the word at `[start, end)` stands on its own rather than inside a longer one.
///
/// The same test [`crate::quoted`] makes and for the same reason: `mean` is inside `meaning`
/// and `means`, and both are ordinary English in these notes.
fn standalone(text: &str, start: usize, end: usize) -> bool {
    let before = text.get(..start).and_then(|head| head.chars().next_back());
    let after = text.get(end..).and_then(|tail| tail.chars().next());
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

/// Whether the `mean` at `start` is the auxiliary verb, by the word in front of it.
fn is_verb(text: &str, start: usize) -> bool {
    let Some(head) = text.get(..start) else {
        return false;
    };
    let previous = head
        .split_whitespace()
        .next_back()
        .unwrap_or_default()
        .trim_matches(|c: char| !c.is_ascii_alphabetic())
        .to_ascii_lowercase();
    NOT_A_MEASURE.contains(&previous.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "  issue7891_bc1.pdf page 1: CONTRADICTED — ours at worst mean 0.17 \
                        worst tile 6.73 differing 5.54% ssim 0.9995; bound mean 5.00 worst \
                        tile 6.04 differing 5.00% ssim 0.9000";

    const PAGE: &str = "issue7891_bc1.pdf page 1";

    fn note(body: &[&str], pages: &[&str]) -> Note {
        Note {
            name: "A_LIST_OF_PAGES".to_owned(),
            file: crate::quoted::GATE_NOTES.to_owned(),
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

    fn read(line: &str) -> BTreeMap<String, BTreeSet<Measure>> {
        contradicted(line, &crate::quoted::report(line))
    }

    /// The worst tile and the differing fraction are outside; the mean and the ssim are not.
    #[test]
    fn the_failing_measures_are_the_ones_outside_their_own_bound() {
        let failing = read(LINE);
        assert_eq!(
            failing.get(PAGE),
            Some(&BTreeSet::from([Measure::WorstTile, Measure::Differing]))
        );
    }

    /// An `ambiguous` page's bound decided nothing, so the sweep may not ask a note about it.
    #[test]
    fn only_a_contradicted_verdict_is_a_failing_bound() {
        let ambiguous = LINE.replace(CONTRADICTED, "ambiguous");
        assert!(read(&ambiguous).is_empty());
    }

    /// The archetype: a note arguing about the one measure its page passes.
    #[test]
    fn a_note_explaining_a_passing_measure_is_the_closest_rung() {
        let failing = read(LINE);
        let note = note(
            &["Each fails only on mean absolute difference, 5.4 of 5.00."],
            &[PAGE],
        );
        let report = sweep(std::slice::from_ref(&note), &failing);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rung, Rung::Elsewhere);
        assert_eq!(report.unpriced(), 2);
    }

    /// Naming one of the two is milder than naming neither, and says so.
    #[test]
    fn naming_one_failing_measure_and_missing_another_is_the_mildest_rung() {
        let failing = read(LINE);
        let note = note(
            &["The page fails one bound and it is the worst tile."],
            &[PAGE],
        );
        let report = sweep(std::slice::from_ref(&note), &failing);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rung, Rung::Partial);
        assert_eq!(report.findings[0].unpriced.len(), 1);
        assert_eq!(report.findings[0].unpriced[0].0, Measure::Differing);
    }

    /// A note that names both is not a finding at all, with no figure anywhere in it.
    #[test]
    fn naming_the_bounds_without_quoting_them_is_enough() {
        let failing = read(LINE);
        let note = note(
            &["All three pages fail on the worst tile and the differing fraction."],
            &[PAGE],
        );
        let report = sweep(std::slice::from_ref(&note), &failing);
        assert!(report.findings.is_empty());
        assert_eq!(report.priced, 2);
    }

    /// Prose with no measure in it is the middle rung.
    #[test]
    fn a_note_naming_no_measure_is_the_middle_rung() {
        let failing = read(LINE);
        let note = note(
            &["A substituted face is 3.6% lighter at every scale."],
            &[PAGE],
        );
        let report = sweep(std::slice::from_ref(&note), &failing);
        assert_eq!(report.findings[0].rung, Rung::Silent);
        assert!(report.findings[0].named.is_empty());
    }

    /// `mean` the auxiliary is not `mean` the measure, which is the sweep's one guess.
    #[test]
    fn the_verb_does_not_name_the_measure() {
        let verb = note(&["It does not mean the page is ours."], &[PAGE]);
        assert!(named_in(&verb).is_empty());
        let measure = note(&["Ours is a mean of 5.4 against the bound."], &[PAGE]);
        assert_eq!(named_in(&measure), BTreeSet::from([Measure::Mean]));
    }

    /// All three spellings of the similarity are the one measure.
    #[test]
    fn the_similarity_is_named_three_ways() {
        for spelling in ["ssim", "similarity", "structural similarity"] {
            let note = note(&[&format!("what fails is the {spelling}")], &[PAGE]);
            assert_eq!(
                named_in(&note),
                BTreeSet::from([Measure::Ssim]),
                "{spelling}"
            );
        }
    }

    /// A contradicted page on no list is counted, because nobody is accounting for it at all.
    #[test]
    fn a_page_no_note_holds_is_counted() {
        let failing = read(LINE);
        let report = sweep(&[], &failing);
        assert_eq!(report.unheld.len(), 1);
        assert_eq!(report.pages, 1);
    }
}
