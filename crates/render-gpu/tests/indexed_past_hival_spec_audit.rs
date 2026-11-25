//! ISO 32000-2 §8.6.6.3's out-of-range index drawn by the vello backend: the documents
//! `tools/conformance/test-support/indexed_past_hival.rs` writes, one per route an `Indexed`
//! value reaches, and the colour the clause gives each patch — the cross-backend comparison of
//! `pdf-model`'s CPU test of the same documents.

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/indexed_past_hival.rs"]
mod indexed_past_hival;

use pdf_render::{Rasterizer as _, TargetSpec};
use render_gpu::GpuRasterizer;

#[test]
fn every_route_s_index_is_adjusted_to_the_nearest_entry() {
    let _gpu = gpu_lock::Hold::unless_software("vello-device");
    let mut backend = GpuRasterizer::new_headless().unwrap_or_else(|error| {
        panic!("the headless GPU adapter is required for this backend assertion: {error}")
    });
    let name = format!("render-gpu on {}", backend.context().adapter_description());
    for fixture in indexed_past_hival::every() {
        let list = indexed_past_hival::interpreted(&fixture);
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("an 80 by 20 target");
        let raster = backend
            .rasterize(&list, target)
            .unwrap_or_else(|error| panic!("{}: refused on {name}: {error}", fixture.route));
        fixture.assert_drawn(&raster, &name);
    }
}
