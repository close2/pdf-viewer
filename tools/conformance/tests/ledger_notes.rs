//! The ledger's notes state what is, and an `out-of-scope` row says which exclusion and why.
//!
//! # A note is not a record
//!
//! `CLAUDE.md`'s comment rule — a comment carries the current reason and nothing about how it got
//! there — binds the ledger's notes as well as the code's comments (ADR 1547). The ledger is the
//! *current* status of each clause with its reasoning, read by every round that touches the
//! clause; `doc/adr/` and `doc/history/` are where the chronology is kept. A session ordinal in a
//! note is the counted fact ADR 0281 keeps out of every live document, and a retired sentence kept
//! beside its retirement makes the reading cost of a clause its history.
//!
//! The notes were written the other way for a long time, so the rule comes off them one clause
//! family at a time rather than in one sweep that would lose an argument. This file is what keeps
//! the direction: the count of session ordinals in the ledger is held by equality, so a row may
//! not add one and a round that removes some lowers the bound in the same pass.
//!
//! # What an ordinal is here
//!
//! A hyphenated number word that holds `hundred` or `thousand` and ends as an ordinal does — in
//! `first`, `second`, `third` or `th` — which is the only form a session number past the
//! ninety-ninth takes in this prose, and the word `session` or `round` followed by digits. A bare `hundredth` or `thousandth` is a fraction in these notes, never a
//! session, and is not counted. A small ordinal before the word `session` is left to the reader:
//! the population is the large one.
//!
//! # The second test
//!
//! An `out-of-scope` row rests on one of `CLAUDE.md` principle 5's closed list of exclusions, and
//! the `exclusion` field already names which. What the field cannot say is why the clause falls
//! under it. So the note names the exclusion in prose and quotes, verbatim, a sentence of the
//! clause that puts it there — the discipline ADR 1535 gave `inapplicable` rows, with the same
//! [`conformance::ledger::grounding`] reading the quotation. A heading the standard gives no text
//! of its own has no sentence to quote, and says so; that claim is checked against `doc/md/`
//! rather than believed (ADR 1548).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use conformance::clause::ClauseIndex;
use conformance::ledger::{Exclusion, Grounding, Ledger, Status};

/// The session ordinals the ledger's notes may still carry.
///
/// Held by equality: a count above it is a note written with its history in it, and a count below
/// it is a round that took some out and owes the lower number here. The clause families still
/// carrying them are printed on every run, largest first, so the next round knows where to start.
const ORDINALS_IN_THE_LEDGER: usize = 1278;

/// Whether `word`, a maximal run of lowercase letters and hyphens, is a session ordinal.
fn is_spelled_ordinal(word: &str) -> bool {
    word.contains('-')
        && (word.contains("hundred") || word.contains("thousand"))
        && ["first", "second", "third", "th"]
            .iter()
            .any(|ending| word.ends_with(ending))
}

/// The session ordinals in `text`: spelled ones, and `session <digits>` or `round <digits>`.
fn ordinals(text: &str) -> usize {
    let spelled = text
        .split(|c: char| !(c.is_ascii_lowercase() || c == '-'))
        .filter(|word| is_spelled_ordinal(word))
        .count();
    let words: Vec<&str> = text.split_whitespace().collect();
    let numbered = words
        .windows(2)
        .filter(|pair| {
            let lead = pair[0].to_ascii_lowercase();
            let digits = pair[1].trim_end_matches(|c: char| !c.is_ascii_digit());
            matches!(lead.as_str(), "session" | "sessions" | "round" | "rounds")
                && !digits.is_empty()
                && digits.chars().all(|c| c.is_ascii_digit())
        })
        .count();
    spelled.saturating_add(numbered)
}

#[test]
fn the_ledgers_session_ordinals_only_fall() {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER)).expect("the ledger");

    let mut by_family: BTreeMap<String, usize> = BTreeMap::new();
    for row in &ledger.rows {
        let Some(note) = &row.note else { continue };
        let count = ordinals(note);
        if count > 0 {
            let family = row.clause.to_string();
            let family = family.split('.').next().unwrap_or_default().to_owned();
            *by_family.entry(family).or_default() += count;
        }
    }
    let total: usize = by_family.values().sum();
    let mut families: Vec<(&String, &usize)> = by_family.iter().collect();
    families.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
    println!("{total} session ordinals in the ledger's notes (bound {ORDINALS_IN_THE_LEDGER}):");
    for (family, count) in &families {
        println!("  {count:5}  clause {family}");
    }

    assert!(
        total <= ORDINALS_IN_THE_LEDGER,
        "the ledger's notes carry {total} session ordinals against a bound of \
         {ORDINALS_IN_THE_LEDGER}: a note states the clause as it is and cites the ADR, never the \
         session that changed it (ADR 1547)"
    );
    assert!(
        total >= ORDINALS_IN_THE_LEDGER,
        "the ledger's notes carry {total} session ordinals and the bound is still \
         {ORDINALS_IN_THE_LEDGER}: lower `ORDINALS_IN_THE_LEDGER` to {total} in this pass, so the \
         ones taken out cannot come back"
    );
}

/// The words a note uses to name each exclusion, as `CLAUDE.md` principle 5 states them.
fn named_in_prose(exclusion: Exclusion) -> &'static str {
    match exclusion {
        Exclusion::Multimedia => "clause 13",
        Exclusion::Xfa => "XFA exclusion",
        Exclusion::Script => "JavaScript and script-driven form behaviour",
        Exclusion::WriterSide => "writer-side",
    }
}

/// What a note says when its clause is a heading with no text of its own.
const HEADING: &str = "A heading with no text of its own";

/// Whether the standard prints nothing under `heading` before its first subclause.
fn states_nothing(index: &ClauseIndex, position: usize) -> bool {
    let headings = index.headings();
    let Some(heading) = headings.get(position) else {
        return false;
    };
    let end = headings
        .get(position.saturating_add(1))
        .filter(|next| heading.number.is_ancestor_of(&next.number))
        .map_or(heading.span.end, |next| next.span.start);
    let own = index.text_in(heading.span.start..end);
    own.lines().skip(1).all(|line| line.trim().is_empty())
}

#[test]
fn every_out_of_scope_row_names_its_exclusion_and_quotes_its_clause() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let ledger = Ledger::read(&root.join(conformance::LEDGER)).expect("the ledger");

    let mut wrong = String::new();
    let mut rows = 0usize;
    let mut headings = 0usize;
    for row in ledger
        .rows
        .iter()
        .filter(|row| row.status == Status::OutOfScope)
    {
        rows = rows.saturating_add(1);
        let note = row.note.as_deref().unwrap_or_default();
        let Some(exclusion) = row.exclusion else {
            continue; // `the_ledger_agrees_with_the_standard_and_with_the_tree` reports that.
        };
        if !(note.contains("principle 5") && note.contains(named_in_prose(exclusion))) {
            let _ = writeln!(
                wrong,
                "§{}: the note does not name principle 5's exclusion in words (`{}`)",
                row.clause,
                named_in_prose(exclusion)
            );
        }
        if note.starts_with(HEADING) {
            let position = index
                .headings()
                .iter()
                .position(|heading| heading.number == row.clause);
            if position.is_some_and(|position| states_nothing(&index, position)) {
                headings = headings.saturating_add(1);
            } else {
                let _ = writeln!(
                    wrong,
                    "§{}: the note says the clause has no text of its own, and doc/md/ prints \
                     some — quote it",
                    row.clause
                );
            }
            continue;
        }
        let grounding = conformance::ledger::grounding(row, &index);
        if grounding != Grounding::Quoted {
            let _ = writeln!(
                wrong,
                "§{}: the note {grounding} — quote the clause's own sentence that puts it under \
                 the exclusion",
                row.clause
            );
        }
    }

    assert!(wrong.is_empty(), "\n{wrong}");
    println!(
        "{rows} out-of-scope rows, each naming its exclusion; {} quote their clause and \
         {headings} are headings with no text of their own",
        rows.saturating_sub(headings)
    );
}
