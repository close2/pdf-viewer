//! Sweeps the ledger for a parent row asserting what its own children deny.
//!
//! ```sh
//! cargo run --release -p conformance --bin overstated
//! ```
//!
//! `doc/todo/01`'s eighteenth sweep — the thirteenth of them to be a program — and the only one
//! that opens no source file at all: both sides of the comparison are sentences in `ledger.toml`.
//! [`conformance::overstated`] says why the twelve committed programs before it are blind to the
//! shape — an overstating row claims the opposite of a debt, so the sweep that reads a debt and
//! the sweep that reads a lacking thing both look past it — and what each of the three rungs
//! means.
//!
//! It prints one block per contradiction, closest rung first, and ends with the population it
//! read so that a clean run says what it was clean over. It exits non-zero only where the ledger
//! cannot be read: a contradiction is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::ledger::{Ledger, LedgerError};
use conformance::overstated::{self, Finding, Rung};

/// How much of a part is printed before it is cut.
const SHOWN: usize = 220;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("overstated: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), LedgerError> {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;

    let report = overstated::sweep(&ledger);
    for finding in &report.findings {
        print(finding);
    }

    println!();
    println!(
        "{} row(s) with descendants, asserting {} term(s) between them: {} corroborated by a \
         child, {} contradicted — {} where the child denies the term itself, {} where the child \
         owns it and denies reading, {} where the child's denial names another one of its kind. \
         {} of the {} carry a mark that demotes them.",
        report.population,
        report.asserted,
        report.corroborated,
        report.findings.len(),
        report.on(Rung::Denied),
        report.on(Rung::Owned),
        report.on(Rung::Elsewhere),
        report.marked(),
        report.findings.len(),
    );
    println!(
        "A parent row is allowed to summarise, so a hit is a reading list and not a verdict. The \
         dominant noise is a table read in part — the parent naming the entries it reads and the \
         child the entries nobody does — which is marked where the parent enumerates entries of \
         the asserted table and the child's denial names none of them, and is left to the reader \
         where there is no table to divide. Read the words before believing a hit."
    );
    Ok(())
}

/// Prints one contradiction: what was claimed, by whom, and who denies it.
fn print(finding: &Finding) {
    let mut marks = String::new();
    if finding.in_part {
        marks.push_str(" [a table read in part]");
    }
    if finding.history {
        marks.push_str(" [a correction quoting its retired wording]");
    }
    println!(
        "{}:{} (§{}, {}) asserts {} — {}{}",
        conformance::LEDGER,
        finding.parent_line,
        finding.parent,
        finding.status.as_str(),
        finding.term,
        finding.rung.as_str(),
        marks,
    );
    println!("    §{} says: {}", finding.parent, cut(&finding.asserted));
    println!(
        "    §{} (line {}) says: {}",
        finding.child,
        finding.child_line,
        cut(&finding.denied)
    );
}

/// One part, cut to a line a reader can scan.
fn cut(part: &str) -> String {
    if part.chars().count() <= SHOWN {
        return part.to_owned();
    }
    let kept: String = part.chars().take(SHOWN).collect();
    format!("{kept}…")
}
