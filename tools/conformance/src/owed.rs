//! The fourteenth sweep: a `partial` row whose note names nothing owed.
//!
//! # The shape it exists for
//!
//! The ledger's own definition of the status is "some are; the note says which, which are not,
//! and what is reported". A row that is `partial` above a note naming nothing owed breaks that
//! definition, and no other sweep in `doc/todo/01` can see it: sweeps 1, 3 and 4 all read a
//! *reason* — a blocker, a capability, a retired string — and a row in this shape gives none.
//! The sixth sweep's arithmetic cannot either, because it asks whether every child is settled and
//! a family carrying this error usually disqualifies itself with it.
//!
//! Its first run, in the four-hundred-and-thirty-seventh session, moved two statuses. §9.3 had
//! been `partial` since the thirteenth session on a sentence false from the seventy-second —
//! **365 sessions**, the longest stale claim `doc/todo/01` had recorded at the time — and
//! §11.3.7.1 since the fifteenth.
//!
//! # Why the hand-runs' number said nothing
//!
//! Nine runs printed 16, 24, 5, 15, 19, 10, 8, 9 and 19 hits over rounds that moved almost no
//! rows, because each session wrote the debt vocabulary that morning: "writer-side", "may be
//! ignored", "is not read", "counted, not parsed", "aggregate of the two below". A level that
//! cannot be compared with the one before it says nothing at all — ADR 0360's argument for the
//! caller sweep, and ADR 0388's for the seventh.
//!
//! **And widening the word list is not the answer**, which the four-hundred-and-thirty-seventh's
//! own run established: it makes the count smaller and leaves §9.3.1's inverse — a row that names
//! a debt and is *wrong* about it, and is therefore invisible to any instrument that only asks
//! whether a debt is named — exactly where it was.
//!
//! # The discriminator, and it is the seventh sweep's with the sign reversed
//!
//! [`crate::inapplicable`] replaced a stop-list with a **count taken from the tree**: how many
//! sources name a term the row itself states. This sweep asks the same measurement of the same
//! extraction and reads the other end of it.
//!
//! | | population | the term that matters |
//! |---|---|---|
//! | seventh sweep | `inapplicable` — the row claims the tree does not do this | one the tree **names** |
//! | this sweep | `partial` — the row must say what is *not* done | one the tree **lacks** |
//!
//! A note that names a debt names a *thing*: an entry nothing reads, a function nobody wrote, a
//! type this tree has no equivalent of. That thing has a name, and the name is absent from
//! `crates/`, `tools/` and `fuzz/` — which is a count rather than a guess, and moves with the
//! ledger and the tree instead of with the session. So the reading list is every `partial` row
//! **none of whose own vocabulary the tree lacks**: a row all of whose named things this tree
//! already has, sitting above a status that says something is missing.
//!
//! **And the same count orders them**, which is the seventh sweep's second rule read the same way
//! round. There a term one file names sorts first, because a rare word under a row claiming
//! absence is where the defects are. Here a row is ranked by its **rarest** term's reach, commonest
//! first: a row whose most distinctive word is still named by a hundred files has named no *thing*
//! at all, and a row stating no name whatever — §8.5.3.3.1's is the extreme case — heads the list.
//!
//! # The noise, printed rather than filtered
//!
//! - **A debt named in prose rather than by name.** "Conditional on importing the target page",
//!   "defers each to a subclause", "the vertical branch of the displacement formula" — a real
//!   debt with no identifier in it. This is the same noise the hand-runs recorded, and it is not
//!   removable by any word list either: what a program can settle is whether a *name* is missing,
//!   never whether a sentence means one is.
//! - **One short key, three clauses.** `doc/todo/01`'s oldest false positive. A key the tree
//!   names under another table reads as present, so the row loses a term that was a real debt.
//! - **A row whose note narrates its own correction** reproduces the retired vocabulary, which
//!   every sweep in `doc/todo/01` has.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, unchanged, and this sweep is where `doc/todo/01` states it most
//! plainly: **it is a reading list and not a gate.** Half the `partial` population lands on it,
//! most of them rows that name their debt in prose, and no tighter predicate separates those from
//! the rows that name nothing without deciding what an English sentence means.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::clause::ClauseNumber;
use crate::inapplicable::{self, Kind};
use crate::ledger::{Ledger, Status};

/// How many witnessing files one named term reports.
pub const WITNESSES: usize = 2;

/// How many of a row's terms one block prints.
///
/// A `partial` note runs to a paragraph and states dozens of names; what a reader needs is enough
/// of them to see what kind of vocabulary the row is made of.
pub const SHOWN: usize = 4;

/// One term out of a `partial` row's own title and note, with what the tree says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    /// The word itself, without a solidus.
    pub text: String,
    /// How it was written where the row states it.
    pub kind: Kind,
    /// How many Rust sources name it. Zero is the row naming a debt by name.
    pub reach: usize,
    /// Up to [`WITNESSES`] of the naming files, in the order the scan reads them.
    pub witnesses: Vec<String>,
}

/// One `partial` row whose vocabulary the tree names in full.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The clause whose row it is.
    pub clause: ClauseNumber,
    /// The 1-based line of the ledger it starts on.
    pub line: usize,
    /// The standard's title for the clause.
    pub title: String,
    /// Every term the row states, in the order it states them.
    pub terms: Vec<Term>,
}

impl Finding {
    /// How many terms the row states.
    #[must_use]
    pub fn stated(&self) -> usize {
        self.terms.len()
    }

    /// The reach of this row's rarest term, and [`usize::MAX`] where it states none.
    ///
    /// **What the rows are ordered by, commonest first.** A row all of whose words a hundred
    /// files carry has named nothing this tree could be missing; a row naming something one file
    /// carries has named a thing, and the thing is here. A row stating no name at all is the
    /// extreme of the first, which is why an absent minimum is the largest value there is rather
    /// than the smallest.
    #[must_use]
    pub fn rarest(&self) -> usize {
        self.terms
            .iter()
            .map(|term| term.reach)
            .min()
            .unwrap_or(usize::MAX)
    }
}

/// What one run of the sweep read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many `partial` rows the ledger holds.
    pub population: usize,
    /// How many terms those rows state, counted with repeats across rows.
    pub stated: usize,
    /// How many stated terms no Rust source names — a debt named by name.
    pub absent: usize,
    /// How many rows state at least one absent term, and so say what is owed in a word.
    pub naming: usize,
    /// The rows whose every stated term the tree names, least vocabulary first.
    pub findings: Vec<Finding>,
}

/// Runs the sweep over the ledger's `partial` rows and the tree's sources.
///
/// The checker's own directory witnesses nothing, for [`crate::NOT_SCANNED`]'s reasons and this
/// module's own: the documentation above names the rows the sweep exists to find.
#[must_use]
pub fn sweep(ledger: &Ledger, sources: &[(PathBuf, String)]) -> Report {
    let scanned: Vec<(String, &String)> = sources
        .iter()
        .map(|(path, text)| (path.to_string_lossy().replace('\\', "/"), text))
        .filter(|(path, _)| !path.starts_with(crate::NOT_SCANNED))
        .collect();
    // One term recurs across dozens of rows — `/Subtype` and `Table` are in half the ledger — and
    // the reach of a term is a property of the tree rather than of the row that states it.
    let mut measured: BTreeMap<(String, Kind), (usize, Vec<String>)> = BTreeMap::new();
    let mut report = Report::default();
    for row in &ledger.rows {
        if row.status != Status::Partial {
            continue;
        }
        report.population = report.population.saturating_add(1);
        let mut terms = Vec::new();
        let mut absent_here = 0usize;
        for (text, kind) in inapplicable::terms_of(row) {
            let (reach, witnesses) = measured
                .entry((text.clone(), kind))
                .or_insert_with(|| named_by(&text, kind, &scanned))
                .clone();
            report.stated = report.stated.saturating_add(1);
            if reach == 0 {
                report.absent = report.absent.saturating_add(1);
                absent_here = absent_here.saturating_add(1);
            }
            terms.push(Term {
                text,
                kind,
                reach,
                witnesses,
            });
        }
        if absent_here > 0 {
            report.naming = report.naming.saturating_add(1);
            continue;
        }
        terms.sort_by_key(|term| term.reach);
        report.findings.push(Finding {
            clause: row.clause.clone(),
            line: row.line,
            title: row.title.clone(),
            terms,
        });
    }
    report
        .findings
        .sort_by_key(|finding| (std::cmp::Reverse(finding.rarest()), finding.stated()));
    report
}

/// How many sources name a term, and up to [`WITNESSES`] of them.
fn named_by(text: &str, kind: Kind, sources: &[(String, &String)]) -> (usize, Vec<String>) {
    let mut reach = 0usize;
    let mut witnesses = Vec::new();
    for (path, source) in sources {
        if !inapplicable::names(source, text, kind) {
            continue;
        }
        reach = reach.saturating_add(1);
        if witnesses.len() < WITNESSES {
            witnesses.push(path.clone());
        }
    }
    (reach, witnesses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Row;

    fn row(clause: &str, status: Status, title: &str, note: &str) -> Row {
        Row {
            clause: clause.parse().expect("a clause number"),
            title: title.to_owned(),
            status,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    fn source(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    /// Only `partial` rows are the population: this sweep is about the status whose own
    /// definition promises a debt.
    #[test]
    fn only_partial_rows_are_swept() {
        let ledger = Ledger {
            rows: vec![
                row("9.3", Status::Partial, "state", "only /TK is drawn."),
                row(
                    "9.3.1",
                    Status::Implemented,
                    "general",
                    "only /TK is drawn.",
                ),
                row("9.3.8", Status::Reported, "knockout", "only /TK is drawn."),
            ],
        };
        let report = sweep(&ledger, &[source("crates/a.rs", "\"TK\"\n")]);
        assert_eq!(report.population, 1);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].clause.to_string(), "9.3");
    }

    /// A term the tree does not name is the row saying what is owed in a word, so the row is not
    /// on the reading list and is counted instead.
    #[test]
    fn a_term_the_tree_lacks_takes_the_row_off_the_list() {
        let ledger = Ledger {
            rows: vec![row(
                "8.6.6",
                Status::Partial,
                "special colour spaces",
                "what nothing reads is /NChannel.",
            )],
        };
        let report = sweep(&ledger, &[source("crates/a.rs", "let colourants = 1;\n")]);
        assert!(report.findings.is_empty());
        assert_eq!(report.absent, 1);
        assert_eq!(report.naming, 1);
    }

    /// A row whose every named thing the tree already has is the reading list, and the terms come
    /// back with the count that put it there.
    #[test]
    fn a_row_the_tree_names_in_full_is_the_finding() {
        let ledger = Ledger {
            rows: vec![row(
                "9.3",
                Status::Partial,
                "text state parameters",
                "eight of the nine are implemented; the ninth is /TK.",
            )],
        };
        let sources = vec![
            source("crates/a.rs", "get_key(dict, \"TK\")\n"),
            source("crates/b.rs", "// the /TK of it\n"),
        ];
        let report = sweep(&ledger, &sources);
        assert_eq!(report.findings.len(), 1);
        let found = report.findings[0]
            .terms
            .iter()
            .find(|term| term.text == "TK")
            .expect("the row states /TK");
        assert_eq!(found.reach, 2);
        assert_eq!(found.witnesses.len(), WITNESSES);
        assert_eq!(report.absent, 0);
        assert_eq!(report.naming, 0);
    }

    /// The row whose rarest term is commonest comes first, and a row stating no name at all heads
    /// the list: neither has named a thing this tree could be missing.
    #[test]
    fn the_row_naming_nothing_specific_comes_first() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "12.5.6",
                    Status::Partial,
                    "annotation types",
                    "a Popup is drawn, and STANDARD_SUBTYPES holds it.",
                ),
                row(
                    "7.7",
                    Status::Partial,
                    "document structure",
                    "the trailer's Popup, whatever that means.",
                ),
                row("8.5.3.3.1", Status::Partial, "general", "the f operator."),
            ],
        };
        let sources = vec![
            source("crates/a.rs", "Popup STANDARD_SUBTYPES\n"),
            source("crates/b.rs", "Popup\n"),
        ];
        let report = sweep(&ledger, &sources);
        let order: Vec<String> = report
            .findings
            .iter()
            .map(|finding| finding.clause.to_string())
            .collect();
        assert_eq!(order, vec!["8.5.3.3.1", "7.7", "12.5.6"]);
        assert_eq!(report.findings[0].rarest(), usize::MAX, "it states no name");
        assert_eq!(report.findings[1].rarest(), 2, "`Popup`, in both sources");
        assert_eq!(
            report.findings[2].terms[0].text, "STANDARD_SUBTYPES",
            "a row's own terms are printed rarest first"
        );
    }

    /// A term is measured once however many rows state it: reach is a property of the tree.
    #[test]
    fn a_term_two_rows_state_is_measured_once() {
        let ledger = Ledger {
            rows: vec![
                row("8.9.5.1", Status::Partial, "general", "a Widget again."),
                row("12.5.6", Status::Partial, "types", "a Widget once more."),
            ],
        };
        let report = sweep(&ledger, &[source("crates/a.rs", "Widget\n")]);
        assert_eq!(report.stated, 2, "the count is of statements, not of terms");
        assert!(
            report
                .findings
                .iter()
                .all(|finding| finding.terms.iter().all(|term| term.reach == 1))
        );
    }

    /// The checker's own directory witnesses nothing: this module's documentation names the very
    /// rows the sweep exists to find.
    #[test]
    fn the_checkers_own_documentation_is_not_a_witness() {
        let ledger = Ledger {
            rows: vec![row(
                "8.6.6",
                Status::Partial,
                "special colour spaces",
                "nothing reads /NChannel.",
            )],
        };
        let sources = vec![source(
            "tools/conformance/src/owed.rs",
            "//! /NChannel is the standing example.\n",
        )];
        assert!(sweep(&ledger, &sources).findings.is_empty());
    }
}
