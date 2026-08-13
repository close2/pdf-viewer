//! Sweeps this project's own Markdown documents for quotations that misquote the standard.
//!
//! ```sh
//! cargo run --release -p conformance --bin quotations
//! ```
//!
//! The population `doc/todo/48` names and nothing reads: `doc/*.md`, `doc/todo/`, `doc/history/`
//! and the ADRs. [`conformance::prose`] says what a quotation is here, why this is a sweep
//! rather than a gate, and what the discriminator is; this binary is the invocation and the
//! report.
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

use conformance::prose::{self, Conversion, Verdict};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("quotations: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), prose::Error> {
    let root = conformance::workspace_root();
    let conversion = Conversion::read(&root.join("doc/md"))?;
    let documents = prose::documents(&root.join("doc"))?;

    let mut read = 0usize;
    let mut verbatim = 0usize;
    let mut suspect = 0usize;
    for path in &documents {
        let text = std::fs::read_to_string(path).map_err(|source| prose::Error::Unreadable {
            path: path.clone(),
            source,
        })?;
        let shown = path.strip_prefix(&root).unwrap_or(path);
        for (line, shape, quotation) in prose::quotations(&text) {
            read = read.saturating_add(1);
            match conversion.judge(&quotation) {
                Verdict::Verbatim => verbatim = verbatim.saturating_add(1),
                Verdict::Unrelated => {}
                Verdict::Diverges {
                    matched,
                    total,
                    standard,
                } => {
                    suspect = suspect.saturating_add(1);
                    report(shown, line, &shape.to_string(), &quotation);
                    println!("    matched {matched} of {total} words, then diverged");
                    println!("    standard: {standard}");
                }
            }
        }
    }

    println!();
    println!(
        "{read} quotations in {} documents: {verbatim} verbatim in a specification, {suspect} \
         matching one for at least {} words and then diverging, {} sharing too little with any \
         of them to be a quotation of one.",
        documents.len(),
        prose::MIN_MATCH,
        read.saturating_sub(verbatim).saturating_sub(suspect)
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
