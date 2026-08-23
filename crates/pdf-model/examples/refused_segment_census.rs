//! How many real first pages state ISO 32000-2 §8.5.2.1's error, asked of the *interpreter*.
//!
//! > Most operators that add a segment to the current path start at the current point; if the
//! > current point is undefined, an error shall be generated.
//!
//! `examples/operator_shape_census` answers this question over a **token stream**: it lexes a
//! page's content and counts an `l`, `c`, `v` or `y` keyword with no `m` or `re` before it. That
//! is the right instrument for the row's negative and it is the wrong one for the row's *cost*,
//! because the interpreter asks a second thing the lexer does not: an operator only runs when its
//! operands are numbers. `issue6342.pdf` is the whole of the difference — the one curated first
//! page the token census names, whose form `XObject` the file itself titles a form with errors —
//! and every `c` it writes after the error has operands the lexer splits into keywords,
//! so not one of them ever reaches a path. The token count is an upper bound on this one and the
//! six-hundred-and-ninety-sixth session's figures should be read as one (ADR 0563).
//!
//! So this census counts what the display list actually loses: `Unsupported::UndefinedCurrentPoint`
//! over one first page per document, beside the number of paths that page paints, so that a zero
//! is legible as "no page does this" rather than as "nothing was interpreted".
//!
//! ```sh
//! cargo run --release -p pdf-model --example refused_segment_census              # curated
//! cargo run --release -p pdf-model --example refused_segment_census -- --pdfjs
//! cargo run --release -p pdf-model --example refused_segment_census -- --crawl   # CC-MAIN-2021-31
//! cargo run --release -p pdf-model --example refused_segment_census -- <file.pdf>...
//! ```
//!
//! The three scopes are `examples/long_mitre_census`'s, for ADR 0490's reason: a negative decays
//! when the population grows, so the control run is stated beside the crawl run rather than merged
//! with it. **A document whose interpretation panics is counted rather than fatal**, for ADR 0493's
//! reason: over sixty-five thousand crawled files an instrument that dies on one of them measures
//! nothing, and a population an instrument could not reach is part of what it has to say.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: its output is the point, and every count below is bounded \
              by the number of path commands on one page"
)]

use std::path::{Path, PathBuf};

use pdf_render::Command;
use rayon::prelude::*;

/// How many witnessing pages are named before the list is truncated.
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone.
    PdfJs,
    /// That, the `doc/corpora/` submodules, and this project's own fixtures.
    Curated,
    /// The `SafeDocs` `CC-MAIN-2021-31` crawl under `corpus-cache/`, and nothing else.
    Crawl,
    /// Whatever files the command line named.
    Named,
}

/// Every PDF this census measures over, in the scope asked for.
fn corpus(scope: Scope, named: &[String]) -> Vec<PathBuf> {
    if scope == Scope::Named {
        return named.iter().map(PathBuf::from).collect();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let roots: &[&str] = match scope {
        Scope::PdfJs => &["doc/pdf.js/test/pdfs"],
        Scope::Curated => &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"],
        Scope::Crawl => &["corpus-cache/safedocs/cc-main-2021-31"],
        Scope::Named => &[],
    };
    for relative in roots {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
///
/// `is_dir` and `read_dir` both follow symbolic links, which is not incidental: a parallel
/// worktree reaches the corpora through symlinks, and a walk that did not follow them would
/// report the emptiest possible false zero.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
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

/// What one first page said.
struct Page {
    /// Segments §8.5.2.1 refused for having no current point.
    refused: usize,
    /// Paths the page paints, which is the control: a page that paints none says nothing about
    /// this clause whatever its `refused` is.
    painted: usize,
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let named: Vec<String> = arguments
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    let scope = if named.is_empty() {
        if arguments.iter().any(|a| a == "--crawl") {
            Scope::Crawl
        } else if arguments.iter().any(|a| a == "--pdfjs") {
            Scope::PdfJs
        } else {
            Scope::Curated
        }
    } else {
        Scope::Named
    };

    let files = corpus(scope, &named);
    eprintln!("{} PDF(s) in the population", files.len());

    let measured: Vec<(String, Option<Page>)> = files
        .par_iter()
        .map(|path| {
            let label = path.to_string_lossy().into_owned();
            let page = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path)))
                .unwrap_or(None);
            (label, page)
        })
        .collect();

    report(&measured);
}

/// Prints the witnesses and then the totals.
fn report(measured: &[(String, Option<Page>)]) {
    let mut opened = 0_usize;
    let mut painted = 0_usize;
    let mut refused = 0_usize;
    let mut witnesses: Vec<String> = Vec::new();

    for (file, page) in measured {
        let Some(page) = page else {
            continue;
        };
        opened += 1;
        painted += page.painted;
        refused += page.refused;
        if page.refused > 0 {
            witnesses.push(format!(
                "{file}: {} segment(s) with no current point, beside {} painted path(s)",
                page.refused, page.painted
            ));
        }
    }

    for line in witnesses.iter().take(MAX_NAMED) {
        println!("{line}");
    }
    if witnesses.len() > MAX_NAMED {
        println!("… and {} more", witnesses.len() - MAX_NAMED);
    }
    println!(
        "{opened} first pages of {} files: {} page(s) refuse a segment for having no current \
         point, {refused} segment(s) in all, over {painted} painted path(s)",
        measured.len(),
        witnesses.len()
    );
    println!(
        "  {} file(s) this census could not reach at all",
        measured.len() - opened
    );
}

/// Interprets one document's first page.
fn measure(path: &Path) -> Option<Page> {
    let bytes = std::fs::read(path).ok()?;
    let document = pdf_syntax::Document::open(bytes).ok()?;
    let page = pdf_model::Pages::new(&document).get(0)?;
    let interpretation = pdf_model::interpret(&document, &page);
    let refused = interpretation
        .unsupported
        .iter()
        .find_map(|report| match report {
            pdf_model::Unsupported::UndefinedCurrentPoint { segments } => Some(*segments),
            _ => None,
        })
        .unwrap_or(0);
    Some(Page {
        refused,
        painted: painted_paths(interpretation.display_list.commands()),
    })
}

/// How many `Fill` and `Stroke` commands a list holds, groups included.
fn painted_paths(commands: &[Command]) -> usize {
    commands
        .iter()
        .map(|command| match command {
            Command::Fill { .. } | Command::Stroke { .. } => 1,
            Command::Group { commands, .. } => painted_paths(commands),
            _ => 0,
        })
        .sum()
}
