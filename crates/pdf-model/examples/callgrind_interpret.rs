//! Interprets one page repeatedly, for deterministic instruction counting.
//!
//! Run under callgrind, which counts instructions rather than measuring time and is
//! therefore unaffected by CPU frequency, background load or thermal state:
//!
//! ```sh
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     cargo run --release -p pdf-model --example callgrind_interpret
//! ```
//!
//! With no arguments it interprets page 101 of ISO 32000-2 itself, which is committed in
//! `doc/` and is the page `callgrind_rasterise.rs` draws, so the two numbers are about one
//! page. That page is text and vector graphics; a change to how *images* are decoded costs
//! nothing measurable on it, because image decode happens here rather than in the backend
//! and that page carries no image. So a path and a 1-based page number may be given
//! instead:
//!
//! ```sh
//! cargo run --release -p pdf-model --example callgrind_interpret -- \
//!     doc/pdf.js/test/pdfs/issue19971.pdf 1
//! ```
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool should stop loudly if the corpus file is missing, and \
              its command counter is bounded by fifty pages"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let named = args.next();
    let index = args
        .next()
        .map_or(101, |n| n.parse::<usize>().expect("a page number"));
    // A photograph is decoded once per interpretation and dwarfs everything else on its
    // page, so fifty repetitions of one would be fifty times a measurement one already
    // makes. The specification page is the opposite: small, and worth repeating to lift the
    // signal above process start-up.
    let repetitions = if named.is_some() { 1 } else { 50 };
    let path = named.map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );

    let bytes = std::fs::read(&path).expect("corpus file is readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");

    let mut total = 0usize;
    for _ in 0..repetitions {
        total += pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .len();
    }
    println!("{total}");
}
