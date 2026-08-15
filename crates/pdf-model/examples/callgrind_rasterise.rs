//! Rasterises one page repeatedly, for deterministic instruction counting.
//!
//! The companion to `callgrind_interpret.rs`, and it exists because that one cannot see the
//! backend at all: it stops at the display list, so a change to how a backend *draws* a
//! command measures as exactly zero there. Image resampling (ADR 0025) is the first change
//! this tree made that lives entirely on the far side of that line.
//!
//! Wall clock is not usable for this — see `benches/instructions.rs` for the two runs twenty
//! minutes apart that disagreed by 32% on identical code — so count instructions:
//!
//! ```sh
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     cargo run --release -p pdf-model --example callgrind_rasterise
//! ```
//!
//! With no arguments it draws page 101 of ISO 32000-2 itself, which is committed in `doc/`
//! and is the page `callgrind_interpret.rs` uses, so the two numbers are about one page.
//! A path and a 1-based page number may be given instead, which is how a corpus document
//! carrying a deeply reduced image gets measured:
//!
//! ```sh
//! cargo run --release -p pdf-model --example callgrind_rasterise -- \
//!     doc/pdf.js/test/pdfs/firefox_logo.pdf 1
//! ```
//!
//! A third argument is how many times the page is drawn, and twenty is only a default. A page
//! whose cost is one command can be a second of work on its own — a type 1 shading is a
//! function evaluated per device pixel — and twenty of those under callgrind is an hour where
//! one is a minute. Lower it for such a page and say so beside the number; the count is part
//! of the invocation, so two arms are only comparable when they share it.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool should stop loudly if its input is missing, and its ink \
              counter is bounded by the pixels of the pages it draws"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );
    let index = args
        .next()
        .map_or(101, |n| n.parse::<usize>().expect("a page number"));
    let repeats = args
        .next()
        .map_or(20, |n| n.parse::<usize>().expect("a repeat count"));

    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");

    // Interpreted once, outside the loop: this example is about the backend, and leaving
    // interpretation inside would bury the difference under the larger constant that
    // `callgrind_interpret.rs` already measures on its own.
    let list = pdf_model::interpret(&document, &page).display_list;
    // The last argument is a ceiling on *total pixels*, not on either extent: a page-sized
    // raster is half a million of them, and passing a plausible-looking 4096 here made every
    // run of this example panic while callgrind dutifully counted the panic.
    let target = pdf_render::TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");

    // Summing a byte of every raster keeps the optimiser from discarding the work, and is
    // cheap enough beside rasterisation not to distort the count.
    let mut ink = 0u64;
    for _ in 0..repeats {
        let raster = <render_cpu::CpuRasterizer as pdf_render::Rasterizer>::rasterize(
            &mut render_cpu::CpuRasterizer::new(),
            &list,
            target,
        )
        .expect("every command is supported");
        ink += raster.data.iter().map(|b| u64::from(*b)).sum::<u64>();
    }
    println!("{ink}");
}
