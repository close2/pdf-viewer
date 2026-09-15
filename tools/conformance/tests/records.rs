//! A round's record is at most forty lines, and this is what counts them.
//!
//! # A number a document states is a number nobody measures
//!
//! `doc/todo/02` section 8 has capped a round's record at forty lines for as long as the batch
//! loop has existed, and every brief repeats it. Nothing counted it. The six records of sessions
//! 1086–1091 ran 44, 47, 19, 40, 40 and 40 — two over, and the only reason anybody knows is that
//! a later round ran `wc -l`. That is the shape `CLAUDE.md` ("Where knowledge lives") names: a
//! fact that can be counted is not written down, and what is written down is the command that
//! counts it.
//!
//! # Why the bound starts at a number rather than at the beginning
//!
//! **Because a record may not be rewritten.** `CLAUDE.md` says `doc/adr/`, `doc/history/` and
//! `doc/reviews/` keep their chronology and that rewriting one for tidiness is the single thing a
//! record may not have done to it — so a check whose only remedy is an edit may not be pointed at
//! a record written before the check existed. [`COUNTED_FROM`] is where the counting began, which
//! is the batch that built the instrument and trimmed its own two.
//!
//! The twelve most recent are *printed* whatever their number, because the ones below the bound
//! are what the bound has to be read against: a budget nobody was over would be a budget nobody
//! needed.

#![expect(
    clippy::print_stdout,
    reason = "the gate prints the population it counted, which is what makes its verdict readable"
)]

use std::fmt::Write as _;

/// The most lines one round's record may run to.
///
/// `doc/todo/02` section 8's figure, and the only copy of it that anything reads.
const BUDGET: usize = 40;

/// The first session whose record this check holds to [`BUDGET`].
///
/// Sessions 1086–1091 are the batch that built the check; two of the six were over and were
/// trimmed by the round that built it, once, by cutting restatement rather than fact. Everything
/// before them is a record nothing may edit, and so is a record nothing may fail.
const COUNTED_FROM: u32 = 1086;

/// How many of the most recent records are printed beside the budget.
const PRINTED: usize = 12;

/// Where the records are.
const HISTORY: &str = "doc/history";

/// Every record a round wrote since the budget was counted fits inside it.
#[test]
fn every_record_since_the_budget_was_counted_fits_inside_it() {
    let root = conformance::workspace_root();
    let mut records: Vec<(u32, String, usize)> = Vec::new();
    for entry in std::fs::read_dir(root.join(HISTORY)).expect("the records") {
        let path = entry.expect("a record").path();
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        // A record is `<session>-<slug>.md`. `README.md` is the directory's own note and is not
        // one, and it is told apart by the number rather than by its name — a list of exceptions
        // is trap 25's shape however short it is.
        let Some(session) = name
            .split('-')
            .next()
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("a record's text");
        records.push((session, name, text.lines().count()));
    }
    records.sort_unstable();

    let mut over = String::new();
    for (session, name, lines) in &records {
        if *session >= COUNTED_FROM && *lines > BUDGET {
            let _ = writeln!(
                over,
                "{HISTORY}/{name}: {lines} lines, over the budget of {BUDGET}. Cut restatement, \
                 never a fact: what the round found, what it measured and what it left owed are \
                 the record, and the sentences saying them again are not."
            );
        }
    }

    println!("the last {PRINTED} records, against a budget of {BUDGET} lines:");
    for (session, name, lines) in records.iter().rev().take(PRINTED).rev() {
        let verdict = if *lines > BUDGET {
            if *session >= COUNTED_FROM {
                "over"
            } else {
                "over, and written before the count existed"
            }
        } else {
            ""
        };
        println!("  {session} {lines:4} {verdict:<38} {name}");
    }
    println!(
        "{} records, {} of them counted against the budget",
        records.len(),
        records
            .iter()
            .filter(|(session, _, _)| *session >= COUNTED_FROM)
            .count()
    );

    assert!(
        records.len() > 100,
        "only {} records found: the walk is not reaching {HISTORY}",
        records.len()
    );
    assert!(over.is_empty(), "\n{over}");
}
