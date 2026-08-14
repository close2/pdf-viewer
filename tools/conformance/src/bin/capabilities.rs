//! Sweeps the ledger's notes and the tree's comments for a capability reason that may have
//! expired.
//!
//! ```sh
//! cargo run --release -p conformance --bin capabilities
//! ```
//!
//! `doc/todo/01`'s third sweep — "this program has no ___", "no panel", "which this is not" —
//! and the fifth of the fifteen to be a program rather than a description.
//! [`conformance::capabilities`] says what a claim is, what one grep of the tree can settle
//! about one, and why the dominant noise shape — a true statement about a boundary a crate
//! deliberately keeps — is printed rather than filtered.
//!
//! It prints one line per claim sentence, the witnessed program-level ones first, and ends with
//! the counts so that a clean run says what it was clean over. It exits non-zero only where it
//! cannot read what it needs: a capability reason is a question for a person, not a build
//! failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::capabilities::{self, Hit, Subject};
use conformance::entries;
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("capabilities: {error}");
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

    let report = capabilities::sweep(&ledger, &sources);
    for (name, hits) in [("ledger", &report.ledger), ("source", &report.source)] {
        for (subject, witnessed) in [
            (Subject::Program, true),
            (Subject::Crate, true),
            (Subject::Program, false),
            (Subject::Crate, false),
        ] {
            for hit in hits
                .iter()
                .filter(|hit| hit.subject == subject && hit.witness.is_some() == witnessed)
            {
                print(hit);
            }
        }
        println!(
            "{name}: {} capability sentence(s) — {} witnessed by the tree, {} about the \
             program, {} about one crate.",
            hits.len(),
            capabilities::Report::witnessed(hits),
            capabilities::Report::about(hits, Subject::Program),
            capabilities::Report::about(hits, Subject::Crate),
        );
        println!();
    }
    println!(
        "A hit is a reading list rather than a verdict. The dominant noise shape is a true \
         statement about a boundary a crate deliberately keeps — no clock, no filesystem, no \
         toolkit — which a witness does not disprove; the others are a correction quoting the \
         wording it retired — marked [history] — and a witness that is one short identifier in \
         two clauses. Read the sentence before believing a hit."
    );
    Ok(())
}

/// Prints one hit: location, subject, the witness where one exists, the sentence.
fn print(hit: &Hit) {
    let witness = hit.witness.as_ref().map_or_else(
        || " unwitnessed".to_owned(),
        |witness| {
            format!(
                " the tree names `{}` ({}:{})",
                witness.term, witness.path, witness.line
            )
        },
    );
    let history = if hit.history { " [history]" } else { "" };
    println!(
        "{}: about {}, lacking \"{}\" —{witness}{history}\n    {}",
        hit.location, hit.subject, hit.lacking, hit.sentence
    );
}
