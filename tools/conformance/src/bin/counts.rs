//! Sweeps the ledger, the tree's comments and this project's prose for a count of a family that
//! the family contradicts.
//!
//! ```sh
//! cargo run --release -p conformance --bin counts
//! ```
//!
//! `doc/todo/01`'s tenth sweep — a parent row's stated count against its children — and the
//! thirteenth of the fifteen to be a program rather than a description.
//! [`conformance::counts`] says what makes a number a claim about a family, why the discriminator is
//! the ninth sweep's attribution over the sixth sweep's arithmetic rather than a vocabulary of
//! counting, and which two rungs are counted rather than printed.
//!
//! It prints the contradictions first — a place counting one family twice, which is wrong whatever
//! the ledger holds — then the suspects nearest first, then the counts on every rung, so that a
//! clean run says what it was clean over. It exits non-zero only where it cannot read what it
//! needs: a suspect is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use conformance::counts::{self, Contradiction, Counted};
use conformance::entries;
use conformance::ledger::Ledger;
use conformance::retired::Kind;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("counts: {error}");
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
    /// The documents could not be walked.
    #[error(transparent)]
    Documents(#[from] conformance::prose::Error),
    /// A file could not be read.
    #[error("{path}: {source}")]
    Unreadable {
        /// The file.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let sources = entries::sources(&root)?;

    let mut documents: Vec<(PathBuf, String)> = Vec::new();
    for path in conformance::prose::documents(&root.join("doc"))? {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Unreadable {
            path: path.display().to_string(),
            source,
        })?;
        documents.push((
            path.strip_prefix(&root).unwrap_or(&path).to_path_buf(),
            text,
        ));
    }

    let report = counts::sweep(&ledger, &sources, &documents);

    println!("Contradicted — one place counting one family twice, disagreeing:");
    for contradiction in &report.contradictions {
        print_contradiction(contradiction);
    }
    if report.contradictions.is_empty() {
        println!("    none.");
    }

    let suspects = report.suspects();
    println!();
    println!("Absent — the family below the clause can be counted no such way:");
    for counted in &suspects {
        print(counted);
    }
    if suspects.is_empty() {
        println!("    none.");
    }

    println!();
    println!(
        "{} clause(s) have a row below them. {} sentence(s) govern one of the ledger's own words \
         for a row; {} attributed count(s) — {} the family agrees with, {} it can be counted no \
         such way, {} attributed to a clause with no rows below it. {} place(s) count one family \
         twice.",
        report.parents,
        report.sentences,
        report.counted.len(),
        report.agreeing(),
        suspects.len(),
        report.childless(),
        report.contradictions.len(),
    );
    println!(
        "A suspect is a reading list rather than a verdict. A sentence can count the standard's \
         subclauses rather than the ledger's rows, or the rows a `General` row sits *beside* rather \
         than the family below it, and a correction quotes the count it retired — which is why \
         those are marked rather than dropped. Read the sentence before believing a hit."
    );
    Ok(())
}

/// Prints one contradiction: the family, and each number with the sentence stating it.
fn print_contradiction(contradiction: &Contradiction) {
    println!(
        "    §{} — {} counted as {} and as {}",
        contradiction.clause,
        contradiction.noun,
        contradiction.stated.first().map_or(0, |(count, _)| *count),
        contradiction.stated.last().map_or(0, |(count, _)| *count),
    );
    println!("        {}", contradiction.location);
    for (count, sentence) in &contradiction.stated {
        println!("        {count}: {sentence}");
    }
}

/// Prints one suspect: what it claims, what the family can be counted as, and its sentence.
fn print(counted: &Counted) {
    let mark = if counted.kind == Kind::Correction {
        "[correction]"
    } else {
        "[standing]  "
    };
    let attributed = if counted.from_sentence {
        "named in the sentence"
    } else {
        "the row's own"
    };
    println!(
        "    {mark} {} {} of §{} ({attributed}) — {}",
        counted.claim.count, counted.claim.noun, counted.clause, counted.verdict,
    );
    let named: Vec<String> = counted
        .cardinalities
        .iter()
        .map(|(count, name)| format!("{count} = {name}"))
        .collect();
    if !named.is_empty() {
        println!("        the family is: {}", named.join(", "));
    }
    println!("        {}", counted.location);
    println!("        {}", counted.sentence);
}
