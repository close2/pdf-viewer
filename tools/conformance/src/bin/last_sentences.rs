//! Sweeps the ledger for a note whose opening or closing sentence names another row's status.
//!
//! ```sh
//! cargo run --release -p conformance --bin last_sentences
//! ```
//!
//! A `doc/todo/01` sweep (ADR 1249), and the only one whose two sides are both inside one row:
//! the `status` field and the note's own sentence about what the row is.
//! [`conformance::last_sentences`] says what each rung means and what the top one's noise is.
//!
//! It prints one line per hit, closest rung first, with the sentence under it, and ends with the
//! population it read so that a clean run says what it was clean over. It exits non-zero only
//! where the ledger cannot be read: a hit is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::last_sentences::{self, Finding, Rung};
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("last-sentences: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), conformance::ledger::LedgerError> {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let report = last_sentences::sweep(&ledger.rows);

    for finding in &report.findings {
        print(finding);
    }

    println!();
    println!(
        "{} row(s) carry a note; {} opening and closing sentence(s) read. {} state a status the \
         row does not wear — {} about this row, {} about this row in the past tense, {} with no \
         self-reference.",
        report.population,
        report.read,
        report.findings.len(),
        report.on(Rung::AboutThisRow),
        report.on(Rung::Narrated),
        report.on(Rung::Elsewhere),
    );
    println!(
        "A note's closing sentence is the one every later round appends above, and its opening \
         sentence is the one a round that moves a status rewrites around — so a status word \
         written there and not in the `status` field is a sentence that outlived its row. The \
         first rung is the defect; the second is the ledger's own shape for a correction, which \
         states the claim it retires on purpose; the third is a parent naming a child. The noise \
         the first rung keeps is a sentence naming the status a pending question would move the \
         row to. Read the sentence before believing a hit."
    );
    Ok(())
}

/// Prints one hit: where it is, what disagrees, and the sentence.
fn print(finding: &Finding) {
    let stated: Vec<&str> = finding
        .stated
        .iter()
        .map(|status| status.as_str())
        .collect();
    println!(
        "{}:{} (§{}, {}): {} sentence states `{}` [{}]",
        conformance::LEDGER,
        finding.line,
        finding.clause,
        finding.status.as_str(),
        finding.position,
        stated.join("`, `"),
        finding.rung,
    );
    println!("    {}", finding.sentence);
}
