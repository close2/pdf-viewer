//! ISO 32000-2 §8.6.6.3's out-of-range index drawn on the device, under each of raster's three
//! coverages: the documents `tools/conformance/test-support/indexed_past_hival.rs` writes, one per
//! route an `Indexed` value reaches, and the colour the clause gives each patch.
//!
//! The adjustment is made where the display list is built (`pdf_colour`'s `entry_of`), so the
//! device never looks an index up itself; what this proves is that nothing between the list and
//! the target paints a patch differently from `pdf-model`'s CPU test of the same documents.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a coverage that is not available or a page the device refuses is the \
              failure this test exists to report"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;
#[path = "../../../tools/conformance/test-support/indexed_past_hival.rs"]
mod indexed_past_hival;

use pdf_render::{Rasterizer as _, TargetSpec};
use render_raster::QuorraRasterizer;

fn check(coverage: raster_gpu::Coverage) {
    gpu_lock_hook::install();
    let mut backend = QuorraRasterizer::with_options(&raster_gpu::Options {
        coverage,
        ..render_raster::options()
    })
    .expect("the requested coverage is available");
    let name = format!(
        "render-raster {coverage:?} on {}",
        backend.adapter_description()
    );
    for fixture in indexed_past_hival::every() {
        let list = indexed_past_hival::interpreted(&fixture);
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("an 80 by 20 target");
        let raster = backend
            .rasterize(&list, target)
            .unwrap_or_else(|error| panic!("{}: refused on {name}: {error}", fixture.route));
        fixture.assert_drawn(&raster, &name);
    }
}

#[test]
fn cpu_coverage_adjusts_every_route_s_index_to_the_nearest_entry() {
    check(raster_gpu::Coverage::Cpu);
}

#[test]
fn gpu_coverage_adjusts_every_route_s_index_to_the_nearest_entry() {
    check(raster_gpu::Coverage::Gpu);
}

#[test]
fn compute_coverage_adjusts_every_route_s_index_to_the_nearest_entry() {
    check(raster_gpu::Coverage::Compute);
}
