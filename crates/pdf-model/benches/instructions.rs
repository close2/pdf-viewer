//! Instruction counts for the interpretation pass, as a CI-stable performance gate.
//!
//! `benches/interpret.rs` measures wall clock, which is what a user feels but is not
//! something a build can gate on: it moves with CPU frequency, thermal state and whatever
//! else the machine is doing. During this project's own work one change measured as a 24%
//! regression and, twenty minutes later, an 8.5% improvement — the same code, a busier
//! machine.
//!
//! Counting instructions under Valgrind is deterministic. The same binary on the same
//! input gives the same number every time, on any machine, under any load. That is what
//! makes a threshold meaningful.
//!
//! Needs `valgrind` and a matching `cargo install iai-callgrind-runner`.

#![expect(
    clippy::expect_used,
    reason = "benchmark code: a missing corpus file should stop the run loudly"
)]
#![expect(
    missing_docs,
    unused_qualifications,
    reason = "the benchmark macros generate modules and constants of their own, which \
              cannot carry documentation, and expand paths in a nested scope"
)]

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

/// Loads a corpus page once, outside the measured region.
///
/// Parsing the document is not what these benchmarks are about, and including it would
/// bury the interpretation cost under a much larger constant.
fn corpus_page(file: &str, index: usize) -> (pdf_syntax::Document, pdf_model::Page) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(file);
    let bytes = std::fs::read(path).expect("corpus file is readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .expect("the page exists");
    (document, page)
}

#[library_benchmark]
#[bench::spec_body_text(corpus_page("ISO_32000-2_sponsored_EC3.pdf", 100))]
#[bench::spec_title(corpus_page("ISO_32000-2_sponsored_EC3.pdf", 0))]
#[bench::bare_cff_title(corpus_page("PDF20_AN001-BPC.pdf", 0))]
fn interpret(input: (pdf_syntax::Document, pdf_model::Page)) -> usize {
    let (document, page) = input;
    black_box(pdf_model::interpret(&document, &page))
        .display_list
        .commands()
        .len()
}

library_benchmark_group!(name = pages; benchmarks = interpret);
main!(library_benchmark_groups = pages);
