//! How much ink each backend lays on a page, at four resolutions — the instrument that says
//! whether two rasterisers disagree about *where* a mark is or about *how much of it there is*.
//!
//! # What it is for
//!
//! `tests/corpus.rs` names the pages the two backends draw differently and says how far apart
//! they are; it cannot say why, and its three figures — mean, worst tile, similarity — all
//! measure the same thing, pixel against pixel at one scale. Two quite different faults produce
//! the same reading there:
//!
//! - **the same ink in different places**, which is two scan converters disagreeing about a
//!   boundary pixel and is bounded by the coarser one's quantum, and
//! - **different amounts of ink**, which is one backend drawing a shape the other does not, or
//!   drawing it at the wrong size, or losing part of it to a clip or the raster's edge.
//!
//! Only the second is a defect that a clause can arbitrate, and this separates them. A backend
//! whose total is flat across the ladder is measuring the geometry; one whose total walks toward
//! the other's as the scale rises is paying a per-boundary cost that shrinks as the boundary
//! becomes a smaller share of the mark — which is the signature of a quantum, a promotion to a
//! whole pixel, or a product taken at an edge.
//!
//! **The geometry is the reference here, not either backend.** `doc/todo/00-ambiguous-bucket.md`
//! step 6 already measures ink at rising resolution to find the shape a page states; this is the
//! same question asked of two backends at once, so neither is the target and the standard's own
//! area rule is. ISO 32000-2 §10.7.4:
//!
//! > The area covered by painted pixels shall always be at least as large as the area of the
//! > original shape.
//!
//! # What the number is
//!
//! The sum over the raster of `765 - r - g - b`, divided by the scale squared so that the four
//! rungs are comparable. Under §11.3.6's Normal blend over an opaque backdrop a composited
//! channel is linear in the source's coverage, so this sum is linear in the ink the page
//! received: twice the coverage is twice the number, whatever colour it is in. It is not a
//! perceptual measure and is not meant to be — what it has to be is *additive in area*, which is
//! what makes a total at one scale comparable with a total at another.
//!
//! # Running it
//!
//! ```sh
//! cargo run --release -p render-raster --example ink_ladder -- issue20232 endchar
//! ```
//!
//! Each argument is a document stem under `doc/pdf.js/test/pdfs`, and a stem with no document is
//! a panic rather than a skipped row: a ladder that silently omitted a page would print a
//! shorter table than the list it was given and read as though every page had been measured.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: the table on stdout is the point, a missing corpus document \
              must stop the run rather than be reported as a backend's answer, and the sums are \
              over rasters whose pixel budget is bounded below f64's exact integer range"
)]

use std::path::Path;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// The rungs, in multiples of the page's own resolution.
///
/// One is what `tests/corpus.rs` compares at; eight is where a quarter-pixel quantum is a
/// sixty-fourth of a mark's area and any difference left is the geometry's. The two between are
/// what make the shape of the approach legible rather than only its endpoints — a total halving
/// its excess at every rung is a per-boundary cost, and one that does not move is not.
const LADDER: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

/// Pixel budget per page. Generous enough for the top rung of a page-sized document.
const PIXEL_BUDGET: u64 = 1 << 28;

fn main() {
    let stems: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        !stems.is_empty(),
        "name at least one document stem under doc/pdf.js/test/pdfs"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut backend = QuorraRasterizer::with_options(&render_raster::options()).expect("adapter");
    backend.set_coverage(raster_gpu::Coverage::Cpu);
    println!("scale-normalised ink, page one, cpu oracle against raster");
    for stem in &stems {
        let bytes = std::fs::read(root.join(format!("{stem}.pdf"))).expect("the corpus document");
        let document = Document::open(bytes).expect("opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(0).expect("has a first page");
        let list = pdf_model::content::interpret(&document, &page).display_list;
        print!("{stem:26}");
        for scale in LADDER {
            let Ok(target) = TargetSpec::for_page(&list, scale, PIXEL_BUDGET) else {
                print!("  {scale:.0}x        no target");
                continue;
            };
            let oracle = CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("draws");
            let area = f64::from(scale) * f64::from(scale);
            print!("  {scale:.0}x cpu {:10.2}", ink(&oracle) / area);
            match backend.rasterize(&list, target) {
                Ok(ours) => print!(" raster {:10.2}", ink(&ours) / area),
                Err(why) => print!(" raster refused: {why}"),
            }
        }
        println!();
    }
}

/// The ink on one raster: `765 - r - g - b` summed over its pixels.
///
/// See the module comment for why this and not a luminance — it has to be additive in area for
/// the rungs to be comparable, and a per-channel sum is.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| 765 - u64::from(pixel[0]) - u64::from(pixel[1]) - u64::from(pixel[2]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "765 times a raster under 2^28 pixels is inside f64's exact integer range"
    )]
    let sum = sum as f64;
    sum / 765.0
}
