//! Sweeps the ledger for a clause whose own code names none of the entries the clause states.
//!
//! ```sh
//! cargo run --release -p conformance --bin entries
//! ```
//!
//! `doc/todo/01`'s fifteenth sweep, and the first of the fifteen to be a program rather than a
//! description. [`conformance::entries`] says what it asks, why it is a reading list rather than
//! a gate, and what the two findings it has produced were.
//!
//! It prints one block per ledger row whose `code = [...]` files name none of an entry its
//! clause's own tables state, and ends with the population it read, so that a run with nothing in
//! it says what it was clean over. It exits non-zero only where it cannot read what it needs: an
//! entry nobody reads is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::clause::ClauseIndex;
use conformance::entries::{self, Named};
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("entries: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// The standard's conversion could not be indexed.
    #[error(transparent)]
    Standard(#[from] conformance::clause::ClauseIndexError),
    /// The ledger could not be read.
    #[error(transparent)]
    Ledger(#[from] conformance::ledger::LedgerError),
    /// The tree's sources could not be walked.
    #[error(transparent)]
    Sources(#[from] entries::Error),
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD))?;
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let sources = entries::sources(&root)?;

    let report = entries::sweep(&ledger, &index, &sources);
    for finding in &report.findings {
        println!(
            "ledger.toml:{}: §{} {} — {} entr{} its own code does not name",
            finding.line,
            finding.clause,
            finding.status.as_str(),
            finding.entries.len(),
            if finding.entries.len() == 1 {
                "y"
            } else {
                "ies"
            }
        );
        for entry in &finding.entries {
            println!(
                "    Table {}'s /{} — {}{}",
                entry.table,
                entry.key,
                entry.named,
                if entry.disposed_of {
                    ", and the note says so"
                } else {
                    ", AND THE NOTE DOES NOT NAME IT"
                }
            );
        }
    }

    println!();
    println!(
        "{} rows explain themselves by an arrival and name code; {} do so and name none. They \
         state {} table entries between them, of which {} are reported over {} rows: {} named \
         nowhere in the tree, {} named only elsewhere — and {} of the {} are not named by the \
         row's own note either, which is the list to read first.",
        report.population,
        report.without_code,
        report.entries,
        report.reported(),
        report.findings.len(),
        report.counted(Named::Nowhere),
        report.counted(Named::Elsewhere),
        report.undisposed(),
        report.reported(),
    );
    println!(
        "A hit is a reading list rather than a verdict. Ask whether the row's disposal of the \
         entry is a claim about the entry or about the clause; only the first is legible as \
         wrong. And `doc/md/` shifts some tables' columns, so check the PDF in `doc/` with \
         `pdftotext -layout` before believing an entry the standard states is missing."
    );
    Ok(())
}
