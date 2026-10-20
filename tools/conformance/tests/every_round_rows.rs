//! The short trap list every round reads is the full index's rows, byte for byte.
//!
//! # What it guards
//!
//! `doc/traps/every-round.md` is the ten traps the rounds' records cite most, and it says of its
//! table that "the rows are the full index's rows, verbatim: a change to one is made in `README.md`
//! and copied here". The copy is the step a round forgets: a rule cell reworded in the index and not
//! here leaves every round reading, before it starts, a rule the index no longer states — and a
//! reader has no reason to compare the two. One row had drifted by the time the claim was read
//! against the files (ADR 2110 §2); this holds the claim instead of re-reading it (ADR 2127).
//!
//! A row is a table line whose first cell opens with a digit, the shape `tests/traps.rs` reads the
//! index by. Each such line of the short list must be a line of the index with the same first cell,
//! identical to it after trailing whitespace; a number the index has no row for is named too, as is
//! a number the short list carries twice.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The full index, relative to the repository root.
const INDEX: &str = "doc/traps/README.md";

/// The every-round list, relative to the repository root.
const SHORT_LIST: &str = "doc/traps/every-round.md";

/// The rows of a trap table, keyed by the trap number in the first cell, each with its line
/// number; a number given twice keeps every occurrence so that the duplicate can be named.
fn rows(text: &str) -> BTreeMap<String, Vec<(usize, String)>> {
    let mut rows: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for (at, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix('|') else {
            continue;
        };
        let Some((first, _)) = rest.split_once('|') else {
            continue;
        };
        let number = first.trim();
        if !number.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        rows.entry(number.to_owned())
            .or_default()
            .push((at.saturating_add(1), line.trim_end().to_owned()));
    }
    rows
}

/// Where two lines first differ, in characters from the start, so that a failure says where in
/// a long rule cell the copy departs.
fn first_difference(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(a, b)| a == b)
        .count()
}

/// Every way the short list departs from the index, one line each; empty when it does not.
fn departures(index: &str, short_list: &str) -> Vec<String> {
    let index_rows = rows(index);
    let mut found = Vec::new();
    for (number, occurrences) in rows(short_list) {
        if occurrences.len() > 1 {
            let lines: Vec<String> = occurrences.iter().map(|(at, _)| at.to_string()).collect();
            found.push(format!(
                "trap {number}: {} rows, on lines {}",
                occurrences.len(),
                lines.join(", ")
            ));
        }
        for (at, line) in &occurrences {
            match index_rows.get(&number).and_then(|rows| rows.first()) {
                None => found.push(format!(
                    "trap {number} (line {at}): {INDEX} has no row of that number"
                )),
                Some((index_at, index_line)) if index_line != line => {
                    let column = first_difference(line, index_line);
                    let excerpt: String = index_line.chars().skip(column).take(40).collect();
                    let ours: String = line.chars().skip(column).take(40).collect();
                    found.push(format!(
                        "trap {number} (line {at}) differs from {INDEX} line {index_at} at \
                         character {column}: the index has `{excerpt}`, the short list `{ours}`"
                    ));
                }
                Some(_) => {}
            }
        }
    }
    found
}

/// Every row of `doc/traps/every-round.md` is the index's row of that number, byte for byte.
#[test]
fn every_round_rows_are_the_index_rows_verbatim() {
    let root = conformance::workspace_root();
    let read = |path: &str| {
        std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    let index = read(INDEX);
    let short_list = read(SHORT_LIST);
    assert!(
        !rows(&short_list).is_empty(),
        "{SHORT_LIST} holds no row this gate can read"
    );
    let found = departures(&index, &short_list);
    let mut failures = String::new();
    for line in &found {
        let _ = writeln!(failures, "  {line}");
    }
    assert!(
        found.is_empty(),
        "{SHORT_LIST}'s rows are to be {INDEX}'s rows verbatim — a change is made in the index and \
         copied here (ADR 2127):\n{failures}"
    );
}

/// The comparison names a drifted cell, a row the index lacks and a duplicated number, and passes
/// an exact copy: the calibration the gate above rests on, kept beside it (trap 13).
#[test]
fn a_drifted_row_a_missing_row_and_a_duplicate_are_each_named() {
    let index = "| # | when | rule | group |\n|---|---|---|---|\n\
                 | 1 | a mark | render the page | pixels |\n\
                 | 2 | a paint | compose once | pixels |\n";
    assert!(departures(index, "| 1 | a mark | render the page | pixels |\n").is_empty());
    let drifted = departures(index, "| 1 | a mark | render a page | pixels |\n");
    assert_eq!(drifted.len(), 1, "{drifted:?}");
    assert!(
        drifted[0].contains("trap 1 (line 1) differs"),
        "{drifted:?}"
    );
    let missing = departures(index, "| 3 | a font | load it | parsers |\n");
    assert!(missing[0].contains("has no row"), "{missing:?}");
    let twice = departures(
        index,
        "| 2 | a paint | compose once | pixels |\n| 2 | a paint | compose once | pixels |\n",
    );
    assert!(twice[0].contains("2 rows"), "{twice:?}");
}
