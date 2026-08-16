//! Sweeps the ledger's `inapplicable` rows for vocabulary the tree names.
//!
//! ```sh
//! cargo run --release -p conformance --bin inapplicable
//! ```
//!
//! `doc/todo/01`'s seventh sweep — the only one that reads the status nobody expects to come back
//! to — and the eleventh of the fifteen to be a program rather than a description.
//! [`conformance::inapplicable`] says what a term is, why the count of naming files replaces the
//! stop-list every hand-run wrote from memory, and why a cousin row is the pair this sweep exists
//! to print.
//!
//! It prints one block per `inapplicable` row with at least one named term, rarest term first and
//! rarest row first, and ends with the population it read so that a clean run says what it was
//! clean over. It exits non-zero only where it cannot read what it needs: a named term is a
//! question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::entries;
use conformance::inapplicable::{self, Finding};
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("inapplicable: {error}");
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

    let report = inapplicable::sweep(&ledger, &sources);
    for finding in &report.findings {
        print(finding);
    }

    println!();
    println!(
        "{} `inapplicable` row(s) stating {} term(s): {} named by no source — the claims this run \
         confirms — and {} named over {} row(s), of which {} carry a cousin row that is not \
         `inapplicable` and says the same word, which is the list to read first.",
        report.population,
        report.stated,
        report.unnamed,
        report.named(),
        report.findings.len(),
        report.with_cousin(),
    );
    println!(
        "A named term is a reading list rather than a verdict, and the count of naming files is \
         the discriminator: the standard's shared vocabulary reaches dozens of files and sorts \
         last, while a rare word under a row saying this tree does not do the thing is where all \
         five of this sweep's defects have been. Read the witness path and the cousin before \
         believing a hit."
    );
    Ok(())
}

/// Prints one row: where it is, what it is, and each named term rarest first.
fn print(finding: &Finding) {
    println!(
        "{}:{} (§{}) {}",
        conformance::LEDGER,
        finding.line,
        finding.clause,
        finding.title
    );
    for term in &finding.terms {
        let cousins: String = term
            .cousins
            .iter()
            .map(|cousin| format!("§{} {}", cousin.clause, cousin.status.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let pair = if cousins.is_empty() {
            String::new()
        } else {
            format!("\n        cousin row(s): {cousins}")
        };
        println!(
            "    {} — named by {} file(s): {}{pair}",
            term.text,
            term.reach,
            term.witnesses.join(", ")
        );
    }
}
