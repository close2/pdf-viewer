//! Prints, for every `departed` ledger row, the documents a re-reader of its argument needs.
//!
//! ```sh
//! cargo run -q -p conformance --bin departures
//! ```
//!
//! `departed` says a clause is executed except for the one sentence its note names, decided
//! against with its cost recorded. The check that keeps the word honest asks the row to *name* the
//! ADR that recorded the cost and cannot judge whether the argument still fits (ADR 1119), so a
//! `departed` row settles and its argument becomes a dated document that nothing re-reads.
//!
//! This is the mechanical half of the re-reading. For each row it prints the ADR the note's first
//! sentence names and every later ADR that cites that ADR or the row's own clause number — the
//! reading list, and nothing about whether the premise holds. Judging that is a person's, for the
//! reason [`conformance::departures`] states: a program that scored a premise would be a judgement
//! wearing a check.
//!
//! It exits non-zero only where the ledger or `doc/adr/` cannot be read. A stale premise is not a
//! build failure; a row naming an ADR that does not exist is, and that one is in
//! `cargo test -p conformance` rather than here.
//!
//! ADR 1166.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::departures;
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("departures: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the report could not be produced.
#[derive(Debug, thiserror::Error)]
enum Failure {
    /// The ledger could not be read.
    #[error(transparent)]
    Ledger(#[from] conformance::ledger::LedgerError),
    /// The ADRs could not be read.
    #[error(transparent)]
    Adrs(#[from] departures::Error),
}

fn run() -> Result<(), Failure> {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let found = departures::read(&ledger, &root)?;
    print!("{}", departures::report(&found));
    println!();
    println!(
        "The list is mechanical: ADR numbers and clause numbers, never a note's prose. What a \
         re-reader asks of each argument is whether its premise still holds — a capability it \
         says this program lacks that has since been built, a property of the output device, a \
         decision taken under an exclusion CLAUDE.md has since amended, or an inconsistency on \
         its own terms."
    );
    Ok(())
}
