//! Interprets a *run of distinct pages*, for deterministic instruction counting.
//!
//! `callgrind_interpret` interprets one page fifty times, which is the right instrument for what
//! a page costs and the wrong one for what a *document* costs: a reader turns pages, and anything
//! one page leaves behind for the next is invisible to fifty repetitions of one page, which
//! repeat it in the first place. This walks pages 1..=`count` instead, once each.
//!
//! ```sh
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     cargo run --release -p pdf-model --example callgrind_pages -- 20
//! ```
//!
//! **The last argument is the A/B**, and both arms are this one binary, so a comparison needs no
//! second tree and cannot be a comparison of two compilers:
//!
//! - `fresh` (the default) builds a font cache per page and drops it, which is exactly what
//!   [`pdf_model::interpret`] and `interpret_with` do — what every caller did before the
//!   seven-hundred-and-seventieth session, and what the oracle and the corpus gates still do.
//! - `kept` passes one cache for the whole run, which is what `viewer_core`'s `Open` holds
//!   beside its document.
//!
//! **Both arms go through the same function and share one `ViewState`**, so the difference is
//! the cache and nothing else. Calling `interpret` for the `fresh` arm would have been the more
//! obvious spelling and would have measured `ViewState::of` as well — on a one-page document,
//! where the cache can only lose, that term is larger than the thing being weighed.
//!
//! Either word may be followed by `-revisit`, which interprets the **same** page `count` times
//! instead of `count` pages once. That is a different population and a real one: `Open::stale`
//! re-interprets the page on the screen whenever §8.11's layers, §12.7.5's field values or
//! §12.5.5's appearance under the pointer move, and a font is a function of the document rather
//! than of any of those.
//!
//! ```sh
//! cargo run --release -p pdf-model --example callgrind_pages -- \
//!     20 doc/pdf.js/test/pdfs/tracemonkey_annotation_on_page_8.pdf 1 kept
//! ```
//!
//! It prints the total command count of every display list it built, which is the answer the
//! interpretation produced: an A/B whose two arms print different totals has changed the picture
//! and is not the comparison it was meant to be.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool should stop loudly if the corpus file is missing, and its \
              command counter is bounded by the pages it was asked for"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let count = args
        .next()
        .map_or(20, |n| n.parse::<usize>().expect("a page count"));
    let named = args.next();
    let first = args
        .next()
        .map_or(1, |n| n.parse::<usize>().expect("a page number"));
    let arm = args.next().unwrap_or_default();
    let kept = arm.starts_with("kept");
    let revisit = arm.ends_with("-revisit");
    let path = named.map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );

    let bytes = std::fs::read(&path).expect("corpus file is readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let pages = pdf_model::Pages::new(&document);
    let state = pdf_model::view::ViewState::of(&document);
    let fonts = pdf_model::FontCache::new();

    let mut total = 0usize;
    for step in 0..count {
        let index = if revisit { first - 1 } else { first - 1 + step };
        let Some(page) = pages.get(index) else { break };
        let per_page;
        let cache = if kept {
            &fonts
        } else {
            per_page = pdf_model::FontCache::new();
            &per_page
        };
        let interpretation =
            pdf_model::content::interpret_with_fonts(&document, &page, &state, cache);
        total += interpretation.display_list.commands().len();
    }
    println!("{total}");
}
