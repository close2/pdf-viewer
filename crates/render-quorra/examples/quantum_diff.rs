//! The measurement behind `tests/real_pages.rs`'s gates: the glyph-phase
//! quantum's cost against the CPU oracle on a real text page, per §4.5's
//! "measured, never assumed". Run with `--release`; edit the page index and scale
//! to reproduce the numbers quoted in the gate comments.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).unwrap();
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(4).expect("exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).unwrap();
    let cpu = CpuRasterizer::new().rasterize(&list, target).unwrap();

    for quantum in [Some(16_u16), Some(64), None] {
        let mut backend = QuorraRasterizer::with_options(&quorra_gpu::Options {
            glyph_quantum: quantum,
            ..quorra_gpu::Options::default()
        })
        .expect("adapter");
        let ours = backend.rasterize(&list, target).unwrap();
        let c = raster_compare::compare(&cpu, &ours).unwrap();
        println!(
            "quantum {quantum:?}: mean {:.4} worst {:.2} differing {:.4} ssim {:.5}",
            c.mean_error, c.worst_tile_error, c.differing_fraction, c.structural_similarity
        );
    }
}
