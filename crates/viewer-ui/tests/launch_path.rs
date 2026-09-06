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
//! | **memory high-water** | `VmHWM` of the process that did all of the above for one document, less the resident pages of the files it has mapped — what the allocator asked the kernel for | every shared object the Vulkan loader brought in, which is nine tenths of the process's own `VmHWM` and is the kernel's decision rather than this program's (ADR 0910) |
//! | **bytes read** | `rchar` from `/proc/self/io` across the open, which is what principle 2's "reads the trailer and the objects page one needs — not the whole file" is a claim about | the binary's own loading, which is `mmap` rather than `read` |
//! | **instructions an open executes** | the whole of a process that opens the document and stops, counted under callgrind — what principle 2's "cold open" asks about, with no clock in it (see [`open_kinstructions`]) | the disk, and how fast this machine happens to be executing today |
//!
//! **Every duration above is elapsed time less the time the thread spent waiting for a
//! processor**, which the kernel counts exactly and which is not this program's (see
//! [`corrected`]). On a quiet machine that is zero and the figure is unchanged; under a
//! neighbour it is the whole of an excursion.
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
//! - **One figure has no clock in it at all, and it is the one principle 2's cold-open gate is
//!   really asking about.** `open_kinstructions` counts the instructions a process executes to
//!   open the document; it does not move with the processor's clock, with a neighbour inside the
//!   same core, with the page cache or with the disk, so it is judged on every machine under
//!   every load. Round 938 added it after measuring that the clock figures' contention has a
//!   component nothing beside them can subtract — see below.
//! - **Every duration is elapsed time less the wait for a processor.** The kernel counts that
//!   wait exactly, per thread, in nanoseconds (`sched_info.run_delay`), and it is time somebody
//!   else took rather than time this program spent, so it comes off — the same subtraction round
//!   935 made on the memory high-water, in another unit. [`corrected`] has the measurement.
//! - **Every figure is the *minimum* of [`SAMPLES`] fresh processes.** Contention adds time and
//!   never removes it, so the fastest of nine is the closest estimate of a quiet machine that a
//!   loaded one can produce. A run fails only if *every* sample was slow.
//! - **Every child is pinned to the machine's fastest cores**, on a list derived from
//!   `cpuinfo_max_freq` rather than written down ([`the_performance_cores`]). This processor has
//!   two classes of core 57% apart in clock, and where a process lands moved the same fixed work
//!   by a factor of two on an idle machine — a lottery no band can span, and the single change
//!   that took this gate's spread from 100–400% to 0.6–22%.
//! - **Two probes decide, per figure, whether the machine was the machine.** The first is a
//!   fixed, serial, in-memory piece of this tree's own work — opening the small document and
//!   interpreting its first page — run by **every child, after its own phase**, so that what is
//!   asked is whether *the process that produced this figure* had the machine to itself. The
//!   second is a cold read of a fixed file, timed beside every cold sample, because the first is
//!   CPU-bound and a cold open is mostly disk. Each has a band in the check file; a figure whose
//!   probes are out of band is printed and not judged, and the run says so. That is what makes
//!   the bands safe to keep tight, and it is what stops this gate from failing on a machine
//!   nobody measured.
//!
//!   **Both halves were paid for.** A single probe taken once by the parent let two false
//!   failures through in five consecutive runs, because the machine changed during the seven
//!   seconds between it and the figures; and with the probe moved into the children, a cold open
//!   still failed at 0.841 ms against 0.49 .. 0.80 while its child's calibration sat dead centre,
//!   because no amount of CPU probing can see a neighbour queueing the disk.
//!
//!   **And they answer only what a probe can answer, which round 938 measured.** With eight
//!   spinning processes on exactly the eight CPUs these children are pinned to, a warm open rose
//!   43% and the calibration probe rose 74% — while the kernel's wait counter read *exactly zero*
//!   in all twenty samples. The neighbour was not taking a turn on the processor, it was sitting
//!   inside the core: an SMT sibling, a shared cache, a boost clock that four busy cores do not
//!   reach. Nothing in a wall-clock figure can be subtracted for that, and the probe over-reads
//!   it — which is why a *count* was added rather than a wider band. ADR 0916.
//! - **Three of each row's figures cannot be moved by the machine, and are judged always.** How
//!   many bytes an open reads, what an open costs in memory, and how much this program has
//!   allocated when page one is drawn are the same under any load, so the claims principle 2
//!   makes about *what the launch path does* are gated even when the clock is not. The third of
//!   them was the process's whole high-water until round 935 and was **not** in this group then,
//!   because most of that figure belongs to the driver's shared objects — see
//!   [`anonymous_high_water_mib`] and [`Judged::steady`].
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

/// Names the document every child's calibration probe opens.
const CALIBRATION_PATH: &str = "PDFVIEWER_LAUNCH_CALIBRATION";

/// Overrides [`SAMPLES`], and turns judging off.
const SAMPLE_OVERRIDE: &str = "PDFVIEWER_LAUNCH_SAMPLES";

/// Asks for the figures that are wall clocks, which are otherwise measured and not judged.
///
/// **`doc/questions/Q29`'s option 2, which the owner asked for together with option 1.** The
/// figures with no clock in them are properties of this program and are judged on any machine at
/// any load; the ones with a clock in them are claims about a machine, and this tree has had four
/// consecutive rounds in which the machine was not available to make such a claim about. So they
/// are asked for rather than assumed: `doc/todo/02` section 2 runs this gate without the variable
/// and gets the counted figures on every round in about two seconds, and `doc/verify.md` says to
/// run it *with* the variable when a round has the machine to itself.
///
/// **An environment variable rather than a flag** — the owner's own preference, stated in
/// `doc/questions/A28` about a different switch in this tree and taken as the house style here.
const CLOCK_FIGURES: &str = "PDFVIEWER_LAUNCH_CLOCKS";

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

/// How much of a figure may be time its thread spent waiting for a processor before the figure
/// is declined rather than corrected.
///
/// **`doc/questions/Q29`'s option 1, answered with a count instead of an afternoon.** That option
/// asked for a band on a busy probe so that a loaded run declines instead of failing, and said
/// the derivation needed ten minutes of an idle machine — which four rounds in a row did not get.
/// The kernel counts the contention exactly, per thread, so no band is needed: [`corrected`]
/// takes the wait off the figure, and where the wait was more than this share of the elapsed time
/// the sample is declined outright, because a thread preempted that heavily also came back to
/// caches and a branch predictor somebody else had used, and *that* part cannot be subtracted.
///
/// A tenth, which is a dimensionless choice rather than a measured one and says so. What decided
/// the order of magnitude: over fifteen consecutive warm opens on a loaded machine, fourteen
/// waited for nothing at all and one waited for 72% of its own elapsed time (ADR 0916). There is
/// no observed population between those two, so anything from a few per cent to a half would
/// separate the same samples.
const WAITED_SHARE: f64 = 0.10;

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

/// How many kibibytes of this process's resident set are pages of a *file*, off
/// `/proc/self/smaps_rollup`.
///
/// **The other half of a high-water mark, and the half nothing in this program decides.**
/// `Rss - Anonymous` is every resident page that came from a mapped file — the shared objects
/// the dynamic loader and the Vulkan loader brought in, and this binary's own text. Round 935
/// measured what that is worth here: a process that brings the graphics device up and does
/// nothing else has a 108 MiB high-water of which **97 MiB is file-backed**, 52 of those in
/// `libLLVM.so` — which `libvulkan_radeon.so` links directly — and 26 in `libgallium.so`, which
/// comes in with `libEGL_mesa.so`. `libLLVM.so` is 163 MiB on disk, so what that 52 measures is
/// how much of it the kernel happened to keep.
///
/// How many of those pages are resident is the kernel's decision rather than ours: a fault on a
/// mapping whose pages are already in the page cache maps a whole fault-around window, and a
/// fault on one that has been evicted maps a single page. Measured by evicting the two libraries
/// above and nothing else, the same binary's high-water fell from 108.4 MiB to 81.2 MiB — 25% —
/// while its *anonymous* total moved by 30 KiB. ADR 0910.
///
/// Read before [`peak_resident_kib`] by every caller, so that the figure subtracted from a
/// high-water is one this process had already reached.
fn mapped_resident_kib() -> Option<u64> {
    let rollup = std::fs::read_to_string("/proc/self/smaps_rollup").ok()?;
    let field = |key: &str| -> Option<u64> {
        rollup
            .lines()
            .find_map(|line| line.strip_prefix(key))?
            .trim()
            .trim_end_matches(" kB")
            .parse::<u64>()
            .ok()
    };
    Some(field("Rss:")?.saturating_sub(field("Anonymous:")?))
}

/// How long this thread has been on a processor and how long it has waited for one, in
/// nanoseconds — the first two fields of `/proc/thread-self/schedstat`.
///
/// **The second is the part of a wall clock that belongs to somebody else, and the kernel counts
/// it exactly.** Elapsed time is three things: the thread running, the thread blocked on
/// something it asked for, and the thread *ready to run with a processor somebody else had*.
/// `sched_info.run_delay` is the third, accumulated in nanoseconds at every wakeup — a count of
/// what happened to this figure rather than an estimate taken beside it.
///
/// `/proc/thread-self/` rather than `/proc/self/`, and the difference is the whole measurement:
/// `/proc/self/` is the thread *group leader*, libtest runs a test on a thread of its own, and
/// the leader is asleep in a join for the whole of every phase — so it reads zero however busy
/// the machine is. Measured both ways before this was written.
///
/// `None` where there is no `/proc`, which is every platform but Linux, and on a kernel built
/// without `CONFIG_SCHED_INFO`, where the fields are absent.
fn scheduling() -> Option<(u64, u64)> {
    let line = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let mut fields = line.split_whitespace();
    let on_cpu = fields.next()?.parse().ok()?;
    let waiting = fields.next()?.parse().ok()?;
    Some((on_cpu, waiting))
}

/// One phase's elapsed time less the time it spent waiting for a processor, then the elapsed time
/// and the wait themselves — the three numbers every clock figure here is printed as.
///
/// **The subtraction is round 938's, and it is round 935's subtraction in another unit.** That
/// round found nine tenths of a memory high-water was resident pages of somebody else's shared
/// libraries and banded `VmHWM - mapped` in its place; this takes the same step on a clock, on
/// the same argument — a gate should band a quantity that does not contain the mechanism that
/// moves it.
///
/// On a machine with nothing else on it the wait is zero and the figure is exactly what it was,
/// which is why the bands derived before this change still hold. Under a neighbour it is the
/// whole of an excursion: over fifteen consecutive warm opens of the five-page document,
/// fourteen read 0.97 to 1.08 ms with a wait of zero and one read 3.947 ms with a wait of 2.825
/// (ADR 0916).
///
/// **What it cannot remove is the other half, and no clock can.** A neighbour that shares a core
/// rather than queueing for one makes the same instructions take longer and leaves nothing here
/// to subtract: under eight spinning processes on exactly the eight CPUs these children are
/// pinned to, a warm open rose 43% with this wait at exactly zero in all twenty samples. That is
/// what the calibration probes decline on, and what `open_instructions` counts around.
fn corrected(
    elapsed: f64,
    before: Option<(u64, u64)>,
    after: Option<(u64, u64)>,
) -> (String, String, String) {
    let waited = match (before, after) {
        (Some((_, before)), Some((_, after))) => {
            // Integer nanoseconds to milliseconds: `u64` is exact in `f64` far above any wait a
            // process of this length can accumulate.
            #[expect(
                clippy::cast_precision_loss,
                reason = "a nanosecond count of a phase measured in milliseconds; f64 is exact                           to 2^53 and this is bounded by the phase"
            )]
            Some(after.saturating_sub(before) as f64 / 1e6)
        }
        _ => None,
    };
    let judged = elapsed - waited.unwrap_or(0.0);
    (
        format!("{judged:.3}"),
        format!("{elapsed:.3}"),
        waited.map_or_else(|| "-".to_owned(), |value| format!("{value:.3}")),
    )
}

/// One counter of `/proc/self/io`.
fn io_counter(key: &str) -> Option<u64> {
    let io = std::fs::read_to_string("/proc/self/io").ok()?;
    io.lines()
        .find_map(|line| line.strip_prefix(key))?
        .trim()
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
    io_counter("rchar:")
}

/// How many read calls the process has made, off `/proc/self/io`.
///
/// **The other counted half of a cold open, and round 938's second.** A cold open's elapsed time
/// is the program's work plus one round trip to the disk *for each read that misses the page
/// cache*, and the second term is the machine's rather than this program's — it is what moved
/// this gate's smallest rows out of band on afternoons when nothing else had. What belongs to
/// the program is **how many trips it makes**, and `syscr` counts them exactly: a change that
/// read the same bytes in ten times as many calls would cost a cold open ten round trips and
/// would show here as a number, on any machine, with no disk in the claim.
///
/// Beside [`read_chars`], which is the bytes: §7.5's "reads the trailer and the objects page one
/// needs" is a claim about both.
fn read_calls() -> Option<u64> {
    io_counter("syscr:")
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

/// Prints a phase's own fields with [`calibration_fields`]'s two after them.
///
/// Every phase but `calibrate` says it this way, so that a figure and the state of the machine
/// that produced it arrive on one line and cannot be paired wrongly by the parent.
fn measured_beside_the_machine(fields: &[(&str, String)]) {
    let mut all: Vec<(&str, String)> = fields.to_vec();
    all.extend(calibration_fields());
    measured(&all);
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

/// What a figure's own wait for a processor reads, where there was one, as a phrase to append.
///
/// **Nothing at all where the wait was zero**, which is the ordinary case even under heavy load:
/// a figure of a millisecond or two is rarely preempted, because a freshly woken short task is
/// what this scheduler runs first. Printing `+0.000 ms waiting` on every line would bury the
/// samples where it is the whole story — round 938 measured one warm open in fifteen at 3.947 ms
/// of which 2.825 was this.
fn waiting(fields: &Fields, key: &str) -> String {
    match field(fields, key) {
        Some(waited) if waited > 0.0 => {
            format!(" ({waited:.3} ms of waiting for a processor already taken off)")
        }
        _ => String::new(),
    }
}

/// A number a child measured, or `-` where this platform does not offer it.
fn or_absent(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |number| number.to_string())
}

/// **Phase `open`**: what a launch pays before it has a window.
fn phase_open() {
    let path = document_of_the_child();
    let before = read_chars();
    let before_calls = read_calls();
    let scheduled = scheduling();
    let began = Instant::now();
    let (viewer, pages) = open_document(&path);
    let elapsed = ms(began);
    let waited = scheduling();
    drop(viewer);
    let read = match (read_chars(), before) {
        (Some(after), Some(before)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    let calls = match (read_calls(), before_calls) {
        (Some(after), Some(before)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    let (judged, wall, runq) = corrected(elapsed, scheduled, waited);
    measured_beside_the_machine(&[
        ("open_ms", judged),
        ("read_calls", or_absent(calls)),
        ("open_wall_ms", wall),
        ("open_runq_ms", runq),
        ("pages", pages.to_string()),
        ("read_bytes", or_absent(read)),
        ("mapped_kib", or_absent(mapped_resident_kib())),
        ("peak_kib", or_absent(peak_resident_kib())),
    ]);
}

/// **Phase `bring-up`**: the graphics device, in a process that has done nothing else.
///
/// One measurement per process, which is `examples/bring_up`'s rule and its reason: everything
/// here loads drivers, and a second device in the same process is measured with the loader
/// already warm.
fn phase_bring_up() {
    let scheduled = scheduling();
    let began = Instant::now();
    let backend = QuorraRasterizer::new_headless();
    let elapsed = ms(began);
    let waited = scheduling();
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
    let (judged, wall, runq) = corrected(elapsed, scheduled, waited);
    measured_beside_the_machine(&[
        ("bring_up_ms", judged),
        ("bring_up_wall_ms", wall),
        ("bring_up_runq_ms", runq),
        ("adapter", adapter),
        ("mapped_kib", or_absent(mapped_resident_kib())),
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
    // This thread's wait alone, and the document thread's is not in it — which under-corrects
    // rather than over-corrects, so a figure this leaves too large is never one that passes when
    // it should have failed.
    let scheduled = scheduling();
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
    let waited = scheduling();
    let Some((commands, pixels)) = drawn else {
        println!("failed page one was not drawn");
        std::process::exit(1);
    };
    let read = match (read_chars(), before) {
        (Some(after), Some(before)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    let (judged, wall, runq) = corrected(elapsed, scheduled, waited);
    measured_beside_the_machine(&[
        ("first_page_ms", judged),
        ("first_page_wall_ms", wall),
        ("first_page_runq_ms", runq),
        ("device_ms", format!("{device:.3}")),
        ("joined_ms", format!("{joined:.3}")),
        ("pages", pages.to_string()),
        ("commands", commands.to_string()),
        ("pixels", pixels.to_string()),
        ("read_bytes", or_absent(read)),
        ("mapped_kib", or_absent(mapped_resident_kib())),
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
    // The scheduling of the *quickest* turn, which is the one the figure is: a run of five in
    // which one waited for a processor says nothing about the one that did not.
    let mut quickest_scheduling = (None, None);
    for _ in 0..wanted {
        let scheduled = scheduling();
        let began = Instant::now();
        let drawn = draw_one(&mut viewer, &mut backend, Command::GoTo(PageTarget::Next));
        let elapsed = ms(began);
        let after_turn = scheduling();
        let Some((drew, _)) = drawn else {
            println!("failed a page turn drew nothing");
            std::process::exit(1);
        };
        commands = commands.max(drew);
        if turns.iter().copied().fold(f64::INFINITY, f64::min) > elapsed {
            quickest_scheduling = (scheduled, after_turn);
        }
        turns.push(elapsed);
    }
    let slowest = turns.iter().copied().fold(0.0_f64, f64::max);
    let quickest = turns.iter().copied().fold(f64::INFINITY, f64::min);
    let (judged, wall, runq) = corrected(quickest, quickest_scheduling.0, quickest_scheduling.1);
    measured_beside_the_machine(&[
        ("turn_ms", judged),
        ("turn_wall_ms", wall),
        ("turn_runq_ms", runq),
        ("slowest_turn_ms", format!("{slowest:.3}")),
        ("turns", turns.len().to_string()),
        ("pages", pages.to_string()),
        ("commands", commands.to_string()),
        ("mapped_kib", or_absent(mapped_resident_kib())),
        ("peak_kib", or_absent(peak_resident_kib())),
    ]);
}

/// **Phase `count-open`**: the same open as [`phase_open`], with nothing beside it, for counting.
///
/// **This phase's figure is not the one it prints.** What is measured is the whole process, from
/// outside, by callgrind: [`open_kinstructions`] runs this and reads the instruction count off
/// the profile. So the phase does the open and stops — no calibration probe, no `/proc`, no
/// memory reading — because every instruction it executes beside the open is an instruction in
/// the figure.
///
/// It prints its page count all the same, and the parent checks it, because a count is a number
/// whatever the process did: a run that failed to open the document would otherwise report a
/// smaller figure and read as a win (trap 16).
fn phase_count_open() {
    let path = document_of_the_child();
    let (viewer, pages) = open_document(&path);
    drop(viewer);
    measured(&[("pages", pages.to_string())]);
}

/// **Phase `calibrate`**: is this machine the machine the bands were taken on, and is it busy?
///
/// A fixed, serial, in-memory piece of this tree's own work — the smallest document opened from
/// bytes already in memory and its first page interpreted, [`CALIBRATION_PASSES`] times. No file
/// is read after the first, no device is created and no subprocess is spawned, so what moves it
/// is the processor this run got: a core class, or a neighbour.
///
/// **A child rather than the parent's own work**, so that it is drawn from the same lottery as
/// every figure it stands guard over — same pinning, same fresh process, same minimum over
/// [`SAMPLES`] of them.
///
/// **Two numbers come back, and the second exists because the first cannot see what the figures
/// see.** The quickest of fifty passes is the machine's *identity* — its spread over sixteen
/// consecutive quiet runs is under a percent, which is what makes it able to say "this is that
/// processor". It is a poor answer to the other half of the question this probe is asked, *and
/// is it busy*, because by pass fifty the allocator, the caches, the branch predictors and the
/// core's own clock have all been warmed by the forty-nine before it — while **every figure this
/// gate judges is one first pass in a fresh process**. Session 931 measured the gap: over twelve
/// consecutive runs the fifty-pass minimum moved 1.3% (0.703 to 0.749 ms) while the figures it
/// guards moved by factors of two — a five-page document's *warm* open, which has no disk in it
/// at all, read 0.45 ms in nine runs and 1.01 in another, with the probe at 0.705 in both. So the
/// first pass is reported beside it, and it is the one made of the same stuff as a figure.
///
/// It is **printed and not judged** — `doc/todo/05`'s rule for a new figure — because a band for
/// it has to be derived on a quiet machine and this round had none. `doc/todo/42` says what that
/// derivation is.
fn calibration_pass() -> (f64, f64, usize) {
    let Ok(path) = std::env::var(CALIBRATION_PATH) else {
        println!("failed no {CALIBRATION_PATH}");
        std::process::exit(1);
    };
    let path = PathBuf::from(path);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("failed the calibration document does not read");
        std::process::exit(1);
    };
    let mut quickest = f64::INFINITY;
    let mut first = f64::INFINITY;
    let mut commands = 0;
    for pass in 0..CALIBRATION_PASSES {
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
        if pass == 0 {
            first = elapsed;
        }
    }
    if commands == 0 {
        println!("failed the calibration document's first page draws nothing");
        std::process::exit(1);
    }
    (quickest, first, commands)
}

/// The calibration this child measured, as the two fields to print beside its figure.
///
/// **Every child runs it, after its own phase, and that pairing is the point.** A calibration
/// taken once by the parent says what the machine was like when the run began; a figure taken
/// eight seconds later by a process that lost its cores to a neighbour is not covered by it, and
/// this gate produced exactly that — two false failures in five consecutive runs, each with a
/// headline calibration well inside its band. The figure a run judges is the *minimum* over
/// children, so what has to be quiet is the child that produced it, and only that child can say.
///
/// After the phase rather than before it, because before it would warm the allocator and the page
/// cache of a document one of the rows is measured on.
///
/// Both of [`calibration_pass`]'s numbers go out, because a figure that is declined should be able
/// to say which probe declined it and what that probe read — which this gate could not say, and
/// which is why `doc/todo/42` carried three hypotheses for a round to separate rather than one.
fn calibration_fields() -> [(&'static str, String); 2] {
    let (quickest, first, _) = calibration_pass();
    [
        ("calibration_ms", format!("{quickest:.3}")),
        ("calibration_first_ms", format!("{first:.3}")),
    ]
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
        "calibrate" => {
            let (quickest, first, commands) = calibration_pass();
            measured(&[
                ("calibration_ms", format!("{quickest:.3}")),
                ("calibration_first_ms", format!("{first:.3}")),
                ("commands", commands.to_string()),
            ]);
        }
        "open" => phase_open(),
        "count-open" => phase_count_open(),
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
    /// The band on how much *this program* had allocated at its high-water, in mebibytes, in
    /// the process that drew page one.
    ///
    /// **Not the process's high-water mark, which is mostly somebody else's**: see
    /// [`anonymous_high_water_mib`] for the arithmetic and [`mapped_resident_kib`] for what was
    /// measured. The whole-process figure is printed beside it and banded nowhere.
    peak_anon_mib: Pin,
    /// The band on the peak resident size of the process that only *opened* it, in mebibytes.
    ///
    /// The one memory figure with no graphics driver in it, and so the one that answers what a
    /// *document* costs: a device's own allocations are an order of magnitude larger than any
    /// document here and would hide the whole question.
    ///
    /// **This one is still the whole process's `VmHWM` where [`Row::peak_anon_mib`] is not**, and
    /// that is deliberate rather than an oversight: it was identical across all forty-four runs
    /// its band was derived from and has not moved since, because the only large file this
    /// process maps is the test binary itself. It is what the same figure looks like when no
    /// driver is in the process, which is why round 935's finding did not reach it — and the
    /// line prints its allocated share beside it so that a reader can see both halves here too.
    open_peak_mib: Pin,
    /// The band on how many bytes of the file an open reads, in kibibytes.
    read_kib: Pin,
    /// The band on how many read calls an open makes.
    ///
    /// **What a cold open's disk time is proportional to, counted.** Each read that misses the
    /// page cache is one round trip, and on this machine a round trip to a small file costs 0.125
    /// to 0.199 ms against a whole-figure band a fifth of a millisecond wide. The duration is the
    /// machine's; the number of trips is the program's, and this is it (see [`read_calls`]).
    read_calls: Pin,
    /// The band on how many thousands of instructions an open *executes*.
    ///
    /// **The figure with no machine in it at all, and the one this gate was missing.** Every
    /// other duration here is a claim about an afternoon: round 938 measured a warm open rising
    /// 43% under eight spinning neighbours with the kernel's own wait counter at exactly zero,
    /// which is a machine executing the same instructions more slowly and is not something any
    /// clock beside the figure can subtract. An instruction count cannot move that way. It is
    /// the same number on a loaded machine, on a quiet one, on a slower processor and on a
    /// faster one, so it is judged always — and what principle 2 asks a cold-open gate for,
    /// *did opening a document become more expensive*, is exactly what it answers.
    ///
    /// Counted under callgrind, which this tree has used for the same reason since ADR 0180.
    /// Thousands of instructions, the way [`Row::read_kib`] is kibibytes.
    open_kinstructions: Pin,
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
    /// The band the *first pass* of that same work must land in, where the file states one.
    ///
    /// **`None` is the file saying nothing, and then nothing is judged on it** — which is where
    /// this stands since session 931 added the measurement: the quantity is the one every figure
    /// is made of (see [`calibration_pass`]), the band for it has to be derived on a quiet
    /// machine, and that round had none. The code is here so that deriving it is an edit to the
    /// check file rather than to this harness. ADR 0903, `doc/questions/Q29`.
    calibration_first_ms: Option<Band>,
    /// The band on a cold graphics bring-up, which principle 2 makes a gate of its own.
    bring_up_ms: Option<Band>,
    /// The band on what the graphics device costs *in allocated memory*, in mebibytes.
    ///
    /// Principle 2 makes cold bring-up a gate of its own "so that a regression in the driver, the
    /// adapter selection or the shader set is legible as itself"; the driver's memory is part of
    /// what that sentence is about, and until round 935 no figure here carried it. This is the
    /// device's share of a document row's [`Row::peak_anon_mib`], measured in a process that has
    /// done nothing else.
    bring_up_anon_mib: Option<Band>,
    /// The band a cold read of [`IO_PROBE_BYTES`] must land in for a *cold* figure to be judged.
    io_ms: Option<Band>,
    /// The band a cold read of [`IO_LATENCY_BYTES`] must land in for a *cold* figure to be judged.
    ///
    /// **The disk's latency where [`Check::io_ms`] is its throughput**, and the two are not the
    /// same machine's state: this gate's smallest documents fetch a hundred kibibytes in a
    /// handful of seeks, so what decides their cold opens is a round trip rather than a rate.
    /// See [`cold_latency_ms`].
    io_latency_ms: Option<Band>,
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
    /// See [`Row::peak_anon_mib`]. `None` here is "the key was not stated at all".
    peak_anon_mib: Option<Pin>,
    /// See [`Row::open_peak_mib`]. `None` here is "the key was not stated at all".
    open_peak_mib: Option<Pin>,
    /// See [`Row::read_kib`]. `None` here is "the key was not stated at all".
    read_kib: Option<Pin>,
    /// See [`Row::read_calls`]. `None` here is "the key was not stated at all".
    read_calls: Option<Pin>,
    /// See [`Row::open_kinstructions`]. `None` here is "the key was not stated at all".
    open_kinstructions: Option<Pin>,
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
        peak_anon_mib: Some(peak_anon_mib),
        open_peak_mib: Some(open_peak_mib),
        read_kib: Some(read_kib),
        read_calls: Some(read_calls),
        open_kinstructions: Some(open_kinstructions),
        why: Some(why),
    } = partial
    else {
        return Err(format!(
            "the row ending at line {at} is missing one of path, pages, cold_open_ms, \
             warm_open_ms, first_page_ms, turn_ms, peak_anon_mib, open_peak_mib, read_kib, \
             read_calls, open_kinstructions, why"
        ));
    };
    into.push(Row {
        path,
        pages,
        cold_open_ms,
        warm_open_ms,
        first_page_ms,
        turn_ms,
        peak_anon_mib,
        open_peak_mib,
        read_kib,
        read_calls,
        open_kinstructions,
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
                "peak_anon_mib" => row.peak_anon_mib = Some(band(value, at)?),
                "open_peak_mib" => row.open_peak_mib = Some(band(value, at)?),
                "read_kib" => row.read_kib = Some(band(value, at)?),
                "read_calls" => row.read_calls = Some(band(value, at)?),
                "open_kinstructions" => row.open_kinstructions = Some(band(value, at)?),
                other => return Err(format!("line {at} states an unknown row key `{other}`")),
            }
            continue;
        }
        match key {
            "machine" => check.machine = quoted()?,
            "profile" => check.profile = quoted()?,
            "calibration_document" => check.calibration_document = quoted()?,
            "calibration_ms" => check.calibration_ms = band(value, at)?.band(),
            "calibration_first_ms" => check.calibration_first_ms = band(value, at)?.band(),
            "bring_up_ms" => check.bring_up_ms = band(value, at)?.band(),
            "bring_up_anon_mib" => check.bring_up_anon_mib = band(value, at)?.band(),
            "io_ms" => check.io_ms = band(value, at)?.band(),
            "io_latency_ms" => check.io_latency_ms = band(value, at)?.band(),
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

/// How much of a child's high-water mark was memory *this program* asked for, in mebibytes.
///
/// **The figure round 935 put in place of the process's own high-water, and the reason is
/// measured rather than argued.** `VmHWM` counts every resident page, and in a process that has
/// brought a graphics device up nine tenths of them are pages of a *mapped file* — the Vulkan
/// loader's shared objects, `libLLVM.so` and `libgallium.so` above all. How many of those the
/// kernel keeps resident is decided by the page cache and by fault-around, not by this program:
/// evicting those two libraries and changing nothing else moved the whole-process figure by 25%
/// and this one by 30 KiB. So the high-water this gate holds to a band is the *anonymous* one —
/// what the allocator asked the kernel for — and `VmHWM` is printed beside it, unbanded.
///
/// `Rss - Anonymous` is read a moment *before* `VmHWM` in the child, so subtracting it from a
/// high-water cannot go negative in the ordinary case; `saturating_sub` covers the case where the
/// resident set grew between the two reads, which would otherwise print a wrapped figure.
fn anonymous_high_water_mib(fields: &Fields) -> f64 {
    let peak = field(fields, "peak_kib").unwrap_or(0.0);
    let mapped = field(fields, "mapped_kib").unwrap_or(0.0);
    (peak - mapped).max(0.0) / 1024.0
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

/// Whether this run was asked for the figures that are wall clocks.
///
/// Asked once, because a run that measured half its figures under one answer and half under the
/// other would print a table nobody could read. See [`CLOCK_FIGURES`].
fn clocks_are_wanted() -> bool {
    static WANTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *WANTED.get_or_init(|| std::env::var(CLOCK_FIGURES).is_ok())
}

/// The calibration document every child is handed, set once by the gate.
///
/// A `OnceLock` rather than an environment variable of the parent's, because setting one is
/// `unsafe` since Rust 2024 and there is nothing to be gained by it: the value is this process's
/// own and it reaches the children through their own environment.
static CALIBRATION_DOCUMENT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

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
    if let Some(calibration) = CALIBRATION_DOCUMENT.get() {
        child.env(CALIBRATION_PATH, calibration);
    }
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

/// How many thousands of instructions a process that opens `document` and does nothing else
/// executes, and how many pages it found — counted by callgrind.
///
/// **The one figure here with no machine in it**, and the reason round 938 added it. Every other
/// duration this gate judges is a claim about an afternoon, and the check file's own header now
/// says what such a claim is worth on a machine three rounds share. An instruction count is not
/// that kind of claim: it does not move with a processor's clock, with a neighbour inside the
/// same core, with the page cache or with the disk, so it can be judged on any machine under any
/// load — and *did opening a document become more expensive* is the question principle 2 asks a
/// cold-open gate, answered without a stopwatch.
///
/// Measured over five consecutive runs of each document at one-minute load averages between 6
/// and 12, the four figures moved by at most 0.03% and two of them not at all. The residue is
/// this program's hash seeds, which are the process's own and differ from run to run; the bands
/// are the observed range widened by half a percent on each side, which is forty times tighter
/// than the clock bands beside them.
///
/// The child is [`phase_count_open`], which does the open and stops: no calibration probe and no
/// `/proc`, because every instruction beside the open is an instruction in the figure. It is not
/// pinned — a count does not care which core runs it — and it costs a fifth of a second for
/// three of the documents and two fifths for the 1023-page one.
fn open_kinstructions(document: &Path, into: &Path) -> Result<(f64, f64), String> {
    let exe = std::env::current_exe().map_err(|error| format!("no current exe: {error}"))?;
    let profile = into.join("callgrind.out");
    // Removed rather than overwritten, so that a run in which valgrind produced nothing reads
    // the *previous* run's total and reports it as this one's (trap 10a — a stale artefact is a
    // measurement of the past wearing today's date).
    let _ = std::fs::remove_file(&profile);
    let output = Child::new("valgrind")
        .arg("--tool=callgrind")
        .arg(format!("--callgrind-out-file={}", profile.display()))
        .arg(exe)
        .args(["--exact", "launch_probe", "--nocapture", "--test-threads=1"])
        // **A cleared environment, and it is not tidiness.** Every byte of a process's
        // environment is copied and walked at start-up, so the same binary counted under `cargo
        // test` and counted from a shell differed by 22 thousand instructions — a figure that
        // moved with *how the gate was invoked* would not be the property of the program this
        // row claims it is. Two variables in, and nothing else.
        .env_clear()
        .env(PHASE, "count-open")
        .env(DOCUMENT_PATH, document)
        .output()
        .map_err(|error| format!("valgrind did not run: {error}"))?;
    let said = String::from_utf8_lossy(&output.stdout);
    // **The count is a number whatever the child did**, so the child has to say it opened
    // something: a process that failed to find the document executes fewer instructions and
    // would read as a win (trap 16).
    let pages = said
        .lines()
        .find_map(|line| line.find(MARKER).map(|at| &line[at..]))
        .and_then(|line| {
            line.split_whitespace()
                .filter_map(|piece| piece.split_once('='))
                .find(|&(key, _)| key == "pages")
                .and_then(|(_, value)| value.parse::<f64>().ok())
        })
        .ok_or_else(|| {
            format!(
                "the counted open said nothing: {} / {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .next_back()
                    .unwrap_or("no stderr")
            )
        })?;
    let text = std::fs::read_to_string(&profile)
        .map_err(|error| format!("callgrind wrote no profile: {error}"))?;
    let total: f64 = text
        .lines()
        .find_map(|line| line.strip_prefix("totals:"))
        .ok_or_else(|| "callgrind's profile states no totals".to_owned())?
        .trim()
        .parse()
        .map_err(|_| "callgrind's total is not a number".to_owned())?;
    Ok((total / 1e3, pages))
}

/// Whether callgrind is on this machine, asked once.
///
/// **Absence is printed rather than passed over.** A gate that skips a figure in silence is worse
/// than one that never had it, so the run says the figure was not measured and the summary counts
/// it; it is not a failure, because a machine without valgrind is a machine, not a regression.
fn callgrind_is_here() -> bool {
    static HERE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *HERE.get_or_init(|| {
        Child::new("valgrind")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
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
    before_each: &mut dyn FnMut() -> Result<Fields, String>,
) -> Result<(f64, Fields), String> {
    let mut best: Option<(f64, Fields)> = None;
    for _ in 0..samples {
        // Whatever the caller measured while preparing this sample joins the sample's own
        // fields: the cold arm's `io_ms` is the disk's state at the moment this child read, and
        // it is paired with the figure exactly as the child's own calibration is.
        let mut prepared = before_each()?;
        let mut fields = run_phase(phase, document)?;
        fields.append(&mut prepared);
        let value =
            field(&fields, key).ok_or_else(|| format!("the {phase} child printed no {key}"))?;
        if best.as_ref().is_none_or(|(seen, _)| value < *seen) {
            best = Some((value, fields));
        }
    }
    best.ok_or_else(|| format!("{phase} was not sampled at all"))
}

/// How large the file [`cold_latency_ms`] reads is.
///
/// A hundred and twenty-eight kibibytes, which is the order of the two smallest documents here
/// (85 and 106 KiB read), and written in one piece so that it is one extent: what this probe is
/// asked is how long *one* trip to the disk costs, and a fragmented file would answer with
/// several.
const IO_LATENCY_BYTES: usize = 128 << 10;

/// How long a cold read of a *small* fixed file takes right now, in milliseconds.
///
/// **The disk probe made of the same stuff as the figure it guards, which round 938 added and
/// trap 34 asks for.** [`cold_read_ms`] reads eight mebibytes, so what it measures is the disk's
/// *throughput*; a five-page document's cold open is a hundred kibibytes fetched in a handful of
/// seeks, so what decides it is the disk's *latency*, and the two move independently. Measured on
/// a quiet machine, this gate's own copies read cold in 0.125 ms (one extent, 134 KB) and 0.199
/// (two extents, 173 KB) at best over sixty samples, while the eight-mebibyte probe sat at 2.8 ms
/// throughout — 2.6 GB/s, at which rate a hundred kibibytes would be 0.04 ms. Almost all of a
/// small document's cold read is the round trip, and nothing in this gate could see it.
///
/// Session 931 recorded the same measurement at **0.109 min, 0.127 median** on
/// `PDF20_AN001-BPC.pdf`'s copy where this round reads 0.199 and 0.239 on sixty samples of a
/// quiet machine, which is the size of the term that has been putting the smallest rows over
/// their ceilings. ADR 0916.
fn cold_latency_ms(probe: &Path) -> Result<f64, String> {
    drop_the_page_cache(probe)?;
    let began = Instant::now();
    let bytes = std::fs::read(probe)
        .map_err(|error| format!("the latency probe does not read: {error}"))?;
    let elapsed = ms(began);
    if bytes.len() < IO_LATENCY_BYTES {
        return Err(format!(
            "the latency probe is {} bytes and should be {IO_LATENCY_BYTES}",
            bytes.len()
        ));
    }
    Ok(elapsed)
}

/// How long a cold read of a fixed file takes right now, in milliseconds.
///
/// **The guard's other half, and the cold open is why it exists.** The calibration probe is
/// deliberately CPU-bound and in memory, so it cannot see a neighbour queueing the disk — and a
/// cold open is mostly disk. This gate failed on a five-page document's cold open at 0.841 ms
/// against a band of 0.49 .. 0.80 while its child's calibration read 0.707, dead centre, because
/// the two figures are about different machines' worth of contention.
///
/// The probe is a file of this gate's own, beside the copies whose caches it drops, evicted and
/// read the same way an open reads a document. Its band is in the check file, and a cold figure
/// is judged only where it holds.
fn cold_read_ms(probe: &Path) -> Result<f64, String> {
    drop_the_page_cache(probe)?;
    let began = Instant::now();
    let bytes =
        std::fs::read(probe).map_err(|error| format!("the io probe does not read: {error}"))?;
    let elapsed = ms(began);
    if bytes.len() < IO_PROBE_BYTES {
        return Err(format!(
            "the io probe is {} bytes and should be {IO_PROBE_BYTES}",
            bytes.len()
        ));
    }
    Ok(elapsed)
}

/// How large the file [`cold_read_ms`] reads is.
///
/// Eight mebibytes: large enough that the read is the disk rather than the syscall, small enough
/// that thirty-six of them cost a fraction of a second and that evicting it disturbs nothing
/// else. The four documents this gate opens read between 85 KiB and 4.3 MiB, so the probe is of
/// the same order as the largest of them.
const IO_PROBE_BYTES: usize = 8 << 20;

/// Makes the file [`cold_latency_ms`] reads, once per run.
fn io_latency_probe(directory: &Path) -> Result<PathBuf, String> {
    let probe = directory.join("io-latency.bin");
    let wanted = vec![0x5A_u8; IO_LATENCY_BYTES];
    // `u64` against a `usize` constant rather than a cast: exact on every target this builds for.
    if std::fs::metadata(&probe).is_ok_and(|found| found.len() == IO_LATENCY_BYTES as u64) {
        return Ok(probe);
    }
    std::fs::write(&probe, &wanted).map_err(|error| format!("the latency probe: {error}"))?;
    Ok(probe)
}

/// Makes the file [`cold_read_ms`] reads, once per run.
fn io_probe(directory: &Path) -> Result<PathBuf, String> {
    let probe = directory.join("io-probe.bin");
    let wanted = vec![0x5A_u8; IO_PROBE_BYTES];
    // `u64` against a `usize` constant rather than a cast: the constant is what the file must be,
    // and widening the comparison is exact on every target this builds for.
    if std::fs::metadata(&probe).is_ok_and(|found| found.len() == IO_PROBE_BYTES as u64) {
        return Ok(probe);
    }
    std::fs::write(&probe, &wanted).map_err(|error| format!("the io probe: {error}"))?;
    Ok(probe)
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
    /// bytes an open reads, and how much memory this program asks for, are properties of the
    /// reader. Every *duration* is in the other group, and is judged only where the calibration
    /// probe says this is the machine the bands were taken on.
    ///
    /// **Every memory figure here is in the first group since round 935, and one of them used to
    /// be in the second.** What was judged as the memory high-water was the whole process's
    /// `VmHWM`, and that fell away from its band three times — 12%, then 13%, with no change to
    /// any code on the launch path — because nine tenths of it is resident pages of the Vulkan
    /// loader's shared objects and how many of those the kernel keeps is not this program's
    /// decision (see [`anonymous_high_water_mib`] and [`mapped_resident_kib`]). The figure banded
    /// now is the *anonymous* high-water, which held to 30 KiB across an eviction that moved the
    /// whole-process figure by 25%, and the whole-process figure is printed unbanded beside it.
    /// ADR 0910.
    steady: bool,
    /// What the child that produced this figure measured the machine at, right afterwards.
    ///
    /// `None` for a figure no child reported one for, which is judged as though the machine were
    /// unknown — that is, not at all.
    calibration: Option<f64>,
    /// What a cold read of a *small* fixed file cost at the moment this figure's sample was
    /// prepared — the disk's latency, where [`Self::io`] is its throughput.
    ///
    /// `None` for every figure but the cold open, as [`Self::io`].
    io_latency: Option<f64>,
    /// What a cold read of a fixed file cost at the moment this figure's sample was prepared.
    ///
    /// `None` for every figure but the cold open, which is the only one with a disk in it; a
    /// figure with no `io_ms` is not held to the disk's band. See [`cold_read_ms`].
    io: Option<f64>,
    /// What share of this figure was its thread waiting for a processor, before [`corrected`]
    /// took it off.
    ///
    /// `None` where the child did not report one, which is every platform but Linux.
    waited_share: Option<f64>,
    /// What one *first* pass of the calibration work cost in that same child.
    ///
    /// **Printed and not judged**, which is `doc/todo/05`'s rule for a figure whose band has not
    /// been derived yet. It is here because it is the quantity every figure above is made of —
    /// one pass, once, in a process that has not done it before — where [`Self::calibration`] is
    /// the best of fifty in a warmed one. See [`calibration_pass`] and `doc/todo/42`.
    calibration_first: Option<f64>,
}

/// Which of a figure's three conditions failed, in the order they are asked.
///
/// An enumeration rather than three flags, because three booleans in a signature is a place to
/// pass the wrong one — and because the *order* is the answer: a run that is not judging at all
/// says nothing about the disk.
#[derive(Clone, Copy)]
enum Declined {
    /// The whole run declined — a profile, a sample override, or the headline probe.
    TheRun,
    /// The child that produced this figure was not on the machine's own clock.
    ItsProbe,
    /// That child's *first* pass of the same work was outside its band.
    ItsFirstPass,
    /// The disk was outside its band when this figure's sample was prepared.
    TheDisk,
    /// The disk answered a *small* read too slowly when this figure's sample was prepared.
    TheDiskLatency,
    /// The thread that produced it spent too much of its own time waiting for a processor.
    ItWaited,
    /// Nothing declined it — the figure was judged, and these are the readings beside it.
    Nothing,
}

/// Why one figure was not judged, in the words of the probe that declined it.
///
/// **A gate that declines has to say what declined it.** Session 926 read four runs of this gate
/// in which one cold open was outside its band, could not tell a figure the processor declined
/// from one the disk declined, and wrote three hypotheses into `doc/todo/42` for a later round to
/// separate. The reading each probe took goes out beside the reason, because the reason alone
/// does not say by how much.
fn why_not(declined: Declined, figure: &Judged) -> String {
    let reading = |what: &str, value: Option<f64>| match value {
        Some(value) => format!("{what} {value:.3} ms"),
        None => format!("{what} not reported"),
    };
    let machine = format!(
        "{}, {}, {}, {}",
        reading("this child's calibration", figure.calibration),
        reading("its first pass", figure.calibration_first),
        reading("the disk's rate", figure.io),
        reading("its latency", figure.io_latency)
    );
    let machine = match figure.waited_share {
        Some(share) if share > 0.0 => {
            format!(
                "{machine}, {:.1}% of it waiting for a processor",
                share * 100.0
            )
        }
        _ => machine,
    };
    let said = match declined {
        Declined::TheRun => "the run is not judging at all",
        Declined::ItsProbe => "the child that produced it was not on the machine's own clock",
        Declined::ItsFirstPass => {
            "the child's first pass of that same work was outside its band, so the machine was \
             busy in the way a figure feels and the fifty-pass minimum cannot see"
        }
        Declined::TheDisk => "the disk was outside its band when this sample was prepared",
        Declined::TheDiskLatency => {
            "the disk answered a small cold read outside its band when this sample was prepared, \
             and a small document's cold open is round trips rather than bytes"
        }
        Declined::ItWaited => {
            "the thread that produced it spent more than a tenth of its own time waiting for a \
             processor, so what it came back to was not this machine's caches either"
        }
        Declined::Nothing => return machine,
    };
    format!("{said} — {machine}")
}

/// What one child said, and which of its fields carry the elapsed time and the wait.
///
/// A named pair rather than two arguments, because two `&str`s and a `&Fields` in a row is a
/// place to pass the wrong one — and because "this figure has no clock in it" is then the
/// absence of a `waited`, stated once at the call site.
#[derive(Clone, Copy)]
struct Sample<'a> {
    /// Every `key=value` the child printed.
    fields: &'a Fields,
    /// The two field names holding this figure's elapsed time and its wait for a processor, or
    /// `None` for a figure with no clock in it.
    waited: Option<(&'a str, &'a str)>,
}

/// Remembers a figure against the band its row states, and nothing where the row states `none`.
fn band_it(
    into: &mut Vec<(String, Judged)>,
    what: String,
    key: &'static str,
    value: f64,
    pin: Pin,
    steady: bool,
    sample: Sample<'_>,
) {
    let Sample { fields, waited } = sample;
    // A figure with a clock in it is a claim about a machine, and this run was not asked for one.
    if !steady && !clocks_are_wanted() {
        return;
    }
    if let Some(band) = pin.band() {
        into.push((
            what,
            Judged {
                key,
                value,
                band,
                steady,
                calibration: field(fields, "calibration_ms"),
                io: field(fields, "io_ms"),
                io_latency: field(fields, "io_latency_ms"),
                waited_share: waited.and_then(|(elapsed, runq)| {
                    let elapsed = field(fields, elapsed)?;
                    let runq = field(fields, runq)?;
                    (elapsed > 0.0).then(|| runq / elapsed)
                }),
                calibration_first: field(fields, "calibration_first_ms"),
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

    let clocks = clocks_are_wanted();
    let (samples, sampling_is_the_file_s) = match std::env::var(SAMPLE_OVERRIDE) {
        Ok(said) => match said.trim().parse::<usize>() {
            Ok(count) if count > 0 => (count, false),
            _ => panic!("{SAMPLE_OVERRIDE}={said}: expected a count above zero"),
        },
        Err(_) => (SAMPLES, true),
    };
    // One sample is enough for a figure a machine cannot move, and nine is what a claim about a
    // machine costs.
    let samples = if clocks { samples } else { 1 };
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
    // Every child gets it, because every child measures the machine after its own phase.
    let _ = CALIBRATION_DOCUMENT.set(calibration_document.clone());
    let calibration_band = check.calibration_ms;
    let machine_is_the_machine = if clocks {
        let calibration = match quickest(
            "calibrate",
            "calibration_ms",
            Some(&calibration_document),
            samples,
            &mut || Ok(Vec::new()),
        ) {
            Ok((value, _)) => value,
            Err(complaint) => panic!("the calibration probe: {complaint}"),
        };
        println!(
            "launch-path: calibration {calibration:.3} ms, band {}",
            calibration_band.map_or_else(
                || "none stated".to_owned(),
                |band| format!("{:.3} .. {:.3}", band.low, band.high)
            )
        );
        // The same fixed work measured the way every figure below is measured — once, in a
        // process that has not done it before. Printed and not judged; the check file says what
        // round 938 measured when it went to derive a band for it.
        match quickest(
            "calibrate",
            "calibration_first_ms",
            Some(&calibration_document),
            samples,
            &mut || Ok(Vec::new()),
        ) {
            Ok((first, _)) => println!(
                "launch-path: the same work as one first pass {first:.3} ms, no band — the \
                 quantity every figure below is made of"
            ),
            Err(complaint) => println!("launch-path: no first-pass calibration: {complaint}"),
        }
        calibration_band.is_some_and(|band| band.holds(calibration))
    } else {
        println!(
            "launch-path: the figures with a clock in them are not measured — set \
             {CLOCK_FIGURES} and run this when the machine is yours (doc/verify.md). What \
             follows is what this program does, which no machine can move."
        );
        false
    };

    let judging =
        clocks && machine_is_the_machine && sampling_is_the_file_s && profile == check.profile;
    if clocks && !judging {
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
    match quickest("bring-up", "bring_up_ms", None, samples, &mut || {
        Ok(Vec::new())
    }) {
        Ok((value, fields)) => {
            let adapter = fields
                .iter()
                .find(|(key, _)| key == "adapter")
                .map_or("unnamed", |(_, name)| name.as_str());
            let peak = field(&fields, "peak_kib").unwrap_or(0.0) / 1024.0;
            let allocated = anonymous_high_water_mib(&fields);
            println!(
                "launch-path: cold graphics bring-up {value:.1} ms on {adapter}, \
                 {peak:.0} MiB resident of which {allocated:.1} MiB is allocated \
                 and the rest is mapped libraries{}",
                waiting(&fields, "bring_up_runq_ms")
            );
            band_it(
                &mut judged,
                "the graphics device".to_owned(),
                "bring_up_ms",
                value,
                check.bring_up_ms.map_or(Pin::Nothing, Pin::Within),
                false,
                Sample {
                    fields: &fields,
                    waited: Some(("bring_up_wall_ms", "bring_up_runq_ms")),
                },
            );
            band_it(
                &mut judged,
                "what the graphics device allocates".to_owned(),
                "bring_up_anon_mib",
                allocated,
                check.bring_up_anon_mib.map_or(Pin::Nothing, Pin::Within),
                true,
                Sample {
                    fields: &fields,
                    waited: None,
                },
            );
        }
        Err(complaint) => complaints.push(format!("cold bring-up: {complaint}")),
    }

    let cache = cache_directory();
    let probe = cache
        .as_deref()
        .and_then(|directory| match io_probe(directory) {
            Ok(probe) => Some(probe),
            Err(complaint) => {
                println!("launch-path: no io probe: {complaint}");
                None
            }
        });
    let latency_probe = cache
        .as_deref()
        .and_then(|directory| match io_latency_probe(directory) {
            Ok(probe) => Some(probe),
            Err(complaint) => {
                println!("launch-path: no io latency probe: {complaint}");
                None
            }
        });
    let mut absent = 0_usize;
    let mut measured_documents = 0_usize;
    let mut uncounted = 0_usize;
    if !callgrind_is_here() {
        println!(
            "launch-path: NOT COUNTED — valgrind is not on this machine, so no row's \
             open_kinstructions is measured; every clock figure below is a claim about this \
             afternoon and nothing here is a claim about the program"
        );
    }
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
        // **The cold arm is a clock and nothing else.** Its child reports the bytes, the read
        // calls and the memory too, but so does the warm arm's, and dropping a page cache nine
        // times over is most of what this gate costs — so a run that was not asked for clocks
        // reads those three from the warm arm instead.
        let cold = if clocks {
            quickest("open", "open_ms", Some(cold_source), samples, &mut || {
                let Some(copy) = copy.as_deref() else {
                    return Err("there is no writable copy to drop the cache of".to_owned());
                };
                drop_the_page_cache(copy)?;
                let Some(probe) = probe.as_deref() else {
                    return Err("there is no io probe to time the disk with".to_owned());
                };
                let Some(latency) = latency_probe.as_deref() else {
                    return Err("there is no io latency probe to time the disk with".to_owned());
                };
                let io = cold_read_ms(probe)?;
                // Latency after throughput, because the eight-mebibyte read leaves the disk in the
                // state a document's open finds it in, which is what this sample is about.
                let seek = cold_latency_ms(latency)?;
                Ok(vec![
                    ("io_ms".to_owned(), format!("{io:.3}")),
                    ("io_latency_ms".to_owned(), format!("{seek:.3}")),
                ])
            })
        } else {
            quickest("open", "open_ms", Some(cold_source), samples, &mut || {
                Ok(Vec::new())
            })
        };
        let cold = match cold {
            Ok((value, fields)) => Some((value, fields)),
            Err(complaint) => {
                eviction = Some(complaint);
                None
            }
        };
        let warm = if clocks {
            quickest("open", "open_ms", Some(cold_source), samples, &mut || {
                Ok(Vec::new())
            })
        } else {
            Err("not asked for".to_owned())
        };
        let first = quickest(
            "first-page",
            "first_page_ms",
            Some(cold_source),
            samples,
            &mut || Ok(Vec::new()),
        );
        // A one-page document has no page to turn to, and asking for one is not a defect to
        // report. Its row states `turn_ms = none` and this is the other half of that.
        let turn = if row.pages > 1 && clocks {
            Some(quickest(
                "page-turn",
                "turn_ms",
                Some(cold_source),
                samples,
                &mut || Ok(Vec::new()),
            ))
        } else if clocks {
            println!("launch-path:   no page turn: the document has one page");
            None
        } else {
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
            let calls = field(fields, "read_calls").unwrap_or(0.0);
            let peak = field(fields, "peak_kib").unwrap_or(0.0) / 1024.0;
            let allocated = anonymous_high_water_mib(fields);
            println!(
                "launch-path:   {} open {value:.2} ms{}, {pages:.0} pages, \
                 {read:.0} KiB read, {peak:.0} MiB resident of which {allocated:.1} MiB is \
                 allocated, {calls:.0} read calls{}",
                if clocks { "cold" } else { "warm" },
                waiting(fields, "open_runq_ms"),
                match (field(fields, "io_ms"), field(fields, "io_latency_ms")) {
                    (Some(rate), Some(seek)) => format!(
                        ", the disk at {rate:.1} ms for {} MiB and {seek:.3} ms for one small \
                         file",
                        IO_PROBE_BYTES >> 20
                    ),
                    _ => String::new(),
                }
            );
            band_it(
                &mut judged,
                format!("{}: the cold open", row.path),
                "cold_open_ms",
                *value,
                row.cold_open_ms,
                false,
                Sample {
                    fields,
                    waited: Some(("open_wall_ms", "open_runq_ms")),
                },
            );
            band_it(
                &mut judged,
                format!("{}: the bytes an open reads", row.path),
                "read_kib",
                read,
                row.read_kib,
                true,
                Sample {
                    fields,
                    waited: None,
                },
            );
            band_it(
                &mut judged,
                format!("{}: the read calls an open makes", row.path),
                "read_calls",
                calls,
                row.read_calls,
                true,
                Sample {
                    fields,
                    waited: None,
                },
            );
            band_it(
                &mut judged,
                format!("{}: what an open costs in memory", row.path),
                "open_peak_mib",
                peak,
                row.open_peak_mib,
                true,
                Sample {
                    fields,
                    waited: None,
                },
            );
        }
        // The counted open, and the one figure a loaded machine cannot touch. After the cold
        // arm, because it runs the document through callgrind and would leave its pages hot.
        match (callgrind_is_here(), cache.as_deref()) {
            (true, Some(directory)) => match open_kinstructions(cold_source, directory) {
                Ok((value, counted_pages)) => {
                    println!(
                        "launch-path:   the open executes {value:.1} thousand instructions \
                         for {counted_pages:.0} pages"
                    );
                    band_it(
                        &mut judged,
                        format!("{}: the instructions an open executes", row.path),
                        "open_kinstructions",
                        value,
                        row.open_kinstructions,
                        true,
                        Sample {
                            fields: &Vec::new(),
                            waited: None,
                        },
                    );
                }
                Err(complaint) => {
                    complaints.push(format!("{}: the counted open: {complaint}", row.path));
                }
            },
            (true, None) => complaints.push(format!(
                "{}: the counted open has nowhere to write a profile",
                row.path
            )),
            (false, _) => uncounted = uncounted.saturating_add(1),
        }

        match warm {
            Ok((value, fields)) => {
                println!(
                    "launch-path:   warm open {value:.2} ms{}",
                    waiting(&fields, "open_runq_ms")
                );
                band_it(
                    &mut judged,
                    format!("{}: the warm open", row.path),
                    "warm_open_ms",
                    value,
                    row.warm_open_ms,
                    false,
                    Sample {
                        fields: &fields,
                        waited: Some(("open_wall_ms", "open_runq_ms")),
                    },
                );
            }
            Err(complaint) if complaint == "not asked for" => {}
            Err(complaint) => complaints.push(format!("{}: warm open: {complaint}", row.path)),
        }
        match first {
            Ok((value, fields)) => {
                let device = field(&fields, "device_ms").unwrap_or(0.0);
                let joined = field(&fields, "joined_ms").unwrap_or(0.0);
                let commands = field(&fields, "commands").unwrap_or(0.0);
                let peak = field(&fields, "peak_kib").unwrap_or(0.0) / 1024.0;
                let allocated = anonymous_high_water_mib(&fields);
                println!(
                    "launch-path:   first page {value:.1} ms{} (device up at {device:.1}, \
                     document joined at {joined:.1}, {commands:.0} commands), \
                     {peak:.0} MiB resident, {allocated:.1} MiB of it allocated",
                    waiting(&fields, "first_page_runq_ms")
                );
                band_it(
                    &mut judged,
                    format!("{}: time to first page", row.path),
                    "first_page_ms",
                    value,
                    row.first_page_ms,
                    false,
                    Sample {
                        fields: &fields,
                        waited: Some(("first_page_wall_ms", "first_page_runq_ms")),
                    },
                );
                band_it(
                    &mut judged,
                    format!("{}: the memory high-water it allocates", row.path),
                    "peak_anon_mib",
                    allocated,
                    row.peak_anon_mib,
                    true,
                    Sample {
                        fields: &fields,
                        waited: None,
                    },
                );
            }
            Err(complaint) => complaints.push(format!("{}: first page: {complaint}", row.path)),
        }
        match turn.unwrap_or_else(|| Err("not asked for".to_owned())) {
            Ok((value, fields)) => {
                let slowest = field(&fields, "slowest_turn_ms").unwrap_or(0.0);
                println!(
                    "launch-path:   page turn {value:.1} ms{} (slowest of {TURNS}: {slowest:.1})",
                    waiting(&fields, "turn_runq_ms")
                );
                band_it(
                    &mut judged,
                    format!("{}: a page turn", row.path),
                    "turn_ms",
                    value,
                    row.turn_ms,
                    false,
                    Sample {
                        fields: &fields,
                        waited: Some(("turn_wall_ms", "turn_runq_ms")),
                    },
                );
            }
            Err(complaint) if complaint == "not asked for" => {}
            Err(complaint) => complaints.push(format!("{}: page turn: {complaint}", row.path)),
        }
    }

    let mut unjudged = 0_usize;
    for (what, figure) in &judged {
        let held = figure.band.holds(figure.value);
        // **Per figure, and paired with its own child's probe.** A steady figure is judged
        // whatever the machine is doing; every other one is judged only where the process that
        // produced it says the machine was the machine — see [`Judged::steady`] and
        // [`calibration_field`].
        let probe_held = calibration_band
            .is_some_and(|band| figure.calibration.is_some_and(|its| band.holds(its)));
        let first_held = check
            .calibration_first_ms
            .is_none_or(|band| figure.calibration_first.is_none_or(|its| band.holds(its)));
        let disk_held = check
            .io_ms
            .is_none_or(|band| figure.io.is_none_or(|its| band.holds(its)));
        let latency_held = check
            .io_latency_ms
            .is_none_or(|band| figure.io_latency.is_none_or(|its| band.holds(its)));
        // The counted guard, and the one that needs no band: see [`WAITED_SHARE`].
        let waited_little = figure
            .waited_share
            .is_none_or(|share| share <= WAITED_SHARE);
        let machine_was_right =
            probe_held && first_held && disk_held && latency_held && waited_little;
        if !(figure.steady || (judging && machine_was_right)) {
            unjudged = unjudged.saturating_add(1);
            let declined = if judging {
                if !waited_little {
                    Declined::ItWaited
                } else if !probe_held {
                    Declined::ItsProbe
                } else if !first_held {
                    Declined::ItsFirstPass
                } else if !disk_held {
                    Declined::TheDisk
                } else {
                    Declined::TheDiskLatency
                }
            } else {
                Declined::TheRun
            };
            // **Which probe declined it, and what it read.** Session 926 had four runs of this
            // gate and no way to tell a figure declined by the processor from one declined by
            // the disk, which is most of why `doc/todo/42` carried three hypotheses instead of a
            // finding. One line answers it.
            println!(
                "launch-path:   ({what}) not judged: {}",
                why_not(declined, figure)
            );
        }
        if !held && (figure.steady || (judging && machine_was_right)) {
            // **A complaint carries the machine's readings too, and not only a declined figure
            // does.** Session 931 read twenty-six runs of this gate in which eight figures failed
            // and could not ask what their children's probes had said, because only the declined
            // ones printed any — so the question "would a tighter guard have declined this
            // instead of failing it" had no answer in the output. A figure with no clock in it
            // says so rather than printing three dashes.
            let machine = if figure.steady {
                "no clock in this figure".to_owned()
            } else {
                why_not(Declined::Nothing, figure)
            };
            complaints.push(format!(
                "{what} is {:.3}, outside {} {:.3} .. {:.3} ({machine})",
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
         {uncounted} not counted, {} figures banded, {unjudged} not judged, {} outside",
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
