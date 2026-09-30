//! Which documents reach `MAX_FORM_DEPTH`, and whether each one reached it on a chain that
//! re-enters itself or on a chain of distinct streams.
//!
//! The instrument behind ADR 1411. ADR 0793 set the bound at 64 from what one level of the
//! costliest kind of nested stream costs in stack, and re-ran its witnesses to show that the
//! value drew them; what it could not say of a population is how many of those that still reach
//! the bound are cycles — the report was one sentence for both. The interpreter now says which
//! (`Unsupported::NestingCycle` against `Unsupported::LimitReached { limit: "MAX_FORM_DEPTH" }`),
//! so a population's page ones answer the question the bound's value rests on: **a document on
//! the second list is a finite nesting this bound cut short**, and one on the first is a chain no
//! value would finish.
//!
//! ```sh
//! cargo run --release -p pdf-model --example nesting_census -- DIR_OR_FILE…
//! ```
//!
//! Page one of each document, in parallel, a panic counted rather than fatal: a census over a
//! crawl of hostile files that dies on one of them measures nothing.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a measurement whose output is its purpose"
)]

use std::path::{Path, PathBuf};

use pdf_model::Unsupported;
use pdf_syntax::Document;
use rayon::prelude::*;

/// Every `.pdf` under one path, recursively, following symbolic links as `is_dir` does.
fn collect(path: &Path, into: &mut Vec<PathBuf>) {
    if path.is_file() {
        into.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// What page one said about the nesting bound.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reached {
    /// It did not reach it.
    No,
    /// It reached it on a chain that re-enters a stream it is running.
    Cycle,
    /// It reached it on a chain of distinct streams.
    Depth,
    /// It did not open, had no page one, or panicked.
    Unread,
}

fn measure(path: &Path) -> Reached {
    let Ok(bytes) = std::fs::read(path) else {
        return Reached::Unread;
    };
    let Ok(document) = Document::open(bytes) else {
        return Reached::Unread;
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        return Reached::Unread;
    };
    let interpretation = pdf_model::interpret(&document, &page);
    let depth = interpretation.unsupported.iter().any(|report| {
        matches!(
            report,
            Unsupported::LimitReached {
                limit: "MAX_FORM_DEPTH"
            }
        )
    });
    let cycle = interpretation
        .unsupported
        .iter()
        .any(|report| matches!(report, Unsupported::NestingCycle { .. }));
    match (cycle, depth) {
        // A page may hold both; the finite one is the one the value is about.
        (_, true) => Reached::Depth,
        (true, false) => Reached::Cycle,
        (false, false) => Reached::No,
    }
}

fn main() {
    let mut files = Vec::new();
    for argument in std::env::args().skip(1) {
        collect(Path::new(&argument), &mut files);
    }
    files.sort();
    files.dedup();
    eprintln!("{} PDF(s) in the population", files.len());
    let measured: Vec<(PathBuf, Reached)> = files
        .par_iter()
        .map(|path| {
            let reached = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path)))
                .unwrap_or(Reached::Unread);
            (path.clone(), reached)
        })
        .collect();
    let count = |kind: Reached| measured.iter().filter(|(_, got)| *got == kind).count();
    println!(
        "{} documents: {} unread, {} reach MAX_FORM_DEPTH on a chain that re-enters itself, {} on \
         a chain of distinct streams",
        measured.len(),
        count(Reached::Unread),
        count(Reached::Cycle),
        count(Reached::Depth)
    );
    for (path, reached) in &measured {
        let word = match reached {
            Reached::Cycle => "cycle",
            Reached::Depth => "depth",
            Reached::No | Reached::Unread => continue,
        };
        println!("{word}\t{}", path.display());
    }
}
