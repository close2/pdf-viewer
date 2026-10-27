//! Sweeps the ledger's `inapplicable` and `out-of-scope` rows for vocabulary, and a clause
//! number, the tree names.
//!
//! ```sh
//! cargo run -p conformance --bin inapplicable
//! cargo run -p conformance --bin inapplicable -- --root <tree>
//! ```
//!
//! `--root` reads another tree's ledger, sources and `doc/md/` in place of this one's — a past
//! commit's, unpacked by `git archive`, which is how the sweep is calibrated against rows a past
//! tree executed and its ledger still excluded (ADR 2410).
//!
//! `doc/todo/01`'s seventh sweep — the only one that reads the status nobody expects to come back
//! to — and the eleventh of the fifteen to be a program rather than a description.
//! [`conformance::inapplicable`] says what a term is, why the count of naming files replaces the
//! stop-list every hand-run wrote from memory, why a cousin row is the pair this sweep exists
//! to print, and why an `out-of-scope` row is read beside an `inapplicable` one by its own clause
//! number (trap 268, ADR 2376) and by the tables its clause names (ADR 2410).
//!
//! It prints one block per swept row with at least one named term, rarest term first and
//! rarest row first, and ends with the population it read so that a clean run says what it was
//! clean over. It exits non-zero only where it cannot read what it needs: a named term is a
//! question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use conformance::clause::ClauseIndex;
use conformance::entries;
use conformance::inapplicable::{self, Finding};
use conformance::ledger::{Ledger, Status};

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
    /// The standard's conversion could not be read.
    #[error(transparent)]
    Standard(#[from] conformance::clause::ClauseIndexError),
    /// The arguments were not `--root <tree>` or nothing.
    #[error("usage: inapplicable [--root <tree>]")]
    Usage,
}

/// The tree to read: this workspace, or the one `--root` names.
fn root() -> Result<PathBuf, Error> {
    let mut arguments = std::env::args().skip(1);
    match (arguments.next(), arguments.next(), arguments.next()) {
        (None, _, _) => Ok(conformance::workspace_root()),
        (Some(flag), Some(tree), None) if flag == "--root" => Ok(PathBuf::from(tree)),
        _ => Err(Error::Usage),
    }
}

fn run() -> Result<(), Error> {
    let root = root()?;
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let index = ClauseIndex::read(&root.join(conformance::STANDARD))?;
    let sources = entries::sources(&root)?;

    let report = inapplicable::sweep(&ledger, &index, &sources);
    for finding in &report.findings {
        print(finding);
    }

    let holding = |status: Status| {
        ledger
            .rows
            .iter()
            .filter(|row| row.status == status)
            .count()
    };
    println!();
    println!(
        "{} row(s) nobody expects to come back to — {} `inapplicable`, {} `out-of-scope` — of \
         which {} have their own clause cited by a source: a clause the tree executes under a row \
         saying it is not this program's, trap 268's shape, and the first list to read. {} have a \
         table their own clause names in doc/md/ named by a source, by its number or its \
         caption's subject: the clause executed under its dictionary's table (ADR 2410).",
        report.population,
        holding(Status::Inapplicable),
        holding(Status::OutOfScope),
        report.cited,
        report.tabled,
    );
    println!(
        "They state {} term(s), the clause numbers and tables included: {} named by no source — the claims \
         this run confirms — and {} named over {} row(s), of which {} carry a cousin row that is \
         neither `inapplicable` nor `out-of-scope` and says the same word, the second list to \
         read.",
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
        "{}:{} (§{}, {}) {}",
        conformance::LEDGER,
        finding.line,
        finding.clause,
        finding.status.as_str(),
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
