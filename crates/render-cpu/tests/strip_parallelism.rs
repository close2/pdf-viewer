//! A page drawn in strips is the page drawn in one piece, byte for byte.
//!
//! This backend is the oracle the GPU backend is judged against and the reference every
//! corpus and oracle verdict is taken from, so a render that depended on how the page was
//! divided would put the machine's core count into every comparison the project makes. ADR
//! 0138 is what happens when it does: strips cut wherever the cost was even moved four pages
//! out of agreement with the reference consensus, and the cause was one line of geometry —
//! **a curve chopped at a strip's edge is re-parameterised**, so the clipped curve is not the
//! `f32` control points the whole one was and its edge coverage differs by up to 64 of 255.
//!
//! ADR 0139 is the rule that follows: cut only at rows no curve crosses, which
//! [`pdf_render::unsplittable_rows`] names and [`pdf_render::strip_boundaries_avoiding`] respects.
//! This test is the guard on it. It cannot call the planner — the shipped path chooses its own
//! strip count from the machine — so it does the harder thing and asserts the property the
//! planner exists to deliver: over a suite of scenes and a range of divisions **the bytes are
//! equal**.
//!
//! # Why the scenes are these scenes
//!
//! ADR 0138's equality test failed on exactly the three scenes with a *curve* crossing a cut.
//! A suite of rectangles would have passed and let the defect reach the oracle four pages
//! later, so `curves` and `diagonal_stroke` are the two that must be here; the rest are here
//! because a group, a soft mask and a knockout each render through a different surface.

#![expect(
    clippy::expect_used,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly"
)]

use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Renders `list` with the strips the planner grants for `target`, forced to `strips`.
fn drawn(list: &DisplayList, target: TargetSpec, strips: u32) -> Vec<u8> {
    CpuRasterizer::new()
        .with_strips(strips)
        .rasterize(list, target)
        .expect("the scene is supported")
        .data
}

/// Every scene, at every division, drawn identically to the serial render.
#[test]
fn a_page_drawn_in_strips_is_the_page_drawn_whole() {
    let scenes: [(&str, DisplayList); 6] = [
        ("basic", test_scenes::basic()),
        ("curves", test_scenes::curves()),
        ("diagonal_stroke", test_scenes::diagonal_stroke()),
        ("transparency_group", test_scenes::transparency_group()),
        ("knockout_group", test_scenes::knockout_group()),
        ("soft_mask", test_scenes::soft_mask()),
    ];

    for (name, list) in scenes {
        let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
        let serial = drawn(&list, target, 1);
        for strips in [2_u32, 3, 5, 8, 16] {
            let split = drawn(&list, target, strips);
            let differing = serial
                .iter()
                .zip(&split)
                .filter(|(one, other)| one != other)
                .count();
            let worst = serial
                .iter()
                .zip(&split)
                .map(|(one, other)| one.abs_diff(*other))
                .max()
                .unwrap_or(0);
            assert_eq!(
                differing,
                0,
                "{name} at {strips} strips: {differing} of {} bytes differ, worst {worst}",
                serial.len()
            );
        }
    }
}

/// The planner's own promise, on the page that motivated it: asking for more strips than the
/// page's curves permit gives fewer strips, never a different picture.
///
/// `curves` is one Bézier fill over the whole page, so almost every row is forbidden and the
/// grant is small. That it renders identically at sixteen is the case ADR 0138 failed.
#[test]
fn a_page_of_curves_declines_the_cuts_it_cannot_make() {
    let list = test_scenes::curves();
    let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("a valid target");
    let curved = pdf_render::unsplittable_rows(&list, target);
    let extents = pdf_render::command_extents(&list, target);
    let costs = pdf_render::row_costs(&extents, target);
    let boundaries = pdf_render::strip_boundaries_avoiding(&costs, &curved, 16, 32);

    for cut in boundaries.iter().skip(1).take(boundaries.len() - 2) {
        assert!(
            !curved[*cut as usize],
            "cut at row {cut}, which a curve crosses"
        );
    }
    assert_eq!(drawn(&list, target, 16), drawn(&list, target, 1));
}
