//! A digest of every corpus document's first page **as pixels**, for proving a change to a
//! rasteriser drew nothing differently.
//!
//! `examples/display_list_digest` is the same idea one layer up and answers a different question:
//! it proves the *interpreter* produced the list it produced before. A change inside
//! `pdf-render`, `render-cpu` or the medium those two agree on leaves every display list byte for
//! byte where it was and can still move every pixel on the page — which is exactly the shape of
//! change `doc/traps/pixels-and-rasterisers.md` trap 1 is about, and nothing in this tree could
//! see it directly. The gates' *summary* numbers are the wrong instrument for it twice over: a
//! verdict is a comparison against a reference with a tolerance in it, and two different rasters
//! can reach the same one.
//!
//! One line per document: the raster's extent, its byte length and a hash of its bytes. Run it on
//! two revisions and `diff` the two files; an empty diff is the claim.
//!
//! ```sh
//! cargo run --release -p pdf-model --example raster_digest -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! **`render-cpu` and not the device**, because this is the backend every gate rasterises with
//! and the one that is the oracle. A backend whose pixels are the device's own is compared
//! against this one by `render-quorra/tests/corpus.rs`, which is a different question with its
//! own gate.
//!
//! Both of `display_list_digest`'s cautions apply here unchanged and are not repeated: run both
//! arms with the same `pdf-sandbox-worker` on disk, and the hash is
//! [`std::collections::hash_map::DefaultHasher`], whose instability across releases costs nothing
//! when both arms are one sitting and what is wanted is a *difference*.
//!
//! **Calibrated against the defect it is meant to see** (trap 13), and the second calibration is
//! the one worth keeping. With `Medium::PAGE_ONLY`'s 𝑊 moved off white, every hash changes — the
//! instrument can fail. With its *surround* moved off white instead, **193 of the 957** change:
//! those are the pages whose extent is not a whole number of pixels at 72 dpi, so
//! `TargetSpec::for_page` rounds the raster up and a sliver of it lies outside §14.11.2.1's crop
//! box. That population is what a separation done carelessly would have moved on every gate in
//! this tree, and it is why `Medium::is_uniform` is a correctness decision rather than a
//! shortcut.
//!
//! **Cargo will hand you a stale binary here if you let it.** Adding a *new module file* to
//! `pdf-render` left the release-profile fingerprint of every crate above it unaware of the file,
//! so a `cargo build --release` after editing it recompiled nothing and this example printed the
//! previous revision's hashes — twice, in the session that wrote it. `touch` the changed crates'
//! `src/lib.rs` before believing either arm.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash as _, Hasher as _};

use pdf_model::{Pages, interpret};
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixels one page may cost, so that a malformed extent cannot exhaust this process.
const MAX_PIXELS: u64 = 64 << 20;

fn main() {
    let mut documents = 0_usize;
    let mut drawn = 0_usize;
    for path in std::env::args().skip(1) {
        documents = documents.saturating_add(1);
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |file| file.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            println!("{name}\tunreadable");
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            println!("{name}\tunopened");
            continue;
        };
        let Some(page) = Pages::new(&document).get(0) else {
            println!("{name}\tno page");
            continue;
        };
        let interpreted = interpret(&document, &page);
        let list = interpreted.display_list;
        let target = match TargetSpec::for_page(&list, 1.0, MAX_PIXELS) {
            Ok(target) => target,
            Err(problem) => {
                println!("{name}\tno target: {problem}");
                continue;
            }
        };
        match CpuRasterizer::new().rasterize(&list, target) {
            Ok(raster) => {
                let mut hasher = DefaultHasher::new();
                raster.data.hash(&mut hasher);
                drawn = drawn.saturating_add(1);
                println!(
                    "{name}\t{}x{}\t{} bytes\t{:016x}",
                    raster.width,
                    raster.height,
                    raster.data.len(),
                    hasher.finish()
                );
            }
            Err(problem) => println!("{name}\trefused: {problem}"),
        }
    }
    println!("# {documents} documents, {drawn} first pages rasterised");
}
