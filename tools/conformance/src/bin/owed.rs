//! Sweeps the ledger's `partial` rows for a note that names nothing owed.
//!
//! ```sh
//! cargo run --release -p conformance --bin owed
//! ```
//!
//! `doc/todo/01`'s fourteenth sweep — the last of the four descriptions whose *level* moved with
//! the session rather than with the tree — and the twelfth of the fifteen to be a program.
//! [`conformance::owed`] says why a debt vocabulary written from memory cannot be compared across
//! runs, and why the discriminator here is the seventh sweep's with the sign reversed: a term the
//! tree **lacks** is a `partial` row saying what is owed in a word.
//!
//! It prints one block per row whose every stated term the tree names, least vocabulary first,
//! and ends with the population it read so that a clean run says what it was clean over. It exits
//! non-zero only where it cannot read what it needs: a row on the reading list is a question for
//! a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::entries;
use conformance::ledger::Ledger;
use conformance::owed::{self, Finding, SHOWN};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("owed: {error}");
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

    let report = owed::sweep(&ledger, &sources);
    for finding in &report.findings {
        print(finding);
    }

    println!();
    println!(
        "{} `partial` row(s) stating {} term(s): {} named by no source — a debt named in a word — \
         over {} row(s), leaving {} row(s) whose every stated term this tree already names, which \
         is the reading list.",
        report.population,
        report.stated,
        report.absent,
        report.naming,
        report.findings.len(),
    );
    println!(
        "The ledger's own definition of `partial` is that the note says which requirements are \
         not executed. A note that names a debt names a thing, and a thing this tree does not \
         have is a name no source carries — which is a count rather than a vocabulary written \
         from memory. The noise is a debt named in prose with no identifier in it, printed rather \
         than filtered, because what a program can settle is whether a name is missing and never \
         whether a sentence means one is. Read the note before believing a hit."
    );
    Ok(())
}

/// Prints one row: where it is, what it is, and the first few of the terms the tree names.
fn print(finding: &Finding) {
    println!(
        "{}:{} (§{}) {} — {} term(s), every one named",
        conformance::LEDGER,
        finding.line,
        finding.clause,
        finding.title,
        finding.stated(),
    );
    for term in finding.terms.iter().take(SHOWN) {
        println!(
            "    {} — named by {} file(s): {}",
            term.text,
            term.reach,
            term.witnesses.join(", ")
        );
    }
}
