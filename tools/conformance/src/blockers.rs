//! The first sweep: a reason that names a blocker, checked against the ledger's own account
//! of it.
//!
//! # The shape it exists for
//!
//! A ledger note that explains a debt by pointing at something else — "while §11.4.6 does not
//! exist", "needs §12.10.3's external references", "waits on printing" — gives a *trigger*, and
//! nothing fires when the trigger trips. §9.7.5.2's row said vendoring the predefined `CMap`s
//! was "a licensing decision" for a hundred and fifty sessions after the decision was taken;
//! §11.3.7.2 said a group's shape "needs §11.4.6" for eighty sessions after §11.4.6 was built
//! (ADRs 0107, 0108, 0140). The sweep has run as a grep since the hundred-and-eighteenth
//! session; this is the same instrument as a program, per `doc/todo/01`'s binding rule that a
//! sweep round commits one before running any (ADR 0319 records what a description costs).
//!
//! # What a program can settle that the grep could not
//!
//! Where the blocking thing is a *clause*, the ledger itself knows whether it arrived: a
//! sentence blocking on §11.4.6 while §11.4.6's own row is settled is a blocker that has, on
//! the ledger's own account, [expired](Standing::Expired). A blocker on a clause that still
//! owes something [holds](Standing::Holds), and a sentence naming no clause at all — a
//! capability, a dependency's branch, a decision — is [unjudged](Standing::Unjudged) and is
//! the part the reading still owns.
//!
//! # The three false-positive shapes, printed rather than filtered
//!
//! All are `doc/todo/01`'s findings about this sweep and none shrinks under a tighter grep.
//! **A correction quoting the wording it retired** reproduces the blocker inside a history
//! sentence — §11.3.7.2's note both quotes "needs §11.4.6" and explains why it was wrong — so
//! a sentence that reads like history is marked [`Hit::history`] and still printed. **A sweep
//! for a blocker cannot see a tense**: "were blocked on one small piece of clause 7" matches
//! the same phrase in the past tense, which only reading catches. And — the program's own
//! first run — **a clause can be named as the route to something outside the standard**:
//! §12.10.2's "needs §12.10.3's external references" waits on the EPSG registry and ISO 19162
//! that §12.10.3 *points at*, so §12.10.3's own settled row settles nothing about the wait,
//! and the [`Standing::Expired`] label there is the instrument misattributing the blocker. A
//! hit is a reading list, not a verdict.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, the same as every sweep here: the noise population is legitimate
//! prose and does not shrink under a tighter predicate. It runs in seconds and its output is
//! read.

use std::fmt;
use std::path::PathBuf;

use crate::clause::ClauseNumber;
use crate::ledger::{Ledger, Status};

/// The phrases that make a sentence a blocker claim.
///
/// Lower-cased substrings, matched against one sentence at a time. `doc/todo/01`'s lesson on
/// the second sweep — grep the shape, not the wording — applies: these are every way this
/// ledger and this tree have written "X cannot happen until Y does".
pub const CLAIMS: [&str; 7] = [
    "while §",
    "until §",
    "needs §",
    "waits on",
    "waiting on",
    "blocked on",
    "blocked by",
];

/// The phrases that make a claim sentence read like history rather than like a reason.
///
/// A correction quoting the wording it retired is this sweep's oldest false-positive shape,
/// and it is marked rather than filtered: a note that *only* looks like history is exactly the
/// place a live blocker has hidden before (§12.5.6.4's popup, ADR 0294).
pub const HISTORY: [&str; 6] = [
    "used to",
    "said",
    "was corrected",
    "retired",
    "stood",
    "this row",
];

/// What the ledger's own account says about one blocker sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Every clause the sentence names has a settled row — implemented, inapplicable,
    /// out-of-scope or writer-side — so the stated blocker has expired on the ledger's own
    /// account. The list to read first.
    Expired,
    /// At least one named clause still owes something (`partial`, `reported`, `silent` or
    /// `unreviewed`), so the blocker holds today.
    Holds,
    /// The sentence names no clause the ledger has a row for: the blocker is a capability, a
    /// dependency or a decision, which only reading can date.
    Unjudged,
}

impl fmt::Display for Standing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Expired => "EXPIRED by the ledger's own account",
            Self::Holds => "holds",
            Self::Unjudged => "names no clause — read it",
        })
    }
}

/// One blocker sentence, wherever it was found.
#[derive(Debug, Clone)]
pub struct Hit {
    /// Where it is: `doc/conformance/ledger.toml:123 (§11.7.4.4, partial)` or
    /// `crates/pdf-model/src/lib.rs:88`.
    pub location: String,
    /// The claiming sentence, whole.
    pub sentence: String,
    /// Each clause the sentence names, with its ledger status where a row exists.
    pub named: Vec<(ClauseNumber, Option<Status>)>,
    /// What the ledger says about the blocker.
    pub standing: Standing,
    /// Whether the sentence also reads like a correction's history — the known
    /// quoted-retired-wording shape.
    pub history: bool,
}

/// What one run of the sweep read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Hits in the ledger's notes.
    pub ledger: Vec<Hit>,
    /// Hits in comments under [`crate::SOURCE_ROOTS`].
    pub source: Vec<Hit>,
}

impl Report {
    /// How many hits in one population carry the given standing.
    #[must_use]
    pub fn count(hits: &[Hit], standing: Standing) -> usize {
        hits.iter().filter(|hit| hit.standing == standing).count()
    }
}

/// Runs the sweep over the ledger's notes and the tree's comments.
///
/// The sources are [`crate::entries::sources`]', minus [`crate::NOT_SCANNED`]: this crate's
/// own documentation quotes the claim phrases as examples — this module does, twice — and a
/// sweep that reports itself is the fastest way to have one switched off.
#[must_use]
pub fn sweep(ledger: &Ledger, sources: &[(PathBuf, String)]) -> Report {
    let mut report = Report::default();
    for row in &ledger.rows {
        let Some(note) = row.note.as_deref() else {
            continue;
        };
        for sentence in crate::unread::sentences(note) {
            if let Some(hit) = judge(sentence, ledger) {
                report.ledger.push(Hit {
                    location: format!(
                        "{}:{} (§{}, {})",
                        crate::LEDGER,
                        row.line,
                        row.clause,
                        row.status.as_str()
                    ),
                    ..hit
                });
            }
        }
    }
    for (path, text) in sources {
        let shown = path.to_string_lossy().replace('\\', "/");
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        for (line, block) in comment_blocks(text) {
            for sentence in crate::unread::sentences(&block) {
                if let Some(hit) = judge(sentence, ledger) {
                    report.source.push(Hit {
                        location: format!("{shown}:{line}"),
                        ..hit
                    });
                }
            }
        }
    }
    report
}

/// Judges one sentence: a [`Hit`] with an empty location if it claims a blocker, else `None`.
fn judge(sentence: &str, ledger: &Ledger) -> Option<Hit> {
    let lowered = sentence.to_ascii_lowercase();
    if !CLAIMS.iter().any(|phrase| lowered.contains(phrase)) {
        return None;
    }
    let named: Vec<(ClauseNumber, Option<Status>)> = clauses_in(sentence)
        .into_iter()
        .map(|clause| {
            let status = ledger
                .rows
                .iter()
                .find(|row| row.clause == clause)
                .map(|row| row.status);
            (clause, status)
        })
        .collect();
    let with_rows: Vec<Status> = named.iter().filter_map(|(_, status)| *status).collect();
    let standing = if with_rows.is_empty() {
        Standing::Unjudged
    } else if with_rows.iter().all(|status| {
        matches!(
            status,
            Status::Implemented | Status::Inapplicable | Status::OutOfScope | Status::WriterSide
        )
    }) {
        Standing::Expired
    } else {
        Standing::Holds
    };
    Some(Hit {
        location: String::new(),
        sentence: sentence.to_owned(),
        named,
        standing,
        history: HISTORY.iter().any(|phrase| lowered.contains(phrase)),
    })
}

/// The clause numbers a sentence cites with a `§`.
///
/// A number is the SECTION SIGN's run of digits and full stops, trimmed of the full stop that
/// ends a sentence; a possessive (`§11.4.6's`) ends it like any other non-numeric character.
/// An annex citation (`§K.2`) is one upper-case letter before the digits.
#[must_use]
pub fn clauses_in(sentence: &str) -> Vec<ClauseNumber> {
    let mut clauses = Vec::new();
    for (index, character) in sentence.char_indices() {
        if character != '§' {
            continue;
        }
        let rest = &sentence[index.saturating_add(character.len_utf8())..];
        let mut end = 0usize;
        for (offset, candidate) in rest.char_indices() {
            let first_annex_letter = offset == 0 && candidate.is_ascii_uppercase();
            if candidate.is_ascii_digit() || candidate == '.' || first_annex_letter {
                end = offset.saturating_add(candidate.len_utf8());
            } else {
                break;
            }
        }
        let text = rest[..end].trim_end_matches('.');
        if let Ok(clause) = text.parse::<ClauseNumber>()
            && !clauses.contains(&clause)
        {
            clauses.push(clause);
        }
    }
    clauses
}

/// A file's comment blocks: consecutive comment lines joined into one text, with the 1-based
/// line the block starts on.
///
/// A blocker sentence is prose and prose wraps, so matching line by line would miss every
/// claim a rustfmt width splits — "needs" on one line, "§11.4.6" on the next. Only the text
/// after the `//` marker counts: a string literal quoting a phrase is data, not a claim this
/// tree makes about itself.
#[must_use]
pub fn comment_blocks(text: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (index, line) in text.lines().enumerate() {
        let comment = line.find("//").map(|at| {
            line[at..]
                .trim_start_matches('/')
                .trim_start_matches('!')
                .trim()
        });
        match (comment, &mut current) {
            (Some(text), Some((_, block))) => {
                block.push(' ');
                block.push_str(text);
            }
            (Some(text), None) => {
                current = Some((index.saturating_add(1), text.to_owned()));
            }
            (None, Some(_)) => {
                if let Some(block) = current.take() {
                    blocks.push(block);
                }
            }
            (None, None) => {}
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Row;

    fn row(clause: &str, status: Status, note: Option<&str>) -> Row {
        Row {
            clause: clause.parse().expect("a clause number"),
            title: String::new(),
            status,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: note.map(str::to_owned),
            line: 1,
        }
    }

    /// Every spelling this ledger has used for "X cannot happen until Y does" is a claim, and
    /// a sentence about what a clause *says* is not.
    #[test]
    fn every_spelling_of_the_claim_is_one() {
        let ledger = Ledger { rows: Vec::new() };
        assert!(judge("The shape needs §11.4.6.", &ledger).is_some());
        assert!(judge("Refused until §12.10.3 exists.", &ledger).is_some());
        assert!(judge("True while §10.5 does not exist.", &ledger).is_some());
        assert!(judge("What it waits on is printing.", &ledger).is_some());
        assert!(judge("Blocked on the decoder's API.", &ledger).is_some());
        assert!(judge("Both halves are implemented, per §11.4.6.", &ledger).is_none());
    }

    /// A blocker naming a clause whose own row is settled has expired on the ledger's own
    /// account — the finding this sweep exists for (ADR 0108's shape).
    #[test]
    fn a_blocker_naming_a_settled_clause_is_expired() {
        let ledger = Ledger {
            rows: vec![
                row("11.4.6", Status::Implemented, None),
                row(
                    "11.3.7.2",
                    Status::Partial,
                    Some("The group's shape needs §11.4.6."),
                ),
            ],
        };
        let report = sweep(&ledger, &[]);
        assert_eq!(report.ledger.len(), 1);
        let hit = report.ledger.first().expect("one hit");
        assert_eq!(hit.standing, Standing::Expired);
        assert_eq!(
            hit.named,
            vec![(
                "11.4.6".parse().expect("a clause number"),
                Some(Status::Implemented)
            )]
        );
        assert!(!hit.history);
    }

    /// A blocker naming a clause that still owes something holds today, which is a result
    /// rather than a silence.
    #[test]
    fn a_blocker_naming_an_owing_clause_holds() {
        let ledger = Ledger {
            rows: vec![
                row("12.10.3", Status::Reported, None),
                row(
                    "12.10.2",
                    Status::Partial,
                    Some("The rest needs §12.10.3's external references."),
                ),
            ],
        };
        let report = sweep(&ledger, &[]);
        assert_eq!(report.ledger.len(), 1);
        assert_eq!(
            report.ledger.first().expect("one hit").standing,
            Standing::Holds
        );
    }

    /// A sentence naming no clause blocks on a capability or a decision, which only reading
    /// can date — the sweep says so instead of guessing.
    #[test]
    fn a_blocker_naming_no_clause_is_left_to_the_reader() {
        let ledger = Ledger {
            rows: vec![row(
                "12.5.6.22",
                Status::Partial,
                Some("What /FixedPrint waits on is printing."),
            )],
        };
        let report = sweep(&ledger, &[]);
        assert_eq!(
            report.ledger.first().expect("one hit").standing,
            Standing::Unjudged
        );
    }

    /// A correction quoting the wording it retired is the known false-positive shape, marked
    /// rather than filtered.
    #[test]
    fn a_correction_quoting_its_retired_wording_is_marked_as_history() {
        let ledger = Ledger {
            rows: vec![
                row("11.4.6", Status::Implemented, None),
                row(
                    "11.3.7.2",
                    Status::Partial,
                    Some("This row said the shape 'needs §11.4.6' until the seventy-first."),
                ),
            ],
        };
        let report = sweep(&ledger, &[]);
        assert!(report.ledger.first().expect("one hit").history);
    }

    /// A possessive ends a clause number and a sentence's own full stop is not part of one;
    /// an annex letter is.
    #[test]
    fn clause_numbers_are_cut_where_the_prose_resumes() {
        let clauses = clauses_in("needs §11.4.6's knockout groups, §O.2 and §12.10.3.");
        assert_eq!(
            clauses,
            vec![
                "11.4.6".parse().expect("a clause number"),
                "O.2".parse().expect("an annex number"),
                "12.10.3".parse().expect("a clause number"),
            ]
        );
    }

    /// A claim rustfmt wrapped across comment lines is still one sentence, and its hit names
    /// the line the block starts on.
    #[test]
    fn a_wrapped_comment_is_read_as_one_block() {
        let ledger = Ledger {
            rows: vec![row("11.4.6", Status::Implemented, None)],
        };
        let source = "fn a() {}\n// The knockout shape needs\n// §11.4.6 before it can land.\n";
        let report = sweep(
            &ledger,
            &[(PathBuf::from("crates/x/src/lib.rs"), source.into())],
        );
        assert_eq!(report.source.len(), 1);
        let hit = report.source.first().expect("one hit");
        assert_eq!(hit.location, "crates/x/src/lib.rs:2");
        assert_eq!(hit.standing, Standing::Expired);
    }

    /// The checker's own directory quotes the claim phrases as examples and is not scanned —
    /// a sweep that reports itself is a sweep somebody switches off.
    #[test]
    fn the_checkers_own_documentation_is_not_a_finding() {
        let ledger = Ledger { rows: Vec::new() };
        let source = "// A note saying it needs §11.4.6 is the shape this sweep hunts.\n";
        let report = sweep(
            &ledger,
            &[(
                PathBuf::from("tools/conformance/src/blockers.rs"),
                source.into(),
            )],
        );
        assert!(report.source.is_empty());
    }
}
