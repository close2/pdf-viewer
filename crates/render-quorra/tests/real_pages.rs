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
//!   Measured answer: nothing beyond that floor (see the constants).
//! - **The quantum's cost envelope**, at the default 1/16: the caller's own `RENDER_LIBRARY.md` section 4.5
//!   decision (ADR 0131, 5× glyph-cache reuse) moves text by at most 1/32 px,
//!   which trades edge pixels without touching structure. Its measured cost is
//!   pinned so a quantisation regression shows as a failure, not a drift.
//!
//! **The second gate was the only instrument in either tree that could see quorra's ADR
//! 0073**, and its constants were four to nine times what they were holding, so it did not.
//! Both sets are re-derived from the run in ADR 0498 and both are forced against the pin
//! before the fix; the constants' own doc comments carry the numbers and the forcing.
//! `tests/corpus.rs` now runs the 974-page instrument at the shipped quantum too, which is
//! the wider half of that answer — this file gates four pages and that one gates the corpus.

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

/// Fidelity gates, quantum off: the rasteriser-vs-rasteriser antialiasing floor and
/// nothing else. Worst across the eight page/scale combinations is **mean 0.4989 /
/// worst tile 3.15 / ssim 0.99870** (page 6 @1.0 for the first and third, page 0 of
/// `PDF20_AN001-BPC` @1.0 for the second). A missing paragraph is worst tile > 100.
///
/// **Both numbers below were measured on two adapters** — RADV on this machine and
/// llvmpipe, which is what CI has — and they agree to the fourth decimal, which is
/// why a bound this close to the measurement is not an adapter's luck (ADR 0498).
const MAX_MEAN_ERROR: f64 = 0.75;
const MAX_WORST_TILE_ERROR: f64 = 4.5;
const MIN_STRUCTURAL_SIMILARITY: f64 = 0.9975;

/// The quantum's measured envelope at the default 1/16: worst observed **mean 0.5194 /
/// worst tile 3.22 / ssim 0.99857**, on the same three cases as the floor above. The
/// whole of what the quantum costs on these pages is therefore **mean +0.02, worst tile
/// +0.07, ssim −0.00013** — which is what a sub-1/32-pixel trade should look like.
///
/// **These constants were 2.5 / 30.0 / 0.95 and are not any more, because the numbers
/// they were sized for were a defect rather than the trade.** Their doc comment recorded
/// a worst observed of mean 1.85 / worst tile 20.2 / ssim 0.9670; quorra's ADR 0073
/// found `GlyphPlacement::of` rounding a fractional phase up to the bucket count and then
/// taking `% q` of it, which seated 3.1% of phases per axis a whole device pixel from
/// where the placement asked. This gate was the only one in either tree that could see
/// that at all, and an envelope five times the size of what it was holding did not — it
/// is the whole of ADR 0498's argument for tightening rather than a tidy-up. **Forced
/// both ways**: at the previous pin (`cad50156`) every one of the eight cases breaks at
/// least two of these three — worst tile ran from 7.79 to 20.10 and SSIM down to 0.99155
/// — while the old envelope passed all eight, its widest constant holding 20.10 against
/// 30.0. At `97ad95ac` the worst case sits at two thirds of the mean bound.
const QUANTISED_MAX_MEAN: f64 = 0.80;
const QUANTISED_MAX_WORST_TILE: f64 = 4.8;
const QUANTISED_MIN_SSIM: f64 = 0.9970;

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
        ..render_quorra::options()
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
            // **Printed, because a bound is only re-derivable from what it is holding.** This
            // gate asserted three constants for its whole life and printed nothing, so the round
            // that found them oversized by an order of magnitude had to add a line to see it
            // (ADR 0498).
            println!(
                "{what}: mean {:.4} worst tile {:.2} at {:?} ssim {:.5}",
                c.mean_error, c.worst_tile_error, c.worst_tile_at, c.structural_similarity,
            );
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
