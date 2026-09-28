//! A square cut into eight abutting rectangles by a region in its middle, which is what a
//! redaction leaves of a filled square — ISO 32000-2 §10.7.4 and §11.6.2.
//!
//! # What the expected values come from
//!
//! §10.7.4 gives a pixel the region `[i, i+1) × [j, j+1)`, and under this tree's anti-aliasing
//! departure (§10.7.1's NOTE) a pixel is covered by the area of the shape inside it. The eight
//! pieces are portions of one path whose interiors are disjoint and whose union is the square less
//! the region, so that area is `|square ∩ p| − |region ∩ p|`, each term the product of two
//! one-dimensional overlaps, computed here in `f64` from the page coordinates and the scale. No
//! renderer enters an expected value.
//!
//! The scale is the redaction tests' 150 dpi, `150/72`, at which `45.6` lands on device column
//! `95.0` exactly in the reals and a hair below it in `f32`: the closed form puts nothing in column
//! 94, and a coverage floor applied to that ten-thousandth of a pixel put a level in every row of
//! it (ADR 1374).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, and the \
              arithmetic is over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The redaction tests' page side, in points.
const PAGE: f32 = 200.0;

/// 150 dpi: §8.3.2.3's 72 user-space units to the inch.
const SCALE: f64 = 150.0 / 72.0;

/// The square, `[left, right] × [bottom, top]` in page space.
const SQUARE: [f32; 4] = [45.6, 29.0, 162.7, 129.0];

/// The region the square is cut around, widened as a redaction widens it.
const REGION: [f32; 4] = [62.69, 47.79, 150.71, 123.21];

/// Appends `[left, right] × [bottom, top]` as a closed subpath, anticlockwise in page space.
fn rectangle(path: &mut Path, [left, bottom, right, top]: [f32; 4]) {
    path.push(PathCommand::MoveTo(Point::new(left, bottom)));
    path.push(PathCommand::LineTo(Point::new(right, bottom)));
    path.push(PathCommand::LineTo(Point::new(right, top)));
    path.push(PathCommand::LineTo(Point::new(left, top)));
    path.push(PathCommand::Close);
}

/// The eight pieces the region leaves of the square, as one path.
fn eight_pieces() -> Path {
    let xs = [SQUARE[0], REGION[0], REGION[2], SQUARE[2]];
    let ys = [SQUARE[1], REGION[1], REGION[3], SQUARE[3]];
    let mut path = Path::new();
    for column in 0..3 {
        for row in 0..3 {
            if (column, row) != (1, 1) {
                rectangle(
                    &mut path,
                    [xs[column], ys[row], xs[column + 1], ys[row + 1]],
                );
            }
        }
    }
    path
}

/// The square whole, as `re` states it before any region cuts it.
fn the_square() -> Path {
    let mut path = Path::new();
    rectangle(&mut path, SQUARE);
    path
}

/// `path` filled black at 150 dpi.
fn drawn(path: Path) -> Raster {
    let mut list = DisplayList::new(Size {
        width: PAGE,
        height: PAGE,
    });
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, SCALE as f32, 1 << 30).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a fill is supported")
}

/// The area of `[left, right] × [bottom, top]` inside device pixel `(x, y)`, in `f64`.
fn area_in([left, bottom, right, top]: [f32; 4], x: u32, y: u32) -> f64 {
    let along = |low: f64, high: f64, at: f64| (high.min(at + 1.0) - low.max(at)).max(0.0);
    // The device's y runs down from the page's top edge.
    let device_y = |page: f32| (f64::from(PAGE) - f64::from(page)) * SCALE;
    along(
        f64::from(left) * SCALE,
        f64::from(right) * SCALE,
        f64::from(x),
    ) * along(device_y(top), device_y(bottom), f64::from(y))
}

/// The shape's own level at `(x, y)`: the square less the region, rounded to nearest.
fn closed_form(x: u32, y: u32) -> f64 {
    255.0 * (area_in(SQUARE, x, y) - area_in(REGION, x, y))
}

/// The level a black-on-white pixel carries.
fn level(raster: &Raster, x: u32, y: u32) -> f64 {
    f64::from(255 - raster.data[((y * raster.width + x) * 4) as usize])
}

/// The eight pieces are measured in closed form (ADR 0590), so every pixel reads the shape's own
/// level rounded to nearest — outside the square too, where there is nothing to read.
#[test]
fn eight_abutting_pieces_read_the_closed_form_at_every_pixel() {
    let raster = drawn(eight_pieces());
    let mut outside = 0;
    for y in 0..raster.height {
        for x in 0..raster.width {
            let (expected, got) = (closed_form(x, y), level(&raster, x, y));
            // A level within a hundredth of a rounding boundary may land either side of it in
            // `f32`; every other pixel has one answer.
            if (expected - expected.floor() - 0.5).abs() < 0.01 {
                assert!(
                    (got - expected).abs() <= 0.51,
                    "({x}, {y}): {got} against {expected}"
                );
                continue;
            }
            assert!(
                (got - expected.round()).abs() < f64::EPSILON,
                "({x}, {y}) reads {got} levels; the shape's area there is {expected:.4} levels"
            );
            if expected < 0.5 && got > 0.0 {
                outside += 1;
            }
        }
    }
    assert_eq!(outside, 0, "no ink where the shape is under half a level");
}

/// The square drawn whole is one rectangle, which goes to the library's rectangle converter
/// (ADR 0476): outside the region every pixel is within the one level ADR 0476 measured between
/// that converter's 8.8 fixed point and the closed form. That level is why the whole square and
/// its eight pieces can differ by one at a corner pixel the square covers by a sliver — the pieces
/// are the ones reading the clause's number there (ADR 1374).
#[test]
fn the_square_drawn_whole_is_within_one_level_of_the_closed_form() {
    let raster = drawn(the_square());
    for y in 0..raster.height {
        for x in 0..raster.width {
            let expected = 255.0 * area_in(SQUARE, x, y);
            let got = level(&raster, x, y);
            assert!(
                (got - expected).abs() <= 1.0,
                "({x}, {y}) reads {got} levels; the square's area there is {expected:.4} levels"
            );
        }
    }
}
