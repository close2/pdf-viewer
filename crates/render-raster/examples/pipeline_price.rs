//! Where one heavy page's first frame goes on the processor, stage by stage, and what drawing the
//! display list's prefix while the rest is still being read would buy.
//!
//! ADR 1845 asks which change would move the project owner's witness — one content stream of
//! 278 579 415 inflated bytes, 29 million operators, 4.4 million marks — by an order of
//! magnitude, once ADRs 1829, 1835 and 1837 made the drawing cheap. A window's first frame on the
//! processor is today three stages one after the other: open the file, read the page into a
//! display list, draw the list. This example times each of them on one display list, and then
//! prices the candidate that overlaps the last two — the reader hands the list over in slices as
//! it builds it, and a drawer paints each slice as it arrives (okular's partial updates over
//! poppler, pdfium's progressive render, `doc/todo/69` section 4) — without building it:
//!
//! ```sh
//! cargo run --release -p render-raster --example pipeline_price -- <file.pdf> [page] [w] [h] [slices]
//! RAYON_NUM_THREADS=1 cargo run --release -p render-raster --example pipeline_price -- …
//! PIPELINE_READ_ONLY=1 cargo run --release -p render-raster --example pipeline_price -- …
//! ```
//!
//! 1. **The open**: the file's bytes read, `Document::open`, the page found.
//! 2. **The read**: the page interpreted on a thread of its own with a
//!    [`pdf_model::progress::Progress`] watched, and the figure sampled every few milliseconds
//!    from this thread — the reader's production curve, operators against time, which is what a
//!    drawer behind it would be waiting on. `PIPELINE_READ_ONLY` stops here, for a read timed on
//!    one class of core against the other.
//! 3. **What `quorra` predicts**: its two seeds (`Costs::DEVICE_SEED` and
//!    `Costs::PROCESSOR_SEED` in `crates/viewer-ui/src/bin/quorra/renderer.rs`) times this list's
//!    commands, beside what the processor measures below.
//! 4. **The draw, whole**: the list at the window's fit on this process's rayon pool, as
//!    `viewer_ui::software::compose_pages` draws it — strips on the pool's threads.
//! 5. **The prefix, priced**: the same list cut into `slices` consecutive runs of commands, each
//!    drawn on the pool onto a target of its own, and the schedule a reader and a drawer would
//!    keep: slice `k` starts once the reader has passed the operators that state it and once
//!    slice `k − 1` is drawn. The pipelined frame ends when the last slice does; the first
//!    picture is the first slice's.
//! 6. **The same schedule run**: the page read a second time while this thread draws the slices
//!    of the first reading, each as soon as the second reader's figure has passed the operators
//!    that state it — so the drawer's pool and the reader share the machine as a pipelined window
//!    would make them, and what the reader loses to that sharing is in the figure.
//!
//! **What the schedule assumes, and why it may.** The interpreter does not say which operator
//! made which command, so slice `k`'s commands are taken as stated by the first `k / slices` of
//! the page's operators. On the witness that is true by construction — one `q cm m l S Q` group
//! per mark from the first byte to the last, and ADR 1811 measured operators and input bytes
//! rising in step — and on a page whose marks bunch, the figure is the schedule of a page of the
//! same totals rather than of that page. Each slice is drawn onto a fresh target, so a slice pays
//! one target's allocation that an incremental drawer would not; and the composite of the slices
//! into one picture is left out, which an incremental drawer would also not pay, since it draws
//! into one target. Neither is a property of the candidate, and both are a few milliseconds.
//!
//! Every duration of sections 4 and 5 is the minimum of three draws in this process; pin the run
//! with `taskset` to one class of core, as `doc/habits/measuring.md` says.
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "a measurement tool: it stops loudly on a missing input, prints its answer, and its \
              arithmetic is on counts and durations far inside every type it uses"
)]

use std::time::{Duration, Instant};

use pdf_model::progress::Progress;
use pdf_render::{DisplayList, Rasterizer as _, TargetSpec};
use render_cpu::CpuRasterizer;

/// How many draws each arm of sections 4 and 5 takes, of which the quickest is reported.
const DRAWS: usize = 3;

/// How often the reader's figure is sampled.
const SAMPLE_EVERY: Duration = Duration::from_millis(5);

/// `quorra`'s seed for one command on the device, in nanoseconds — the figure
/// `Costs::DEVICE_SEED` states, repeated here because a binary's constant cannot be imported.
const DEVICE_SEED_NS: f64 = 1_788.0;

/// `quorra`'s seed for one command on the processor, in nanoseconds (`Costs::PROCESSOR_SEED`).
const PROCESSOR_SEED_NS: f64 = 667.0;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to draw");
    let index: usize = arguments
        .next()
        .map_or(1, |n| n.parse().expect("a page number"));
    let window = (
        arguments
            .next()
            .map_or(1600, |w| w.parse::<u32>().expect("a window width")),
        arguments
            .next()
            .map_or(1000, |h| h.parse::<u32>().expect("a window height")),
    );
    let slices: usize = arguments
        .next()
        .map_or(64, |n| n.parse().expect("a count of slices"));
    let read_only = std::env::var_os("PIPELINE_READ_ONLY").is_some();
    println!(
        "{path} page {index}, window {} × {}, rayon threads {}",
        window.0,
        window.1,
        rayon_threads()
    );

    // ── 1. the open ──
    let started = Instant::now();
    let bytes = std::fs::read(&path).expect("the document is readable");
    let read_file = started.elapsed();
    let document = pdf_syntax::Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(index.saturating_sub(1))
        .expect("that page exists");
    let opened = started.elapsed();
    println!(
        "open   file read {:>8.1} ms, document opened and page found {:>8.1} ms",
        ms(read_file),
        ms(opened)
    );

    // ── 2. the read, with its production curve ──
    let progress = Progress::new();
    let (interpretation, curve, read) = std::thread::scope(|scope| {
        let watched = progress.clone();
        let (document, page) = (&document, &page);
        let reader = scope.spawn(move || {
            let begun = Instant::now();
            let interpretation = watched.watch(|| pdf_model::content::interpret(document, page));
            (interpretation, begun.elapsed())
        });
        let begun = Instant::now();
        let mut curve = vec![(Duration::ZERO, 0_u64)];
        while !reader.is_finished() {
            std::thread::sleep(SAMPLE_EVERY);
            curve.push((begun.elapsed(), progress.read().operators));
        }
        let (interpretation, read) = reader.join().expect("the reader");
        (interpretation, curve, read)
    });
    let operators = progress.read().operators;
    let list = interpretation.display_list;
    let commands = list.command_count();
    println!(
        "read   {:>8.1} ms on its own thread: {operators} operators, {commands} commands, \
         {} list bytes charged — {:.1} ns an operator, {:.1} ns a command",
        ms(read),
        interpretation.list_bytes,
        read.as_nanos() as f64 / operators.max(1) as f64,
        read.as_nanos() as f64 / commands.max(1) as f64
    );
    for share in [0.1, 0.25, 0.5, 0.75, 0.9] {
        let at = reached(&curve, (operators as f64 * share) as u64, read);
        println!(
            "       {:>3.0} % of the operators read by {:>8.1} ms",
            share * 100.0,
            ms(at)
        );
    }
    if read_only {
        return;
    }

    // ── 3. what quorra predicts from its seeds ──
    println!(
        "seeds  device {:>8.1} ms, processor {:>8.1} ms for {commands} commands \
         (quorra's prediction before it has drawn the page)",
        commands as f64 * DEVICE_SEED_NS / 1e6,
        commands as f64 * PROCESSOR_SEED_NS / 1e6
    );

    // ── 4. the draw, whole, at the window's fit ──
    let unit = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("a target at one");
    let fit = (window.0 as f32 / unit.width as f32).min(window.1 as f32 / unit.height as f32);
    let target = TargetSpec::for_page(&list, fit, u64::MAX).expect("a fit target");
    let whole = quickest(|| draw(&list, target));
    println!(
        "draw   {:>8.1} ms, the fit page ({} × {} px at {fit:.4} px/unit) whole on the pool — \
         {:.1} ns a command, against the processor's seed of {PROCESSOR_SEED_NS:.0}",
        ms(whole),
        target.width,
        target.height,
        whole.as_nanos() as f64 / commands.max(1) as f64
    );
    let serial = opened + read + whole;
    println!(
        "serial {:>8.1} ms to the finished frame: the open, the read, then the draw",
        ms(serial)
    );

    // ── 5. the prefix, priced ──
    let all = list.commands();
    let per = all.len().div_ceil(slices.max(1)).max(1);
    let parts: Vec<DisplayList> = all
        .chunks(per)
        .map(|chunk| list.shape_list(chunk.to_vec()))
        .collect();
    let mut drawn = Vec::with_capacity(parts.len());
    for part in &parts {
        drawn.push(quickest(|| draw(part, target)));
    }
    let summed: Duration = drawn.iter().sum();
    let mut done = Duration::ZERO;
    let mut first = None;
    let mut waited = Duration::ZERO;
    for (k, cost) in drawn.iter().enumerate() {
        let through = ((k + 1) * per).min(all.len());
        let by = reached(
            &curve,
            (operators as f64 * through as f64 / all.len() as f64) as u64,
            read,
        );
        waited += by.saturating_sub(done);
        done = done.max(by) + *cost;
        first.get_or_insert(done);
    }
    let first = first.unwrap_or(done);
    println!(
        "prefix {} slices of {per} commands drawn as they arrive: their draws sum to {:>8.1} ms \
         ({:+.1} % on the whole draw); the drawer waited on the reader {:>8.1} ms",
        parts.len(),
        ms(summed),
        (ms(summed) / ms(whole).max(f64::EPSILON) - 1.0) * 100.0,
        ms(waited)
    );
    println!(
        "       first slice on the screen at {:>8.1} ms, the finished frame at {:>8.1} ms \
         (the read alone ends at {:>8.1} ms; serial {:>8.1} ms)",
        ms(opened + first),
        ms(opened + done),
        ms(opened + read),
        ms(serial)
    );

    // ── 6. the same schedule run, the reader and the drawer at once ──
    let progress = Progress::new();
    let (again, concurrent_first, concurrent_done) = std::thread::scope(|scope| {
        let watched = progress.clone();
        let (document, page) = (&document, &page);
        let begun = Instant::now();
        let reader = scope.spawn(move || {
            let started = Instant::now();
            drop(watched.watch(|| pdf_model::content::interpret(document, page)));
            started.elapsed()
        });
        let mut first = None;
        for (k, part) in parts.iter().enumerate() {
            let through = ((k + 1) * per).min(all.len());
            let needed = (operators as f64 * through as f64 / all.len() as f64) as u64;
            while progress.read().operators < needed && !reader.is_finished() {
                std::thread::sleep(Duration::from_millis(1));
            }
            draw(part, target);
            first.get_or_insert(begun.elapsed());
        }
        let done = begun.elapsed();
        let again = reader.join().expect("the reader");
        (again, first.unwrap_or(done), done)
    });
    println!(
        "both   the read beside the drawer {:>8.1} ms ({:+.1} % on the read alone); first slice \
         on the screen at {:>8.1} ms, the finished frame at {:>8.1} ms",
        ms(again),
        (ms(again) / ms(read).max(f64::EPSILON) - 1.0) * 100.0,
        ms(opened + concurrent_first),
        ms(opened + concurrent_done.max(again))
    );
}

/// The rayon pool's width this process draws on.
fn rayon_threads() -> usize {
    std::env::var("RAYON_NUM_THREADS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
}

/// When the reader's figure first stood at `operators` or more, read off the sampled curve —
/// the sample at or after it, so a drawer is never credited a slice before it was stated — and
/// the read's own end where no sample reached it.
fn reached(curve: &[(Duration, u64)], operators: u64, read: Duration) -> Duration {
    curve
        .iter()
        .find(|(_, seen)| *seen >= operators)
        .map_or(read, |(at, _)| (*at).min(read))
}

/// The quickest of [`DRAWS`] runs of `work`.
fn quickest(mut work: impl FnMut()) -> Duration {
    (0..DRAWS)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed()
        })
        .min()
        .unwrap_or(Duration::MAX)
}

/// One draw on the processor, the oracle's own rasteriser on this process's pool.
fn draw(list: &DisplayList, target: TargetSpec) {
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the processor draws every command on this page");
    drop(raster);
}

/// A duration in milliseconds.
fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}
