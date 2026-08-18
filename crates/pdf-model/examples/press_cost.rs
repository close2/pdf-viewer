//! What sampling a press costs, measured as the difference between a cold and a warm one.
//!
//! `pdf_model::colour`'s press table is a process-wide `static`, so the *first* page of a
//! document that names a press pays for sampling it and every later page pays nothing. A
//! round asking what it would cost to scope that table to an interpretation needs the size of
//! that difference rather than an opinion about it, and this is the A/B in one sitting
//! `doc/habits.md`'s *Measuring* section asks for: interpret page one twice in one process and
//! subtract.
//!
//! ```sh
//! cargo run --release -p pdf-model --example press_cost -- <file>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::time::Instant;

use pdf_syntax::Document;

fn main() {
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let pages = pdf_model::Pages::new(&document);
        let Some(page) = pages.get(0) else {
            continue;
        };
        let before = pdf_model::colour::presses_cached();
        let cold = Instant::now();
        let first = pdf_model::interpret(&document, &page);
        let cold = cold.elapsed();
        let warm = Instant::now();
        let second = pdf_model::interpret(&document, &page);
        let warm = warm.elapsed();
        let third = Instant::now();
        let _ = pdf_model::interpret(&document, &page);
        let third = third.elapsed();
        println!(
            "{path}: presses {before} -> {}, cold {:.1} ms, warm {:.1} ms, warm again {:.1} ms, \
             commands {} then {}",
            pdf_model::colour::presses_cached(),
            cold.as_secs_f64() * 1000.0,
            warm.as_secs_f64() * 1000.0,
            third.as_secs_f64() * 1000.0,
            first.display_list.commands().len(),
            second.display_list.commands().len(),
        );
    }
}
