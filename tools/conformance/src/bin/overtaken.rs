//! Sweeps the tree's page-list notes for one a later decision overtook.
//!
//! ```sh
//! cargo run --release -p conformance --bin overtaken
//! ```
//!
//! `doc/todo/01`'s nineteenth sweep — the fourteenth of them to be a program — and the first to
//! read the oracle's group notes as a population. [`conformance::overtaken`] says what the
//! discriminator is (an ADR number is a date, and a note's citations are a claim about which
//! decisions it has read), why neither the eighth sweep nor the fourth could see the shape, what
//! each of the three rungs means and what the noise on each looks like.
//!
//! It prints one block per note, closest rung first and newest decision first within a rung, and
//! ends with the population it read so that a clean run says what it was clean over. It exits
//! non-zero only where it cannot read what it needs: a note a decision overtook is a question for
//! a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::entries;
use conformance::overtaken::{self, Finding, Rung};

/// How many overtaking decisions are printed for one note before the rest are counted.
const SHOWN: usize = 4;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("overtaken: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// The tree's sources could not be walked.
    #[error(transparent)]
    Sources(#[from] entries::Error),
    /// The decision records could not be read.
    #[error("doc/adr: {0}")]
    Decisions(#[from] std::io::Error),
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let sources = entries::sources(&root)?;
    let notes = overtaken::notes(&sources);
    let decisions = overtaken::decisions(&root)?;

    let report = overtaken::sweep(&notes, &decisions);
    let mut rung = None;
    for finding in &report.findings {
        if rung != Some(finding.rung()) {
            rung = Some(finding.rung());
            println!("\n{}:", finding.rung().as_str());
        }
        print(finding);
    }

    println!();
    println!(
        "{} page-list note(s) over {} document(s), against {} decision record(s): {} overtaken \
         — {} where the ADR names the list, {} where it is about a page the prose argues, {} \
         where it names a member the prose does not. {} note(s) cite no ADR at all and have no \
         left-hand side to compare.",
        report.notes,
        report.corpus,
        report.decisions,
        report.findings.len(),
        report.on(Rung::Group),
        report.on(Rung::Prose),
        report.on(Rung::Member),
        report.uncited,
    );
    println!(
        "A note is allowed to be about one property of a page while a later decision was about \
         another, so a hit is a reading list and not a verdict. The dominant noise is the last \
         rung — a list of hundreds of pages collects a passing mention for free — and the \
         standing witnesses of a clause are named by every decision near it. Read the note \
         before believing a hit."
    );
    Ok(())
}

/// Prints one note: where it is, how far back its reading stops, and what has happened since.
fn print(finding: &Finding) {
    println!(
        "    {} — {}:{}, newest ADR cited {:04}",
        finding.note.name,
        finding.note.file,
        finding.note.line,
        finding.note.newest_cited().unwrap_or_default(),
    );
    for overtaking in finding.overtaking.iter().take(SHOWN) {
        let documents: Vec<&str> = overtaking
            .documents
            .iter()
            .map(String::as_str)
            .take(SHOWN)
            .collect();
        println!(
            "        {} [{}] {}",
            overtaking.file,
            overtaking.rung.as_str(),
            documents.join(", ")
        );
    }
    let rest = finding.overtaking.len().saturating_sub(SHOWN);
    if rest > 0 {
        println!("        … and {rest} more, older");
    }
}
