//! How many documents paint under a matrix that has no inverse, and what it costs them.
//!
//! ISO 32000-2 §8.3.4's third NOTE is the clause this counts against:
//!
//! > When rendering graphics objects, it is sometimes necessary for a PDF reader to perform the
//! > inverse of a transformation -that is, to find the user space coordinates that correspond to
//! > a given pair of device space coordinates. Not all transformations are invertible, however.
//! > For example, if a matrix contains a, b, c, and d elements that are all zero, all user
//! > coordinates map to the same device coordinates and there is no unique inverse
//! > transformation. Such noninvertible transformations are not very useful and generally arise
//! > from unintended operations, such as scaling by 0. Use of a noninvertible matrix when
//! > painting graphics objects can result in unpredictable behaviour.
//!
//! `doc/todo/11` item 8 found that all three backends answered such a matrix by refusing the
//! **whole raster**, so the 282 commands `4605705.pdf`'s page did draw went with the one it could
//! not. Whether that is worth a round is a population question, and this is the instrument for it:
//! trap 11 says derive the condition from the clause and print what it matched.
//!
//! Three nested populations, because the outer one decides nothing on its own:
//!
//! 1. **A marking command under a noninvertible matrix** — `Fill`, `Stroke` or `Image`. This is
//!    the clause's own condition and it is the set on which the standard states no behaviour.
//! 2. **Of those, the ones a backend refused the page for**: a `Fill` or a `Stroke`, whatever its
//!    paint, because `render-cpu` and `render-gpu` inverted the command's transform before they
//!    looked at what the paint was and `render-quorra` resolved a stroke width through a stretch of
//!    zero. An `Image` reached none of those, and drew nothing because it had no area.
//! 3. **Of those, the ones whose paint would genuinely have needed an inverse**: a
//!    `Paint::Shading`, which the two library backends position in the path's own space (trap 2). A
//!    `Paint::Solid` is the same colour at every point, so no space positions it and the refusal
//!    bought nothing at all.
//!
//! The device area of every command in population 1 is **zero**, whichever paint it carries: a
//! page transform is invertible, so a singular command transform makes the device transform
//! singular and the whole path lands on a line or a point. So the third column is what a reader
//! could ever have lost by the mark being refused, and the second is what a reader did lose — the
//! rest of the page.
//!
//! ```sh
//! cargo run --release -p pdf-model --example singular_transform_census -- <dir-or-file>...
//! cargo run --release -p pdf-model --example singular_transform_census -- --all-pages doc/pdf.js/test/pdfs
//! ```
//!
//! One process per directory is the surveys' own method and the reason is theirs: this parses
//! hostile input under `panic = "abort"`, so one document's abort would take every verdict with
//! it.
#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: its output is the point, and every count is bounded by the \
              number of commands on one page"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Command, Paint, Transform};

/// How many pages of one document are looked at when `--all-pages` is not given.
const FIRST_PAGE_ONLY: usize = 1;

/// A ceiling on the pages of one document, so that a malformed page tree cannot hold a worker.
const MAX_PAGES: usize = 4096;

/// What one page's commands said.
#[derive(Default, Clone, Copy)]
struct Page {
    /// Population 1: marking commands under a matrix with no inverse.
    unpositionable: usize,
    /// Population 2: of those, fills and strokes — what a backend refused the page for.
    refused_today: usize,
    /// Population 3: of those, the ones carrying a shading, which needs the inverse.
    shaded: usize,
    /// Commands on the page altogether, so that "what the refusal cost" has a denominator.
    commands: usize,
}

impl Page {
    fn absorb(&mut self, other: Self) {
        self.unpositionable += other.unpositionable;
        self.refused_today += other.refused_today;
        self.shaded += other.shaded;
        self.commands += other.commands;
    }
}

/// The running totals over every file examined.
#[derive(Default)]
struct Tally {
    files: usize,
    opened: usize,
    pages: usize,
    pages_matching: usize,
    documents_matching: usize,
    documents_refused: usize,
    documents_shaded: usize,
    marks: Page,
    /// Commands on the pages that matched, which is what the whole-raster refusal threw away.
    commands_on_matching_pages: usize,
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let all_pages = args.iter().any(|a| a == "--all-pages");
    args.retain(|a| a != "--all-pages");
    if args.is_empty() {
        println!("usage: singular_transform_census [--all-pages] <dir-or-file>...");
        return;
    }

    let mut files = Vec::new();
    for root in &args {
        collect(Path::new(root), &mut files);
    }
    files.sort();

    let limit = if all_pages {
        MAX_PAGES
    } else {
        FIRST_PAGE_ONLY
    };
    let mut total = Tally {
        files: files.len(),
        ..Tally::default()
    };
    for file in &files {
        examine(file, limit, &mut total);
    }

    println!(
        "{} files, {} opened, {} pages read ({})",
        total.files,
        total.opened,
        total.pages,
        if all_pages { "every page" } else { "page one" }
    );
    println!(
        "  §8.3.4 NOTE 3, a marking command under a matrix with no inverse: \
         {} commands on {} pages of {} documents",
        total.marks.unpositionable, total.pages_matching, total.documents_matching,
    );
    println!(
        "  of those, fills and strokes — the page a backend refused: {} commands in {} \
         documents, on pages stating {} commands altogether",
        total.marks.refused_today, total.documents_refused, total.commands_on_matching_pages,
    );
    println!(
        "  of those, carrying a shading, which is the only paint the inverse positions: \
         {} commands in {} documents",
        total.marks.shaded, total.documents_shaded,
    );
}

/// Reads one document and folds its pages into `total`.
fn examine(file: &PathBuf, limit: usize, total: &mut Tally) {
    let Ok(bytes) = std::fs::read(file) else {
        return;
    };
    let Ok(document) = pdf_syntax::Document::open(bytes) else {
        return;
    };
    let pages = pdf_model::Pages::new(&document);
    total.opened += 1;

    let mut document_total = Page::default();
    let mut matched_pages = 0_usize;
    let mut commands_on_matching = 0_usize;
    for index in 0..pages.len().min(limit) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        total.pages += 1;
        let list = pdf_model::interpret(&document, &page).display_list;
        let mut counts = Page::default();
        for command in list.commands() {
            walk(command, &mut counts);
        }
        if counts.unpositionable > 0 {
            matched_pages += 1;
            commands_on_matching += counts.commands;
            println!(
                "{}: page {} states {} marking command(s) under a matrix with no inverse \
                 ({} fill/stroke, {} shaded) of {} commands",
                file.display(),
                index + 1,
                counts.unpositionable,
                counts.refused_today,
                counts.shaded,
                counts.commands,
            );
        }
        document_total.absorb(counts);
    }

    total.marks.absorb(document_total);
    total.pages_matching += matched_pages;
    total.commands_on_matching_pages += commands_on_matching;
    if document_total.unpositionable > 0 {
        total.documents_matching += 1;
    }
    if document_total.refused_today > 0 {
        total.documents_refused += 1;
    }
    if document_total.shaded > 0 {
        total.documents_shaded += 1;
    }
}

/// Counts one command and, for a group, everything inside it.
fn walk(command: &Command, counts: &mut Page) {
    counts.commands += 1;
    match command {
        Command::Fill {
            transform, paint, ..
        }
        | Command::Stroke {
            transform, paint, ..
        } => {
            if singular(*transform) {
                counts.unpositionable += 1;
                counts.refused_today += 1;
                if matches!(paint, Paint::Shading(_)) {
                    counts.shaded += 1;
                }
            }
        }
        Command::Image { transform, .. } => {
            if singular(*transform) {
                counts.unpositionable += 1;
            }
        }
        Command::Group { commands, .. } => {
            for inner in commands {
                walk(inner, counts);
            }
        }
        Command::Shaped { object, shape } => {
            walk(object, counts);
            walk(shape, counts);
        }
        _ => {}
    }
}

/// The clause's own condition: a matrix with no unique inverse.
fn singular(transform: Transform) -> bool {
    transform.invert().is_none()
}

/// Every `.pdf` under `root`, or `root` itself if it is a file.
fn collect(root: &Path, into: &mut Vec<PathBuf>) {
    if root.is_file() {
        into.push(root.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|e| e == "pdf") {
            into.push(path);
        }
    }
}
