//! Where a shading's colours land on the page.
//!
//! Separate from `headless_render.rs` because this asks a different question. That file
//! asks whether geometry reaches the right pixels; this asks whether the *paint* does,
//! and the two fail independently: a gradient drawn in the wrong place still fills
//! exactly the right shape, in colours drawn from exactly the right ramp, and looks
//! entirely plausible.
//!
//! # What the expected values come from
//!
//! ISO 32000-2 §8.7.4.5.3 defines an axial shading by a parametric variable `t` that
//! varies linearly along the axis from `Domain[0]` at the start point to `Domain[1]` at
//! the end, with the colour at a point being the colour function evaluated at that
//! point's `t`. The projection is perpendicular to the axis, so every point on a line
//! across the axis carries the same colour.
//!
//! [`pdf_render::Ramp`] is that colour function already evaluated over `0..=1`, so for a
//! ramp running red to blue along a 100-unit axis, page y = 25 is `t = 0.25` and its
//! colour is `(191, 0, 64)` — three quarters red, one quarter blue. That value is
//! derived from the clause, not from what any renderer produces.
//!
//! # Why every case runs at two scales
//!
//! The defect these tests were written for made the device transform apply twice to the
//! paint. At a scale of exactly 1.0 the page-to-device transform is its own inverse — it
//! is a y-flip about the page's centre — so the second application cancels the geometry
//! and leaves only a mirror, and at every other scale it leaves a displacement that grows
//! with the scale. A single scale can be made to pass by construction. Two cannot.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the \
              arithmetic is on literal page dimensions that cannot overflow"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "every cast here is a clamped colour channel or a page coordinate under \
              200, so both are in range by construction"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Ramp,
    Raster, Rasterizer, Shading, ShadingKind, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The page these scenes are drawn on, in PDF units.
const PAGE: f32 = 100.0;

/// How far a channel may differ from the value the clause gives.
///
/// The ramp carries [`Ramp::RESOLUTION`] samples and the rasteriser interpolates between
/// them in eight-bit precision, so a sample lands within a step or two of the exact
/// value. Wide enough for that, far narrower than any misplacement: a mirror moves this
/// gradient by 127 levels.
const TOLERANCE: i32 = 3;

/// A red-to-blue ramp: `t = 0` is pure red and `t = 1` is pure blue.
fn ramp() -> Ramp {
    Ramp::sample(|t| Color::rgb(1.0 - t, 0.0, t))
}

/// The colour the clause gives for parameter `t` on that ramp.
fn expected(t: f32) -> (u8, u8, u8) {
    let scaled = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    (scaled(1.0 - t), 0, scaled(t))
}

/// A page filled by one axial shading running up the page, red at the bottom.
///
/// `path_space` is the transform the filled rectangle is stated under, and the rectangle
/// is stated in that space so that it still covers the page. The shading itself is
/// always positioned in page space, which is what PDF specifies for a pattern and what
/// `sh` produces directly.
fn vertical_gradient(path_space: Transform) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));

    let inverse = path_space.invert().expect("path space is invertible");
    let mut path = Path::new();
    for corner in [
        Point::new(0.0, 0.0),
        Point::new(PAGE, 0.0),
        Point::new(PAGE, PAGE),
        Point::new(0.0, PAGE),
    ] {
        let corner = inverse.apply(corner);
        if path.is_empty() {
            path.push(PathCommand::MoveTo(corner));
        } else {
            path.push(PathCommand::LineTo(corner));
        }
    }
    path.push(PathCommand::Close);

    list.push(Command::Fill {
        path: Arc::new(path),
        transform: path_space,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(Shading {
            background: None,
            kind: Arc::new(ShadingKind::Axial {
                start: Point::new(0.0, 0.0),
                end: Point::new(0.0, PAGE),
                ramp: ramp(),
                extend: (true, true),
            }),
            transform: Transform::IDENTITY,
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Reads the pixel at a *page* point, at the given scale.
fn at_page_point(raster: &Raster, page: Point, scale: f32) -> (u8, u8, u8) {
    // Device y counts down from the top, so page y = PAGE is device row 0.
    let x = (page.x * scale) as u32;
    let y = ((PAGE - page.y) * scale) as u32;
    assert!(x < raster.width && y < raster.height, "inside the raster");
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 3];
    (p[0], p[1], p[2])
}

fn assert_close(what: &str, actual: (u8, u8, u8), wanted: (u8, u8, u8)) {
    let far = |a: u8, b: u8| (i32::from(a) - i32::from(b)).abs() > TOLERANCE;
    assert!(
        !(far(actual.0, wanted.0) || far(actual.1, wanted.1) || far(actual.2, wanted.2)),
        "{what}: got {actual:?}, expected {wanted:?} within {TOLERANCE}"
    );
}

/// Renders `list` at `scale` and checks the two sample points against the clause.
///
/// The points sit either side of the page's horizontal centre line, which is what a
/// mirror about that line moves and nothing else does.
fn assert_gradient_lands(what: &str, list: &DisplayList, scale: f32) {
    let target = TargetSpec::for_page(list, scale, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("an axial shading is supported");

    for t in [0.25f32, 0.75] {
        assert_close(
            &format!("{what} at scale {scale}, t = {t}"),
            at_page_point(&raster, Point::new(PAGE / 2.0, t * PAGE), scale),
            expected(t),
        );
    }
}

/// The `sh` case: the filled rectangle is already in page space.
#[test]
fn an_axial_shading_lands_where_its_axis_says() {
    let list = vertical_gradient(Transform::IDENTITY);
    for scale in [1.0, 2.0, 0.5] {
        assert_gradient_lands("identity path space", &list, scale);
    }
}

/// The pattern case: the path is stated in its own space, and the shading is not.
///
/// A shading pattern is positioned by the pattern matrix relative to the page, not by
/// whatever transform the path being filled happens to carry (ISO 32000-2 §8.7.3.1). So
/// scaling the space the path is stated in must move the path's *coordinates* and leave
/// the colours exactly where they were.
#[test]
fn a_shading_is_anchored_to_the_page_and_not_to_the_path() {
    let list = vertical_gradient(Transform::scale(2.0, 4.0));
    for scale in [1.0, 2.0] {
        assert_gradient_lands("scaled path space", &list, scale);
    }
}

/// A rotated path space, which a scale alone cannot distinguish from a shear.
#[test]
fn a_shading_is_unmoved_by_a_rotated_path_space() {
    // A quarter turn about the page centre, so the rectangle still covers the page.
    let quarter_turn = Transform::new(0.0, 1.0, -1.0, 0.0, PAGE, 0.0);
    let list = vertical_gradient(quarter_turn);
    for scale in [1.0, 2.0] {
        assert_gradient_lands("rotated path space", &list, scale);
    }
}
