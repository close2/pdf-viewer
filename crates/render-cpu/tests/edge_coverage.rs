//! What this backend paints at the edge of a shape thicker than one device pixel.
//!
//! # What the expected values come from
//!
//! ISO 32000-2 §10.7.4 defines a device pixel as a product of two half-open intervals — "the set
//! of points ( x′ , y′ ) such that i ≤ x′ &lt; i + 1 and j ≤ y′ &lt; j + 1" — and gives a filled
//! shape the same half-open form. An axis-aligned rectangle is a product of two intervals too, so
//! the area it covers of a pixel is the **product of its two one-dimensional overlaps**, exactly,
//! at every placement. `pdf_render::rectangle_coverage` is that arithmetic and this file compares
//! the raster against it: no renderer enters the expected value, and the tolerance below is the
//! raster's own depth rather than anybody's agreement.
//!
//! Under this tree's anti-aliasing departure — §10.7.1's NOTE, `doc/todo/_scan-conversion.md`'s
//! departure (1) — that area *is* the coverage the pixel is painted at. The direction of what is
//! left is the clause's: "[t]he area covered by painted pixels shall always be at least as large
//! as the area of the original shape."
//!
//! # Why the whole raster rather than a sample
//!
//! Because the defect this guards was a *quantum* and not a placement. `tiny-skia`'s path scan
//! converter supersamples four times per axis and an axis-aligned edge looks the same to all four
//! sub-rows, so before ADR 0476 every edge here answered 0, 0.25, 0.50, 0.75 or 1.00 — right at
//! four placements in twenty and wrong at the rest, with anything under an eighth of a pixel
//! painting nothing at all. A test that sampled the boundary pixel at one offset could pass on
//! the rounding. Comparing every pixel of a small page cannot.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything here.
const GENEROUS: u64 = 1 << 30;

/// The page every scene is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 24.0,
    height: 16.0,
};

/// How far a pixel's coverage may differ from the area the clause gives it.
///
/// Three levels of 255. Two of them are arithmetic — `tiny-skia` carries each axis's overlap in
/// 8.8 fixed point, so each of the two factors is within `1/256` before they are multiplied, and
/// the composition rounds once more — and the third is slack. The quantum this guards against is
/// **32** levels, so the tolerance separates the two answers by an order of magnitude.
const TOLERANCE: f32 = 2.0 / 255.0;

/// A closed axis-aligned rectangle.
fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// One black rectangle on a white page, and nothing else.
fn scene(path: Path) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The page-to-device transform at `scale`: device y counts down from the top of the page.
fn to_device(scale: f32) -> Transform {
    Transform::new(scale, 0.0, 0.0, -scale, 0.0, PAGE.height * scale)
}

/// The coverage a pixel carries, on a black-on-white page where darkness is coverage.
fn coverage(raster: &Raster, x: u32, y: u32) -> f32 {
    let at = ((y * raster.width + x) as usize) * 4;
    f32::from(255 - raster.data[at]) / 255.0
}

/// Renders one rectangle and checks **every** pixel against the area the clause gives it.
fn assert_every_pixel_is_the_clauses_area(what: &str, path: &Path, scale: f32) {
    let list = scene(path.clone());
    let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a solid fill is supported");

    let device = pdf_render::device_rectangle(path, to_device(scale))
        .expect("one axis-aligned rectangle under an axis-preserving transform");
    let mut worst = 0.0f32;
    let mut worst_at = (0u32, 0u32);
    for y in 0..raster.height {
        for x in 0..raster.width {
            let wanted = pdf_render::rectangle_coverage(device, x as f32, y as f32);
            let difference = (coverage(&raster, x, y) - wanted).abs();
            if difference > worst {
                worst = difference;
                worst_at = (x, y);
            }
        }
    }
    assert!(
        worst <= TOLERANCE,
        "{what} at scale {scale}: pixel {worst_at:?} differs from §10.7.4's area by {worst}, \
         over a tolerance of {TOLERANCE}"
    );
}

/// An edge at every twentieth of a pixel, on both axes at once.
///
/// The rungs are the ones `render-quorra/examples/edge_coverage_ladder` prints. Four of the
/// twenty-one are exact multiples of a quarter and would pass under the quantum too, which is why
/// the sweep is over all of them rather than over a chosen offset.
#[test]
fn an_edge_is_painted_at_the_area_it_covers_at_every_placement() {
    for rung in 0_i16..=20 {
        let fraction = f32::from(rung) / 20.0;
        assert_every_pixel_is_the_clauses_area(
            &format!("an edge {fraction} of a pixel across"),
            &rectangle(3.0, 3.0, 17.0 + fraction, 11.0 + fraction),
            1.0,
        );
    }
}

/// A corner, where the coverage is the product of two overlaps rather than either alone.
///
/// Device y counts down, so this rectangle spans device x `[3.75, 17.75]` and y `[4.125, 12.5]`,
/// and its four corner pixels carry `0.25 · 0.875`, `0.75 · 0.875`, `0.25 · 0.5` and `0.75 · 0.5`.
/// None is a multiple of a quarter and the smallest is an eighth, so under the old quantum three
/// of the four were wrong and the raster could not tell a corner from the edge it sits on.
#[test]
fn a_corner_is_the_product_of_its_two_overlaps() {
    let path = rectangle(3.75, 3.5, 17.75, 11.875);
    let device = pdf_render::device_rectangle(&path, to_device(1.0)).expect("a rectangle");
    // The clause's own arithmetic, before any raster is asked: the corner is the product.
    let corner = pdf_render::rectangle_coverage(device, 3.0, 4.0);
    assert!(
        (corner - 0.25 * 0.875).abs() < 1e-6,
        "the closed form's own corner: {corner}"
    );
    assert_every_pixel_is_the_clauses_area("a rectangle with four fractional corners", &path, 1.0);
}

/// The same rectangle at three scales, because a scale moves where every boundary falls.
///
/// A single scale can be satisfied by an offset that happens to land well; three cannot, and the
/// two that are not 1.0 also put the shape's own coordinates and the device grid out of step.
#[test]
fn the_area_is_the_clauses_at_every_scale() {
    let path = rectangle(2.3, 2.7, 18.9, 12.1);
    for scale in [1.0, 2.0, 0.5] {
        assert_every_pixel_is_the_clauses_area("a rectangle at three scales", &path, scale);
    }
}

/// An edge under an eighth of a pixel, which is the case the departure could not explain.
///
/// This is §10.7.4's third sentence on its own: "[t]he area covered by painted pixels shall always
/// be at least as large as the area of the original shape". At 0.05 of a pixel the supersampled
/// path converter answered **zero** — the painted area smaller than the shape's, which is a defect
/// rather than the anti-aliasing departure — so this asserts the ink is there at all before it
/// asserts how much.
#[test]
fn an_edge_under_the_old_quantum_is_painted_rather_than_dropped() {
    let path = rectangle(3.0, 3.0, 17.05, 11.0);
    let list = scene(path.clone());
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a solid fill is supported");
    // Device y = PAGE.height - 11 = 5 is inside the rectangle; column 17 is its right edge.
    let painted = coverage(&raster, 17, 5);
    assert!(
        painted > 0.0,
        "an edge covering a twentieth of its pixel painted nothing"
    );
    assert!(
        (painted - 0.05).abs() <= TOLERANCE,
        "an edge covering 0.05 of its pixel painted {painted}"
    );
}
