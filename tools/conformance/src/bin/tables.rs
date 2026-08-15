//! Sweeps the ledger, the tree's comments and this project's prose for a key under the wrong table.
//!
//! ```sh
//! cargo run --release -p conformance --bin tables
//! ```
//!
//! `doc/todo/01`'s ninth sweep — does the table a sentence cites state the key it attributes to it
//! — and the ninth of the fifteen to be a program rather than a description.
//! [`conformance::tables`] says what makes a key a claim about a table, which three answers are
//! counted rather than printed, and why a wrong number arrives as a block.
//!
//! It prints the suspects sharpest first — a standing claim above a correction, and a key exactly
//! one other table states above one several do — then the table numbers the conversion does not
//! caption, then the counts on every rung, so that a clean run says what it was clean over. It
//! exits non-zero only where it cannot read what it needs.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use conformance::entries;
use conformance::ledger::Ledger;
use conformance::retired::Kind;
use conformance::tables::{self, Attribution, Tables, Verdict};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tables: {error}");
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
    let standard = root.join(conformance::STANDARD);
    let text = std::fs::read_to_string(&standard).map_err(|source| Error::Unreadable {
        path: standard.display().to_string(),
        source,
    })?;
    let tables = Tables::read(&text);
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

    let report = tables::sweep(&tables, &ledger, &sources, &documents);

    let suspects = report.suspects();
    println!("Absent — the table states entries and the key it is given is not one of them:");
    for attribution in &suspects {
        print(attribution, &tables);
    }
    if suspects.is_empty() {
        println!("    none.");
    }

    let unknown = report.reaching(Verdict::Unknown);
    println!();
    println!("No such table — the conversion captions no table of that number:");
    for attribution in &unknown {
        print(attribution, &tables);
    }
    if unknown.is_empty() {
        println!("    none.");
    }

    println!();
    println!(
        "{} table(s) captioned, {} stating entries. {} sentence(s) name a table; {} attributed \
         key citation(s) — {} the table agrees with, {} absent, {} a denial the table \
         contradicts, {} under a table that states no entries, {} under no such table.",
        tables.captioned(),
        tables.keyed(),
        report.sentences,
        report.attributions.len(),
        report.reaching(Verdict::Agrees).len(),
        report.reaching(Verdict::Absent).len(),
        report.reaching(Verdict::Denied).len(),
        report.reaching(Verdict::Keyless).len(),
        unknown.len(),
    );
    println!(
        "A suspect is a reading list rather than a verdict. Prose names a table and then a key of \
         the dictionary it describes; a table's *value* is named beside its entry; and a \
         correction quotes the number it retired, which is why those are marked rather than \
         dropped. Read the sentence before believing a hit."
    );
    Ok(())
}

/// Prints one attribution: what it claims, what the standard says, and the sentence it is in.
fn print(attribution: &Attribution, tables: &Tables) {
    let mark = if attribution.kind == Kind::Correction {
        "[correction]"
    } else {
        "[standing]  "
    };
    let title = tables.title(attribution.table).unwrap_or("no such table");
    println!(
        "    {mark} Table {}'s `/{}` — {} ({title})",
        attribution.table, attribution.key, attribution.verdict
    );
    if !attribution.elsewhere.is_empty() {
        let stating: Vec<String> = attribution
            .elsewhere
            .iter()
            .map(|number| format!("Table {number}"))
            .collect();
        println!("        stated by: {}", stating.join(", "));
    }
    println!("        {}", attribution.location);
    println!("        {}", attribution.sentence);
}
