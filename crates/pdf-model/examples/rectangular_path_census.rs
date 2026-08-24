//! How many fills state their region as *several* axis-aligned rectangles, and how many of those
//! put two of them in one device pixel.
//!
//! ISO 32000-2 §11.6.2 is the clause, and it is the one that decides whether such a path may be
//! drawn a rectangle at a time:
//!
//! > Single graphics objects, as defined in 8.2, "Graphics objects", shall be treated as
//! > elementary objects for transparency compositing purposes … Portions of an object shall not
//! > be composited with one another, even if they are described in a way that would seem to cause
//! > overlaps (such as a self-intersecting path, combined fill and stroke of a path, or a shading
//! > pattern containing an overlap or fold-over).
//!
//! So the population splits in two and only one half is reachable without a coverage buffer of the
//! renderer's own. Where the rectangles' device **pixel** footprints are pairwise disjoint, no
//! pixel receives two portions, nothing is composited with anything, and each portion may be
//! measured by §10.7.4's exact closed form — `pdf_render::rectangle_coverage`, ADR 0476 — instead
//! of by `tiny-skia`'s supersampled quarter. Where two portions do share a pixel, drawing them
//! separately would be exactly what the clause forbids, and the whole path keeps the one scan
//! conversion that resolves it today.
//!
//! `doc/todo/11` item 7's remainder is what this sizes: it named "a path stating several
//! rectangles" as the last shape carrying the quarter, and priced it as blocked on item 5's seam.
//! It is not — §11.3.7.3's union is what the standard says to do with two *objects* — and this
//! prints how much of the case the clause's own condition reaches. ADR 0583.
//!
//! Four counts per page, all over `Command::Fill` and all at the scale asked for:
//!
//! 1. **one rectangle** — the population ADR 0476 already measures exactly;
//! 2. **several, no two sharing a device pixel** — what the construction adds;
//! 3. **several, two sharing a device pixel** — what is left, and what a coverage buffer would
//!    cost something for;
//! 4. **several, but overlapping or above the budget** — declined by
//!    `pdf_render::device_rectangles`, and drawn as they are drawn today.
//!
//! ```sh
//! cargo run --release -p pdf-model --example rectangular_path_census -- <dir-or-file>...
//! cargo run --release -p pdf-model --example rectangular_path_census -- --scale 4 doc/pdf.js/test/pdfs
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

use pdf_render::Command;

/// A ceiling on the pages of one document, so that a malformed page tree cannot hold a worker.
const MAX_PAGES: usize = 4096;

/// The largest raster a page is measured against, in pixels.
const PIXEL_BUDGET: u64 = 1 << 30;

/// What one page's fills said.
#[derive(Default, Clone, Copy)]
struct Counts {
    /// Fill commands altogether, so every column has a denominator.
    fills: usize,
    /// Population 1: one axis-aligned rectangle, measured exactly since ADR 0476.
    one: usize,
    /// Population 2: several, and no two of them fall in one device pixel.
    separate: usize,
    /// Population 3: several, and two of them do.
    sharing: usize,
    /// Population 4: several rectangles that overlap, or more than the budget admits.
    declined: usize,
    /// The largest number of rectangles any single answered path stated.
    widest: usize,
}

impl Counts {
    fn absorb(&mut self, other: Self) {
        self.fills += other.fills;
        self.one += other.one;
        self.separate += other.separate;
        self.sharing += other.sharing;
        self.declined += other.declined;
        self.widest = self.widest.max(other.widest);
    }

    fn several(self) -> usize {
        self.separate + self.sharing
    }
}

/// The running totals over every file examined.
#[derive(Default)]
struct Tally {
    files: usize,
    opened: usize,
    pages: usize,
    pages_with_several: usize,
    documents_with_several: usize,
    counts: Counts,
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let all_pages = args.iter().any(|a| a == "--all-pages");
    args.retain(|a| a != "--all-pages");
    let mut scale = 1.0_f32;
    if let Some(at) = args.iter().position(|a| a == "--scale") {
        if let Some(value) = args.get(at + 1).and_then(|v| v.parse::<f32>().ok()) {
            scale = value;
        }
        args.drain(at..(at + 2).min(args.len()));
    }
    if args.is_empty() {
        println!("usage: rectangular_path_census [--all-pages] [--scale N] <dir-or-file>...");
        return;
    }

    let mut files = Vec::new();
    for root in &args {
        collect(Path::new(root), &mut files);
    }
    files.sort();

    let limit = if all_pages { MAX_PAGES } else { 1 };
    let mut total = Tally {
        files: files.len(),
        ..Tally::default()
    };
    for file in &files {
        examine(file, limit, scale, &mut total);
    }

    let c = total.counts;
    println!(
        "{} files, {} opened, {} pages read ({}) at scale {scale}",
        total.files,
        total.opened,
        total.pages,
        if all_pages { "every page" } else { "page one" }
    );
    println!("  {} fill commands altogether", c.fills);
    println!(
        "  one axis-aligned rectangle (exact since ADR 0476): {}",
        c.one
    );
    println!(
        "  several rectangles, none sharing a device pixel — §11.6.2 admits drawing each: {}",
        c.separate
    );
    println!(
        "  several rectangles, two sharing a device pixel — needs one coverage buffer: {}",
        c.sharing
    );
    println!(
        "  a rectangular subpath but declined — overlapping, mixed, or above the budget of {}: {}",
        pdf_render::RECTANGLES_PER_PATH,
        c.declined
    );
    println!(
        "  the widest path answered stated {} rectangles; {} pages of {} documents state a \
         multi-rectangle fill",
        c.widest, total.pages_with_several, total.documents_with_several
    );
}

/// Reads one document and folds its pages into `total`.
fn examine(file: &PathBuf, limit: usize, scale: f32, total: &mut Tally) {
    let Ok(bytes) = std::fs::read(file) else {
        return;
    };
    let Ok(document) = pdf_syntax::Document::open(bytes) else {
        return;
    };
    let pages = pdf_model::Pages::new(&document);
    total.opened += 1;

    let mut document_total = Counts::default();
    for index in 0..pages.len().min(limit) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        total.pages += 1;
        let list = pdf_model::interpret(&document, &page).display_list;
        let Ok(target) = pdf_render::TargetSpec::for_page(&list, scale, PIXEL_BUDGET) else {
            continue;
        };
        let mut counts = Counts::default();
        for command in list.commands() {
            walk(command, target.transform, &mut counts);
        }
        if counts.several() > 0 {
            total.pages_with_several += 1;
            println!(
                "{}: page {} — {} fills, {} one rectangle, {} several separate, {} several \
                 sharing a pixel, {} declined (widest {})",
                file.display(),
                index + 1,
                counts.fills,
                counts.one,
                counts.separate,
                counts.sharing,
                counts.declined,
                counts.widest,
            );
        }
        document_total.absorb(counts);
    }

    total.counts.absorb(document_total);
    if document_total.several() > 0 {
        total.documents_with_several += 1;
    }
}

/// Counts one command and, for a group, everything inside it.
fn walk(command: &Command, to_device: pdf_render::Transform, counts: &mut Counts) {
    match command {
        Command::Fill {
            path, transform, ..
        } => {
            counts.fills += 1;
            let at = transform.then(to_device);
            let Some(rectangles) = pdf_render::device_rectangles(path, at) else {
                // A path with a rectangular subpath that the decomposition still declined: two of
                // its rectangles overlap, one of its subpaths is not a rectangle, or it states
                // more than the budget admits.
                if path.narrowest_rectangle().is_some() {
                    counts.declined += 1;
                }
                return;
            };
            let stated = rectangles.iter().count();
            if stated < 2 {
                counts.one += 1;
                return;
            }
            counts.widest = counts.widest.max(stated);
            if rectangles.share_a_device_pixel() {
                counts.sharing += 1;
            } else {
                counts.separate += 1;
            }
        }
        Command::Group { commands, .. } => {
            for inner in commands {
                walk(inner, to_device, counts);
            }
        }
        Command::Shaped { object, shape } => {
            walk(object, to_device, counts);
            walk(shape, to_device, counts);
        }
        _ => {}
    }
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
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}
