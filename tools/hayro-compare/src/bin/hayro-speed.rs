//! How long we take to put a page on screen, against how long `hayro` takes.
//!
//! ```text
//! cargo run --release -p hayro-compare --bin hayro-speed -- [--scale N] [--repeats N]
//!     [--per-document] <file.pdf>...
//! ```
//!
//! # Why this comparison is fair, and where it is not
//!
//! Every other renderer this project measures itself against is C, so a timing difference
//! confounds the language, the allocator, the build flags and thirty years of tuning.
//! `hayro` is Rust, forbids unsafe as we do, rasterises on the CPU as we do, and is
//! single-threaded here as we are. What is left when you subtract all that is the thing
//! worth measuring.
//!
//! What is measured is **open, interpret and rasterise page one** — time to first page,
//! which `CLAUDE.md` names as the number a user judges a viewer by. Reading the file is
//! excluded from both sides; encoding a PNG is excluded from both sides.
//!
//! Three honest caveats:
//!
//! - **We draw less.** A page we report as incomplete is a page we did not finish drawing,
//!   and finishing it would cost more. Only pages we claim to draw completely are counted
//!   in the headline, and the others are counted separately so the gap is visible.
//! - **The rasterisers differ.** `tiny-skia` against `vello_cpu` is part of what is being
//!   measured and not a variable either project controls independently of the other.
//! - **Wall clock lies under load.** The handover records a change that measured as a 24%
//!   regression and an 8.5% improvement twenty minutes apart. Both sides are therefore
//!   measured in the same process, alternating, in one sitting, and `--repeats` takes the
//!   best of several passes per document so a scheduling hiccup lands on one sample rather
//!   than on one renderer.

#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a measurement tool: its output is its purpose"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget per page, the same one the corpus gate uses.
const PIXEL_BUDGET: u64 = 64 << 20;

/// What one invocation was asked to do.
struct Options {
    scale: f32,
    repeats: usize,
    /// Print one tab-separated line per document as well as the summary.
    ///
    /// The summary compares two renderers because two are linked in. A *third* renderer —
    /// one that is a separate program rather than a crate — can only be compared by running
    /// it separately and joining on the document name, which needs this.
    per_document: bool,
    files: Vec<PathBuf>,
}

/// One document's measurement.
struct Measured {
    name: String,
    ours: Option<Duration>,
    theirs: Option<Duration>,
    /// Whether we reported anything we could not draw, which makes our time incomparable.
    complete: bool,
}

fn main() -> std::process::ExitCode {
    let options = match parse_arguments() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            eprintln!(
                "usage: hayro-speed [--scale N] [--repeats N] [--per-document] <file.pdf>...\n\
                 \n\
                 Renders page one of each file with this project and with hayro, and reports\n\
                 time to first page for both. --per-document adds one tab-separated line per\n\
                 file, which is how a third renderer's own measurements are joined to these."
            );
            return std::process::ExitCode::from(2);
        }
    };

    let mut measured = Vec::with_capacity(options.files.len());
    for path in &options.files {
        // Named before the work, and flushed, so a document that never returns can be
        // identified from a killed run — the same reason the corpus gate traces.
        eprint!("\r{: <70}", path.display());
        measured.push(measure(path, &options));
    }
    eprintln!("\r{: <70}\r", "");

    report(&measured, &options);
    std::process::ExitCode::SUCCESS
}

fn parse_arguments() -> Result<Options, String> {
    let mut scale = 1.0;
    let mut repeats = 3;
    let mut per_document = false;
    let mut files = Vec::new();

    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--scale" => {
                scale = arguments
                    .next()
                    .and_then(|value| value.parse().ok())
                    .ok_or("--scale needs a number")?;
            }
            "--repeats" => {
                repeats = arguments
                    .next()
                    .and_then(|value| value.parse().ok())
                    .filter(|value| *value > 0)
                    .ok_or("--repeats needs a positive integer")?;
            }
            "--per-document" => per_document = true,
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => files.push(PathBuf::from(other)),
        }
    }

    if files.is_empty() {
        return Err("no files given".to_owned());
    }
    Ok(Options {
        scale,
        repeats,
        per_document,
        files,
    })
}

/// Times both renderers on one document, alternating.
///
/// Alternating rather than running all of one and then all of the other: the machine's state
/// drifts over a corpus sweep, and a sweep that measures us first would attribute the drift
/// to `hayro`.
fn measure(path: &Path, options: &Options) -> Measured {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Ok(bytes) = std::fs::read(path) else {
        return Measured {
            name,
            ours: None,
            theirs: None,
            complete: false,
        };
    };

    let mut ours = None;
    let mut theirs = None;
    let mut complete = false;
    for _ in 0..options.repeats {
        if let Some((taken, drew_completely)) = time_ours(&bytes, options.scale) {
            ours = Some(ours.map_or(taken, |best: Duration| best.min(taken)));
            complete = drew_completely;
        }
        if let Some(taken) = time_hayro(&bytes, options.scale) {
            theirs = Some(theirs.map_or(taken, |best: Duration| best.min(taken)));
        }
    }

    Measured {
        name,
        ours,
        theirs,
        complete,
    }
}

/// Opens, interprets and rasterises page one, returning the time and whether it was complete.
fn time_ours(bytes: &[u8], scale: f32) -> Option<(Duration, bool)> {
    let started = Instant::now();
    let document = Document::open(bytes.to_vec()).ok()?;
    let page = pdf_model::Pages::new(&document).get(0)?;
    let interpretation = pdf_model::interpret(&document, &page);
    let target = TargetSpec::for_page(&interpretation.display_list, scale, PIXEL_BUDGET).ok()?;
    let raster = CpuRasterizer::new().rasterize(&interpretation.display_list, target);
    let taken = started.elapsed();
    raster.ok()?;
    Some((taken, interpretation.unsupported.is_empty()))
}

/// The same work, in `hayro`.
fn time_hayro(bytes: &[u8], scale: f32) -> Option<Duration> {
    use hayro::hayro_interpret::InterpreterSettings;
    use hayro::hayro_syntax::Pdf;
    use hayro::{RenderCache, RenderSettings, render};

    let started = Instant::now();
    let document = Pdf::new(bytes.to_vec()).ok()?;
    let pages = document.pages();
    let page = pages.first()?;
    let pixmap = render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings {
            x_scale: scale,
            y_scale: scale,
            width: None,
            height: None,
            bg_color: hayro::vello_cpu::color::palette::css::WHITE,
        },
    );
    let taken = started.elapsed();
    // Touched after the clock stops, so the compiler cannot decide the render was dead code.
    if pixmap.width() == 0 {
        return None;
    }
    Some(taken)
}

/// Prints the comparison.
fn report(measured: &[Measured], options: &Options) {
    let both: Vec<&Measured> = measured
        .iter()
        .filter(|entry| entry.ours.is_some() && entry.theirs.is_some())
        .collect();
    let complete: Vec<&&Measured> = both.iter().filter(|entry| entry.complete).collect();

    println!(
        "{} documents, scale {}, best of {} passes each, alternating\n",
        measured.len(),
        options.scale,
        options.repeats
    );

    if options.per_document {
        println!("# name\tours_ms\thayro_ms\tcomplete");
        for entry in measured {
            let milliseconds = |taken: Option<Duration>| {
                taken.map_or_else(
                    || "-".to_owned(),
                    |taken| format!("{:.2}", taken.as_secs_f64() * 1e3),
                )
            };
            println!(
                "{}\t{}\t{}\t{}",
                entry.name,
                milliseconds(entry.ours),
                milliseconds(entry.theirs),
                entry.complete
            );
        }
        println!();
    }

    if both.len() != measured.len() {
        println!(
            "  {} document(s) one side or the other could not render, excluded\n",
            measured.len().saturating_sub(both.len())
        );
    }

    summarise("pages we draw completely", &complete);
    summarise(
        "every page both rendered (ours may be missing content)",
        &both.iter().collect::<Vec<_>>(),
    );

    // The slowest pages are where a viewer feels slow, so they are named rather than
    // averaged away — the same argument as the oracle's worst-tile metric.
    let mut worst: Vec<&&Measured> = complete.clone();
    worst.sort_by(|a, b| ratio(b).total_cmp(&ratio(a)));
    println!("\n  the ten complete pages where we are furthest behind:");
    for entry in worst.iter().take(10) {
        let (Some(ours), Some(theirs)) = (entry.ours, entry.theirs) else {
            continue;
        };
        println!(
            "    {:>6.2}x  {:>9.2?} vs {:>9.2?}  {}",
            ratio(entry),
            ours,
            theirs,
            entry.name
        );
    }
}

/// Our time divided by theirs; above one means we are slower.
fn ratio(entry: &Measured) -> f64 {
    match (entry.ours, entry.theirs) {
        (Some(ours), Some(theirs)) if theirs.as_secs_f64() > 0.0 => {
            ours.as_secs_f64() / theirs.as_secs_f64()
        }
        _ => 0.0,
    }
}

/// Totals, the median ratio, and who won more pages.
///
/// The median rather than the mean of the ratios: one page that takes us a second and them a
/// millisecond would otherwise decide the number for a thousand pages that do not.
fn summarise(label: &str, entries: &[&&Measured]) {
    if entries.is_empty() {
        println!("  {label}: nothing to compare");
        return;
    }

    let ours: Duration = entries.iter().filter_map(|entry| entry.ours).sum();
    let theirs: Duration = entries.iter().filter_map(|entry| entry.theirs).sum();
    let mut ratios: Vec<f64> = entries.iter().map(|entry| ratio(entry)).collect();
    ratios.sort_by(f64::total_cmp);
    let median = ratios.get(ratios.len() / 2).copied().unwrap_or(0.0);
    let faster = entries
        .iter()
        .filter(|entry| ratio(entry) < 1.0 && ratio(entry) > 0.0)
        .count();

    println!("  {label}: {} pages", entries.len());
    println!(
        "    total   ours {ours:.2?}, hayro {theirs:.2?}  ({:.2}x)",
        ours.as_secs_f64() / theirs.as_secs_f64().max(f64::MIN_POSITIVE)
    );
    println!("    median  {median:.2}x  (below 1.00 means we are faster)");
    println!("    we are faster on {faster} of {} pages\n", entries.len());
}
