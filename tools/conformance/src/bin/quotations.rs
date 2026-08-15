//! Sweeps this project's own prose for quotations that misquote the standard.
//!
//! ```sh
//! cargo run --release -p conformance --bin quotations
//! ```
//!
//! Two populations `doc/todo/48` named and nothing read. The first is every Markdown document
//! this project wrote — `doc/*.md`, `doc/todo/`, `doc/history/` and the ADRs. The second is
//! **the conformance ledger's own notes**, ADR 0249's population, which had been swept by hand
//! since the four-hundred-and-twelfth session and by no committed program at all: a hand sweep
//! is a sweep whose rule is retyped each time, and the rule is where the findings are.
//! [`conformance::prose`] says what a quotation is here, why this is a sweep rather than a
//! gate, and what the discriminator is; this binary is the invocation and the report.
//!
//! It prints every quotation that matches one of the specifications for at least
//! [`conformance::prose::MIN_MATCH`] words and at least half its length **and then diverges**,
//! with the standard's own continuation under it, and it ends with the population it read so
//! that a clean run says what it was clean over. It exits non-zero only where it cannot read
//! what it needs: a divergence is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::Path;
use std::process::ExitCode;

use conformance::ledger::Ledger;
use conformance::prose::{self, Conversion, Shape, Verdict};

/// Why the sweep could not read what it needs.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// A document, or the conversion, could not be read.
    #[error(transparent)]
    Prose(#[from] prose::Error),
    /// The ledger could not be read.
    #[error(transparent)]
    Ledger(#[from] conformance::ledger::LedgerError),
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("quotations: {error}");
            ExitCode::FAILURE
        }
    }
}

/// What one population's run added up to.
#[derive(Default)]
struct Tally {
    /// Quotations read.
    read: usize,
    /// Of those, found in a specification as written.
    verbatim: usize,
    /// Of those, matching one and then diverging.
    suspect: usize,
    /// How many were delimited by single quotes — the half no sweep could see until this one.
    ///
    /// Printed rather than derived, because a population that reports nothing is only evidence
    /// if the run says it was read at all.
    single: usize,
}

impl Tally {
    /// Judges one quotation and prints it where it diverges.
    fn judge(
        &mut self,
        conversion: &Conversion,
        file: &Path,
        line: usize,
        shape: Shape,
        quotation: &str,
    ) {
        self.read = self.read.saturating_add(1);
        if shape == Shape::Apostrophed {
            self.single = self.single.saturating_add(1);
        }
        match conversion.judge(quotation) {
            Verdict::Verbatim => self.verbatim = self.verbatim.saturating_add(1),
            Verdict::Unrelated => {}
            Verdict::Diverges {
                matched,
                total,
                standard,
            } => {
                self.suspect = self.suspect.saturating_add(1);
                report(file, line, &shape.to_string(), quotation);
                println!("    matched {matched} of {total} words, then diverged");
                println!("    standard: {standard}");
            }
        }
    }

    /// The quotations that share too little with any specification to be a quotation of one.
    fn unrelated(&self) -> usize {
        self.read
            .saturating_sub(self.verbatim)
            .saturating_sub(self.suspect)
    }
}

fn run() -> Result<(), Error> {
    let root = conformance::workspace_root();
    let conversion = Conversion::read(&root.join("doc/md"))?;
    let documents = prose::documents(&root.join("doc"))?;

    let mut prose_tally = Tally::default();
    for path in &documents {
        let text = std::fs::read_to_string(path).map_err(|source| prose::Error::Unreadable {
            path: path.clone(),
            source,
        })?;
        let shown = path.strip_prefix(&root).unwrap_or(path);
        for (line, shape, quotation) in prose::quotations(&text) {
            prose_tally.judge(&conversion, shown, line, shape, &quotation);
        }
    }

    // The ledger's notes. A row's note is one paragraph rather than a document, so it needs
    // none of `prose::quotations`'s block scanning — what it needs is the span rule, and the
    // line it reports is the row's, which is what `ledger.toml:NNN` means everywhere else.
    let ledger_path = root.join(conformance::LEDGER);
    let ledger = Ledger::read(&ledger_path)?;
    let shown = ledger_path.strip_prefix(&root).unwrap_or(&ledger_path);
    let mut ledger_tally = Tally::default();
    for row in &ledger.rows {
        let Some(note) = &row.note else {
            continue;
        };
        for (shape, quotation) in prose::quoted_spans(note) {
            ledger_tally.judge(&conversion, shown, row.line, shape, &quotation);
        }
    }

    println!();
    println!(
        "{} quotations in {} documents ({} single-quoted): {} verbatim in a specification, {} \
         matching one for at least {} words and then diverging, {} sharing too little with any \
         of them to be a quotation of one.",
        prose_tally.read,
        documents.len(),
        prose_tally.single,
        prose_tally.verbatim,
        prose_tally.suspect,
        prose::MIN_MATCH,
        prose_tally.unrelated()
    );
    println!(
        "{} quotations in {} ledger notes ({} single-quoted): {} verbatim, {} diverging, {} \
         unrelated.",
        ledger_tally.read,
        ledger.rows.iter().filter(|row| row.note.is_some()).count(),
        ledger_tally.single,
        ledger_tally.verbatim,
        ledger_tally.suspect,
        ledger_tally.unrelated()
    );
    println!(
        "A divergence is a question rather than a verdict: `doc/md/` is a conversion that breaks \
         words and drops content, so check the PDF in `doc/` with `pdftotext -layout` before \
         editing anything."
    );
    Ok(())
}

/// One finding's heading, wrapped so that a long quotation stays readable in a terminal.
fn report(file: &Path, line: usize, shape: &str, quotation: &str) {
    println!("{}:{line}: {shape}", file.display());
    let mut column = 0usize;
    print!("    quoted:  ");
    for word in quotation.split_whitespace() {
        if column > 0 && column.saturating_add(word.len()) > 84 {
            print!("\n             ");
            column = 0;
        }
        print!("{word} ");
        column = column.saturating_add(word.len()).saturating_add(1);
    }
    println!();
}
