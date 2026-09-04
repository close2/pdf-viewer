//! The four numbers `CLAUDE.md` principle 2 names, measured, with a band on each.
//!
//! # Why this exists
//!
//! Principle 2 says "[p]erf gates run in CI: cold open, time-to-first-page, page-turn latency,
//! memory high-water. A regression fails the build", and makes cold graphics bring-up "its own
//! gate, separate from time-to-first-page, so that a regression in the driver, the adapter
//! selection or the shader set is legible as itself rather than as a slower page". Nothing in
//! this tree printed any of those five figures on demand: `doc/performance.md` records launch
//! timelines a round took by hand, `examples/open_cost`, `examples/bring_up` and
//! `examples/first_frame` are the instruments a person points at one document, and no gate ran
//! any of them. Fifty sessions of conformance and robustness work happened underneath a sentence
//! about a gate that did not exist (round 921's `Q24`).
//!
//! This is the gate. It is deliberately **not** a benchmark suite: it measures the launch path of
//! the program this crate contains, in the shape `pdf-viewer.rs`'s `main` runs it, and holds each
//! figure to a band in [`doc/checks/launch-path.toml`](../../../doc/checks/launch-path.toml).
//!
//! # What each number is
//!
//! | number | what is inside it | what is not |
//! |---|---|---|
//! | **cold open** | `FileBytes::on_disk`, `Viewer::new`, `Command::Restrict`, `Command::Open` — `pdf-viewer.rs`'s `open_document` exactly, on a file whose page cache has just been dropped | the process's own creation, the window, the device |
//! | **warm open** | the same, second time, with the file in the page cache | — |
//! | **time to first page** | the document opening on one thread while the graphics device comes up on this one, joined, given a viewport, and page one's pixels drawn on the device | winit's `EventLoop::new`, the window, the surface and the present |
//! | **cold bring-up** | `QuorraRasterizer::new_headless` in a process that has done nothing else | everything else |
//! | **page turn** | `Command::GoTo(Next)`, the interpretation it causes, and the frame drawn on the device | — |
//! | **peak resident** | `VmHWM` of the process that did all of the above for one document | — |
//! | **bytes read** | `rchar` from `/proc/self/io` across the open, which is what principle 2's "reads the trailer and the objects page one needs — not the whole file" is a claim about | the binary's own loading, which is `mmap` rather than `read` |
//!
//! **What the first-page figure leaves out is the window, and it is left out on purpose.**
//! `EventLoop::new` and the first present need a display server; a gate that skipped silently
//! without one would be worse than no gate (`doc/environment.md` says the same about `Xvfb`), and
//! one that failed without one would be a coin toss. So the figure here is the launch path minus
//! winit, which is the half this project's own code owns — and `pdf-viewer --trace` under `Xvfb`
//! remains the instrument for the whole of it, recorded in `doc/performance.md`.
//!
//! # Why it may be believed on a machine running three other rounds
//!
//! A wall-clock gate that fires under a neighbour's load gets switched off, and this tree has
//! three recorded false failures of exactly that kind (`doc/todo/02` section 2). Five things
//! answer it, and none of them is a wider band:
//!
//! - **Every figure is the *minimum* of [`SAMPLES`] fresh processes.** Contention adds time and
//!   never removes it, so the fastest of nine is the closest estimate of a quiet machine that a
//!   loaded one can produce. A run fails only if *every* sample was slow.
//! - **Every child is pinned to the machine's fastest cores**, on a list derived from
//!   `cpuinfo_max_freq` rather than written down ([`the_performance_cores`]). This processor has
//!   two classes of core 57% apart in clock, and where a process lands moved the same fixed work
//!   by a factor of two on an idle machine — a lottery no band can span, and the single change
//!   that took this gate's spread from 100–400% to 0.6–22%.
//! - **A calibration probe decides whether this machine is the machine the bands were taken on.**
//!   It is a fixed, serial, in-memory piece of this tree's own work — opening the small document
//!   and interpreting its first page — and its own band is in the check file. Out of band, the
//!   gate prints every figure and judges none, saying why. That is what makes the bands safe to
//!   keep tight, and it is what stops this gate from failing on a machine nobody measured.
//! - **Two of each row's figures cannot be moved by the machine, and are judged always.** How many
//!   bytes an open reads and what *this program* spends on one are the same to the byte under any
//!   load, so the claims principle 2 makes about *what the launch path does* are gated even when
//!   the clock is not. The high-water mark of a process that has brought a graphics device up is
//!   **not** in that group, on evidence rather than principle — see [`Judged::steady`].
//! - **The profile is checked.** `[profile.gates]` costs `Document::open` 4.06% to 12.30% against
//!   `[profile.release]` (`Cargo.toml`'s own table, ADR 0666), which is larger than the band; a
//!   launch number is a claim about the program a person runs, so this judges under `release`
//!   and prints-without-judging under anything else.
//!
//! # Running it
//!
//! ```text
//! cargo build --release -p pdf-sandbox --bins           # trap 10: nothing else builds it
//! cargo test  --release -p viewer-ui --test launch_path -- --ignored --nocapture
//! ```
//!
//! `PDFVIEWER_LAUNCH_SAMPLES` overrides the sample count and **turns judging off**, saying so:
//! the minimum of three is not the minimum of nine, so a band taken at one is not a band at the
//! other. Three of the documents are `doc/`'s own, which the specification zip provides (`NOTICE`
//! section 3), and the fourth is `doc/pdf.js`'s; rows whose document is absent are skipped and
//! counted, and a run that finds none of them fails rather than passing quietly.

#![expect(
    clippy::print_stdout,
    reason = "a gate whose output is the numbers a person reads to see what moved"
)]

use std::path::{Path, PathBuf};
use std::process::Command as Child;
use std::time::Instant;

use pdf_render::Rasterizer as _;
use pdf_syntax::FileBytes;
use render_quorra::QuorraRasterizer;
use viewer_core::{Command, DocumentId, Event, PageTarget, Rendered, RestrictionLevel, Viewer};

/// What a child's one line of numbers begins with.
const MARKER: &str = "measured ";

/// Names the phase a child process is to run; unset in the parent.
const PHASE: &str = "PDFVIEWER_LAUNCH_PHASE";

/// Names the document a child process is to open.
const DOCUMENT_PATH: &str = "PDFVIEWER_LAUNCH_DOCUMENT";

/// Overrides [`SAMPLES`], and turns judging off.
const SAMPLE_OVERRIDE: &str = "PDFVIEWER_LAUNCH_SAMPLES";

/// The identity a host gives the one document it opens — `pdf-viewer.rs`'s own.
const DOCUMENT: DocumentId = DocumentId(0);

/// The viewport every figure here is measured at, in device pixels at scale 1.0.
///
/// Stated rather than derived: a page-turn latency is a function of how many pixels are drawn,
/// so a gate that took the machine's screen size would compare two different questions on two
/// machines. This is a window a person would have.
const VIEWPORT: (u32, u32) = (1600, 1000);

/// How many pages a page-turn sample turns.
///
/// Five, because five arrow keys is what every by-hand launch measurement in
/// `doc/performance.md` has used since the two-hundred-and-ninety-second session, and a figure
/// comparable with the ones already written down is worth more than a rounder number.
const TURNS: usize = 5;

/// How many fresh processes each figure is the minimum of.
///
/// **A minimum rather than a mean, because contention adds time and never removes it**: the
/// fastest of several is the closest thing to a quiet measurement that a machine running three
/// other rounds can produce, and a run fails only if every one of the samples was slow. Fresh
/// processes rather than repetitions inside one, because that is what makes each sample an
/// independent draw — and because three of these figures are about a process's own start.
///
/// **Nine is a choice with a cost beside it rather than a derivation, and it says so.** This
/// began at five and was raised when the spread had to come down; what actually took the spread
/// down — from 100–400% to 0.6–22% — was the pinning in [`the_performance_cores`], measured, and
/// no controlled comparison of five samples against nine was run beside it. Nine is kept because
/// the whole gate still costs about six seconds, which is not a number worth tuning against
/// (ADR 0884).
const SAMPLES: usize = 9;

/// How many times the calibration probe repeats inside its child.
///
/// Fifty passes of a millisecond and a half — a tenth of a second — of which the quickest is
/// kept, so that a scheduling hiccup inside one child cannot make the machine look busy. The
/// *core* lottery is not what this is for: the child is pinned like every other, which is why
/// the probe's own spread over sixteen consecutive quiet runs is under a percent.
const CALIBRATION_PASSES: usize = 50;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// ---------------------------------------------------------------------------------------------
// The child: one process, one phase, one line of numbers.
// ---------------------------------------------------------------------------------------------

/// The process's resident high-water mark in kibibytes, off `/proc/self/status`.
///
/// A high-water mark rather than a sample, because the kernel keeps it across frees and it does
/// not move with the machine's load — `examples/open_cost` and `examples/confined_peak` quote it
/// for the same reason. `None` where there is no `/proc`, which is every platform but Linux.
fn peak_resident_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))?
        .trim()
        .trim_end_matches(" kB")
        .parse()
        .ok()
}

/// How many bytes this process has had returned by a read, off `/proc/self/io`.
///
/// `rchar` rather than `read_bytes`: the second counts what actually reached the block layer,
/// which is zero for a file in the page cache and is therefore a measurement of the cache rather
/// than of the reader. What principle 2's "not the whole file" claims is about how much the
/// reader *asked for*, and that is this.
fn read_chars() -> Option<u64> {
    let io = std::fs::read_to_string("/proc/self/io").ok()?;
    io.lines()
        .find_map(|line| line.strip_prefix("rchar:"))?
        .trim()
        .parse()
        .ok()
}

/// Milliseconds since `began`.
fn ms(began: Instant) -> f64 {
    began.elapsed().as_secs_f64() * 1e3
}

/// What `pdf-viewer.rs`'s `open_document` does, with nothing added and nothing left out.
///
/// Kept as one function because that is what makes this gate a measurement of the launch path
/// rather than of a sequence somebody wrote down beside it: the steps, their order and the
/// commands are the host's, and the only difference is that no fragment and no `--page` are
/// given.
fn open_document(path: &Path) -> (Viewer, usize) {
    let bytes = match FileBytes::on_disk(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("failed cannot open {}: {error}", path.display());
            std::process::exit(1);
        }
    };
    let mut viewer = Viewer::new(0, 0, 1.0);
    drop(viewer.handle(Command::Restrict(RestrictionLevel::On)));
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();
    let pages = opened
        .iter()
        .find_map(|event| match event {
            Event::Opened { pages, .. } => Some(*pages),
            _ => None,
        })
        .unwrap_or(0);
    (viewer, pages)
}

/// Gives the viewer a viewport and draws whatever it asks for, once.
///
/// Returns the page's command count and its pixel count, which are the witnesses that the work
/// happened: a band on a duration that fell because nothing was drawn would otherwise read as an
/// improvement (trap 27 — an assertion is only as good as what it excludes).
fn draw_one(
    viewer: &mut Viewer,
    backend: &mut QuorraRasterizer,
    command: Command,
) -> Option<(usize, usize)> {
    let events: Vec<Event> = viewer.handle(command).collect();
    let request = events.into_iter().find_map(|event| match event {
        Event::NeedsRender(request) => Some(request),
        _ => None,
    })?;
    let commands = request.list.command_count();
    let drawn = backend.rasterize(&request.list, request.target).ok()?;
    let pixels = drawn.data.len() / 4;
    // The answer matters rather than the events it produces: a request the host never answers
    // leaves its page unrendered, so the next turn would be measured against a scheduler still
    // waiting for this one.
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Presented,
        })
        .for_each(drop);
    Some((commands, pixels))
}

/// The document this child was told to open.
fn document_of_the_child() -> PathBuf {
    let Ok(path) = std::env::var(DOCUMENT_PATH) else {
        println!("failed no {DOCUMENT_PATH}");
        std::process::exit(1);
    };
    PathBuf::from(path)
}

/// Prints one `key=value` line, which is the only thing a child says.
///
/// **The leading newline is load-bearing.** libtest prints `test launch_probe ... ` without one
/// and the test's own output continues that same line, so a parent looking for a line that
/// *starts with* the marker finds nothing — which is exactly what the first run of this gate did,
/// while a by-hand run with `2>&1` looked right because the child's stderr happened to break the
/// line for it. The parent slices from the marker rather than from the line's start for the same
/// reason, and the two together are belt and braces.
fn measured(fields: &[(&str, String)]) {
    let mut line = String::from("\nmeasured");
    for (key, value) in fields {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(value);
    }
    println!("{line}");
}

/// A number a child measured, or `-` where this platform does not offer it.
fn or_absent(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |number| number.to_string())
}

/// **Phase `open`**: what a launch pays before it has a window.
fn phase_open() {
    let path = document_of_the_child();
    let before = read_chars();
    let began = Instant::now();
    let (viewer, pages) = open_document(&path);
    let elapsed = ms(began);
    drop(viewer);
    let read = match (read_chars(), before) {
        (Some(after), Some(before)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    measured(&[
        ("open_ms", format!("{elapsed:.3}")),
        ("pages", pages.to_string()),
        ("read_bytes", or_absent(read)),
        ("peak_kib", or_absent(peak_resident_kib())),
    ]);
}

/// **Phase `bring-up`**: the graphics device, in a process that has done nothing else.
///
/// One measurement per process, which is `examples/bring_up`'s rule and its reason: everything
/// here loads drivers, and a second device in the same process is measured with the loader
/// already warm.
fn phase_bring_up() {
    let began = Instant::now();
    let backend = QuorraRasterizer::new_headless();
    let elapsed = ms(began);
    let adapter = match backend {
        // **Named rather than counted**, because the number above is a claim about *this*
        // adapter: a machine that quietly fell back to a software rasteriser reports a bring-up
        // that is a different measurement wearing the same figure, which is precisely what
        // principle 2 means by a regression in adapter selection being "legible as itself".
        Ok(ref quorra) => quorra.adapter_description().replace(' ', "_"),
        Err(ref error) => {
            println!("failed no graphics device: {error}");
            std::process::exit(1);
        }
    };
    measured(&[
        ("bring_up_ms", format!("{elapsed:.3}")),
        ("adapter", adapter),
        ("peak_kib", or_absent(peak_resident_kib())),
    ]);
}

/// **Phase `first-page`**: process start to page one's pixels, the device on the critical path.
///
/// The two threads are `main`'s: the document opens on one while the graphics stack comes up on
/// the other, because "[r]eading a document depends on none of it" (`pdf-viewer.rs`). What stands
/// in for the window is nothing at all — see the module comment.
fn phase_first_page() {
    let path = document_of_the_child();
    let before = read_chars();
    let began = Instant::now();
    let opening = std::thread::spawn(move || open_document(&path));
    let mut backend = match QuorraRasterizer::new_headless() {
        Ok(backend) => backend,
        Err(error) => {
            println!("failed no graphics device: {error}");
            std::process::exit(1);
        }
    };
    let device = ms(began);
    let Ok((mut viewer, pages)) = opening.join() else {
        println!("failed the document thread panicked");
        std::process::exit(1);
    };
    let joined = ms(began);
    let drawn = draw_one(
        &mut viewer,
        &mut backend,
        Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        },
    );
    let elapsed = ms(began);
    let Some((commands, pixels)) = drawn else {
        println!("failed page one was not drawn");
        std::process::exit(1);
    };
    let read = match (read_chars(), before) {
        (Some(after), Some(before)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    measured(&[
        ("first_page_ms", format!("{elapsed:.3}")),
        ("device_ms", format!("{device:.3}")),
        ("joined_ms", format!("{joined:.3}")),
        ("pages", pages.to_string()),
        ("commands", commands.to_string()),
        ("pixels", pixels.to_string()),
        ("read_bytes", or_absent(read)),
        ("peak_kib", or_absent(peak_resident_kib())),
    ]);
}

/// **Phase `page-turn`**: five arrow keys, each timed, on a viewer that has already drawn.
fn phase_page_turn() {
    let path = document_of_the_child();
    let (mut viewer, pages) = open_document(&path);
    let mut backend = match QuorraRasterizer::new_headless() {
        Ok(backend) => backend,
        Err(error) => {
            println!("failed no graphics device: {error}");
            std::process::exit(1);
        }
    };
    if draw_one(
        &mut viewer,
        &mut backend,
        Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        },
    )
    .is_none()
    {
        println!("failed page one was not drawn");
        std::process::exit(1);
    }
    // As many turns as the document has pages to turn to, up to [`TURNS`]. A five-page document
    // has four `Next`s in it and the fifth draws nothing, which is not a defect and must not read
    // as one — the first run of this gate failed on exactly that.
    let wanted = TURNS.min(pages.saturating_sub(1));
    if wanted == 0 {
        println!("failed a document of {pages} pages has no page to turn to");
        std::process::exit(1);
    }
    let mut turns = Vec::new();
    let mut commands = 0;
    for _ in 0..wanted {
        let began = Instant::now();
        let drawn = draw_one(&mut viewer, &mut backend, Command::GoTo(PageTarget::Next));
        let elapsed = ms(began);
        let Some((drew, _)) = drawn else {
            println!("failed a page turn drew nothing");
            std::process::exit(1);
        };
        commands = commands.max(drew);
        turns.push(elapsed);
    }
    let slowest = turns.iter().copied().fold(0.0_f64, f64::max);
    let quickest = turns.iter().copied().fold(f64::INFINITY, f64::min);
    measured(&[
        ("turn_ms", format!("{quickest:.3}")),
        ("slowest_turn_ms", format!("{slowest:.3}")),
        ("turns", turns.len().to_string()),
        ("pages", pages.to_string()),
        ("commands", commands.to_string()),
        ("peak_kib", or_absent(peak_resident_kib())),
    ]);
}

/// **Phase `calibrate`**: is this machine the machine the bands were taken on, and is it busy?
///
/// A fixed, serial, in-memory piece of this tree's own work — the smallest document opened from
/// bytes already in memory and its first page interpreted, the quickest of
/// [`CALIBRATION_PASSES`]. No file is read after the first, no device is created and no
/// subprocess is spawned, so what moves it is the processor this run got: a core class, or a
/// neighbour.
///
/// **A child rather than the parent's own work**, so that it is drawn from the same lottery as
/// every figure it stands guard over — same pinning, same fresh process, same minimum over
/// [`SAMPLES`] of them.
fn phase_calibrate() {
    let path = document_of_the_child();
    let Ok(bytes) = std::fs::read(&path) else {
        println!("failed the calibration document does not read");
        std::process::exit(1);
    };
    let mut quickest = f64::INFINITY;
    let mut commands = 0;
    for _ in 0..CALIBRATION_PASSES {
        let began = Instant::now();
        let Ok(opened) = pdf_syntax::Document::open(bytes.clone()) else {
            println!("failed the calibration document does not open");
            std::process::exit(1);
        };
        let pages = pdf_model::Pages::new(&opened);
        let Some(page) = pages.get(0) else {
            println!("failed the calibration document has no first page");
            std::process::exit(1);
        };
        let interpreted = pdf_model::interpret(&opened, &page);
        let elapsed = ms(began);
        commands = interpreted.display_list.command_count();
        quickest = quickest.min(elapsed);
    }
    if commands == 0 {
        println!("failed the calibration document's first page draws nothing");
        std::process::exit(1);
    }
    measured(&[
        ("calibration_ms", format!("{quickest:.3}")),
        ("commands", commands.to_string()),
    ]);
}

/// The one test a child runs: a no-op in the parent's own run, one phase in a child's.
///
/// Not `#[ignore]`d, so `cargo nextest run --workspace` runs it — where it does nothing and
/// costs nothing, because [`PHASE`] is unset. A child is this same binary re-executed with
/// `--exact`, which is `pdf-vfs`'s `tests/confined.rs` idiom and exists for the same reason: the
/// thing under measurement is a *process*, and a process cannot measure its own creation twice.
#[test]
fn launch_probe() {
    let Ok(phase) = std::env::var(PHASE) else {
        return;
    };
    match phase.as_str() {
        "calibrate" => phase_calibrate(),
        "open" => phase_open(),
        "bring-up" => phase_bring_up(),
        "first-page" => phase_first_page(),
        "page-turn" => phase_page_turn(),
        other => {
            println!("failed no such phase: {other}");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The parent: the check file, the samples, and the verdict.
// ---------------------------------------------------------------------------------------------

/// A band, `low .. high`, on one figure.
#[derive(Debug, Clone, Copy)]
struct Band {
    /// Smallest value that passes.
    ///
    /// **Not decoration.** A figure that falls out of the bottom of its band is either a win to
    /// record or an instrument that stopped measuring — a cold open that is suddenly warm, a page
    /// that drew nothing — and the two are told apart by the witnesses printed beside it.
    low: f64,
    /// Largest value that passes.
    high: f64,
}

impl Band {
    /// Whether `value` is inside.
    fn holds(self, value: f64) -> bool {
        value >= self.low && value <= self.high
    }
}

/// What a row states about one figure.
///
/// A named pair rather than an `Option`, because "this row deliberately pins nothing" is a
/// statement a round makes and not the absence of one — `doc/checks/fixed-documents.toml`'s
/// `Ink::Unpinned` for the same reason. A one-page document has no page to turn to, and a row
/// that had to invent a band for the turn it cannot make would be pinning a number nobody
/// measured.
#[derive(Debug, Clone, Copy)]
enum Pin {
    /// `none` in the file: printed, never judged.
    Nothing,
    /// The figure must lie inside this band.
    Within(Band),
}

impl Pin {
    /// The band this pin states, or `None` where it states none.
    fn band(self) -> Option<Band> {
        match self {
            Self::Nothing => None,
            Self::Within(band) => Some(band),
        }
    }
}

/// One `[[document]]` of `doc/checks/launch-path.toml`.
#[derive(Debug)]
struct Row {
    /// Where the document is, relative to the repository root.
    path: String,
    /// How many pages it has, as a witness that this is the document the bands were taken on.
    pages: usize,
    /// The band on a cold open — the file's page cache dropped first.
    cold_open_ms: Pin,
    /// The band on a warm one.
    warm_open_ms: Pin,
    /// The band on process start to page one's pixels, the device on the critical path.
    first_page_ms: Pin,
    /// The band on one page turn.
    turn_ms: Pin,
    /// The band on the peak resident size of the process that drew page one, in mebibytes.
    peak_mib: Pin,
    /// The band on the peak resident size of the process that only *opened* it, in mebibytes.
    ///
    /// The one memory figure with no graphics driver in it, and so the one that answers what a
    /// *document* costs: a device's own allocations are an order of magnitude larger than any
    /// document here and would hide the whole question.
    open_peak_mib: Pin,
    /// The band on how many bytes of the file an open reads, in kibibytes.
    read_kib: Pin,
    /// What this row is here to say, in one line.
    why: String,
}

/// The whole check file.
#[derive(Debug, Default)]
struct Check {
    /// What the machine the bands were taken on was.
    machine: String,
    /// Which cargo profile the bands are a claim about.
    profile: String,
    /// The document the calibration probe opens, relative to the repository root.
    calibration_document: String,
    /// The band the calibration probe must land in for anything below to be judged.
    calibration_ms: Option<Band>,
    /// The band on a cold graphics bring-up, which principle 2 makes a gate of its own.
    bring_up_ms: Option<Band>,
    /// The documents.
    documents: Vec<Row>,
}

/// Reads a `low .. high` band, or `none` where a row deliberately pins nothing.
///
/// `None` is a statement a row makes rather than the absence of one —
/// `doc/checks/fixed-documents.toml`'s `Ink::Unpinned` for the same reason. A one-page document
/// has no page to turn to, and a row that had to invent a band for the turn it cannot make would
/// be pinning a number nobody measured.
fn band(value: &str, at: usize) -> Result<Pin, String> {
    if value.trim() == "none" {
        return Ok(Pin::Nothing);
    }
    let (low, high) = value
        .split_once("..")
        .ok_or_else(|| format!("line {at} is not `low .. high`"))?;
    let number = |part: &str| {
        part.trim()
            .parse::<f64>()
            .map_err(|_| format!("line {at}'s bound is not a number"))
    };
    Ok(Pin::Within(Band {
        low: number(low)?,
        high: number(high)?,
    }))
}

/// A row under construction: every field optional until the row ends.
#[derive(Default)]
struct Partial {
    /// See [`Row::path`].
    path: Option<String>,
    /// See [`Row::pages`].
    pages: Option<usize>,
    /// See [`Row::cold_open_ms`]. `None` here is "the key was not stated at all".
    cold_open_ms: Option<Pin>,
    /// See [`Row::warm_open_ms`]. `None` here is "the key was not stated at all".
    warm_open_ms: Option<Pin>,
    /// See [`Row::first_page_ms`]. `None` here is "the key was not stated at all".
    first_page_ms: Option<Pin>,
    /// See [`Row::turn_ms`]. `None` here is "the key was not stated at all".
    turn_ms: Option<Pin>,
    /// See [`Row::peak_mib`]. `None` here is "the key was not stated at all".
    peak_mib: Option<Pin>,
    /// See [`Row::open_peak_mib`]. `None` here is "the key was not stated at all".
    open_peak_mib: Option<Pin>,
    /// See [`Row::read_kib`]. `None` here is "the key was not stated at all".
    read_kib: Option<Pin>,
    /// See [`Row::why`].
    why: Option<String>,
}

/// Turns a finished [`Partial`] into a [`Row`], or says which field it lacks.
fn finish(partial: Partial, at: usize, into: &mut Vec<Row>) -> Result<(), String> {
    let Partial {
        path: Some(path),
        pages: Some(pages),
        cold_open_ms: Some(cold_open_ms),
        warm_open_ms: Some(warm_open_ms),
        first_page_ms: Some(first_page_ms),
        turn_ms: Some(turn_ms),
        peak_mib: Some(peak_mib),
        open_peak_mib: Some(open_peak_mib),
        read_kib: Some(read_kib),
        why: Some(why),
    } = partial
    else {
        return Err(format!(
            "the row ending at line {at} is missing one of path, pages, cold_open_ms, \
             warm_open_ms, first_page_ms, turn_ms, peak_mib, open_peak_mib, read_kib, why"
        ));
    };
    into.push(Row {
        path,
        pages,
        cold_open_ms,
        warm_open_ms,
        first_page_ms,
        turn_ms,
        peak_mib,
        open_peak_mib,
        read_kib,
        why,
    });
    Ok(())
}

/// Reads the check file, or says what is wrong with it.
///
/// A hand-written parser for a hand-written file, following `doc/checks/fixed-documents.toml`'s
/// precedent: **anything it does not recognise is an error rather than a skipped line**, which is
/// the whole difference between a check a round can append to and one a round can silently append
/// nothing to.
fn parse(text: &str) -> Result<Check, String> {
    let mut check = Check::default();
    let mut partial: Option<Partial> = None;
    for (index, line) in text.lines().enumerate() {
        let at = index.saturating_add(1);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[document]]" {
            if let Some(previous) = partial.take() {
                finish(previous, at, &mut check.documents)?;
            }
            partial = Some(Partial::default());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {at} is neither a key nor a header: {line}"));
        };
        let (key, value) = (key.trim(), value.trim());
        let quoted = || {
            value
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_owned)
                .ok_or_else(|| format!("line {at}'s value is not a quoted string"))
        };
        if let Some(row) = partial.as_mut() {
            match key {
                "path" => row.path = Some(quoted()?),
                "why" => row.why = Some(quoted()?),
                "pages" => {
                    row.pages = Some(
                        value
                            .parse()
                            .map_err(|_| format!("line {at}'s pages is not a number"))?,
                    );
                }
                "cold_open_ms" => row.cold_open_ms = Some(band(value, at)?),
                "warm_open_ms" => row.warm_open_ms = Some(band(value, at)?),
                "first_page_ms" => row.first_page_ms = Some(band(value, at)?),
                "turn_ms" => row.turn_ms = Some(band(value, at)?),
                "peak_mib" => row.peak_mib = Some(band(value, at)?),
                "open_peak_mib" => row.open_peak_mib = Some(band(value, at)?),
                "read_kib" => row.read_kib = Some(band(value, at)?),
                other => return Err(format!("line {at} states an unknown row key `{other}`")),
            }
            continue;
        }
        match key {
            "machine" => check.machine = quoted()?,
            "profile" => check.profile = quoted()?,
            "calibration_document" => check.calibration_document = quoted()?,
            "calibration_ms" => check.calibration_ms = band(value, at)?.band(),
            "bring_up_ms" => check.bring_up_ms = band(value, at)?.band(),
            other => return Err(format!("line {at} states an unknown key `{other}`")),
        }
    }
    if let Some(last) = partial {
        finish(last, text.lines().count(), &mut check.documents)?;
    }
    Ok(check)
}

/// The `key=value` fields one child printed.
type Fields = Vec<(String, String)>;

/// One field of a child's line, as a number, or `None` where it said `-` or nothing.
fn field(fields: &Fields, key: &str) -> Option<f64> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| value.parse().ok())
}

/// The CPUs this machine runs fastest on, as `taskset -c` spells them.
///
/// **Derived from the machine rather than written down, and it is why this gate can have a band
/// at all.** The processor here is an AMD Ryzen AI 9 HX 370: four Zen 5 cores at 5.16 GHz and
/// eight denser Zen 5c cores at 3.29 GHz, and `cpuinfo_max_freq` says so per CPU. A process the
/// scheduler puts on the second kind runs a *fixed, serial, in-memory* probe in twice the time —
/// measured, on a machine whose load average was under two — so an unpinned wall-clock figure is
/// a lottery no band can span, and every launch number in `doc/performance.md` was drawn from
/// that lottery without anybody knowing.
///
/// `None` where the machine has one class of core, or no `cpufreq` at all, which is the usual
/// case on a server and on CI: there is then nothing to choose between and nothing to pin.
fn the_performance_cores() -> Option<String> {
    let mut speeds: Vec<(usize, u64)> = Vec::new();
    for cpu in 0..1024_usize {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_max_freq");
        let Ok(text) = std::fs::read_to_string(&path) else {
            break;
        };
        let Ok(speed) = text.trim().parse::<u64>() else {
            return None;
        };
        speeds.push((cpu, speed));
    }
    let fastest = speeds.iter().map(|&(_, speed)| speed).max()?;
    if speeds.iter().all(|&(_, speed)| speed == fastest) {
        return None;
    }
    let list: Vec<String> = speeds
        .iter()
        .filter(|&&(_, speed)| speed == fastest)
        .map(|&(cpu, _)| cpu.to_string())
        .collect();
    Some(list.join(","))
}

/// The core list every child is pinned to, asked for once.
///
/// `None` where the machine has nothing to choose between, or where `taskset` is not installed —
/// in which case the gate says so once and the figures are the machine's own lottery.
fn pinning() -> Option<String> {
    static CORES: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    CORES
        .get_or_init(|| {
            let cores = the_performance_cores()?;
            Child::new("taskset")
                .arg("--version")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| cores)
        })
        .clone()
}

/// Runs one phase in a fresh child and reads the line it printed.
///
/// The child is this same test binary, re-executed with a filter that selects [`launch_probe`] —
/// `pdf-vfs`'s `tests/confined.rs` idiom. `--test-threads=1` and `--nocapture` are what make the
/// child's own line reach this process's pipe.
fn run_phase(phase: &str, document: Option<&Path>) -> Result<Fields, String> {
    let exe = std::env::current_exe().map_err(|error| format!("no current exe: {error}"))?;
    // Pinned to the machine's fastest cores where it has more than one kind, and never to a
    // list this file wrote down: see [`the_performance_cores`].
    let mut child = match pinning() {
        Some(cores) => {
            let mut wrapper = Child::new("taskset");
            wrapper.arg("-c").arg(cores).arg(exe);
            wrapper
        }
        None => Child::new(exe),
    };
    child
        .args(["--exact", "launch_probe", "--nocapture", "--test-threads=1"])
        // **No display, deliberately.** Every figure here is measured headless, and a graphics
        // stack that finds a `DISPLAY` it has no authority cookie for spends the difference
        // failing an X handshake — which this machine's agent user does on every run, printing
        // *Authorization required* twice per child. What is wanted is the device's own cost.
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env(PHASE, phase);
    if let Some(path) = document {
        child.env(DOCUMENT_PATH, path);
    }
    let output = child
        .output()
        .map_err(|error| format!("the {phase} child did not run: {error}"))?;
    let said = String::from_utf8_lossy(&output.stdout);
    let line = said
        .lines()
        .find_map(|line| line.find(MARKER).map(|at| &line[at..]))
        .ok_or_else(|| {
            // Its own words, and its `stderr` where it had none: a gate whose child died must
            // say what the child said, or a round debugging it has only "nothing happened".
            let failure = said
                .lines()
                .find_map(|line| line.find("failed ").map(|at| &line[at..]))
                .map_or_else(
                    || {
                        let complaint = String::from_utf8_lossy(&output.stderr);
                        let last = complaint
                            .lines()
                            .rev()
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(" / ");
                        format!("{} — its last words: {last}", output.status)
                    },
                    str::to_owned,
                );
            format!("the {phase} child measured nothing: {failure}")
        })?;
    Ok(line
        .split_whitespace()
        .skip(1)
        .filter_map(|piece| piece.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect())
}

/// Drops one file's pages from the page cache, and says whether it could.
///
/// `posix_fadvise(POSIX_FADV_DONTNEED)`, which is what `dd oflag=nocache` is, and which an
/// unprivileged user may do to a file they can open for writing. `/proc/sys/vm/drop_caches` is
/// root's and is not available here — and it would be the wrong instrument anyway, since it
/// empties the *machine's* cache and would evict every neighbouring round's working set.
///
/// `conv=fdatasync` first, because a page that is still dirty cannot be dropped, and
/// `conv=notrunc` with `count=0` so that nothing is written: the file is opened for writing and
/// zero bytes go into it.
fn drop_the_page_cache(path: &Path) -> Result<(), String> {
    let output = Child::new("dd")
        .arg("if=/dev/null")
        .arg(format!("of={}", path.display()))
        .arg("oflag=nocache")
        .arg("conv=notrunc,fdatasync")
        .arg("count=0")
        .output()
        .map_err(|error| format!("dd did not run: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "dd refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Where the copies this gate drops the page cache of live.
///
/// **A copy, and never the repository's own file.** Dropping a file's cache means opening it for
/// writing, and a gate that opens a document of `doc/` for writing is one bad flag away from
/// changing it. The copy is made once per run, beside the build directory rather than under
/// `/tmp`, which on this machine is a `tmpfs` whose pages cannot be dropped at all.
fn cache_directory() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // `<target>/<profile>/deps/<binary>` — two levels up is the profile directory.
    let directory = exe.parent()?.parent()?.join("launch-path");
    std::fs::create_dir_all(&directory).ok()?;
    Some(directory)
}

/// Which cargo profile this binary was built with, from the directory it sits in.
///
/// Derived rather than assumed, and it decides whether anything is judged: `[profile.gates]`
/// costs `Document::open` between 4.06% and 12.30% against `[profile.release]` (`Cargo.toml`),
/// which is wider than these bands, so a figure taken under the wrong profile is a figure about
/// a program nobody runs.
fn profile_of_this_binary() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()?
                .parent()?
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A launch that
/// cannot decode an image is a *faster* launch, so without this the numbers below would improve
/// silently on a build that draws less (trap 16, ADR 0557).
#[expect(
    clippy::panic,
    reason = "a gate that cannot decode the images it is timing must stop rather than print a \
              number about a different program"
)]
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the figures below would be \
             wrong: {error}"
        );
    }
}

/// The smallest of `samples` runs of one phase, and the fields the quickest of them printed.
fn quickest(
    phase: &str,
    key: &str,
    document: Option<&Path>,
    samples: usize,
    before_each: &mut dyn FnMut() -> Result<(), String>,
) -> Result<(f64, Fields), String> {
    let mut best: Option<(f64, Fields)> = None;
    for _ in 0..samples {
        before_each()?;
        let fields = run_phase(phase, document)?;
        let value =
            field(&fields, key).ok_or_else(|| format!("the {phase} child printed no {key}"))?;
        if best.as_ref().is_none_or(|(seen, _)| value < *seen) {
            best = Some((value, fields));
        }
    }
    best.ok_or_else(|| format!("{phase} was not sampled at all"))
}

/// One figure, its band, and whether it held.
struct Judged {
    /// What it is called in the check file.
    key: &'static str,
    /// What was measured.
    value: f64,
    /// What the check file allows.
    band: Band,
    /// Whether the machine's state can move this figure.
    ///
    /// **The split this whole gate rests on.** A figure the machine cannot move is judged on any
    /// machine and under any load, because a neighbour has no way to make it wrong: how many
    /// bytes an open reads, and what *this program* spends on one, are properties of the reader.
    /// Everything else — every duration, and the memory high-water of a process that has brought
    /// a graphics device up — is judged only where the calibration probe says this is the machine
    /// the bands were taken on.
    ///
    /// The high-water mark is in the second group **on the evidence rather than on principle**:
    /// it was identical in all forty-four runs the bands were derived from, and an hour later, on
    /// an idle machine, all four rows had fallen by about 12% together. What moved is the
    /// driver's own allocation, and nothing in this process can see why. [`Row::open_peak_mib`]
    /// is the memory figure with no device in it, and that one has not moved at all.
    steady: bool,
}

/// Remembers a figure against the band its row states, and nothing where the row states `none`.
fn band_it(
    into: &mut Vec<(String, Judged)>,
    what: String,
    key: &'static str,
    value: f64,
    pin: Pin,
    steady: bool,
) {
    if let Some(band) = pin.band() {
        into.push((
            what,
            Judged {
                key,
                value,
                band,
                steady,
            },
        ));
    }
}

/// The gate.
///
/// `#[ignore]` for `doc/todo/02`'s reason: it spawns dozens of processes and takes tens of
/// seconds, so it is a gate line rather than a unit test.
#[test]
#[ignore = "spawns a process per sample and takes tens of seconds; run it from doc/todo/02's \
            sequence"]
#[expect(
    clippy::too_many_lines,
    reason = "one gate, printed in the order a reader wants it: the machine, the calibration, \
              the device, then a block per document. A split would scatter one table."
)]
fn the_launch_path_stays_inside_its_bands() {
    require_the_sandbox();
    let root = root();
    let file = root.join("doc/checks/launch-path.toml");
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) => panic!("{} does not read: {error}", file.display()),
    };
    let check = match parse(&text) {
        Ok(check) => check,
        Err(complaint) => panic!("{}: {complaint}", file.display()),
    };

    let (samples, sampling_is_the_file_s) = match std::env::var(SAMPLE_OVERRIDE) {
        Ok(said) => match said.trim().parse::<usize>() {
            Ok(count) if count > 0 => (count, false),
            _ => panic!("{SAMPLE_OVERRIDE}={said}: expected a count above zero"),
        },
        Err(_) => (SAMPLES, true),
    };
    let profile = profile_of_this_binary();
    println!(
        "launch-path: bands taken on {} under `{}`",
        check.machine, check.profile
    );
    println!(
        "launch-path: this run is `{profile}`, {samples} samples per figure, viewport {}x{}",
        VIEWPORT.0, VIEWPORT.1
    );

    println!(
        "launch-path: children pinned to {}",
        pinning().map_or_else(
            || "nothing — this machine has one class of core, or no `taskset`".to_owned(),
            |cores| format!("CPUs {cores}, which is where this machine is fastest")
        )
    );

    // Every figure is the minimum of `samples` fresh processes, so the calibration comes first
    // and decides whether any of them is judged.
    let calibration_document = root.join(&check.calibration_document);
    let calibration = match quickest(
        "calibrate",
        "calibration_ms",
        Some(&calibration_document),
        samples,
        &mut || Ok(()),
    ) {
        Ok((value, _)) => value,
        Err(complaint) => panic!("the calibration probe: {complaint}"),
    };
    let calibration_band = check.calibration_ms;
    let machine_is_the_machine = calibration_band.is_some_and(|band| band.holds(calibration));
    println!(
        "launch-path: calibration {calibration:.3} ms, band {}",
        calibration_band.map_or_else(
            || "none stated".to_owned(),
            |band| format!("{:.3} .. {:.3}", band.low, band.high)
        )
    );

    let judging = machine_is_the_machine && sampling_is_the_file_s && profile == check.profile;
    if !judging {
        println!(
            "launch-path: NOT JUDGED — {}",
            if !sampling_is_the_file_s {
                "the sample count was overridden, and a minimum of n is not a minimum of five"
            } else if profile != check.profile {
                "this build's profile is not the one the bands are a claim about"
            } else {
                "the calibration probe is outside its band, so this is not the machine the \
                 bands were taken on"
            }
        );
    }

    let mut complaints: Vec<String> = Vec::new();
    let mut judged: Vec<(String, Judged)> = Vec::new();

    // Principle 2 makes cold bring-up a gate of its own, "so that a regression in the driver,
    // the adapter selection or the shader set is legible as itself rather than as a slower page".
    match quickest("bring-up", "bring_up_ms", None, samples, &mut || Ok(())) {
        Ok((value, fields)) => {
            let adapter = fields
                .iter()
                .find(|(key, _)| key == "adapter")
                .map_or("unnamed", |(_, name)| name.as_str());
            println!("launch-path: cold graphics bring-up {value:.1} ms on {adapter}");
            band_it(
                &mut judged,
                "the graphics device".to_owned(),
                "bring_up_ms",
                value,
                check.bring_up_ms.map_or(Pin::Nothing, Pin::Within),
                false,
            );
        }
        Err(complaint) => complaints.push(format!("cold bring-up: {complaint}")),
    }

    let cache = cache_directory();
    let mut absent = 0_usize;
    let mut measured_documents = 0_usize;
    for row in &check.documents {
        let path = root.join(&row.path);
        if !path.exists() {
            absent = absent.saturating_add(1);
            println!("launch-path: {} is not here — skipped", row.path);
            continue;
        }
        measured_documents = measured_documents.saturating_add(1);
        println!("launch-path: {} — {}", row.path, row.why);

        // The cold arm reads a copy whose pages have just been dropped; the warm arm reads the
        // same copy with them in place. Both arms on the copy rather than one on each, so that
        // the only difference between the two figures is the cache.
        let copy = cache.as_ref().map(|directory| {
            let copy = directory.join(path.file_name().unwrap_or_else(|| "document.pdf".as_ref()));
            let _ = std::fs::copy(&path, &copy);
            copy
        });
        let cold_source = copy.as_deref().unwrap_or(path.as_path());
        let mut eviction: Option<String> = None;
        let cold = quickest(
            "open",
            "open_ms",
            Some(cold_source),
            samples,
            &mut || match copy.as_deref() {
                Some(copy) => drop_the_page_cache(copy),
                None => Err("there is no writable copy to drop the cache of".to_owned()),
            },
        );
        let cold = match cold {
            Ok((value, fields)) => Some((value, fields)),
            Err(complaint) => {
                eviction = Some(complaint);
                None
            }
        };
        let warm = quickest(
            "open",
            "open_ms",
            Some(cold_source),
            samples,
            &mut || Ok(()),
        );
        let first = quickest(
            "first-page",
            "first_page_ms",
            Some(cold_source),
            samples,
            &mut || Ok(()),
        );
        // A one-page document has no page to turn to, and asking for one is not a defect to
        // report. Its row states `turn_ms = none` and this is the other half of that.
        let turn = if row.pages > 1 {
            Some(quickest(
                "page-turn",
                "turn_ms",
                Some(cold_source),
                samples,
                &mut || Ok(()),
            ))
        } else {
            println!("launch-path:   no page turn: the document has one page");
            None
        };

        if let Some(said) = eviction {
            println!("launch-path:   the cold arm did not run: {said}");
            complaints.push(format!(
                "{}: the cold open was not measured: {said}",
                row.path
            ));
        }
        if let Some((value, fields)) = cold.as_ref() {
            let pages = field(fields, "pages").unwrap_or(0.0);
            #[expect(
                clippy::cast_precision_loss,
                reason = "a page count; f64 is exact to 2^53"
            )]
            let stated = row.pages as f64;
            if (pages - stated).abs() > f64::EPSILON {
                complaints.push(format!(
                    "{}: the check file says {} pages and the document has {pages:.0}",
                    row.path, row.pages
                ));
            }
            let read = field(fields, "read_bytes").unwrap_or(0.0) / 1024.0;
            let peak = field(fields, "peak_kib").unwrap_or(0.0) / 1024.0;
            println!(
                "launch-path:   cold open {value:.2} ms, {pages:.0} pages, \
                 {read:.0} KiB read, {peak:.0} MiB resident"
            );
            band_it(
                &mut judged,
                format!("{}: the cold open", row.path),
                "cold_open_ms",
                *value,
                row.cold_open_ms,
                false,
            );
            band_it(
                &mut judged,
                format!("{}: the bytes an open reads", row.path),
                "read_kib",
                read,
                row.read_kib,
                true,
            );
            band_it(
                &mut judged,
                format!("{}: what an open costs in memory", row.path),
                "open_peak_mib",
                peak,
                row.open_peak_mib,
                true,
            );
        }
        match warm {
            Ok((value, _)) => {
                println!("launch-path:   warm open {value:.2} ms");
                band_it(
                    &mut judged,
                    format!("{}: the warm open", row.path),
                    "warm_open_ms",
                    value,
                    row.warm_open_ms,
                    false,
                );
            }
            Err(complaint) => complaints.push(format!("{}: warm open: {complaint}", row.path)),
        }
        match first {
            Ok((value, fields)) => {
                let device = field(&fields, "device_ms").unwrap_or(0.0);
                let joined = field(&fields, "joined_ms").unwrap_or(0.0);
                let commands = field(&fields, "commands").unwrap_or(0.0);
                let peak = field(&fields, "peak_kib").unwrap_or(0.0) / 1024.0;
                println!(
                    "launch-path:   first page {value:.1} ms (device up at {device:.1}, \
                     document joined at {joined:.1}, {commands:.0} commands), \
                     {peak:.0} MiB resident"
                );
                band_it(
                    &mut judged,
                    format!("{}: time to first page", row.path),
                    "first_page_ms",
                    value,
                    row.first_page_ms,
                    false,
                );
                band_it(
                    &mut judged,
                    format!("{}: the memory high-water", row.path),
                    "peak_mib",
                    peak,
                    row.peak_mib,
                    false,
                );
            }
            Err(complaint) => complaints.push(format!("{}: first page: {complaint}", row.path)),
        }
        match turn.unwrap_or_else(|| Err("not asked for".to_owned())) {
            Ok((value, fields)) => {
                let slowest = field(&fields, "slowest_turn_ms").unwrap_or(0.0);
                println!(
                    "launch-path:   page turn {value:.1} ms (slowest of {TURNS}: {slowest:.1})"
                );
                band_it(
                    &mut judged,
                    format!("{}: a page turn", row.path),
                    "turn_ms",
                    value,
                    row.turn_ms,
                    false,
                );
            }
            Err(complaint) if complaint == "not asked for" => {}
            Err(complaint) => complaints.push(format!("{}: page turn: {complaint}", row.path)),
        }
    }

    for (what, figure) in &judged {
        let held = figure.band.holds(figure.value);
        // A steady figure is judged whatever the machine is doing; see [`Judged::steady`].
        if !held && (judging || figure.steady) {
            complaints.push(format!(
                "{what} is {:.3}, outside {} {:.3} .. {:.3}",
                figure.value, figure.key, figure.band.low, figure.band.high
            ));
        } else if !held {
            println!(
                "launch-path: (not judged) {what} is {:.3}, outside {:.3} .. {:.3}",
                figure.value, figure.band.low, figure.band.high
            );
        }
    }

    println!(
        "launch-path: {measured_documents} documents measured, {absent} absent, \
         {} figures banded, {} outside",
        judged.len(),
        judged
            .iter()
            .filter(|(_, figure)| !figure.band.holds(figure.value))
            .count()
    );

    assert!(
        measured_documents > 0 || check.documents.is_empty(),
        "not one of the {} documents this check names is on the disk, so it measured nothing \
         and would have passed quietly",
        check.documents.len()
    );
    assert!(
        complaints.is_empty(),
        "the launch path moved:\n  {}",
        complaints.join("\n  ")
    );
}
