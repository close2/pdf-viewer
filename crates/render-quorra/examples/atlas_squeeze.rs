//! What a small glyph atlas does to a page of text, frame after frame.
//!
//! The owner's report is that text goes wrong at high zoom and **stays** wrong after zooming out
//! again, with some glyphs missing and at least one drawn as another (`extensive` came back as
//! `extens:ve`). That is a statement about state that survives a frame, and the state a glyph has
//! is quorra's **atlas** — whose budget is its own `Options::atlas_budget`, separate from the
//! resource budget `ResourceCaches` manages.
//!
//! At 6400% one glyph tile is hundreds of pixels on a side, so the atlas fills at a magnification
//! nothing else in this tree exercises. This squeezes the same effect into a page at its own
//! scale by making the budget small, so the question "does the atlas evict something the frame
//! still needs" can be asked on any adapter rather than only on the one that showed it.
//!
//! ```sh
//! cargo run --release -p render-quorra --example atlas_squeeze -- [file.pdf] [page]
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    missing_docs
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/PDF20_AN001-BPC.pdf")
            .to_string_lossy()
            .into_owned()
    });
    let page: usize = arguments
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(3);

    let document = Document::open(std::fs::read(&path).unwrap()).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(page.saturating_sub(1)).expect("the page exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 2.0, 1 << 30).unwrap();
    let cpu = CpuRasterizer::new().rasterize(&list, target).unwrap();

    let default = quorra_gpu::Options::default();
    println!("{path} — {} × {}", target.width, target.height);
    println!("default atlas budget: {} bytes", default.atlas_budget);
    println!(
        "{:>14}  {:>6}  {:>9} {:>9} {:>9}",
        "atlas budget", "frame", "mean", "worst", "ssim"
    );

    // **The lane comes first**, because the viewer switches to the GPU one above 10x
    // magnification (`pdf-viewer.rs`'s `GPU_COVERAGE_MAGNIFICATION`) and that is where the
    // owner's report begins.
    for (name, coverage) in [
        ("cpu lane", quorra_gpu::Coverage::Cpu),
        ("gpu lane", quorra_gpu::Coverage::Gpu),
    ] {
        let mut backend = QuorraRasterizer::with_options(&quorra_gpu::Options {
            coverage,
            ..render_quorra::options()
        })
        .expect("adapter");
        for frame in 1..=3 {
            match backend.rasterize(&list, target) {
                Ok(ours) => {
                    let c = raster_compare::compare(&cpu, &ours).unwrap();
                    println!(
                        "{name:>14}  {frame:>6}  {:>9.4} {:>9.2} {:>9.5}",
                        c.mean_error, c.worst_tile_error, c.structural_similarity
                    );
                }
                Err(error) => println!("{name:>14}  {frame:>6}  refused: {error}"),
            }
        }
    }

    for budget in [
        default.atlas_budget,
        1 << 20,
        1 << 18,
        1 << 16,
        1 << 14,
        1 << 12,
    ] {
        let mut backend = match QuorraRasterizer::with_options(&quorra_gpu::Options {
            atlas_budget: budget,
            coverage: quorra_gpu::Coverage::Gpu,
            ..render_quorra::options()
        }) {
            Ok(backend) => backend,
            Err(error) => {
                println!("{budget:>14}  refused at start-up: {error}");
                continue;
            }
        };
        // Three frames through one device: the first fills the atlas, and the ones after it are
        // where an entry evicted while still referenced would show.
        for frame in 1..=3 {
            match backend.rasterize(&list, target) {
                Ok(ours) => {
                    let c = raster_compare::compare(&cpu, &ours).unwrap();
                    println!(
                        "{budget:>14}  {frame:>6}  {:>9.4} {:>9.2} {:>9.5}",
                        c.mean_error, c.worst_tile_error, c.structural_similarity
                    );
                }
                Err(error) => println!("{budget:>14}  {frame:>6}  refused: {error}"),
            }
        }
    }
}
