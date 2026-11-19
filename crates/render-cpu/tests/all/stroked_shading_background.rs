//! ISO 32000-2 §8.7.4.3 Table 77's `/Background` on a **stroke**, on the correctness oracle.
//!
//! > If present, this colour shall be used, before any painting operation involving the
//! > shading, to fill those portions of the area to be painted that lie outside the bounds of
//! > the shading object.
//!
//! §8.7.4.1 lets a shading pattern paint "with painting operators such as f (fill), S (stroke)",
//! so a stroke's area to be painted is its own outline, and the wash fills the part of it that
//! lies outside the shading (ADR 3094). `test_scenes::shading_background_stroked` strokes the four
//! quadrants of `test_scenes::shading_background` through their middles.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and every index \
              is a probe the scene places inside a 200-pixel raster, far from overflow"
)]

use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use test_scenes::{BACKGROUND_STROKES, WASH_RGB8, shading_background, shading_background_stroked};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The RGBA of device pixel `(column, row)` of `list` drawn at scale 1 on a transparent page.
fn drawn(list: &DisplayList) -> impl Fn(u32, u32) -> [u8; 4] + use<> {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    // No medium, so that a pixel's alpha says whether anything was painted there.
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(list, target)
        .expect("the scene is drawn");
    move |column, row| {
        let at = ((row as usize) * (raster.width as usize) + column as usize) * 4;
        [
            raster.data[at],
            raster.data[at + 1],
            raster.data[at + 2],
            raster.data[at + 3],
        ]
    }
}

/// `test_scenes::WASH` as the opaque eight-bit pixel it paints.
fn wash() -> [u8; 4] {
    let [r, g, b] = WASH_RGB8;
    [r, g, b, 255]
}

/// The wash paints the stroke's ends, where each quadrant's shading does not reach, and nothing
/// above or below the stroke: the area to be painted is the stroke's, not the square's.
#[test]
fn a_stroke_s_background_fills_its_outline_outside_the_shading() {
    let stroked = drawn(&shading_background_stroked());
    // Columns outside each quadrant's shading and inside its stroke: left of the axial band and
    // of the mesh, outside the radial's outer disc, left of the sampled domain.
    let washed = [(15, 0), (110, 0), (15, 1), (110, 1)];
    for (column, quadrant) in washed {
        let row = BACKGROUND_STROKES[quadrant];
        assert_eq!(stroked(column, row), wash(), "({column}, {row})");
        // Fifteen units above the stroke's middle: inside the square, outside the stroke.
        assert_eq!(stroked(column, row - 15)[3], 0, "({column}, {})", row - 15);
    }
}

/// Inside the stroke, every pixel is the fill's: the wash and the shading are one raster, built
/// once by `pdf_render::ShadingRaster`, and the stroke only takes its own region of it.
#[test]
fn a_washed_stroke_paints_the_fills_pixels_inside_it() {
    let (stroked, filled) = (
        drawn(&shading_background_stroked()),
        drawn(&shading_background()),
    );
    for middle in BACKGROUND_STROKES {
        // Five rows either side of the middle lie inside the twenty-unit stroke, and the columns
        // stay two units clear of the quadrant's edges, where the stroke's butt end is.
        for row in middle - 5..middle + 5 {
            for column in (13..92).chain(108..187) {
                assert_eq!(
                    stroked(column, row),
                    filled(column, row),
                    "({column}, {row})"
                );
            }
        }
    }
}
