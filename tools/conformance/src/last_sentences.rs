//! Which ledger rows open or close a note on a status word that is not the row's own.
//!
//! A `doc/todo/01` sweep (ADR 1249). Every other sweep in that catalogue reads a row against
//! something outside the ledger — the standard, the tree, the disk, the decision records. This
//! one reads a row against **itself**: the status field is one claim and the note's own sentence
//! about what the row is, is another, and the two are written by different rounds months apart.
//!
//! # The shape it exists for
//!
//! `doc/habits/the-ledger-and-claims-about-this-tree.md` states it: a note's last sentence — the
//! *what keeps this row `partial`* clause — is the one most likely to be stale, because every
//! later round appends **above** it. The opening sentence decays the same way for the mirror
//! reason: a round that moves a status rewrites the paragraph it is working in and leaves the
//! first line of the note saying what the row used to be. Both sites carry the same tell, and it
//! is mechanical — the sentence writes a status word in backticks, and it is not the word in the
//! row's `status` field.
//!
//! # Three rungs, closest first
//!
//! - [`Rung::AboutThisRow`] — the sentence says *this row*, *this aggregate* or *this clause's
//!   row*, and nothing in it is in the past tense. This is the defect: a present-tense claim
//!   about the row, in the row, disagreeing with the row.
//! - [`Rung::Narrated`] — the same self-reference with a past-tense marker beside it (*was*,
//!   *until*, *kept*, *said*, *no longer*). This is the ledger's house style for a correction,
//!   which `doc/habits`'s own rule asks for: a correction states the retired claim in words a
//!   sweep can still match. Marked rather than dropped, for that reason.
//! - [`Rung::Elsewhere`] — no self-reference, so the status word is probably a neighbour's.
//!   A parent naming a child's status is the dominant shape here and is correct prose.
//!
//! The noise the top rung keeps is a sentence naming the status a *pending question* would move
//! the row to: §8.6.6.5's note ends on `doc/questions/Q100` recommending `implemented` while the
//! row is `departed`, which is a forward-looking sentence and not a stale one. Read the sentence
//! before believing a hit.
//!
//! # Why it prints rather than fails
//!
//! For the reason every reading sweep here prints: whether a sentence is about this row is a
//! question about English, and a gate that answered it would be believed. The map-vs-ledger
//! assertion beside it in `doc/todo/01` *is* a gate, because its two sides are both lists.

use std::fmt;

use crate::ledger::{Row, Status};

/// The words this sweep recognises as a status, which is every word the ledger writes.
///
/// Taken from [`Status`] rather than spelled out, so a status added to the enum joins the sweep
/// without anybody remembering to add it here.
const STATUSES: [Status; 9] = [
    Status::Implemented,
    Status::Partial,
    Status::Departed,
    Status::Reported,
    Status::Silent,
    Status::Inapplicable,
    Status::WriterSide,
    Status::OutOfScope,
    Status::Unreviewed,
];

/// The phrases that make a sentence a claim about the row it is written in.
///
/// Three, and no more: a sweep that read *the row* as a self-reference would take in every
/// sentence a parent writes about a child ("the row below is `partial`"), which is rung 3's
/// whole population.
const SELF_REFERENCE: [&str; 3] = ["this row", "this aggregate", "this clause's row"];

/// The markers that make a self-referential sentence a narrated correction rather than a claim.
///
/// Matched as whole words, so `wasted` is not `was`.
const NARRATION: [&str; 9] = [
    "was", "were", "had", "until", "kept", "said", "stopped", "longer", "used",
];

/// Where in the note a sentence sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// The note's first sentence, which a round that moves a status rewrites around.
    Opening,
    /// The note's last sentence, which every later round appends above.
    Closing,
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match *self {
            Self::Opening => "opening",
            Self::Closing => "closing",
        };
        formatter.write_str(word)
    }
}

/// How close a hit is to being a defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// Present tense, about this row, disagreeing with this row.
    AboutThisRow,
    /// About this row and in the past tense: the ledger's shape for a correction.
    Narrated,
    /// No self-reference, so the word is probably a neighbour's status.
    Elsewhere,
}

impl fmt::Display for Rung {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match *self {
            Self::AboutThisRow => "about this row",
            Self::Narrated => "about this row, in the past tense",
            Self::Elsewhere => "no self-reference",
        };
        formatter.write_str(word)
    }
}

/// One sentence that states a status the row does not wear.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The row's clause number, as the ledger writes it.
    pub clause: String,
    /// The status the row actually wears.
    pub status: Status,
    /// The 1-based ledger line the row starts on.
    pub line: usize,
    /// Which end of the note the sentence is.
    pub position: Position,
    /// How close the hit is to being a defect.
    pub rung: Rung,
    /// The status words the sentence states that the row does not wear.
    pub stated: Vec<Status>,
    /// The sentence itself.
    pub sentence: String,
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The hits, closest rung first.
    pub findings: Vec<Finding>,
    /// Rows carrying a note, which is what this sweep can read at all.
    pub population: usize,
    /// Sentences read: two per row with a note, or one where the note is a single sentence.
    pub read: usize,
}

impl Report {
    /// How many hits sit on one rung.
    #[must_use]
    pub fn on(&self, rung: Rung) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rung == rung)
            .count()
    }
}

/// Reads every row's opening and closing sentence for a status word that is not the row's.
#[must_use]
pub fn sweep(rows: &[Row]) -> Report {
    let mut report = Report::default();
    for row in rows {
        let Some(note) = row.note.as_deref() else {
            continue;
        };
        let sentences = crate::unread::sentences(note);
        let (Some(first), Some(last)) = (sentences.first(), sentences.last()) else {
            continue;
        };
        report.population = report.population.saturating_add(1);
        let ends = if sentences.len() == 1 {
            vec![(Position::Opening, *first)]
        } else {
            vec![(Position::Opening, *first), (Position::Closing, *last)]
        };
        for (position, sentence) in ends {
            report.read = report.read.saturating_add(1);
            let stated = disagreeing(sentence, row.status);
            if stated.is_empty() {
                continue;
            }
            report.findings.push(Finding {
                clause: row.clause.to_string(),
                status: row.status,
                line: row.line,
                position,
                rung: rung_of(sentence),
                stated,
                sentence: sentence.to_owned(),
            });
        }
    }
    report.findings.sort_by(|left, right| {
        left.rung
            .cmp(&right.rung)
            .then_with(|| left.line.cmp(&right.line))
    });
    report
}

/// The status words one sentence writes in backticks that are not `status`.
fn disagreeing(sentence: &str, status: Status) -> Vec<Status> {
    let mut stated = Vec::new();
    for span in backticked(sentence) {
        for candidate in STATUSES {
            if candidate.as_str() == span && candidate != status && !stated.contains(&candidate) {
                stated.push(candidate);
            }
        }
    }
    stated
}

/// The spans one sentence writes between backticks.
///
/// A lone backtick at the end of a sentence closes nothing and is dropped, which is what the
/// split below does by taking only the odd-indexed pieces of a complete pairing.
fn backticked(sentence: &str) -> Vec<&str> {
    let pieces: Vec<&str> = sentence.split('`').collect();
    let complete = pieces.len() % 2 == 1;
    let last = if complete {
        pieces.len()
    } else {
        pieces.len().saturating_sub(1)
    };
    pieces
        .get(..last)
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .filter_map(|(index, piece)| (index % 2 == 1).then_some(*piece))
        .collect()
}

/// Which rung one sentence sits on.
fn rung_of(sentence: &str) -> Rung {
    let lowered = sentence.to_ascii_lowercase();
    if !SELF_REFERENCE.iter().any(|phrase| lowered.contains(phrase)) {
        return Rung::Elsewhere;
    }
    if NARRATION.iter().any(|word| holds_word(&lowered, word)) {
        return Rung::Narrated;
    }
    Rung::AboutThisRow
}

/// Whether a lower-cased sentence holds `word` as a whole word.
fn holds_word(lowered: &str, word: &str) -> bool {
    lowered.match_indices(word).any(|(at, _)| {
        let before = lowered[..at].chars().next_back();
        let after = lowered[at.saturating_add(word.len())..].chars().next();
        let boundary = |character: Option<char>| {
            character.is_none_or(|character| !character.is_ascii_alphanumeric())
        };
        boundary(before) && boundary(after)
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::{Position, Rung, sweep};
    use crate::clause::ClauseNumber;
    use crate::ledger::{Row, Status};

    /// A row whose note is `note`, for planting one sentence at a time.
    fn row(clause: &str, status: Status, note: &str) -> Row {
        let number = ClauseNumber::from_str(clause).expect("a clause number this test wrote");
        let mut row = Row::unreviewed(number, "planted".to_owned());
        row.status = status;
        row.note = Some(note.to_owned());
        row.line = 1;
        row
    }

    /// Trap 13: the sweep comes back clean over today's ledger on its top rung, so what proves it
    /// fires is a planted defect — the exact sentence session 1200 found on §12.11.6's row, whose
    /// status was `implemented` while its opening sentence said what the row was `partial` for.
    #[test]
    fn a_planted_opening_sentence_naming_another_status_is_named_on_the_closest_rung() {
        let planted = row(
            "12.11.6",
            Status::Implemented,
            "No reader can instruct otherwise, which is what this row is `partial` for. The \
             processing rule, and the one place this program departs on purpose.",
        );
        let report = sweep(std::slice::from_ref(&planted));
        assert_eq!(report.on(Rung::AboutThisRow), 1);
        let finding = report.findings.first().expect("the planted sentence");
        assert_eq!(finding.position, Position::Opening);
        assert_eq!(finding.stated, vec![Status::Partial]);
    }

    /// And the closing end, which is the site the habit names: every later round appends above it.
    #[test]
    fn a_planted_closing_sentence_is_named_too() {
        let planted = row(
            "11.7.2",
            Status::Implemented,
            "The clause is read whole. What keeps this row `partial` is the press ceiling.",
        );
        let report = sweep(std::slice::from_ref(&planted));
        assert_eq!(report.on(Rung::AboutThisRow), 1);
        assert_eq!(
            report.findings.first().map(|finding| finding.position),
            Some(Position::Closing)
        );
    }

    /// Restoring the correction takes the plant off the top rung and leaves it marked, which is
    /// the other half of the calibration: a sweep that cannot tell a correction from a claim
    /// reports the ledger's own house style as a finding.
    #[test]
    fn a_correction_that_states_what_it_retired_is_marked_rather_than_dropped() {
        let corrected = row(
            "12.11.6",
            Status::Implemented,
            "The sentence said no reader can instruct otherwise, which is what this row was \
             `partial` for, until the levels reached this operation.",
        );
        let report = sweep(std::slice::from_ref(&corrected));
        assert_eq!(report.on(Rung::AboutThisRow), 0);
        assert_eq!(report.on(Rung::Narrated), 1);
    }

    /// A parent naming a child's status is correct prose and sits on the last rung.
    #[test]
    fn a_status_word_with_no_self_reference_is_the_last_rung() {
        let parent = row(
            "8.11",
            Status::Partial,
            "Aggregate. §8.11.4.5 is `implemented` and §8.11.4.1 is not.",
        );
        let report = sweep(std::slice::from_ref(&parent));
        assert_eq!(report.on(Rung::Elsewhere), 1);
    }

    /// The row's own word is not a hit, however often the note writes it.
    #[test]
    fn a_sentence_stating_the_rows_own_status_is_not_a_hit() {
        let agreeing = row(
            "8.11",
            Status::Partial,
            "What keeps this row `partial` is §8.11.4.1, still `partial` itself.",
        );
        assert!(sweep(std::slice::from_ref(&agreeing)).findings.is_empty());
    }
}
