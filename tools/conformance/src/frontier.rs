//! The frontier map's population against the ledger's own open rows.
//!
//! `doc/todo/65-the-remaining-frontier.md` groups the work that is left by *why* it is not done.
//! Its population is stated in the document itself: the ledger's `partial` and `reported` rows,
//! and only those. That is a claim with two lists behind it, so it is checkable — and until it
//! was checked it drifted, which is ADR 1237's finding: four `implemented` rows sat in the map's
//! aggregate list and six `departed` rows sat in its buckets as though they were owed work.
//!
//! # Why this one is a gate where the sweeps beside it are reports
//!
//! `doc/todo/01`'s sweeps read a row against prose — the standard's, a note's, a comment's — and
//! a program that judged prose would be believed. Here both sides are lists of clause numbers:
//! the map's bullets and aggregate paragraph on one side, `grep 'status ='` on the other. A
//! disagreement is a fact, not a reading, so it fails `cargo test -p conformance` rather than
//! printing.
//!
//! # What counts as *mapped*
//!
//! Two places, because the map has two shapes for a row:
//!
//! - **A bullet's head** — the run of clause numbers before the first EM DASH of a line opening
//!   `- §`. The prose after the dash names neighbouring clauses constantly ("the row moves with
//!   the family above it"), and reading those would make every bullet claim rows it only
//!   mentions.
//! - **The aggregate paragraph** — the lines under the *Aggregate rows* heading that open with a
//!   `§`. Those rows have no debt of their own and get a list rather than a bullet each.
//!
//! Everything else in the document is prose about a row, which is allowed to name any clause it
//! likes: a mention is not a placement.

use std::collections::BTreeMap;

use crate::ledger::{Ledger, Status};

/// The map, relative to the tree's root.
pub const MAP: &str = "doc/todo/65-the-remaining-frontier.md";

/// The heading the aggregate list sits under.
const AGGREGATE_HEADING: &str = "### Aggregate rows";

/// Where the map places one clause number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// The head of a bullet, before its EM DASH.
    Bullet,
    /// The aggregate paragraph, which lists rows with no debt of their own.
    Aggregate,
}

/// One clause number the map places, and where.
#[derive(Debug, Clone)]
pub struct Placement {
    /// The clause number as the map writes it, without the SECTION SIGN.
    pub clause: String,
    /// The 1-based line of the map it is written on.
    pub line: usize,
    /// Which of the map's two shapes holds it.
    pub place: Place,
}

/// Why the map and the ledger disagree about one clause.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Disagreement {
    /// The map places a clause whose row is not open work.
    #[error(
        "{MAP}:{line} places §{clause}, whose ledger row is `{status}` rather than open work — \
         the map's population is the ledger's `partial` and `reported` rows and only those"
    )]
    NotOpen {
        /// The clause the map places.
        clause: String,
        /// Where the map places it.
        line: usize,
        /// What the ledger says instead.
        status: String,
    },
    /// The map places a clause the ledger has no row for at all.
    #[error("{MAP}:{line} places §{clause}, which the conformance ledger has no row for")]
    NoRow {
        /// The clause the map places.
        clause: String,
        /// Where the map places it.
        line: usize,
    },
    /// An open row the map places nowhere.
    #[error(
        "{LEDGER}:{line} is §{clause}, `{status}`, and {MAP} places it in no bucket and in no \
         aggregate list — an open row the map cannot steer",
        LEDGER = crate::LEDGER
    )]
    Unmapped {
        /// The row nothing places.
        clause: String,
        /// The ledger line it starts on.
        line: usize,
        /// The status that makes it open work.
        status: String,
    },
    /// One clause placed in two places, so two buckets claim it.
    #[error("{MAP} places §{clause} more than once: on lines {lines}")]
    Twice {
        /// The clause placed twice.
        clause: String,
        /// The map lines, comma-separated.
        lines: String,
    },
}

/// Every clause number the map places, in the order it writes them.
#[must_use]
pub fn placements(map: &str) -> Vec<Placement> {
    let mut found = Vec::new();
    let mut in_aggregate = false;
    for (index, line) in map.lines().enumerate() {
        let number = index.saturating_add(1);
        if line.starts_with(AGGREGATE_HEADING) {
            in_aggregate = true;
            continue;
        }
        if in_aggregate && line.starts_with("### ") {
            in_aggregate = false;
        }
        if let Some(rest) = line.strip_prefix("- §") {
            let head = rest.split('—').next().unwrap_or(rest);
            for clause in clause_numbers(&format!("§{head}")) {
                found.push(Placement {
                    clause,
                    line: number,
                    place: Place::Bullet,
                });
            }
        } else if in_aggregate && line.starts_with('§') {
            for clause in clause_numbers(line) {
                found.push(Placement {
                    clause,
                    line: number,
                    place: Place::Aggregate,
                });
            }
        }
    }
    found
}

/// The clause numbers one span writes with a SECTION SIGN.
///
/// A number is the sign followed by a digit or an annex letter, then digits and FULL STOPs; a
/// trailing stop is the sentence's and is dropped.
fn clause_numbers(span: &str) -> Vec<String> {
    let mut numbers = Vec::new();
    for (index, _) in span.match_indices('§') {
        let rest = &span[index.saturating_add('§'.len_utf8())..];
        let mut end = 0usize;
        for (offset, character) in rest.char_indices() {
            if character.is_ascii_digit() || character == '.' || character.is_ascii_uppercase() {
                end = offset.saturating_add(character.len_utf8());
            } else {
                break;
            }
        }
        let number = rest[..end].trim_end_matches('.');
        if !number.is_empty() && !numbers.iter().any(|seen| seen == number) {
            numbers.push(number.to_owned());
        }
    }
    numbers
}

/// Where the map and the ledger disagree, the map's own placements first.
#[must_use]
pub fn compare(ledger: &Ledger, map: &str) -> Vec<Disagreement> {
    let placed = placements(map);
    let mut disagreements = Vec::new();

    let mut lines_by_clause: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for placement in &placed {
        lines_by_clause
            .entry(placement.clause.as_str())
            .or_default()
            .push(placement.line);
    }
    for (clause, lines) in &lines_by_clause {
        if lines.len() > 1 {
            let written: Vec<String> = lines.iter().map(usize::to_string).collect();
            disagreements.push(Disagreement::Twice {
                clause: (*clause).to_owned(),
                lines: written.join(", "),
            });
        }
    }

    for placement in &placed {
        let row = ledger
            .rows
            .iter()
            .find(|row| row.clause.to_string() == placement.clause);
        match row {
            None => disagreements.push(Disagreement::NoRow {
                clause: placement.clause.clone(),
                line: placement.line,
            }),
            Some(row) if !is_open(row.status) => {
                disagreements.push(Disagreement::NotOpen {
                    clause: placement.clause.clone(),
                    line: placement.line,
                    status: row.status.as_str().to_owned(),
                });
            }
            Some(_) => {}
        }
    }

    for row in &ledger.rows {
        if !is_open(row.status) {
            continue;
        }
        let clause = row.clause.to_string();
        if !lines_by_clause.contains_key(clause.as_str()) {
            disagreements.push(Disagreement::Unmapped {
                clause,
                line: row.line,
                status: row.status.as_str().to_owned(),
            });
        }
    }

    disagreements
}

/// Whether a status makes a row the map's business.
///
/// The map states its own population in its opening lines and this is it: `partial` and
/// `reported`. `silent` and `unreviewed` owe something too and are deliberately outside — a
/// `silent` row has nobody's reading behind it to bucket, and `unreviewed` is the ledger's
/// initial state.
fn is_open(status: Status) -> bool {
    matches!(status, Status::Partial | Status::Reported)
}

#[cfg(test)]
mod tests {
    use super::{Place, clause_numbers, placements};

    #[test]
    fn a_bullets_head_is_read_and_its_prose_is_not() {
        let map = "- §11.4.4, §11.4.6 — the element §11.3.7.2's row also names.\n";
        let placed = placements(map);
        let clauses: Vec<&str> = placed
            .iter()
            .map(|placement| placement.clause.as_str())
            .collect();
        assert_eq!(clauses, ["11.4.4", "11.4.6"]);
        assert!(
            placed
                .iter()
                .all(|placement| placement.place == Place::Bullet)
        );
    }

    #[test]
    fn the_aggregate_paragraph_is_read_and_the_sentence_under_it_is_not() {
        let map = "### Aggregate rows — no debt of their own\n\n§7.6, §8.9.6,\n§12.8.3.4.\n\n\
                   Two of them carry no child: §11.4.3 defers to §11.4.4.\n\n### Not owed\n\n\
                   - §12.7.5.4 — a selection mark.\n";
        let placed = placements(map);
        let clauses: Vec<&str> = placed
            .iter()
            .map(|placement| placement.clause.as_str())
            .collect();
        assert_eq!(clauses, ["7.6", "8.9.6", "12.8.3.4", "12.7.5.4"]);
    }

    #[test]
    fn a_trailing_full_stop_is_the_sentences_and_not_the_numbers() {
        assert_eq!(clause_numbers("§12.8.3.4."), ["12.8.3.4"]);
        assert_eq!(clause_numbers("§F.3 and §K.2"), ["F.3", "K.2"]);
    }
}
