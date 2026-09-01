//! What re-placing §12.5.3's annotations would cost, against re-interpreting the page.
//!
//! `doc/todo/46` asks three questions before the seam in `content::interpret_into` can be
//! written, and the first of them is a measurement rather than a reading: a render request
//! carries one `Arc<DisplayList>`, so a re-placed page's list is the content prefix plus a
//! new tail, and the prefix is **copied** once per notch unless it is unshared. This prints
//! that copy beside the interpretation it would replace.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example replace_cost -- <file.pdf> <page> [runs]
//! ```
//!
//! Best-of rather than a median, because each of the three is a deterministic amount of work
//! on one input and what is wanted is the figure with the least of this machine's noise in it
//! — the same choice ADR 0775 made for the same pair of arms.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]

use std::time::{Duration, Instant};

use pdf_model::content::FontCache;
use pdf_syntax::Document;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(page)) = (arguments.next(), arguments.next()) else {
        println!("usage: replace_cost <file.pdf> <page> [runs]");
        return;
    };
    let Ok(index) = page.parse::<usize>() else {
        println!("the page is one-based, as a reader counts");
        return;
    };
    let runs: usize = arguments
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(12);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };
    let Ok(document) = Document::open(bytes) else {
        println!("{path} did not open");
        return;
    };
    let pages = pdf_model::page::Pages::new(&document);
    let Some(page) = pages.get(index.saturating_sub(1)) else {
        println!("{path} has no page {index}");
        return;
    };
    let state = pdf_model::view::ViewState::of(&document);
    // The cache a viewer already holds across a zoom, so the interpretation arm is what a notch
    // actually costs rather than what a first open does (ADR 0710).
    let fonts = FontCache::new();
    let mut interpret = Duration::MAX;
    let mut clone = Duration::MAX;
    let mut interpretation =
        pdf_model::content::interpret_with_fonts(&document, &page, &state, &fonts);
    for _ in 0..runs {
        let started = Instant::now();
        interpretation = pdf_model::content::interpret_with_fonts(&document, &page, &state, &fonts);
        interpret = interpret.min(started.elapsed());

        let started = Instant::now();
        let copy = interpretation.display_list.clone();
        clone = clone.min(started.elapsed());
        // Kept until after the clock is read so that nothing is optimised away, and dropped
        // outside both spans so that neither carries the free.
        drop(copy);
    }
    println!("{path} page {index}");
    println!(
        "  commands {}, view-dependent {}",
        interpretation.display_list.commands().len(),
        interpretation.view_dependent,
    );
    println!("  interpret          {interpret:>12.3?}");
    println!("  clone display list {clone:>12.3?}");
}
