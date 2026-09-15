//! What a page's clips cost each backend in ink — the instrument that says whether a clip is
//! *cutting* a mark or a backend is *losing* one at it.
//!
//! # What it is for
//!
//! `examples/ink_ladder` separates two rasterisers disagreeing about where a mark is from two
//! disagreeing about how much of it there is. Where they disagree about the amount and the gap
//! closes as the scale rises, the cost is being paid per boundary pixel — and a clip's boundary
//! is the commonest place for that, because both backends compose a clip with a mark by an
//! arithmetic that is not the intersection ISO 32000-2 §10.7.4 asks for:
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by a
//! > fill operation. Subsequent painting operations shall affect a region that is the
//! > intersection of the set of pixels defined by the clipping region with the set of pixels for
//! > the region to be painted.
//!
//! This draws the page twice on each backend — once as it is, and once with every command's clip
//! taken off — so the difference is what that page's clips cost *that backend*. A clip that cuts
//! nothing costs an intersection nothing at all, so anything this prints for such a page is the
//! composition rather than the document.
//!
//! **Read it beside the top rung.** At 8× a boundary pixel is a sixty-fourth of what it is at
//! page scale and §10.7.4's substitutions have nothing left to substitute, so a cost that is
//! there at 1× and gone at 8× is a boundary's and a cost that survives is a clip genuinely
//! cutting.
//!
//! # Running it
//!
//! ```sh
//! cargo run --release -p render-raster --example clip_cost -- bug1844576 issue16473
//! ```
//!
//! Each argument is a document stem under `doc/pdf.js/test/pdfs`, and a stem with no document is
//! a panic rather than a skipped row, for `ink_ladder`'s reason: a table shorter than the list it
//! was given reads as though every page had been measured.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: the table on stdout is the point, a missing corpus document \
              must stop the run rather than be reported as a backend's answer, and the sums are \
              over rasters whose pixel budget is bounded below f64's exact integer range"
)]

use pdf_render::{Command, DisplayList, Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// The rungs: the page's own scale, and the one where a boundary pixel is a sixty-fourth of what
/// it was.
const LADDER: [f32; 2] = [1.0, 8.0];

/// Pixel budget per page, as `ink_ladder`'s.
const PIXEL_BUDGET: u64 = 1 << 28;

/// Takes every command's clip off, recursing into groups.
///
/// §8.5.4's chains are held in a table beside the commands, so this leaves the table alone and
/// unnames it: what is drawn is each command's own geometry, which is what the clause's
/// intersection reduces to when the region contains the mark.
fn unclip(commands: &mut [Command]) {
    for command in commands {
        if let Command::Group { commands, .. } = command {
            unclip(commands);
        }
        command.set_clip(None);
    }
}

/// The ink on one raster: `765 - r - g - b` summed over its pixels, as `ink_ladder` counts it.
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

fn main() {
    let stems: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        !stems.is_empty(),
        "name at least one document stem under doc/pdf.js/test/pdfs"
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut backend = QuorraRasterizer::with_options(&render_raster::options()).expect("adapter");
    backend.set_coverage(raster_gpu::Coverage::Cpu);
    println!("scale-normalised ink, page one, as stated and with every clip taken off");
    for stem in &stems {
        let bytes = std::fs::read(root.join(format!("{stem}.pdf"))).expect("the corpus document");
        let document = Document::open(bytes).expect("opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(0).expect("has a first page");
        let list = pdf_model::content::interpret(&document, &page).display_list;
        let mut bare = DisplayList::new(list.page_size);
        let mut commands = list.commands().to_vec();
        unclip(&mut commands);
        for command in commands {
            bare.push(command);
        }
        print!("{stem:24}");
        for scale in LADDER {
            let Ok(target) = TargetSpec::for_page(&list, scale, PIXEL_BUDGET) else {
                print!("  {scale:.0}x        no target");
                continue;
            };
            let area = f64::from(scale) * f64::from(scale);
            let mut measure = |list: &DisplayList| {
                let oracle = CpuRasterizer::new().rasterize(list, target).expect("draws");
                (ink(&oracle) / area, backend.rasterize(list, target))
            };
            let (clipped_cpu, clipped_ours) = measure(&list);
            let (bare_cpu, bare_ours) = measure(&bare);
            print!("  {scale:.0}x cpu {clipped_cpu:10.2} of {bare_cpu:10.2}");
            match (clipped_ours, bare_ours) {
                (Ok(clipped), Ok(bare)) => print!(
                    "  raster {:10.2} of {:10.2}",
                    ink(&clipped) / area,
                    ink(&bare) / area
                ),
                (Err(why), _) | (_, Err(why)) => print!("  raster refused: {why}"),
            }
        }
        println!();
    }
}
