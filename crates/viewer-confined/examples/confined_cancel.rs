//! A hostile document, and a host taking its thread back — from **either** of the two places the
//! drawing can happen.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example confined_cancel -- [levels] [--finish] [--marks]
//! ```
//!
//! What it shows is the thing a deadline cannot be set for. The document is a few hundred bytes
//! and draws for as long as its author chose — `levels` levels of form `XObject`, each drawing ten
//! of the level below — and the host is a thread with no way to know how long it should be.
//! `--finish` runs the same document to completion first, so that the two numbers stand beside
//! each other: what it costs to let it run, and what it costs to stop it.
//!
//! Three runs rather than one, because a single sample of a latency is not a measurement.
//!
//! # The two arms, and why they need two mechanisms
//!
//! **Default — the worker draws, and a `Canceller` is a kill.** At [`amplification::LEVELS`] the
//! page's marks are larger than any raster this boundary permits, so it crosses as *pixels*: the
//! confined process rasterises it, and `doc/todo/34` §3's cancel ends that process. The document
//! goes with it, which is what the "afterwards" line prints.
//!
//! **`--marks` — the host draws, and a kill reaches nothing.** One level shallower the marks are
//! 990 kB, which is under a window's pixels, so since ADR 0633 they cross as marks and since ADR
//! 0640 the worker does not draw them at all. The expensive thing then happens in the **host**,
//! outside the confinement, on the host's own thread. `Canceller::cancel` would end a worker that
//! has already finished and answered; what stops the draw is `pdf_render::Interrupt`, raised on
//! the rasteriser the host handed it to. ADR 0650.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::panic,
    reason = "an example whose whole output is what it printed; a run that cannot do the thing \
              should stop loudly rather than print a number about something else"
)]

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use pdf_render::{Interrupt, Rasterizer as _};
use render_cpu::CpuRasterizer;
use viewer_confined::{Confined, ConfinedError, Payload, Reply};
use viewer_core::{Command, DocumentId, Query};

/// The hostile document, shared with `tests/confined.rs`.
#[path = "../tests/support/amplification.rs"]
mod amplification;

/// How long the work is left running before it is stopped, so that there is something to stop.
///
/// One constant for both arms, and they are the same question asked of different processes: the
/// worker's rasterisation on the default arm, the host's on `--marks`.
const BEFORE_CANCEL: Duration = Duration::from_millis(250);

/// The viewport this draws into, in device pixels.
const VIEWPORT: (u32, u32) = (900, 1200);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut levels = amplification::LEVELS;
    let mut finish = false;
    let mut marks = false;
    for argument in arguments.by_ref() {
        if argument == "--finish" {
            finish = true;
        } else if argument == "--marks" {
            marks = true;
            levels = amplification::LEVELS.saturating_sub(1);
        } else if let Ok(parsed) = argument.parse() {
            levels = parsed;
        } else {
            eprintln!("usage: confined_cancel [levels] [--finish] [--marks]");
            std::process::exit(2);
        }
    }

    let bytes = amplification::document(levels, amplification::BRANCH);
    println!(
        "a {}-byte document that draws {} page-covering fills, {levels} levels deep",
        bytes.len(),
        amplification::fills(levels)
    );

    if marks {
        the_host_draws(levels, finish);
        return;
    }

    if finish {
        let at = Instant::now();
        let mut confined = start();
        confined
            .handle(&open(amplification::document(
                levels,
                amplification::BRANCH,
            )))
            .expect("an open crosses");
        println!("allowed to finish: {:.1} s", at.elapsed().as_secs_f64());
    }

    for run in 1..=3 {
        let mut confined = start();
        let canceller = confined.canceller();
        // The instant the cancel was *called*, so that what is printed is the host's wait rather
        // than the sleep before it. Written by the cancelling thread and read by this one.
        let called: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let recorded = Arc::clone(&called);
        let cancelling = std::thread::spawn(move || {
            std::thread::sleep(BEFORE_CANCEL);
            *recorded.lock().unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
            canceller.cancel();
        });

        let at = Instant::now();
        let outcome = confined.handle(&open(amplification::document(
            levels,
            amplification::BRANCH,
        )));
        let returned = Instant::now();
        cancelling.join().expect("the cancelling thread ends");

        let waited = called
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .map(|called| returned.duration_since(called));
        let blocked = returned.duration_since(at);
        match outcome {
            Err(ConfinedError::Cancelled) => println!(
                "run {run}: blocked {:.0} ms, then {:.3} ms from the cancel to the host having \
                 its thread back",
                blocked.as_secs_f64() * 1e3,
                waited.unwrap_or_default().as_secs_f64() * 1e3
            ),
            Err(other) => println!("run {run}: {other}"),
            Ok(events) => println!(
                "run {run}: it finished in {:.0} ms, so nothing was cancelled — {} events",
                blocked.as_secs_f64() * 1e3,
                events.len()
            ),
        }

        // And the viewer is finished with, which is what a cancel costs: the document went with
        // the worker, so a host that wants to carry on starts another one.
        println!(
            "  afterwards: is_cancelled {}, a question answers {:?}",
            confined.is_cancelled(),
            confined
                .query(Query::PageCount)
                .err()
                .map_or_else(|| "a count".to_owned(), |error| error.to_string())
        );
    }
}

/// The `--marks` arm: the worker ships the page undrawn and the **host** owns the drawing.
///
/// Three claims, in the order they have to hold. The page crosses as marks — **checked** rather
/// than assumed, because the level count that decides it is a comparison against the window's
/// pixels, and the round that changed one side of that comparison broke the cancel test by
/// assuming the other (ADR 0640). The worker answers in milliseconds, because since that ADR it
/// draws nothing it does not send. And the drawing that is left is the host's, where a
/// `Canceller` reaches nothing and an [`Interrupt`] reaches the command loop.
fn the_host_draws(levels: usize, finish: bool) {
    let mut confined = start();
    let at = Instant::now();
    confined
        .handle(&open(amplification::document(
            levels,
            amplification::BRANCH,
        )))
        .expect("an open crosses");
    let Reply::Frame(frames) = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds a frame for the page on the screen");
    };
    let answered = at.elapsed();
    let [shown] = frames.as_slice() else {
        panic!("a single-page arrangement crosses as one frame: {frames:?}")
    };
    let Payload::List { list, target } = &shown.payload else {
        panic!(
            "this arm is about the marks; {} fills crossed as pixels instead",
            amplification::fills(levels)
        )
    };
    let sent = viewer_confined::wire::encode_display_list(list).expect("a codable page");
    println!(
        "the worker answered in {:.0} ms with {} B of marks for a {}x{} target, and drew none \
         of it",
        answered.as_secs_f64() * 1e3,
        sent.len(),
        target.width,
        target.height
    );

    if finish {
        let at = Instant::now();
        CpuRasterizer::new()
            .rasterize(list, *target)
            .expect("the host draws the marks it was handed");
        println!(
            "the host allowed to finish: {:.1} s",
            at.elapsed().as_secs_f64()
        );
    }

    for run in 1..=3 {
        let interrupt = Interrupt::new();
        let raiser = interrupt.clone();
        // The instant the interrupt was *raised*, so that what is printed is the drawing thread's
        // wait rather than the sleep before it.
        let called: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let recorded = Arc::clone(&called);
        let raising = std::thread::spawn(move || {
            std::thread::sleep(BEFORE_CANCEL);
            *recorded.lock().unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
            raiser.raise();
        });

        let at = Instant::now();
        let outcome = CpuRasterizer::new()
            .interruptible(interrupt)
            .rasterize(list, *target);
        let returned = Instant::now();
        raising.join().expect("the raising thread ends");

        let waited = called
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .map(|called| returned.duration_since(called));
        let drawing = returned.duration_since(at);
        match outcome {
            Err(problem) => println!(
                "run {run}: drew for {:.0} ms, then {:.3} ms from the interrupt to the drawing \
                 thread having itself back — {problem}",
                drawing.as_secs_f64() * 1e3,
                waited.unwrap_or_default().as_secs_f64() * 1e3
            ),
            Ok(raster) => println!(
                "run {run}: it finished in {:.0} ms, so nothing was interrupted — {} B",
                drawing.as_secs_f64() * 1e3,
                raster.data.len()
            ),
        }
    }

    // **And the document is still open**, which is the whole difference from the kill above: the
    // interrupt was about one draw, and the worker never knew.
    println!(
        "  afterwards: is_cancelled {}, a question answers {:?}",
        confined.is_cancelled(),
        confined
            .query(Query::PageCount)
            .map_or_else(|error| error.to_string(), |reply| format!("{reply:?}"))
    );
}

/// A confined worker, sized to [`VIEWPORT`].
fn start() -> Confined {
    let mut confined = Confined::start().expect("a confined viewer starts");
    confined
        .handle(&Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        })
        .expect("a resize crosses");
    confined
}

/// The command that hands the document over.
fn open(bytes: Vec<u8>) -> Command {
    Command::Open {
        id: DocumentId(1),
        bytes,
        password: None,
        fragment: None,
    }
}
