//! Sweeps the oracle's page-list notes for a bound the gate fails on that the note never names.
//!
//! ```sh
//! cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture > oracle.log
//! cargo run --release -p conformance --bin unpriced -- oracle.log
//! ```
//!
//! `doc/todo/01`'s twenty-first sweep — the sixteenth of them to be a program, and the second
//! whose right-hand side is another gate's output. [`conformance::unpriced`] says what the
//! discriminator is (a measure the gate fails one of a note's own pages on, in a verdict of
//! contradicted, that the note's prose never names), why the population is that verdict and no
//! other, what each of the three rungs means and what the noise on each looks like.
//!
//! **It is the twentieth sweep's missing half.** `--bin quoted` checks a figure a note quotes;
//! it cannot ask for one that is missing, and five rounds recorded that debt in the same words
//! — *nothing links a group's note to which bound the gate fails its pages on*. This asks.
//!
//! It renders nothing and re-measures nothing: the oracle already prints all four measures
//! beside all four bounds for every page it does not call agreement, so which bound fails is
//! arithmetic on a line the round has already run. Reading from standard input works too.
//!
//! It prints one block per note, closest rung first, and under every unnamed failing measure
//! the note's own pages that fail on it. It exits non-zero only where it cannot read what it
//! needs: a note owing a number is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::io::Read as _;
use std::process::ExitCode;

use conformance::entries;
use conformance::overtaken;
use conformance::quoted;
use conformance::unpriced::{self, Finding, Rung};

/// How many of a measure's pages are named under it before the rest are counted.
const NAMED: usize = 3;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("unpriced: {error}");
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
    /// The gate's report could not be read.
    #[error("the gate's report: {0}")]
    Report(#[from] std::io::Error),
    /// The report held no contradicted page, so there is no bound to account for.
    #[error(
        "no contradicted page in the report — it is the output of `cargo test --profile gates \
         -p pdf-model --test oracle -- --ignored --nocapture` that is wanted, with `--nocapture`"
    )]
    Empty,
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let sources = entries::sources(&root)?;
    let notes: Vec<overtaken::Note> = overtaken::notes(&sources)
        .into_iter()
        .filter(|note| note.file == quoted::GATE_NOTES)
        .collect();

    let text = gate_report()?;
    let failing = unpriced::contradicted(&text, &quoted::report(&text));
    if failing.is_empty() {
        return Err(Error::Empty);
    }

    let report = unpriced::sweep(&notes, &failing);
    let mut rung = None;
    for finding in &report.findings {
        if rung != Some(finding.rung) {
            rung = Some(finding.rung);
            println!("\n{}:", finding.rung.as_str());
        }
        print(finding);
    }

    println!();
    println!(
        "{} note(s) of {} hold a contradicted page: {} failing bound(s) over {} page(s), {} of \
         them named by the note that holds the page, {} not — {} note(s) naming measures none of \
         its pages fail, {} naming no measure at all, {} naming one and missing another.",
        report.holding,
        report.notes,
        report.bounds,
        report.pages,
        report.priced,
        report.unpriced(),
        report.on(Rung::Elsewhere),
        report.on(Rung::Silent),
        report.on(Rung::Partial),
    );
    name(
        "prints a line inside every bound once rounded to the two decimals the gate writes, so \
         its own line cannot say what its verdict rests on",
        &report.rounded,
    );
    name(
        "sits in no page-list note at all, so nobody is accounting for its bound",
        &report.unheld,
    );
    println!(
        "A note may name a bound and argue that the failure is not ours, which is a good note \
         and is counted as one — this sweep asks whether the bound is named, never whether the \
         argument persuades."
    );
    Ok(())
}

/// A population, named rather than counted.
///
/// **A count beside a list is not the list** (`doc/todo/02` §6): both of these are small
/// populations a round has to open, and the number 5 becoming 6 is not something anybody can
/// act on.
fn name(what: &str, pages: &std::collections::BTreeSet<String>) {
    if pages.is_empty() {
        println!("No contradicted page {what}.");
        return;
    }
    println!(
        "{} contradicted page(s), each of which {what}:",
        pages.len()
    );
    for page in pages {
        println!("    {page}");
    }
}

/// The gate's report: the file named on the command line, or standard input.
fn gate_report() -> Result<String, std::io::Error> {
    match std::env::args().nth(1) {
        Some(path) if path != "-" => std::fs::read_to_string(path),
        _ => {
            let mut text = String::new();
            std::io::stdin().read_to_string(&mut text)?;
            Ok(text)
        }
    }
}

/// One note's block: where it is, what it names, and every failing bound it does not.
fn print(finding: &Finding) {
    let named: Vec<&str> = finding
        .named
        .iter()
        .map(|measure| measure.as_str())
        .collect();
    println!(
        "    {} — {}:{} — names {}",
        finding.note.name,
        finding.note.file,
        finding.note.line,
        if named.is_empty() {
            "no measure".to_owned()
        } else {
            named.join(", ")
        }
    );
    for (measure, pages) in &finding.unpriced {
        println!(
            "        {} — failed by {} of this note's page(s) and named nowhere in it",
            measure.as_str(),
            pages.len()
        );
        for page in pages.iter().take(NAMED) {
            println!("            {page}");
        }
        let rest = pages.len().saturating_sub(NAMED);
        if rest > 0 {
            println!("            … and {rest} more");
        }
    }
}
