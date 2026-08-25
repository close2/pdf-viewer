//! Sweeps the ledger, the tree's comments and this project's prose for a cardinal counting one of
//! this tree's own parts that the workspace counts differently.
//!
//! ```sh
//! cargo run --release -p conformance --bin parts
//! ```
//!
//! `doc/todo/01`'s twenty-second sweep — the seventeenth of them to be a program.
//! [`conformance::parts`] says which words make a cardinal a claim about this tree, why the answer
//! is read off the workspace's own files rather than written into the program, and what the three
//! rungs mean.
//!
//! **It is a decay detector**, so most of the population it walks is correct sentences: "both
//! backends" was right until the tree grew a third rasteriser. It prints one line per
//! disagreement, closest rung first, and ends with the populations it judged against so that a
//! clean run says what it was clean over. It exits
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
use conformance::parts::{self, Finding, Membership, Rung};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("parts: {error}");
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
    /// The workspace's own membership could not be read.
    #[error(transparent)]
    Workspace(#[from] parts::Error),
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
    let membership = Membership::read(&root)?;
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

    let report = parts::sweep(&membership, &ledger, &sources, &documents);

    for rung in [Rung::Whole, Rung::Tree] {
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
    // The last rung is **counted rather than listed**, which is the nineteenth sweep's rule for
    // its own last rung: a decision record is dated by its own number and was right on its date,
    // and a comment inside the population is usually about the pair a comparison names. Listing
    // them would put several hundred correct sentences above the two rungs a round can act on.
    println!();
    println!(
        "{} — {}: {} not listed.",
        name(Rung::Dated),
        Rung::Dated.as_str(),
        report.on(Rung::Dated),
    );

    let judged: Vec<String> = report
        .populations
        .iter()
        .map(|(part, count)| format!("{count} {part}"))
        .collect();
    println!();
    println!(
        "{} cardinal(s) govern one of this tree's own parts immediately, {} of which the \
         workspace agrees with and {} of which it does not — {} on the closest rung, {} in the \
         ledger or an undated document, {} in a dated record or inside the population. The \
         workspace states: {}.",
        report.mentions,
        report.agreeing,
        report.findings.len(),
        report.on(Rung::Whole),
        report.on(Rung::Tree),
        report.on(Rung::Dated),
        judged.join(", "),
    );
    println!(
        "This sweep finds sentences that stopped being true rather than sentences that were \
         wrong, so a hit is a reading list. The dominant noise is the pair a cross-backend \
         comparison is about; beside it are a modifier that *follows* the noun (\"four \
         submodules under `doc/corpora/`\"), this project's own aphorisms repeated verbatim, and \
         a round's own record of running this. Read the sentence before believing a hit."
    );
    Ok(())
}

/// The heading one rung is printed under.
fn name(rung: Rung) -> &'static str {
    match rung {
        Rung::Whole => "Upstream of the whole population",
        Rung::Tree => "Written about the tree",
        Rung::Dated => "Dated, or inside the population",
    }
}

/// Prints one disagreement: what was counted, what the workspace holds, and the sentence.
fn print(finding: &Finding) {
    println!(
        "    {} — `{}` presupposes {} {}, and the workspace states {}",
        finding.location, finding.word, finding.stated, finding.part, finding.population,
    );
    println!("        {}", finding.sentence);
}
