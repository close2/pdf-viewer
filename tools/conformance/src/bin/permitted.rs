//! Sweeps the ledger's `partial` rows for a debt the standard states as a permission.
//!
//! ```sh
//! cargo run --release -p conformance --bin permitted
//! ```
//!
//! `doc/todo/01`'s twenty-fourth sweep, and the first whose discriminator is neither side of the
//! comparison every other one makes: it is the **clause's own modal verb**.
//! [`conformance::permitted`] has the argument, ADR 0896 the shape it exists for, and ADR 0897
//! the reason the second column is printed beside every hit.
//!
//! It prints one block per `partial` row that quotes no requirement — closest rank first — with
//! the standard's own sentence under each quotation and the verb read off it, and ends with the
//! population it read so that a clean run says what it was clean over. It exits non-zero only
//! where it cannot read what it needs: a row on the reading list is a question for a person, not
//! a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::clause::ClauseIndex;
use conformance::entries;
use conformance::ledger::Ledger;
use conformance::permitted::{self, Finding, Rank, SHOWN};
use conformance::prose::Conversion;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("permitted: {error}");
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
    /// The Markdown conversion of the specifications could not be read.
    #[error(transparent)]
    Conversion(#[from] conformance::prose::Error),
    /// The standard's clause index could not be built.
    #[error(transparent)]
    Index(#[from] conformance::clause::ClauseIndexError),
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let conversion = Conversion::read(&root.join("doc/md"))?;
    let index = ClauseIndex::read(&root.join(conformance::STANDARD))?;
    let standard = std::fs::read_to_string(root.join(conformance::STANDARD)).map_err(|source| {
        conformance::clause::ClauseIndexError::Unreadable {
            path: conformance::STANDARD.to_owned(),
            source,
        }
    })?;
    let described = entries::descriptions_in(&standard);

    let report = permitted::sweep(&ledger, &conversion, &index, &described);
    let mut rank = None;
    for finding in &report.findings {
        if rank != Some(finding.rank) {
            println!();
            println!("== {} ==", finding.rank.as_str());
            rank = Some(finding.rank);
        }
        print(finding);
    }

    println!();
    println!(
        "{} `partial` row(s): {} quote a requirement of the standard and are not read here, \
         leaving {} — {} every one of whose named table entries the standard states as \
         optional, {} whose strongest quoted verb is a permission, {} quoting the standard \
         with no modal verb at all, {} resting on a recommendation, and {} quoting nothing the \
         conversion holds.",
        report.rows,
        report.quoting_a_requirement,
        report.findings.len(),
        report.on(Rank::Optional),
        report.on(Rank::Permission),
        report.on(Rank::Bare),
        report.on(Rank::Recommendation),
        report.on(Rank::Unquoted),
    );
    println!(
        "`partial` means some normative requirement is not executed, so a row resting on a \
         sentence the standard states with `may` or `can` is a claim nobody made. The verb is \
         read off the standard's own sentence rather than off the note, because a note is not \
         evidence. The `shall` count beside each row is the clause's own prose outside its \
         tables and NOTEs: a hit over a clause with none is a status with nothing under it, and \
         a hit over a clause with several is a row that has read the wrong half of it. Read the \
         clause before moving anything."
    );
    Ok(())
}

/// Prints one row: where it is, what it is, and the first few of its located quotations.
fn print(finding: &Finding) {
    println!(
        "{}:{} (§{}) {} — {} `shall` sentence(s) in its prose outside tables",
        conformance::LEDGER,
        finding.line,
        finding.clause,
        finding.title,
        finding.shalls,
    );
    for found in finding.located.iter().take(SHOWN) {
        println!("    {}: \"{}\"", found.verb.as_str(), found.quotation);
        println!("        standard: {}", found.sentence);
    }
    for entry in &finding.entries {
        println!(
            "    Table {} `/{}` — {}, {}: {}",
            entry.table,
            entry.key,
            match entry.optional {
                Some(true) => "optional",
                Some(false) => "required",
                None => "neither word",
            },
            entry.verb.as_str(),
            entry.description,
        );
    }
    if finding.unlocated > 0 {
        println!(
            "    ({} further quotation(s) the conversion does not hold)",
            finding.unlocated
        );
    }
}
