//! ISO 32000-2 §8.7.4.3 Table 77's `/AntiAlias`, on the correctness oracle.
//!
//! > ( Optional ) A flag indicating whether to filter the shading function to prevent aliasing
//! > artifacts.
//!
//! The filter is ADR 3024's: each device pixel is the mean of the shading at a 4 × 4 lattice of
//! sub-pixel centres. `test_scenes::shading_anti_alias` puts a black-to-white step of three
//! kinds through the centre of one pixel each, where sampling once a pixel takes one side and the
//! filter takes half of each — so ignoring the flag fails the first test, and filtering a shading
//! that did not ask fails the second.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and every index \
              is a probe the scene places inside a 200-pixel raster, far from overflow"
)]

use pdf_render::{Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use test_scenes::{ANTI_ALIAS_EDGES, shading_anti_alias};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The red channel of device pixel `(column, row)` of the scene drawn with `/AntiAlias` as given.
fn drawn(anti_alias: bool) -> impl Fn((u32, u32)) -> u8 {
    let list = shading_anti_alias(anti_alias);
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the scene is drawn");
    move |(column, row)| {
        raster.data[((row as usize) * (raster.width as usize) + column as usize) * 4]
    }
}

/// The step's own pixel is grey under the filter: eight of its sixteen samples on each side, so
/// half-way between black and white, give or take the curvature of the radial's circle.
#[test]
fn a_shading_that_asks_for_anti_aliasing_is_filtered_at_its_step() {
    let red = drawn(true);
    for [edge, black, white] in ANTI_ALIAS_EDGES {
        let value = red(edge);
        assert!(
            (96..=160).contains(&value),
            "{edge:?} straddles the step and is the mean of both sides under the filter: {value}"
        );
        assert!(
            red(black) <= 2,
            "{black:?} is a pixel short of the step: {}",
            red(black)
        );
        assert!(
            red(white) >= 253,
            "{white:?} is a pixel past the step: {}",
            red(white)
        );
    }
}

/// Without the flag the function is sampled once a pixel, at its centre, as before — Table 77's
/// "Default value: false" leaves every other shading as it was.
#[test]
fn a_shading_that_does_not_ask_keeps_its_hard_step() {
    let red = drawn(false);
    for [edge, black, white] in ANTI_ALIAS_EDGES {
        let value = red(edge);
        assert!(
            value <= 2 || value >= 253,
            "{edge:?} is sampled at its centre and takes one side: {value}"
        );
        assert!(red(black) <= 2, "{black:?}: {}", red(black));
        assert!(red(white) >= 253, "{white:?}: {}", red(white));
    }
}
