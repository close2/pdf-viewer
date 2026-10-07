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
//! No note carries a session ordinal, and this file holds the count at zero with `==`, as
//! `CONDITION_UNQUOTED_CEILING` is held (ADR 1535): a row may not add one, and the shapes counted
//! are every shape the notes were found to use (ADR 1610).
//!
//! # What an ordinal is here
//!
//! Four shapes, each a session named or counted:
//!
//! - a hyphenated number word that holds `hundred` or `thousand` and ends as an ordinal does — in
//!   `first`, `second`, `third` or `th` — which is the form a session number past the ninety-ninth
//!   takes in this prose. A bare `hundredth` or `thousandth` is a fraction in these notes, never a
//!   session, and is not counted;
//! - the word `session` or `round` followed by digits;
//! - any other spelled ordinal followed by the word `session` — "the twenty-seventh session", "the
//!   fifth session". Without the noun a small ordinal is a fraction ("a twentieth of a pixel"), a
//!   position ("the twenty-eighth page") or a sweep's name, so the noun is what makes it a session;
//! - a number followed by `sessions` — "for eighteen sessions" — which counts sessions rather than
//!   naming one, and is the same counted fact (ADR 1610).
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

/// The session ordinals the ledger's notes may carry, which is none.
///
/// Held by equality, so that a ceiling raised to admit one is a visible edit to this line rather
/// than a note slipping past it. A failing run prints the families that carry one, largest first.
const ORDINALS_IN_THE_LEDGER: usize = 0;

/// Whether `word`, a maximal run of lowercase letters and hyphens, is a session ordinal.
fn is_spelled_ordinal(word: &str) -> bool {
    word.contains('-')
        && (word.contains("hundred") || word.contains("thousand"))
        && ["first", "second", "third", "th"]
            .iter()
            .any(|ending| word.ends_with(ending))
}

/// The units a spelled ordinal below a hundred ends in.
const ORDINAL_UNITS: [&str; 27] = [
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
    "twentieth",
    "thirtieth",
    "fortieth",
    "fiftieth",
    "sixtieth",
    "seventieth",
    "eightieth",
    "ninetieth",
];

/// The number words a spelled cardinal below a hundred is made of.
const CARDINAL_PARTS: [&str; 27] = [
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
];

/// `word` with the emphasis, brackets and punctuation around it taken off, in lowercase.
fn bare(word: &str) -> String {
    word.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .to_ascii_lowercase()
}

/// Whether `word` is a spelled ordinal below a hundred: `fifth`, `twenty-seventh`.
fn is_small_ordinal(word: &str) -> bool {
    let mut parts = word.rsplit('-');
    let unit = parts.next().unwrap_or_default();
    ORDINAL_UNITS.contains(&unit) && parts.all(|part| CARDINAL_PARTS.contains(&part))
}

/// Whether `word` is a number: digits, or a spelled cardinal such as `seventy-one` or `hundred`.
fn is_cardinal(word: &str) -> bool {
    let spelled = |part: &str| {
        CARDINAL_PARTS.contains(&part) || matches!(part, "hundred" | "thousand" | "and")
    };
    !word.is_empty()
        && !word.starts_with("and")
        && (word.chars().all(|c| c.is_ascii_digit()) || word.split('-').all(spelled))
}

/// Whether `word` is the noun `session`, singular or possessive, or `sessions` when `plural`.
fn is_session(word: &str, plural: bool) -> bool {
    let word = word.to_ascii_lowercase();
    let word = word.trim_start_matches(|c: char| !c.is_ascii_alphabetic());
    let noun = word.trim_end_matches(|c: char| !c.is_ascii_alphabetic());
    if plural {
        noun == "sessions"
    } else {
        noun == "session" || noun == "sessions" || word.starts_with("session's")
    }
}

/// The session ordinals in `text`, in the four shapes this file's comment names.
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
    let small = words
        .windows(2)
        .filter(|pair| is_small_ordinal(&bare(pair[0])) && is_session(pair[1], false))
        .count();
    let counted = words
        .windows(2)
        .filter(|pair| is_cardinal(&bare(pair[0])) && is_session(pair[1], true))
        .count();
    spelled
        .saturating_add(numbered)
        .saturating_add(small)
        .saturating_add(counted)
}

#[test]
fn each_shape_of_ordinal_is_counted_and_a_fraction_is_not() {
    assert_eq!(ordinals("Both, as of the twenty-seventh session; and"), 1);
    assert_eq!(ordinals("reviewed in the **fifth session**, and"), 1);
    assert_eq!(ordinals("the nineteenth session's reading"), 1);
    assert_eq!(ordinals("argued for eighteen sessions that"), 1);
    assert_eq!(ordinals("for seventy-one sessions and for 12 sessions"), 2);
    assert_eq!(ordinals("for a hundred sessions"), 1);
    assert_eq!(ordinals("in session 1201 and round 4"), 2);
    assert_eq!(ordinals("an edge moved a twentieth of a pixel"), 0);
    assert_eq!(ordinals("so the twenty-eighth page is `BB`"), 0);
    assert_eq!(ordinals("does not raise a second round, because"), 0);
    assert_eq!(ordinals("the twenty-second sweep ranks it"), 0);
    assert_eq!(ordinals("a session that opens"), 0);
}

#[test]
fn no_ledger_note_names_a_session() {
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
        total == ORDINALS_IN_THE_LEDGER,
        "the ledger's notes carry {total} session ordinals against {ORDINALS_IN_THE_LEDGER}: a note \
         states the clause as it is and cites the ADR, never the session that changed it or how \
         many sessions a sentence stood (ADRs 1547, 1610)"
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
