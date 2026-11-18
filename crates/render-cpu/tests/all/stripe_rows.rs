//! An axial shading's stripe boundaries land on the device rows the file puts them on.
//!
//! # What the expected rows come from
//!
//! ISO 32000-2 §8.7.4.5.3 places the colour of a point by its projection onto the axis alone:
//! Table 79's `/Domain` says "[t]he variable is considered to vary linearly between these two
//! values as the colour gradient varies between the starting and ending points of the axis",
//! and past a non-extended end "t is undefined and the point shall be left unpainted". So the
//! row a colour boundary falls on is arithmetic on the file's numbers — `/Coords`, `/Domain`,
//! the stitching function's `/Bounds` and the pattern matrix — and [`boundary_row`] does that
//! arithmetic in `f64`, never reading a pixel of any renderer.
//!
//! # The shape
//!
//! `issue10572.pdf`'s page reduced to one column: an axis 1 800 units long, `/Domain [-6 6]`,
//! twenty-four hard stripes (a boundary every half unit of `t`), a pattern matrix that flips the
//! axis, and a filled rectangle that shows `t` from 0 to 3. Every boundary falls on a whole
//! device row at every integer scale, so a boundary drawn one row off is a failure rather than a
//! rounding question — and the displacement a moved stop produces grows with the axis and the
//! scale, which is why the cases run at 1×, 2×, 4× and 8×. A ramp compressed by a twentieth of a
//! percent at each non-extended end (the construction ADR 1387 replaced) put these boundaries up
//! to three rows off at 8× with `/Extend [false false]` and five with `[true false]`, and none at
//! 1× for the first — which is how it went unseen.
//!
//! §10.7.3's smoothness tolerance does not reach this: it bounds the colour error of a
//! piecewise-linear approximation to the function, and a stripe drawn at a displaced row is a
//! whole-range error in the geometry.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the \
              arithmetic is on literal page coordinates and small integer scales"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "every cast is a device row of a small test raster or a literal stripe index"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Ramp,
    Raster, Rasterizer, Shading, ShadingKind, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The page, in PDF units: one narrow column as tall as the stripes need.
const PAGE: (f32, f32) = (20.0, 470.0);

/// `/Coords`' two ends on the axis, in pattern space: `issue10572.pdf`'s own.
const AXIS: (f64, f64) = (-900.0, 900.0);

/// `/Domain`.
const DOMAIN: (f64, f64) = (-6.0, 6.0);

/// The pattern matrix's vertical translation; the matrix is `[1 0 0 -1 2 PATTERN_TY]`.
const PATTERN_TY: f64 = 460.0;

/// The filled rectangle, in page space: x from 2 to 18, y from 10 to 460.
const FILLED: (f32, f32, f32, f32) = (2.0, 10.0, 18.0, 460.0);

/// The scales every case runs at.
const SCALES: [u32; 4] = [1, 2, 4, 8];

/// Green up to each half unit of `t`, blue after it: the stitching function's twelve copies of
/// a type 3 whose inner `/Bounds [0.5 0.5]` make a step, as the file states them. The domain's
/// top belongs to the last stripe, because §7.10.4 closes a stitching function's last subdomain.
fn stripe_colour(t: f64) -> Color {
    let half_units = (((t - DOMAIN.0) * 2.0).floor() as i64).min(23);
    if half_units % 2 == 0 {
        Color::rgb(0.0, 1.0, 0.0)
    } else {
        Color::rgb(0.0, 0.0, 1.0)
    }
}

/// The ramp the interpreter builds: the function over the shading's parameter mapped onto
/// `0..=1`, with a break at each of the twenty-three boundaries.
fn stripes() -> Ramp {
    let span = DOMAIN.1 - DOMAIN.0;
    let breaks: Vec<f32> = (1..24)
        .map(|j| ((f64::from(j) * 0.5) / span) as f32)
        .collect();
    Ramp::sample_across(&breaks, |at| stripe_colour(DOMAIN.0 + f64::from(at) * span))
}

/// The column scene, with the shading stated along `axis` under `extend`.
fn scene(axis: (f64, f64), extend: (bool, bool)) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE.0, PAGE.1));
    let (left, bottom, right, top) = FILLED;
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(left, bottom)));
    path.push(PathCommand::LineTo(Point::new(right, bottom)));
    path.push(PathCommand::LineTo(Point::new(right, top)));
    path.push(PathCommand::LineTo(Point::new(left, top)));
    path.push(PathCommand::Close);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(Shading {
            background: None,
            kind: Arc::new(ShadingKind::Axial {
                start: Point::new(0.0, axis.0 as f32),
                end: Point::new(0.0, axis.1 as f32),
                ramp: stripes(),
                extend,
            }),
            transform: Transform::new(1.0, 0.0, 0.0, -1.0, 2.0, PATTERN_TY as f32),
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The device row a point of the axis at parameter `t` falls on, in exact arithmetic.
///
/// §8.7.4.5.3's linear map from `t` to the axis, then the pattern matrix into page space, then
/// the device's flip and scale. The fixture is built so that the answer is a whole number.
fn boundary_row(axis: (f64, f64), t: f64, scale: u32) -> u32 {
    let along = (t - DOMAIN.0) / (DOMAIN.1 - DOMAIN.0);
    let pattern_y = axis.0 + along * (axis.1 - axis.0);
    let page_y = PATTERN_TY - pattern_y;
    let row = (f64::from(PAGE.1) - page_y) * f64::from(scale);
    assert!(
        (row - row.round()).abs() < 1e-9,
        "the fixture must put t = {t} on a whole row, got {row}"
    );
    row.round() as u32
}

/// What a pixel of the column is: green stripe, blue stripe, or the unpainted page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seen {
    Green,
    Blue,
    Unpainted,
}

/// Classifies the pixel of `row` in the column's middle.
fn seen(raster: &Raster, row: u32, scale: u32) -> Seen {
    let x = (10 * scale) as usize;
    let index = ((row as usize) * (raster.width as usize) + x) * 4;
    let pixel = raster
        .data
        .get(index..index + 4)
        .expect("the row is inside the raster");
    let (red, green, blue) = (pixel[0], pixel[1], pixel[2]);
    if red == 0 && green == 255 && blue == 0 {
        Seen::Green
    } else if red == 0 && green == 0 && blue == 255 {
        Seen::Blue
    } else if red == green && green == blue {
        Seen::Unpainted
    } else {
        panic!("row {row} at {scale}×: ({red}, {green}, {blue}) is neither stripe nor the page")
    }
}

/// The stripe `t` lies inside, as [`seen`] reports it.
fn stripe_seen(t: f64) -> Seen {
    if stripe_colour(t).g > 0.5 {
        Seen::Green
    } else {
        Seen::Blue
    }
}

fn render(list: &DisplayList, scale: u32) -> Raster {
    let target = TargetSpec::for_page(list, scale as f32, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("an axial shading is supported")
}

/// Every stripe boundary the rectangle shows is drawn between the two rows the file puts it
/// between, under all four `/Extend` pairs and at every scale.
#[test]
fn every_stripe_boundary_lands_on_its_row_at_every_scale() {
    for extend in [(false, false), (true, true), (false, true), (true, false)] {
        let list = scene(AXIS, extend);
        for scale in SCALES {
            let raster = render(&list, scale);
            // The rectangle shows t from 0 to 3; the boundaries strictly inside it are at
            // 0.5, 1.0, …, 2.5.
            for j in 1..6 {
                let t = f64::from(j) * 0.5;
                let row = boundary_row(AXIS, t, scale);
                assert_eq!(
                    (seen(&raster, row - 1, scale), seen(&raster, row, scale)),
                    (stripe_seen(t - 0.25), stripe_seen(t + 0.25)),
                    "/Extend {extend:?} at {scale}×: the boundary at t = {t} belongs between \
                     rows {} and {row}",
                    row - 1
                );
            }
        }
    }
}

/// A non-extended end is cut on its own row: §8.7.4.5.3 leaves the point unpainted past it.
///
/// The axis is shortened to pattern y 150..300, which the matrix puts at page y 310..160 — inside
/// the rectangle, so both cuts are visible, and each on a whole row.
#[test]
fn a_non_extended_end_is_cut_on_its_row() {
    let axis = (150.0, 300.0);
    let list = scene(axis, (false, false));
    for scale in SCALES {
        let raster = render(&list, scale);
        // Device rows grow downwards and the pattern matrix flips the axis, so the start is the
        // upper cut and the end the lower one; a row belongs to the side its centre is on.
        let upper = boundary_row(axis, DOMAIN.0, scale);
        let lower = boundary_row(axis, DOMAIN.1, scale);
        assert_eq!(
            seen(&raster, upper - 1, scale),
            Seen::Unpainted,
            "{scale}×: the row above the start is past the axis"
        );
        assert_ne!(
            seen(&raster, upper, scale),
            Seen::Unpainted,
            "{scale}×: the start's own row is painted"
        );
        assert_ne!(
            seen(&raster, lower - 1, scale),
            Seen::Unpainted,
            "{scale}×: the row above the end is painted"
        );
        assert_eq!(
            seen(&raster, lower, scale),
            Seen::Unpainted,
            "{scale}×: the end's own row is past the axis"
        );
    }
}
