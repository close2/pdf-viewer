//! What a document asks §10.7.5 for, and which half of the clause reaches a stroke.
//!
//! ISO 32000-2 §10.7.5 states two requirements, both conditional on the stroke adjustment
//! parameter Table 51 initialises to `false`:
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness. The
//! > thickness shall be as near as possible to the requested line width -no more than half a
//! > pixel different.
//!
//! > If stroke adjustment is enabled and the requested line width, transformed into device
//! > space, is less than half a pixel, the stroke shall be rendered as a single-pixel line.
//!
//! `Stroke::device_width` implements the second. The first is a grid-fitting of a stroke's
//! *coordinates*, and this census is the population question that decision needs: **how many
//! strokes are actually drawn with the parameter enabled, and which of them could a grid fit
//! move at all.** `examples/absence_audit`'s §10.7.5 block counts the documents that *state*
//! `/SA true` in a dictionary; this one counts what reaches the display list, which is a
//! different and smaller number — a state can be set and never used, set inside a `q` that is
//! popped before anything is stroked, or set on a page that strokes nothing.
//!
//! For each document's first page, over `Command::Stroke` alone, it prints:
//!
//! - **enabled** — strokes carrying `/SA true` at the moment they are painted;
//! - **promoted** — of those, the ones under half a device pixel, where the clause's second
//!   requirement already fires and the first has nothing left to adjust;
//! - **axis-aligned** — of the rest, the ones whose every segment lies along a device axis,
//!   which is the only shape a grid fit is defined for: a diagonal or a curve has no pair of
//!   edges to put on integers, and the one reference that grid-fits leaves both alone;
//! - **off-grid** — of those, the ones whose two device-space edges are not already on pixel
//!   boundaries, which is the population a grid fit would actually move.
//!
//! The four are nested, so the last is the answer to "how many strokes would change" and every
//! earlier column says how much of the population each condition removed.
//!
//! ```sh
//! cargo run --release -p pdf-model --example stroke_adjustment_census              # curated
//! cargo run --release -p pdf-model --example stroke_adjustment_census -- --pdfjs
//! cargo run --release -p pdf-model --example stroke_adjustment_census -- --crawl
//! cargo run --release -p pdf-model --example stroke_adjustment_census -- <file.pdf>...
//! ```
//!
//! The scopes are `long_mitre_census`'s, and for its reason: a claim about a population decays
//! when the population grows (ADR 0490), so a run over `doc/pdf.js` — the corpus every gate in
//! `doc/todo/02` §2 walks — is stated beside a run over the crawl rather than instead of one.
//! A document whose interpretation panics is counted rather than fatal, for the same reason
//! that census gives.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: its output is the point, and every count below is bounded \
              by the number of path commands on one page"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Command, PathCommand, Point, Transform};
use rayon::prelude::*;

/// How close to a whole number a device coordinate must be to count as *on* the pixel grid.
///
/// §10.7.4 puts pixel boundaries on integers, so an edge within this of one is already where a
/// grid fit would put it. A twentieth of a pixel is far below anything a reader could see and
/// far above the error of an `f32` page transform.
const ON_THE_GRID: f32 = 0.05;

/// How close to a device axis a segment must lie to count as axis-aligned.
///
/// In device pixels of departure over the segment's own length, so it is a slope rather than an
/// angle: a segment 400 pixels long may wander this far and still be a rule a grid fit could
/// move without visibly turning it.
const ALONG_AN_AXIS: f32 = 0.5;

/// How many witnessing pages are named before the list is truncated.
const MAX_NAMED: usize = 40;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone — the population `doc/todo/02` §2's gates walk.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures.
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

/// What one first page said about stroke adjustment.
#[derive(Default)]
struct Page {
    /// Strokes on the page, whatever their state.
    strokes: usize,
    /// Of those, the ones painted with the parameter enabled.
    enabled: usize,
    /// Of those, the ones the clause's second requirement already promotes.
    promoted: usize,
    /// Of the rest, the ones whose every segment lies along a device axis.
    axis_aligned: usize,
    /// Of those, the ones whose device-space edges are not already on the pixel grid.
    off_grid: usize,
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
            // A hostile file may panic somewhere under `interpret`; over this population that is
            // a count rather than the end of the run. See the module comment.
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
    let mut pages_enabled = 0_usize;
    let mut pages_movable = 0_usize;
    let mut strokes = 0_usize;
    let mut enabled = 0_usize;
    let mut promoted = 0_usize;
    let mut axis_aligned = 0_usize;
    let mut off_grid = 0_usize;
    let mut witnesses: Vec<String> = Vec::new();

    for (file, page) in measured {
        let Some(page) = page else {
            continue;
        };
        opened += 1;
        strokes += page.strokes;
        enabled += page.enabled;
        promoted += page.promoted;
        axis_aligned += page.axis_aligned;
        off_grid += page.off_grid;
        if page.enabled == 0 {
            continue;
        }
        pages_enabled += 1;
        if page.off_grid > 0 {
            pages_movable += 1;
        }
        witnesses.push(format!(
            "{file}: {} of {} strokes under /SA true — {} promoted by the second requirement, \
             {} axis-aligned, {} of those off the grid",
            page.enabled, page.strokes, page.promoted, page.axis_aligned, page.off_grid
        ));
    }

    for line in witnesses.iter().take(MAX_NAMED) {
        println!("{line}");
    }
    if witnesses.len() > MAX_NAMED {
        println!("… and {} more", witnesses.len() - MAX_NAMED);
    }

    println!(
        "{opened} first pages of {} files: {pages_enabled} paint a stroke with /SA enabled, \
         {pages_movable} of them a stroke a grid fit could move",
        measured.len()
    );
    println!(
        "  strokes: {strokes} in all, {enabled} under /SA, {promoted} promoted to one device \
         pixel, {axis_aligned} axis-aligned and not promoted, {off_grid} of those off the grid"
    );
    let unreached = measured.len() - opened;
    println!("  {unreached} file(s) this census could not reach at all");
}

/// Interprets one document's first page and counts what §10.7.5 reaches.
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
        counts.strokes += 1;
        if !stroke.adjust {
            continue;
        }
        counts.enabled += 1;
        let at = transform.then(target.transform);
        let Some(one_pixel) = pdf_render::thinnest_line(at) else {
            continue;
        };
        // The clause's second requirement, asked exactly as `Stroke::device_width` asks it.
        if stroke.width < 0.5 * one_pixel {
            counts.promoted += 1;
            continue;
        }
        let Some(axis) = single_axis(path, at) else {
            continue;
        };
        counts.axis_aligned += 1;
        // The stroke's two edges in device space, half a device width either side of the rule.
        let half = stroke.device_width(at) / one_pixel / 2.0;
        if !on_the_grid(axis - half) || !on_the_grid(axis + half) {
            counts.off_grid += 1;
        }
    }
    Some(counts)
}

/// Whether a device coordinate is already where a grid fit would put it.
fn on_the_grid(at: f32) -> bool {
    let fraction = at - at.round();
    fraction.abs() <= ON_THE_GRID
}

/// The one device-space coordinate every segment of `path` shares, or `None`.
///
/// A grid fit adjusts a stroke's coordinates onto pixel boundaries, and that is only defined
/// where the stroke *has* a pair of edges parallel to the grid: one straight run along a device
/// axis. A path with a curve, a turn, or two runs at different offsets is not such a shape, and
/// the reference that grid-fits leaves every one of them where the document put it.
fn single_axis(path: &pdf_render::Path, at: Transform) -> Option<f32> {
    let mut from: Option<Point> = None;
    let mut start: Option<Point> = None;
    let mut horizontal: Option<f32> = None;
    let mut vertical: Option<f32> = None;
    for command in path.commands() {
        let (a, b) = match *command {
            PathCommand::MoveTo(p) => {
                from = Some(at.apply(p));
                start = from;
                continue;
            }
            PathCommand::LineTo(p) => {
                let a = from?;
                let b = at.apply(p);
                from = Some(b);
                (a, b)
            }
            PathCommand::Close => {
                let a = from?;
                let b = start?;
                from = Some(b);
                (a, b)
            }
            // A curve has no pair of straight edges to put on the grid.
            PathCommand::CurveTo(..) => return None,
        };
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        if dx.abs() <= ALONG_AN_AXIS && dy.abs() <= ALONG_AN_AXIS {
            // A segment of no length states no direction; it is neither run nor turn.
            continue;
        }
        if dx.abs() <= ALONG_AN_AXIS {
            if vertical.is_some_and(|x| (x - a.x).abs() > ON_THE_GRID) || horizontal.is_some() {
                return None;
            }
            vertical = Some(a.x);
        } else if dy.abs() <= ALONG_AN_AXIS {
            if horizontal.is_some_and(|y| (y - a.y).abs() > ON_THE_GRID) || vertical.is_some() {
                return None;
            }
            horizontal = Some(a.y);
        } else {
            return None;
        }
    }
    vertical.or(horizontal)
}
