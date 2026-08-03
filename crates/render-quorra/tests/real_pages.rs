//! Real specification pages through the quorra backend, against the CPU oracle.
//!
//! The cross-backend scenes are small on purpose, and trap 12b names the limit of
//! that: a suite of small scenes tests small scenes. These are the pages the brief
//! was measured on — page 6 of ISO 32000-2 is the 5 933-command witness — rendered
//! by both backends from the same display list and compared with the same
//! machinery as everything else.
//!
//! Two gates, separating two questions:
//!
//! - **Fidelity**, with the glyph-phase quantum off: what the adapter and quorra
//!   add on top of the unavoidable rasteriser-vs-rasteriser antialiasing floor.
//!   Measured answer: nothing — quorra sits at the Vello backend's exact distance
//!   from the oracle on every case (see the constants).
//! - **The quantum's cost envelope**, at the default 1/16: the caller's own `RENDER_LIBRARY.md` section 4.5
//!   decision (ADR 0131, 5× glyph-cache reuse) moves text by at most 1/32 px,
//!   which trades edge pixels without touching structure. Its measured cost is
//!   pinned so a quantisation regression shows as a failure, not a drift.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: an explanatory panic is the intended failure mode, and the \
              expects live in helpers the allow-expect-in-tests config cannot see"
)]

use std::path::Path;

use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

const GENEROUS: u64 = 1 << 30;

/// Fidelity gates, quantum off. Measured on this machine (RADV): quorra's worst
/// case across the eight page/scale combinations is mean 1.18 / worst tile 5.4 /
/// ssim 0.9944 (pages 5 and 6 @1.0) — the Vello backend measures mean 1.16 /
/// worst 3.2 / ssim 0.9955 on the same cases, so the two rasterisers' glyph
/// antialiasing is essentially the whole floor. A missing paragraph is worst
/// tile > 100.
const MAX_MEAN_ERROR: f64 = 1.5;
const MAX_WORST_TILE_ERROR: f64 = 7.0;
const MIN_STRUCTURAL_SIMILARITY: f64 = 0.99;

/// The quantum's measured envelope at the default 1/16 (worst observed: mean 1.85,
/// worst tile 20.2, ssim 0.9670 — page 4 @1.0 and page 6 @1.9008), with margin.
const QUANTISED_MAX_MEAN: f64 = 2.5;
const QUANTISED_MAX_WORST_TILE: f64 = 30.0;
const QUANTISED_MIN_SSIM: f64 = 0.95;

/// The specification's own PDFs, which every checkout has (the same cases as
/// render-gpu's real-page suite; page 6 of ISO 32000-2 is the dense witness).
const CASES: [(&str, usize); 4] = [
    ("ISO_32000-2_sponsored_EC3.pdf", 5),
    ("ISO_32000-2_sponsored_EC3.pdf", 6),
    ("ISO_32000-2_sponsored_EC3.pdf", 4),
    ("PDF20_AN001-BPC.pdf", 0),
];

fn interpret(name: &str, index: usize) -> DisplayList {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("{} is committed: {e}", path.display()));
    let document = Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(index).expect("the page exists");
    pdf_model::content::interpret(&document, &page).display_list
}

#[test]
fn real_pages_agree_with_the_cpu_oracle() {
    // Exact glyph phases: this gate isolates translation and rasterisation
    // fidelity from the quantum's deliberate, separately-gated trade.
    let mut quorra = QuorraRasterizer::with_options(&quorra_gpu::Options {
        glyph_quantum: None,
        ..quorra_gpu::Options::default()
    })
    .unwrap_or_else(|e| panic!("no adapter available for quorra: {e}"));
    println!("adapter: {}", quorra.adapter_description());

    for (name, index) in CASES {
        let list = interpret(name, index);
        // 1.0 is what the reference comparison uses; 1.9008 is the window scale
        // that once produced a silently blank page on the Vello backend.
        for scale in [1.0_f32, 1.9008] {
            let what = format!("{name} p{index} @{scale}");
            let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("within budget");
            let cpu = CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("the oracle draws its own corpus");
            let ours = quorra
                .rasterize(&list, target)
                .unwrap_or_else(|e| panic!("{what}: quorra refused: {e}"));
            let c = raster_compare::compare(&cpu, &ours).expect("same dimensions");
            println!(
                "{what}: mean {:.4} worst tile {:.2} at {:?} differing {:.4} ssim {:.5}",
                c.mean_error,
                c.worst_tile_error,
                c.worst_tile_at,
                c.differing_fraction,
                c.structural_similarity,
            );
            assert!(
                c.mean_error < MAX_MEAN_ERROR,
                "{what}: mean error {:.4} exceeds {MAX_MEAN_ERROR}",
                c.mean_error
            );
            assert!(
                c.worst_tile_error < MAX_WORST_TILE_ERROR,
                "{what}: worst tile {:.4} at {:?} exceeds {MAX_WORST_TILE_ERROR}",
                c.worst_tile_error,
                c.worst_tile_at
            );
            assert!(
                c.structural_similarity > MIN_STRUCTURAL_SIMILARITY,
                "{what}: structural similarity {:.5} below {MIN_STRUCTURAL_SIMILARITY}",
                c.structural_similarity
            );
        }
    }
}

/// The default 1/16 quantum's cost stays inside its measured envelope: sub-1/32-px
/// text movement trades edge pixels (worst tiles rise), never structure (ssim
/// stays high, and a missing mark would crater it).
#[test]
fn the_glyph_quantum_cost_stays_bounded() {
    let mut quorra = QuorraRasterizer::new_headless()
        .unwrap_or_else(|e| panic!("no adapter available for quorra: {e}"));
    for (name, index) in CASES {
        let list = interpret(name, index);
        for scale in [1.0_f32, 1.9008] {
            let what = format!("{name} p{index} @{scale} (quantised)");
            let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("within budget");
            let cpu = CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("the oracle draws its own corpus");
            let ours = quorra
                .rasterize(&list, target)
                .unwrap_or_else(|e| panic!("{what}: quorra refused: {e}"));
            let c = raster_compare::compare(&cpu, &ours).expect("same dimensions");
            assert!(
                c.mean_error < QUANTISED_MAX_MEAN
                    && c.worst_tile_error < QUANTISED_MAX_WORST_TILE
                    && c.structural_similarity > QUANTISED_MIN_SSIM,
                "{what}: mean {:.4} worst {:.2} ssim {:.5} left the quantum's envelope",
                c.mean_error,
                c.worst_tile_error,
                c.structural_similarity
            );
        }
    }
}
