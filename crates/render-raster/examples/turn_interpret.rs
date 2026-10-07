//! One page interpreted the way `turn_path`'s `interp` column interprets it, a given number of
//! times in one process, for callgrind.
//!
//! # What it is for
//!
//! `tests/support/frame_cost.rs` times `pdf_model::content::interpret_with_fonts` on a document
//! opened for the round, after whatever the rounds before it left in the process's own caches —
//! the presses `pdf-colour` samples and their ink tables among them. A single interpretation under
//! callgrind counts those caches being filled as though every turn paid for them, so this
//! interprets the page `n` times and a profile of the turn is the difference between a run of
//! two and a run of one, function by function (`doc/habits/measuring.md` 65: count per render,
//! not per process).
//!
//! # Running it
//!
//! ```sh
//! cargo build --profile gates -p render-raster --example turn_interpret
//! valgrind --tool=callgrind --callgrind-out-file=one.out \
//!     target/gates/examples/turn_interpret doc/pdf.js/test/pdfs/bug1721218_reduced.pdf 1 1
//! valgrind --tool=callgrind --callgrind-out-file=two.out \
//!     target/gates/examples/turn_interpret doc/pdf.js/test/pdfs/bug1721218_reduced.pdf 1 2
//! ```
//!
//! The arguments are a path, a 1-based page number and how many times to interpret it. Each
//! interpretation opens the document afresh, as `frame_cost::read` does, and prints its wall
//! clock and its list's command count, so the run is also a quick look at the column outside
//! callgrind.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a measurement example: a missing document must stop the run rather than print a \
              figure about nothing, and the lines on stdout are the point"
)]

use std::time::Instant;

use pdf_syntax::Document;

/// Opens `path` and interprets its page `index` (1-based) as `frame_cost::read` does, returning
/// the milliseconds the interpretation took and how many top-level commands it built.
fn interpret_once(path: &str, index: usize) -> (f64, usize) {
    let document = Document::open(std::fs::read(path).expect("the document is readable"))
        .expect("the document opens");
    let pages = pdf_model::Pages::new(&document);
    let state = pdf_model::view::ViewState::of(&document);
    let fonts = pdf_model::content::FontCache::new();
    let page = pages
        .get(index.saturating_sub(1))
        .expect("the document has that page");
    let began = Instant::now();
    let interpretation = pdf_model::content::interpret_with_fonts(&document, &page, &state, &fonts);
    let elapsed = began.elapsed().as_secs_f64() * 1000.0;
    (elapsed, interpretation.display_list.command_count())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a document path");
    let index = args
        .next()
        .map_or(1, |n| n.parse::<usize>().expect("a 1-based page number"));
    let times = args
        .next()
        .map_or(1, |n| n.parse::<usize>().expect("how many interpretations"));
    for round in 1..=times {
        let (elapsed, commands) = interpret_once(&path, index);
        println!("interpretation {round}: {elapsed:.2} ms, {commands} commands");
    }
}
