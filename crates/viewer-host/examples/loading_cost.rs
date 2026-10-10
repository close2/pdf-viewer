//! What the loading bar's figure costs a page, and what it says while the page is read.
//!
//! ```text
//! cargo run --release -p viewer-host --example loading_cost -- document.pdf [page] [rounds]
//! ```
//!
//! Interprets one page `rounds` times in alternation — once with nothing watching, once under a
//! `pdf_model::progress::Progress` with a second thread reading it every
//! `viewer_host::loading::TICK` the way a window does — and prints the wall time of each, the
//! difference as a share, and the figures the reading thread saw (ADR 1811). Alternating rather
//! than running one arm and then the other, so that a neighbour's build lands on both.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use pdf_model::progress::{Progress, Reading};

#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where its document is missing"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: loading_cost document.pdf [page] [rounds]");
    let index = args
        .next()
        .map_or(1, |n| n.parse::<usize>().expect("a page number"));
    let rounds = args
        .next()
        .map_or(3, |n| n.parse::<usize>().expect("a count of rounds"));

    let bytes = pdf_syntax::FileBytes::on_disk(std::path::Path::new(&path)).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("a PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index.saturating_sub(1))
        .expect("the page exists");

    let (mut bare, mut watched) = (Duration::ZERO, Duration::ZERO);
    let mut seen: Vec<(Duration, Reading)> = Vec::new();
    for round in 0..rounds {
        let began = Instant::now();
        let commands = pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .len();
        let took = began.elapsed();
        bare = bare.saturating_add(took);

        let progress = Progress::new();
        let done = Arc::new(AtomicBool::new(false));
        let reader = std::thread::spawn({
            let (progress, done) = (progress.clone(), Arc::clone(&done));
            move || {
                let began = Instant::now();
                let mut seen = Vec::new();
                while !done.load(Ordering::Relaxed) {
                    std::thread::sleep(viewer_host::loading::TICK);
                    seen.push((began.elapsed(), progress.read()));
                }
                seen
            }
        });
        let began = Instant::now();
        let under = progress.watch(|| {
            pdf_model::interpret(&document, &page)
                .display_list
                .commands()
                .len()
        });
        let took_watched = began.elapsed();
        watched = watched.saturating_add(took_watched);
        done.store(true, Ordering::Relaxed);
        let looks = reader.join().expect("the reading thread");
        assert_eq!(commands, under, "the figure changed what was drawn");
        println!(
            "round {round}: bare {:.3} s, watched {:.3} s, {commands} commands, {} looks",
            took.as_secs_f64(),
            took_watched.as_secs_f64(),
            looks.len()
        );
        if round == 0 {
            seen = looks;
        }
    }
    let share = (watched.as_secs_f64() - bare.as_secs_f64()) / bare.as_secs_f64() * 100.0;
    println!(
        "total: bare {:.3} s, watched {:.3} s, difference {share:+.2} %",
        bare.as_secs_f64(),
        watched.as_secs_f64()
    );
    let step = (seen.len() / 10).max(1);
    for (at, reading) in seen.iter().step_by(step) {
        println!(
            "  at {:6.2} s: {}",
            at.as_secs_f64(),
            viewer_host::loading::reading(*reading)
        );
    }
}
