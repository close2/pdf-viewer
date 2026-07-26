//! How long it takes to turn a real page into a display list.
//!
//! Interpretation is on the critical path for time-to-first-page, and it is where text
//! rendering spends its time: a dense page resolves and outlines several thousand glyphs.
//! This measures the whole pass rather than a synthetic loop, so a change that helps one
//! stage and hurts another shows up as the net effect a user would feel.

#![expect(
    clippy::expect_used,
    reason = "benchmark code: a missing corpus file should stop the run loudly"
)]

use criterion::{Criterion, criterion_group, criterion_main};

/// A page from the corpus, chosen for being dense with text rather than for being typical.
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

fn interpret_pages(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpret");

    for (file, index, label) in [
        // A body page of the specification: dense running text, the case text rendering
        // has to be fast for.
        ("ISO_32000-2_sponsored_EC3.pdf", 100, "spec-body-text"),
        // A title page: few glyphs, several fonts, one of them substituted.
        ("ISO_32000-2_sponsored_EC3.pdf", 0, "spec-title"),
        // Bare CFF simple fonts throughout.
        ("PDF20_AN001-BPC.pdf", 0, "bare-cff-title"),
    ] {
        let (document, page) = corpus_page(file, index);
        group.bench_function(label, |b| {
            b.iter(|| std::hint::black_box(pdf_model::interpret(&document, &page)));
        });
    }

    group.finish();
}

criterion_group!(benches, interpret_pages);
criterion_main!(benches);
