//! A hostile document, and a host taking its thread back.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example confined_cancel -- [levels] [--finish]
//! ```
//!
//! What it shows is the thing a deadline cannot be set for. The document is a few hundred bytes
//! and draws for as long as its author chose — `levels` levels of form `XObject`, each drawing ten
//! of the level below — and the host is a thread blocked in `Confined::handle` with no way to
//! know how long it should be. `--finish` runs the same document to completion first, so that the
//! two numbers stand beside each other: what it costs to let it run, and what it costs to stop it.
//!
//! Three cancels rather than one, because a single sample of a latency is not a measurement.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "an example whose whole output is what it printed; a run that cannot do the thing \
              should stop loudly rather than print a number about something else"
)]

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use viewer_confined::{Confined, ConfinedError};
use viewer_core::{Command, DocumentId, Query};

/// The hostile document, shared with `tests/confined.rs`.
#[path = "../tests/support/amplification.rs"]
mod amplification;

/// How long the document is left running before the cancel, so that there is something to cancel.
const BEFORE_CANCEL: Duration = Duration::from_millis(250);

/// The viewport this draws into, in device pixels.
const VIEWPORT: (u32, u32) = (900, 1200);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut levels = amplification::LEVELS;
    let mut finish = false;
    for argument in arguments.by_ref() {
        if argument == "--finish" {
            finish = true;
        } else if let Ok(parsed) = argument.parse() {
            levels = parsed;
        } else {
            eprintln!("usage: confined_cancel [levels] [--finish]");
            std::process::exit(2);
        }
    }

    let bytes = amplification::document(levels, amplification::BRANCH);
    println!(
        "a {}-byte document that draws {} page-covering fills, {levels} levels deep",
        bytes.len(),
        amplification::fills(levels)
    );

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
