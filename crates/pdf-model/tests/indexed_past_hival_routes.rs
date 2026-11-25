//! ISO 32000-2 §8.6.6.3's out-of-range index on every route an `Indexed` value reaches, drawn on
//! the CPU backend: one test per route, over the documents
//! `tools/conformance/test-support/indexed_past_hival.rs` writes and whose module documentation
//! quotes the clause. `render-raster` and `render-gpu` draw the same documents on the device.
//!
//! `tests/indexed_out_of_range.rs` reads the same sentence on a corpus document's `sc` operators;
//! these documents are written by the test so that no route waits on a checkout.

#![expect(
    clippy::expect_used,
    reason = "test code: a page of fixed size that is no valid target, or that the CPU backend \
              refuses, is the failure this test exists to report"
)]

use pdf_render::{Rasterizer as _, TargetSpec};
use render_cpu::CpuRasterizer;

#[path = "../../../tools/conformance/test-support/indexed_past_hival.rs"]
mod indexed_past_hival;

use indexed_past_hival::Fixture;

/// Draws `fixture` on the CPU backend and asserts each patch is the entry the clause selects.
fn assert_drawn(fixture: &Fixture) {
    let list = indexed_past_hival::interpreted(fixture);
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("an 80 by 20 page is a target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("nothing on the page is refused by the CPU backend");
    fixture.assert_drawn(&raster, "render-cpu");
}

#[test]
fn an_image_sample_past_hival_is_adjusted_to_hival_at_every_depth() {
    for bits in [8, 4, 2] {
        assert_drawn(&indexed_past_hival::image_at_depth(bits));
    }
}

#[test]
fn a_sixteen_bit_image_sample_past_hival_is_adjusted_to_hival() {
    assert_drawn(&indexed_past_hival::sixteen_bit_image());
}

#[test]
fn a_decoded_index_below_zero_is_adjusted_to_entry_zero() {
    assert_drawn(&indexed_past_hival::decoded_below_zero());
}

#[test]
fn an_image_with_a_matte_adjusts_its_index_before_restoring_the_entry() {
    assert_drawn(&indexed_past_hival::matted_image());
}

#[test]
fn an_inline_image_sample_past_hival_is_adjusted_to_hival() {
    assert_drawn(&indexed_past_hival::inline_image());
}

#[test]
fn a_colour_operator_index_out_of_range_is_adjusted_to_the_nearest_end() {
    assert_drawn(&indexed_past_hival::colour_operators());
}

#[test]
fn an_uncoloured_pattern_over_an_indexed_base_adjusts_its_index() {
    assert_drawn(&indexed_past_hival::uncoloured_pattern());
}

#[test]
fn a_mesh_shading_vertex_index_past_hival_is_adjusted_to_hival() {
    assert_drawn(&indexed_past_hival::mesh_shading());
}

#[test]
fn an_axial_shading_function_output_past_hival_is_adjusted_to_hival() {
    assert_drawn(&indexed_past_hival::axial_shading());
}

#[test]
fn an_image_inside_a_luminosity_soft_mask_adjusts_its_index() {
    assert_drawn(&indexed_past_hival::luminosity_soft_mask());
}
