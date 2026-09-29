//! A curve stroked wider than it bends is the set of points within half the width of it —
//! ISO 32000-2 §8.4.3.2.
//!
//! # What the expected values come from
//!
//! §8.4.3.2 defines the stroke as a set: "stroking a path shall entail painting all points whose
//! perpendicular distance from the path in user space is less than or equal to half the line
//! width". So the ink a stroke deposits on a white page at one device pixel per unit is the area
//! of that set, and the set is a closed form or a quadrature of the path alone:
//!
//! - **A circle of radius 5 at `30 w`** is the disk of radius 20, `400π` = 1256.64; the four
//!   cubics that state the circle lie within 0.03% of it, and the set they state measures
//!   1256.72.
//! - **An ellipse of 6 by 3 at `14 w`**, and **an open arch at `16 w` with round caps**, have no
//!   closed form. Their sets were measured by counting the points of a grid a sixty-fourth of a
//!   unit apart whose distance from four thousand points along each cubic is at most the
//!   half-width: 413.99 and 565.50. The counting is `numpy` over the path's own control points,
//!   and no renderer enters it.
//!
//! The tolerance is the construction's own, stated in ADR 1348: its chords and arcs lie within
//! 1/64 and 1/256 of a pixel of the set's boundary, and the
//! library's supersampled converter it is measured by states a boundary pixel's coverage in
//! sixteenths — under a unit of ink over the disk's rim of a hundred and sixty boundary pixels,
//! where the ring the stroker drew for it was three hundred and fourteen short.
//!
//! Before ADR 1348 the circle read a ring: `tiny-skia`'s stroker offsets a closed curve's inside
//! as a second contour, reversed, which turns inside out once the half-width passes the radius,
//! and its winding cancelled the outer contour's over a disk of radius 10.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, and \
              the arithmetic is over a hundred-unit page and a raster of known size"
)]

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use std::sync::Arc;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units, drawn at one device pixel per unit.
const PAGE: f32 = 100.0;

/// The ink the flattening and the converter's sixteenths may move, in pixels.
const TOLERANCE: f64 = 1.0;

/// The constant that makes four cubics a circle, as the fixtures state it.
const KAPPA: f32 = 0.5523;

/// An ellipse about `(cx, cy)` as four cubics, closed.
fn ellipse(path: &mut Path, (cx, cy): (f32, f32), (rx, ry): (f32, f32)) {
    let (kx, ky) = (KAPPA * rx, KAPPA * ry);
    path.push(PathCommand::MoveTo(Point::new(cx + rx, cy)));
    path.push(PathCommand::CurveTo(
        Point::new(cx + rx, cy + ky),
        Point::new(cx + kx, cy + ry),
        Point::new(cx, cy + ry),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(cx - kx, cy + ry),
        Point::new(cx - rx, cy + ky),
        Point::new(cx - rx, cy),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(cx - rx, cy - ky),
        Point::new(cx - kx, cy - ry),
        Point::new(cx, cy - ry),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(cx + kx, cy - ry),
        Point::new(cx + rx, cy - ky),
        Point::new(cx + rx, cy),
    ));
    path.push(PathCommand::Close);
}

/// `path` stroked black at `width` with `cap` and `join`, alone on the page.
fn stroked(path: Path, width: f32, (cap, join): (LineCap, LineJoin)) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            cap,
            join,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The raster of `list` at one device pixel per unit.
fn drawn(list: &DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a stroke is supported")
}

/// Total darkness on the page, in fully black pixels.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| u64::from(255 - pixel[0]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "bounded by four bytes per pixel of a ten-thousand-pixel raster"
    )]
    let sum = sum as f64;
    sum / 255.0
}

/// The red channel at device pixel `(x, y)`.
fn red(raster: &Raster, (x, y): (usize, usize)) -> u8 {
    raster.data[(y * raster.width as usize + x) * 4]
}

/// A radius-5 circle at `30 w` is the disk of radius 20, its centre as black as its rim.
#[test]
fn a_circle_stroked_wider_than_its_diameter_is_a_disk() {
    let mut path = Path::new();
    ellipse(&mut path, (50.0, 50.0), (5.0, 5.0));
    let raster = drawn(&stroked(path, 30.0, (LineCap::Butt, LineJoin::Miter)));
    assert_eq!(red(&raster, (50, 50)), 0, "the centre is inside the set");
    let ink = ink(&raster);
    assert!(
        (ink - 1256.72).abs() < TOLERANCE,
        "the disk of radius 20 is 1256.72 units; drawn {ink:.2}"
    );
}

/// A 6 by 3 ellipse at `14 w` fills the lens its inside offset used to leave empty.
#[test]
fn an_ellipse_stroked_wider_than_it_bends_is_its_whole_distance_set() {
    let mut path = Path::new();
    ellipse(&mut path, (50.0, 50.0), (6.0, 3.0));
    let raster = drawn(&stroked(path, 14.0, (LineCap::Butt, LineJoin::Miter)));
    assert_eq!(red(&raster, (50, 50)), 0, "the centre is 3 from the curve");
    let ink = ink(&raster);
    assert!(
        (ink - 413.99).abs() < TOLERANCE,
        "the distance set measures 413.99 units; drawn {ink:.2}"
    );
}

/// An open arch at `16 w`, round caps and joins: the union of the disks along it.
#[test]
fn an_open_arch_stroked_wider_than_it_bends_is_its_whole_distance_set() {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(20.0, 20.0)));
    path.push(PathCommand::CurveTo(
        Point::new(20.0, 32.0),
        Point::new(32.0, 32.0),
        Point::new(32.0, 20.0),
    ));
    let raster = drawn(&stroked(path, 16.0, (LineCap::Round, LineJoin::Round)));
    let ink = ink(&raster);
    assert!(
        (ink - 565.50).abs() < TOLERANCE,
        "the distance set measures 565.50 units; drawn {ink:.2}"
    );
}

/// A circle stroked far narrower than it bends is left to the stroker's own curves: the ring
/// between radii 29.5 and 30.5, `2π · 30 · 1` = 188.50.
#[test]
fn a_circle_stroked_narrower_than_it_bends_is_its_ring() {
    let mut path = Path::new();
    ellipse(&mut path, (50.0, 50.0), (30.0, 30.0));
    let raster = drawn(&stroked(path, 1.0, (LineCap::Butt, LineJoin::Miter)));
    assert_eq!(
        red(&raster, (50, 50)),
        255,
        "the centre is 30 from the curve"
    );
    let ink = ink(&raster);
    assert!(
        (ink - 188.50).abs() < TOLERANCE,
        "the ring measures 188.50 units; drawn {ink:.2}"
    );
}

/// A curve that folds between straight stretches that do not, at `16 w` with round caps and
/// joins: the straight stretches are the stroker's and the curve is pieces (ADR 1359), and their
/// union is still the one distance set — 2024.62, counted by `numpy` on a grid a sixty-fourth of a
/// unit apart against the lines and two thousand chords of the cubic. Its boundary is some two
/// hundred and eighty pixels long, near twice the disk's rim, so the converter's sixteenths are
/// allowed twice the disk's unit; the stroke stated wholly as pieces reads 2025.86 here.
#[test]
fn a_folding_curve_between_straight_stretches_is_one_distance_set() {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 60.0)));
    path.push(PathCommand::LineTo(Point::new(40.0, 60.0)));
    path.push(PathCommand::CurveTo(
        Point::new(40.0, 72.0),
        Point::new(52.0, 72.0),
        Point::new(52.0, 60.0),
    ));
    path.push(PathCommand::LineTo(Point::new(52.0, 30.0)));
    path.push(PathCommand::LineTo(Point::new(85.0, 30.0)));
    let raster = drawn(&stroked(path, 16.0, (LineCap::Round, LineJoin::Round)));
    let ink = ink(&raster);
    assert!(
        (ink - 2024.62).abs() < 2.0 * TOLERANCE,
        "the distance set measures 2024.62 units; drawn {ink:.2}"
    );
}

/// A subpath that does not fold beside one that does, wound either way: a circle of radius 15 at
/// `16 w` is the stroker's ring from 7 to 23, a circle of radius 5 inside it folds into the disk of
/// radius 13, and their union is the disk of radius 23, `529π` = 1661.90 — whichever way the ring
/// runs, since a ring wound against the pieces would cancel them where the two overlap. The rim is
/// a hundred and forty-five pixels, the disk's; the stroker's own offset curves, cut at its default
/// resolution, overshot this ring by four units, which is why the stretches are stroked at sixteen
/// times the device's resolution (ADR 1359).
#[test]
fn a_ring_the_stroker_draws_adds_to_the_pieces_whichever_way_it_runs() {
    for reversed in [false, true] {
        let mut ring = Path::new();
        ellipse(&mut ring, (50.0, 50.0), (15.0, 15.0));
        let mut path = if reversed { ring.reversed() } else { ring };
        ellipse(&mut path, (50.0, 50.0), (5.0, 5.0));
        let raster = drawn(&stroked(path, 16.0, (LineCap::Butt, LineJoin::Miter)));
        let ink = ink(&raster);
        assert!(
            (ink - 1661.90).abs() < 2.0 * TOLERANCE,
            "reversed {reversed}: the disk of radius 23 is 1661.90 units; drawn {ink:.2}"
        );
    }
}
