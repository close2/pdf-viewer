//! Sweeps the ledger for an entry a note claims is unread while the tree quotes it.
//!
//! ```sh
//! cargo run --release -p conformance --bin unread
//! ```
//!
//! `doc/todo/01`'s second sweep — "every `/Key` in a claim of unreadness, grepped against the
//! tree" — and the third of the fifteen to be a program rather than a description.
//! [`conformance::unread`] says what a claim is, what a hit is not, and why the dominant noise
//! shape (one short key, three clauses) is settled by reading the witness path rather than by a
//! tighter grep.
//!
//! It prints one block per ledger row claiming an entry unread that some source quotes, each
//! entry with the files that quote it, and ends with the population it read so that a clean run
//! says what it was clean over. It exits non-zero only where it cannot read what it needs: a
//! quoted key is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::entries;
use conformance::ledger::Ledger;
use conformance::unread;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("unread: {error}");
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

    let report = unread::sweep(&ledger, &sources);
    for finding in &report.findings {
        println!(
            "ledger.toml:{}: §{} {} — {} entr{} claimed unread that the tree quotes",
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
                "    /{} — {}: {}",
                entry.key,
                entry.named,
                entry.witnesses.join(", ")
            );
        }
    }

    println!();
    println!(
        "{} rows claim an entry unread, {} keys between them: {} confirmed (quoted by no \
         source), {} quoted somewhere over {} rows — {} of those by the row's own code, which \
         is the list to read first.",
        report.population,
        report.claimed,
        report.confirmed,
        report.live(),
        report.findings.len(),
        report.by_own_code(),
    );
    println!(
        "A hit is a reading list rather than a verdict. The dominant noise is one short key in \
         three clauses — §8.4.5's /BG is black generation while appearance.rs's \"BG\" is a \
         widget background — so read the witness path before believing a hit."
    );
    Ok(())
}
