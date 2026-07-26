//! Interprets one page repeatedly, for deterministic instruction counting.
//!
//! Run under callgrind, which counts instructions rather than measuring time and is
//! therefore unaffected by CPU frequency, background load or thermal state:
//!
//! ```sh
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     cargo run --release -p pdf-model --example callgrind_interpret
//! ```
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool should stop loudly if the corpus file is missing, and \
              its command counter is bounded by fifty pages"
)]
fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(path).expect("corpus file is readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(100)
        .expect("page exists");

    let mut total = 0usize;
    for _ in 0..50 {
        total += pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .len();
    }
    println!("{total}");
}
