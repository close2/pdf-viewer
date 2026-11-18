//! ISO 32000-2 §8.7.4.3 Table 77's `/AntiAlias`, on the correctness oracle.
//!
//! > ( Optional ) A flag indicating whether to filter the shading function to prevent aliasing
//! > artifacts.
//!
//! The filter is ADR 3024's: each device pixel is the mean of the shading at a 4 × 4 lattice of
//! sub-pixel centres. `test_scenes::shading_anti_alias` puts a black-to-white step of three
//! kinds through the centre of one pixel each, where sampling once a pixel takes one side and the
//! filter takes half of each — so ignoring the flag fails the first test, and filtering a shading
//! that did not ask fails the second. `test_scenes::shading_anti_alias_stroked` paints the same
//! shadings with strokes, which §8.7.4.1 lets a shading pattern paint as it paints fills, and the
//! filter is the same there (ADR 3074).

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and every index \
              is a probe the scene places inside a 200-pixel raster, far from overflow"
)]

use pdf_render::DisplayList;
use pdf_render::{Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use test_scenes::{ANTI_ALIAS_EDGES, shading_anti_alias, shading_anti_alias_stroked};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The red channel of device pixel `(column, row)` of the fill scene drawn with `/AntiAlias` as
/// given.
fn drawn(anti_alias: bool) -> impl Fn((u32, u32)) -> u8 {
    drawn_list(&shading_anti_alias(anti_alias))
}

/// The red channel of device pixel `(column, row)` of `list` drawn at scale 1.
fn drawn_list(list: &DisplayList) -> impl Fn((u32, u32)) -> u8 + use<> {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
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

/// The same filter where the shadings paint strokes: the step's pixel inside each stroke is grey and
/// its neighbours are not, as in the fill (ADR 3074).
#[test]
fn a_stroke_whose_shading_asks_for_anti_aliasing_is_filtered_at_its_step() {
    let red = drawn_list(&shading_anti_alias_stroked(true));
    for [edge, black, white] in ANTI_ALIAS_EDGES {
        let value = red(edge);
        assert!(
            (96..=160).contains(&value),
            "{edge:?} straddles the step inside the stroke and is filtered: {value}"
        );
        assert!(red(black) <= 2, "{black:?}: {}", red(black));
        assert!(red(white) >= 253, "{white:?}: {}", red(white));
    }
}

/// The stroke draws the fill's very raster: inside the stroke, the stroked scene's pixels are the
/// filled scene's, so a stroke whose region or placement of the raster differed shows here.
#[test]
fn a_filtered_stroke_paints_the_fills_pixels_inside_it() {
    let (stroked, filled) = (drawn_list(&shading_anti_alias_stroked(true)), drawn(true));
    // Rows 95 to 104 lie inside every stroke (page y 90.5 to 110.5); the columns span the three
    // shadings, each well inside its own stroke's length.
    for row in 95..105 {
        for column in (15..65).chain(100..160).chain(170..190) {
            assert_eq!(
                stroked((column, row)),
                filled((column, row)),
                "({column}, {row})"
            );
        }
    }
}
