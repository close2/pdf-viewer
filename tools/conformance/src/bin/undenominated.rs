//! Sweeps the ledger, the tree's comments and this project's prose for a claim quantified over a
//! corpus the sentence does not name.
//!
//! ```sh
//! cargo run --release -p conformance --bin undenominated
//! ```
//!
//! `doc/todo/01`'s twenty-third sweep — the eighteenth of them to be a program.
//! [`conformance::undenominated`] states the predicate exactly, says which cases it cannot see,
//! and says why the populations are read off the disk rather than written into the program.
//!
//! **It is a decay detector**, so most of what it walks is sentences that were right when they
//! were written. It prints one line per undenominated claim, closest rung first, and ends with
//! the populations it judged against so that a clean run says what it was clean over. It exits
//! non-zero only where it cannot read what it needs: a hit is a question for a person, not a
//! build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use conformance::entries;
use conformance::ledger::Ledger;
use conformance::undenominated::{self, Finding, Populations, Rung, What};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("undenominated: {error}");
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
    /// The corpora on disk could not be counted.
    #[error(transparent)]
    Corpora(#[from] undenominated::Error),
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
    let populations = Populations::read(&root)?;
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

    let report = undenominated::sweep(&populations, &ledger, &sources, &documents);

    for rung in [
        Rung::Recipe,
        Rung::Ledger,
        Rung::Code,
        Rung::Prose,
        Rung::Counted,
        Rung::Unmatched,
    ] {
        println!();
        println!("{} — {}:", name(rung), rung.as_str());
        let mut printed = 0usize;
        for finding in report.findings.iter().filter(|found| found.rung == rung) {
            print(finding);
            printed = printed.saturating_add(1);
        }
        if printed == 0 {
            println!("    none.");
        }
    }

    println!();
    let judged: Vec<String> = report
        .populations
        .iter()
        .map(|corpus| format!("{} in {} ({})", corpus.documents, corpus.path, corpus.name))
        .collect();
    let named: Vec<String> = report
        .named
        .iter()
        .map(|(name, count)| format!("{count} by `{name}`"))
        .collect();
    println!(
        "{} claim(s) quantify over a corpus. {} name a population — {} — and {} sit in a dated \
         record; {} name none, of which {} are an absence or a uniqueness ({} in the ledger, {} \
         beside the code, {} in a document) and {} are a count; {} state a denominator no \
         population has. {} invocation(s) walk a corpus and {} of them walk some and not the \
         rest. This tree holds {} document(s): {}.",
        report.claims,
        report.denominated,
        named.join(", "),
        report.dated,
        report
            .findings
            .len()
            .saturating_sub(report.on(Rung::Unmatched))
            .saturating_sub(report.on(Rung::Recipe)),
        report
            .on(Rung::Ledger)
            .saturating_add(report.on(Rung::Code))
            .saturating_add(report.on(Rung::Prose)),
        report.on(Rung::Ledger),
        report.on(Rung::Code),
        report.on(Rung::Prose),
        report.on(Rung::Counted),
        report.on(Rung::Unmatched),
        report.recipes,
        report.on(Rung::Recipe),
        populations.whole(),
        judged.join(", "),
    );
    println!(
        "This sweep finds sentences whose denominator cannot be read off them rather than \
         sentences that are wrong, so a hit is a reading list. It cannot see a denominator \
         stated in the sentence before, an absence written without a quantifier, or a claim \
         counted in anything but documents; and a claim written when this tree held one corpus \
         is not a defect. A recipe that walks one corpus on purpose is right to, and says so in \
         the prose around it, which this reads nothing of. Read the sentence before believing a \
         hit."
    );
    Ok(())
}

/// The heading one rung is printed under.
fn name(rung: Rung) -> &'static str {
    match rung {
        Rung::Recipe => "An invocation whose population is not the tree's",
        Rung::Ledger => "Refutable by one witness, and a row's status rests on it",
        Rung::Code => "Refutable by one witness, beside the code",
        Rung::Prose => "Refutable by one witness, in a document",
        Rung::Counted => "Counted over a corpus with no name",
        Rung::Unmatched => "Denominated by a number nothing on disk has",
    }
}

/// Prints one finding: what it is about, and the sentence or the invocation.
fn print(finding: &Finding) {
    match &finding.what {
        What::Claim { claim, stated } => {
            let stated = stated.map_or_else(String::new, |count| format!(" (states {count})"));
            println!(
                "    {} — `{}` governs `{}`, {}{}",
                finding.location, claim.word, claim.noun, claim.shape, stated,
            );
        }
        What::Recipe { names, omits } => {
            println!(
                "    {} — walks {}, and not {}",
                finding.location,
                names.join(", "),
                omits.join(", "),
            );
        }
    }
    println!("        {}", finding.sentence);
}
