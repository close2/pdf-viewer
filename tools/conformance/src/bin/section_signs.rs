//! Prints every `§` in another tree's instruction documents that follows another standard's name.
//!
//! ```sh
//! cargo run -q -p conformance --bin section_signs -- <root> [<file relative to root>...]
//! ```
//!
//! `tests/documents.rs` holds the `§` rule over the instruction documents of the tree it was
//! compiled in, and a batch worktree compiles it over the worktree. The main checkout holds
//! instruction documents no merge carries — a question file written beside an answer and left
//! untracked there — and the rule binds them as it binds a committed one (ADR 1452), so the main
//! checkout's own `cargo test -p conformance` fails on one while every worktree run passes. This
//! reads the population [`conformance::documents::instructions`] names under `<root>`, narrowed to
//! the files given when any are, through the same scanner, and never writes. `tools/main-checkout.py`
//! runs it over the main checkout's untracked files (ADR 1440).
//!
//! It prints one line per finding in the test's own words and a count, and exits non-zero only
//! where `<root>` or a document cannot be read: a finding is a reading list for the owner, whose
//! files a round may not edit.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(root) = arguments.next().map(PathBuf::from) else {
        eprintln!("section_signs: usage: section_signs <root> [<file relative to root>...]");
        return ExitCode::FAILURE;
    };
    let only: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
    match run(&root, &only) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("section_signs: {}: {error}", root.display());
            ExitCode::FAILURE
        }
    }
}

fn run(root: &std::path::Path, only: &[PathBuf]) -> std::io::Result<()> {
    let mut documents = 0usize;
    let mut findings = 0usize;
    for relative in conformance::documents::instructions(root)? {
        if !only.is_empty() && !only.contains(&relative) {
            continue;
        }
        documents = documents.saturating_add(1);
        let text = std::fs::read_to_string(root.join(&relative))?;
        for foreign in conformance::citation::scan_prose(&text).foreign {
            findings = findings.saturating_add(1);
            println!(
                "  {}:{}: a `\u{a7}` after {} — write \"{} section N\"",
                relative.display(),
                foreign.line,
                foreign.document,
                foreign.document
            );
        }
    }
    println!(
        "{findings} `\u{a7}` after another standard's name in {documents} instruction document(s)"
    );
    Ok(())
}
