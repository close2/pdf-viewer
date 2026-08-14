//! Sweeps the ledger's notes and the tree's comments for a blocker that may have expired.
//!
//! ```sh
//! cargo run --release -p conformance --bin blockers
//! ```
//!
//! `doc/todo/01`'s first sweep — "while §X does not exist", "needs §Y", "until §Z" — and the
//! fourth of the fifteen to be a program rather than a description. [`conformance::blockers`]
//! says what a claim is, what the ledger's own account can settle about one, and why the two
//! known false-positive shapes (a correction quoting the wording it retired, and a past tense
//! no grep can see) are printed rather than filtered.
//!
//! It prints one line per blocker sentence, the expired ones first, and ends with the counts so
//! that a clean run says what it was clean over. It exits non-zero only where it cannot read
//! what it needs: a stated blocker is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::blockers::{self, Hit, Standing};
use conformance::entries;
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("blockers: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// The ledger could not be read.
    #[error(transparent)]
    Ledger(#[from] conformance::ledger::LedgerError),
    /// The tree's sources could not be walked.
    #[error(transparent)]
    Sources(#[from] entries::Error),
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let sources = entries::sources(&root)?;

    let report = blockers::sweep(&ledger, &sources);
    for (name, hits) in [("ledger", &report.ledger), ("source", &report.source)] {
        for standing in [Standing::Expired, Standing::Holds, Standing::Unjudged] {
            for hit in hits.iter().filter(|hit| hit.standing == standing) {
                print(hit);
            }
        }
        println!(
            "{name}: {} blocker sentence(s) — {} expired by the ledger's own account, {} \
             holding, {} naming no clause.",
            hits.len(),
            blockers::Report::count(hits, Standing::Expired),
            blockers::Report::count(hits, Standing::Holds),
            blockers::Report::count(hits, Standing::Unjudged),
        );
        println!();
    }
    println!(
        "A hit is a reading list rather than a verdict. The two known noise shapes are a \
         correction quoting the wording it retired — marked [history] — and a past tense, \
         which no grep can see. Read the sentence before believing a hit."
    );
    Ok(())
}

/// Prints one hit: location, standing, the named clauses' statuses, the sentence.
fn print(hit: &Hit) {
    let named: Vec<String> = hit
        .named
        .iter()
        .map(|(clause, status)| {
            let status = status.map_or("no row", |status| status.as_str());
            format!("§{clause} is {status}")
        })
        .collect();
    let named = if named.is_empty() {
        String::new()
    } else {
        format!(" [{}]", named.join(", "))
    };
    let history = if hit.history { " [history]" } else { "" };
    println!(
        "{}: {}{named}{history}\n    {}",
        hit.location, hit.standing, hit.sentence
    );
}
