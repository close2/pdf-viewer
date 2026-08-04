//! Opens one document repeatedly, for deterministic instruction counting.
//!
//! The launch path's first step and its largest one on a big file: §7.5's trailer and
//! cross-reference table, and nothing after them. `CLAUDE.md` says "a 500-page document must open
//! no slower than a 5-page one" and `doc/todo/42` says it does not, so this is where that is
//! attributed — under callgrind, which counts instructions and is therefore unaffected by CPU
//! frequency, background load or thermal state (`open_cost`'s wall clock moves by 2× between
//! runs on this machine).
//!
//! ```sh
//! cargo build --release -p pdf-syntax --example callgrind_open
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     target/release/examples/callgrind_open [file.pdf]
//! ```
//!
//! With no argument it opens ISO 32000-2 itself — 1023 pages, 101 318 objects — which is the
//! largest document committed to this tree and the one every other startup number is quoted on.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool should stop loudly if its file is missing, and its counter is \
              bounded by the repetition count above it"
)]
fn main() {
    let named = std::env::args().nth(1);
    // Ten, because opening this document is milliseconds rather than microseconds and the
    // process's own start-up is a fixed cost the ratio has to be lifted above.
    let repetitions = if named.is_some() { 1 } else { 10 };
    let path = named.map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );
    // An `Arc<[u8]>` cloned per repetition rather than a `Vec` cloned per repetition, and the
    // trailer counted rather than the table: the first put a 19 MB memcpy inside the loop and
    // the second walked 101 318 entries, and both were the harness measuring itself.
    let bytes: std::sync::Arc<[u8]> = std::fs::read(&path).expect("the file is readable").into();

    let mut total = 0usize;
    for _ in 0..repetitions {
        let document =
            pdf_syntax::Document::open(std::sync::Arc::clone(&bytes)).expect("valid PDF");
        total += document.xref().trailer().len();
    }
    println!("{total}");
}
