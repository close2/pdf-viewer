//! How often a document asks for a mitre a rasteriser's own join code will not draw.
//!
//! ISO 32000-2 §8.4.3.5 bounds the ratio of mitre length to line width by the file's own `M`, and
//! `doc/todo/11` §6 found `tiny-skia`'s stroker refusing every ratio above 90.51 — an angle test
//! applied *before* the limit is read — so a file stating a large limit gets a bevel where its own
//! arithmetic asks for a spike. That defect has one witness in `pdf-differences` and none anybody
//! had counted, and the count is what says whether the construction that fixes it is worth its
//! risk: it is the population on which `render-cpu` and the two graphics-device backends drew
//! different pictures.
//!
//! For each document's first page this prints, over `Command::Stroke`s alone:
//!
//! - how many state §8.4.3.4's mitre join with a limit at or above the ratio the stroker refuses,
//!   which is the cheap pre-filter the fix itself uses;
//! - how many actually **have** such a join — the ratio `1 / sin(φ/2)` at or above 90.51 while at
//!   or under the file's own limit — and the sharpest ratio and mitre length each page states;
//! - how many of those the fix declines and why: a dash pattern, whose cuts decide where a join
//!   still exists, and a stroke at or under one device pixel, where the library draws a hairline
//!   with no joins at all and §10.7.4's own substitutions own the geometry.
//!
//! ```sh
//! cargo run --release -p pdf-model --example long_mitre_census              # curated
//! cargo run --release -p pdf-model --example long_mitre_census -- --pdfjs
//! cargo run --release -p pdf-model --example long_mitre_census -- --crawl   # CC-MAIN-2021-31
//! cargo run --release -p pdf-model --example long_mitre_census -- <file.pdf>...
//! ```
//!
//! **The three scopes are the six-hundred-and-eighty-sixth session's**, and they are what makes
//! the two declined shapes measurable at all: ADR 0398 recorded them as having "no witness" over
//! a population of 1441 first pages, before `CC-MAIN-2021-31` was on this disk, and a negative
//! decays when the population grows (ADR 0490). The control run is stated beside the crawl run
//! rather than merged with it.
//!
//! **A document whose interpretation panics is counted rather than fatal.** This is the only
//! census in this directory that runs `pdf_model::interpret`, and over sixty-five thousand crawled
//! files an instrument that dies on one of them measures nothing; the count is printed, because a
//! population an instrument could not reach is part of what it has to say (ADR 0493).
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: its output is the point, and every count below is bounded \
              by the number of path commands on one page"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Command, LineJoin};
use rayon::prelude::*;

/// The ratio `tiny-skia`'s stroker bevels above whatever the file's limit says — `render-cpu`'s
/// `BEVELLED_BY_THE_STROKER`, restated here because an example may not reach into a backend.
const REFUSED_ABOVE: f64 = 90.51;

/// How many witnessing pages are named before the list is truncated.
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures — the 1441
    /// first pages ADR 0398's sentence was measured over.
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

/// What one first page said about its mitre joins.
#[derive(Default)]
struct Page {
    /// Strokes whose join is a mitre and whose limit admits a ratio the stroker refuses.
    limits: usize,
    /// Of those, the ones whose geometry actually reaches such a ratio.
    long: usize,
    /// Of those, the ones the fix declines because the stroke is dashed.
    dashed: usize,
    /// Of those, the ones it declines because the stroke is at or under a device pixel.
    thin: usize,
    /// The sharpest ratio `1 / sin(φ/2)` this page states.
    sharpest: f64,
    /// The longest mitre this page states, in device pixels.
    longest: f64,
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let named: Vec<String> = arguments
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    let scope = if !named.is_empty() {
        Scope::Named
    } else if arguments.iter().any(|a| a == "--crawl") {
        Scope::Crawl
    } else if arguments.iter().any(|a| a == "--pdfjs") {
        Scope::PdfJs
    } else {
        Scope::Curated
    };

    let files = corpus(scope, &named);
    eprintln!("{} PDF(s) in the population", files.len());

    let measured: Vec<(String, Option<Page>)> = files
        .par_iter()
        .map(|path| {
            let label = path.to_string_lossy().into_owned();
            // A hostile file may panic somewhere under `interpret`; over this population that is a
            // count rather than the end of the run. See the module comment.
            let page = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(path)))
                .unwrap_or(None);
            (label, page)
        })
        .collect();

    report(&measured);
}

/// Prints the witnesses and then the totals, which is the order a reader wants them in.
fn report(measured: &[(String, Option<Page>)]) {
    let mut opened = 0_usize;
    let mut pages_with_a_large_limit = 0_usize;
    let mut pages_with_a_long_mitre = 0_usize;
    let mut strokes_with_a_large_limit = 0_usize;
    let mut strokes_with_a_long_mitre = 0_usize;
    let mut declined_dashed = 0_usize;
    let mut declined_thin = 0_usize;
    let mut witnesses: Vec<String> = Vec::new();
    let mut dashed_pages: Vec<String> = Vec::new();
    let mut thin_pages: Vec<String> = Vec::new();

    for (file, page) in measured {
        let Some(page) = page else {
            continue;
        };
        opened += 1;
        strokes_with_a_large_limit += page.limits;
        strokes_with_a_long_mitre += page.long;
        declined_dashed += page.dashed;
        declined_thin += page.thin;
        if page.limits > 0 {
            pages_with_a_large_limit += 1;
        }
        if page.long == 0 {
            continue;
        }
        pages_with_a_long_mitre += 1;
        if page.dashed > 0 {
            dashed_pages.push(file.clone());
        }
        if page.thin > 0 {
            thin_pages.push(file.clone());
        }
        witnesses.push(format!(
            "{file}: {} of {} strokes state a mitre over the ratio the stroker draws; sharpest \
             {:.3}, longest mitre {:.1} device pixels{}{}",
            page.long,
            page.limits,
            page.sharpest,
            page.longest,
            if page.dashed > 0 {
                format!(", {} dashed (declined)", page.dashed)
            } else {
                String::new()
            },
            if page.thin > 0 {
                format!(", {} at or under a device pixel (declined)", page.thin)
            } else {
                String::new()
            }
        ));
    }

    for line in witnesses.iter().take(MAX_NAMED) {
        println!("{line}");
    }
    if witnesses.len() > MAX_NAMED {
        println!("… and {} more", witnesses.len() - MAX_NAMED);
    }

    println!(
        "{opened} first pages of {} files: {pages_with_a_large_limit} state a mitre join whose \
         limit admits a ratio over {REFUSED_ABOVE}, {pages_with_a_long_mitre} actually have one",
        measured.len()
    );
    println!(
        "  strokes: {strokes_with_a_large_limit} with such a limit, {strokes_with_a_long_mitre} \
         with such a join, of which {declined_dashed} dashed and {declined_thin} at or under a \
         device pixel are declined"
    );
    println!(
        "  the dashed ones are on {} page(s): {}",
        dashed_pages.len(),
        names(&dashed_pages)
    );
    println!(
        "  the thin ones are on {} page(s): {}",
        thin_pages.len(),
        names(&thin_pages)
    );
    let unreached = measured.len() - opened;
    println!("  {unreached} file(s) this census could not reach at all");
}

/// A truncated list of page names.
fn names(pages: &[String]) -> String {
    if pages.len() > MAX_NAMED {
        format!(
            "{}, … and {} more",
            pages[..MAX_NAMED].join(", "),
            pages.len() - MAX_NAMED
        )
    } else {
        pages.join(", ")
    }
}

/// Interprets one document's first page and counts its mitre joins.
fn measure(path: &Path) -> Option<Page> {
    let bytes = std::fs::read(path).ok()?;
    let document = pdf_syntax::Document::open(bytes).ok()?;
    let page = pdf_model::Pages::new(&document).get(0)?;
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, 1.0, 1 << 30).ok()?;

    let mut counts = Page::default();
    for command in list.commands() {
        let Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } = command
        else {
            continue;
        };
        if stroke.join != LineJoin::Miter || f64::from(stroke.miter_limit) < REFUSED_ABOVE {
            continue;
        }
        counts.limits += 1;
        let at = transform.then(target.transform);
        let width = stroke.device_width(at);
        let ratio = pdf_render::sharpest_admitted_mitre(path, stroke).map_or(0.0, f64::from);
        if ratio < REFUSED_ABOVE {
            continue;
        }
        counts.long += 1;
        counts.sharpest = counts.sharpest.max(ratio);
        counts.longest = counts.longest.max(ratio * f64::from(width));
        if !stroke.dash_array.is_empty() {
            counts.dashed += 1;
        }
        if pdf_render::thinnest_line(at).is_some_and(|one_pixel| width <= one_pixel) {
            counts.thin += 1;
        }
    }
    Some(counts)
}
