//! Sweeps the oracle's page-list notes for a figure the gate's own output contradicts.
//!
//! ```sh
//! cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture > oracle.log
//! cargo run --release -p conformance --bin quoted -- oracle.log
//! ```
//!
//! `doc/todo/01`'s twentieth sweep — the fifteenth of them to be a program, and the first whose
//! right-hand side is another gate's output rather than the tree's own sources.
//! [`conformance::quoted`] says what the discriminator is (a figure quoted in the gate's
//! vocabulary that no page of its own note carries), why the precision a figure is written to
//! tells one instrument from another, what each of the three rungs means and what the noise on
//! each looks like.
//!
//! **It renders nothing and re-measures nothing.** The oracle already prints all four figures for
//! every page it does not call agreement, so the left-hand side of this comparison is a report
//! the round has already run — seconds, over a file. Reading from standard input works too, which
//! is what a `|` after the gate line does.
//!
//! It prints one block per note, closest rung first, and under every contradicted figure what the
//! gate prints instead over that note's own pages, nearest value first — because a note is
//! corrected off the gate's output rather than by reasoning about it. It exits non-zero only
//! where it cannot read what it needs: a stale figure is a question for a person, not a build
//! failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::io::Read as _;
use std::process::ExitCode;

use conformance::entries;
use conformance::overtaken;
use conformance::quoted::{self, Finding, Rung};

/// How many of the gate's own values are offered under one contradicted figure.
const OFFERED: usize = 3;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("quoted: {error}");
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
    /// The report held no per-page line, so there is nothing to compare against.
    #[error(
        "no page line in the report — it is the output of `cargo test --profile gates -p \
         pdf-model --test oracle -- --ignored --nocapture` that is wanted, with `--nocapture`"
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

    let printed = quoted::report(&gate_report()?);
    if printed.is_empty() {
        return Err(Error::Empty);
    }

    let report = quoted::sweep(&notes, &printed);
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
        "{} note(s) of {}, {} quoting a figure the gate prints: {} figure(s) read, {} confirmed, \
         {} contradicted — {} beside a confirmed figure, {} written as the gate writes it, {} \
         written finer. {} figure(s) sit in a note the report names no page of.",
        report.quoting,
        report.notes,
        quoted::GATE_NOTES,
        report.quotations,
        report.confirmed,
        report.contradicted(),
        report.on(Rung::Line),
        report.on(Rung::Gate),
        report.on(Rung::Finer),
        report.unanchored,
    );
    println!(
        "The report named {} page(s). A note narrating its own correction quotes the figure it \
         supersedes, and another instrument's table borrows the gate's words — so a hit is a \
         reading list and not a verdict. Read the sentence before believing one.",
        report.pages
    );
    Ok(())
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

/// One note's block: where it is, and every figure under it the gate contradicts.
fn print(finding: &Finding) {
    println!(
        "    {} — {}:{}",
        finding.note.name, finding.note.file, finding.note.line
    );
    for entry in &finding.contradicted {
        println!(
            "        {}: `{}` — the gate prints no {} of {} on this note's pages",
            entry.quotation.line,
            entry.quotation.text,
            entry.quotation.measure.as_str(),
            entry.quotation.shown(),
        );
        for (page, printed) in entry.instead.iter().take(OFFERED) {
            println!(
                "            {} {} {} — {page}",
                printed.shown(entry.quotation.compared_at()),
                entry.quotation.measure.as_str(),
                printed.side.as_str(),
            );
        }
        let rest = entry.instead.len().saturating_sub(OFFERED);
        if rest > 0 {
            println!("            … and {rest} more, further away");
        }
    }
}
