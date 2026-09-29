//! §8.4.3.4: **a join is shaped by the directions the two segments run at the point they
//! meet**, and where one of them is a curve that direction is its tangent, not its last
//! chord's (ADR 1397).
//!
//! > Join styles shall be significant only at points where consecutive segments of a path
//! > connect at an angle
//!
//! Table 54 builds each join from the two segments' own strokes there — the miter extends
//! "[t]he outer edges of the strokes for the two segments", and for the bevel "[t]he two
//! segments shall be finished with butt caps (see 8.4.3.3, "Line cap style") and the
//! resulting notch beyond the ends of the segments shall be filled with a triangle" — so the
//! fixture below states the join's region in closed form from the curve's end tangent alone
//! and compares it with every pixel, at 1×, 2×, 4× and 8×, with the path drawn both ways.
//!
//! The second half holds the stroker's word that a stroke's pieces tile its set
//! ([`Stroked::tiles`](crate::raster::stroke::Stroked)): where it vouches, the fill keeps its
//! integral without asking, and that must be the set's area to the byte.

use raster_scene::{LineCap, LineJoin, Point, Segment};

use crate::raster::flatten::FLATTEN_TOLERANCE;
use crate::raster::{Rule, fill_mask, fill_mask_settled, flatten_stroke, stroke_pieces};

use super::stroke_set::{RUNGS, line_path, mask, reversed, scaled, stroke};

/// A quarter of the circle of radius 30 about `(20, 50)`, from `(20, 20)` to `(50, 50)`,
/// arriving there straight down its tangent `(0, 1)` — `c2 → to` is vertical to the bit —
/// and a line leaving along `(1, 0)` to `(80, 50)`: a right angle between a curve and a line.
fn arc_then_line() -> Vec<Segment> {
    let k = 0.552_284_8 * 30.0;
    vec![
        Segment::MoveTo(Point::new(20.0, 20.0)),
        Segment::CubicTo {
            c1: Point::new(20.0 + k, 20.0),
            c2: Point::new(50.0, 50.0 - k),
            to: Point::new(50.0, 50.0),
        },
        Segment::LineTo(Point::new(80.0, 50.0)),
    ]
}

/// The area of the convex polygon `polygon` inside the unit pixel at `(x, y)`, by clipping it
/// to the pixel's four sides (Sutherland–Hodgman) and taking the shoelace area of what is
/// left, in `f64`.
fn area_in_pixel(polygon: &[(f64, f64)], column: f64, row: f64) -> f64 {
    let mut out = polygon.to_vec();
    // Each side as `(axis, bound, keep_below)`.
    for (axis, bound, below) in [
        (0, column, false),
        (0, column + 1.0, true),
        (1, row, false),
        (1, row + 1.0, true),
    ] {
        let input = std::mem::take(&mut out);
        let at = |point: (f64, f64)| if axis == 0 { point.0 } else { point.1 };
        let inside = |point: (f64, f64)| {
            if below {
                at(point) <= bound
            } else {
                at(point) >= bound
            }
        };
        for i in 0..input.len() {
            let (from, to) = (input[i], input[(i + 1) % input.len()]);
            if inside(from) {
                out.push(from);
            }
            if inside(from) != inside(to) {
                let t = (bound - at(from)) / (at(to) - at(from));
                out.push((from.0 + t * (to.0 - from.0), from.1 + t * (to.1 - from.1)));
            }
        }
    }
    (0..out.len())
        .map(|i| {
            let (from, to) = (out[i], out[(i + 1) % out.len()]);
            from.0 * to.1 - to.0 * from.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// **A miter and a bevel where an arc meets a line at a right angle**, at `8 w`: in the
/// quadrant beyond both segments' ends (`x < 50`, `y > 50`), which neither segment's stroke
/// reaches because each is squared off at `(50, 50)` — the arc's by its tangent `(0, 1)`, the
/// line's by its own direction — the set is exactly the join. §8.4.3.5's ratio at 90° is
/// `1 / sin 45°`, under the limit of 10, so the miter stands: the square from `(46, 50)` to
/// `(50, 54)`. The bevel is the triangle `(50, 50)`, `(46, 50)`, `(50, 54)`. Every pixel of the
/// quadrant holds its area of that polygon to within a byte's rounding, at every rung, drawn
/// either way. Squared to the arc's last chord instead (ADR 1397 measured the plant),
/// the miter's worst pixel there is 85, 94, 194 and 200 steps from it at 1×, 2×, 4× and 8×.
#[test]
fn a_join_where_an_arc_meets_a_line_is_square_to_its_tangent() {
    let square = [(46.0, 50.0), (46.0, 54.0), (50.0, 54.0), (50.0, 50.0)];
    let triangle = [(50.0, 50.0), (46.0, 50.0), (50.0, 54.0)];
    for (join, region) in [
        (LineJoin::Miter, &square[..]),
        (LineJoin::Bevel, &triangle[..]),
    ] {
        for (back, path) in [(false, arc_then_line()), (true, reversed(&arc_then_line()))] {
            let mut ladder = Vec::new();
            for s in RUNGS {
                let m = mask(&path, stroke(8.0, LineCap::Butt, join, 10.0), s);
                let polygon: Vec<(f64, f64)> = region
                    .iter()
                    .map(|&(x, y)| (x * f64::from(s), y * f64::from(s)))
                    .collect();
                let (half, full) = (m.width / 2, m.height);
                let mut worst = 0.0_f64;
                for y in half..full {
                    for x in 0..half {
                        let want = 255.0 * area_in_pixel(&polygon, f64::from(x), f64::from(y));
                        let got = f64::from(m.coverage[(y * m.width + x) as usize]);
                        worst = worst.max((got - want).abs());
                    }
                }
                ladder.push(worst);
            }
            assert!(
                ladder.iter().all(|worst| *worst <= 1.0),
                "{join:?} (reversed: {back}): the pixels beyond both ends are {ladder:.2?} steps \
                 from the join's closed form at 1×, 2×, 4× and 8×"
            );
        }
    }
}

/// **The round join at the same corner** is Table 54's pie slice, "[a]n arc of a circle with
/// a diameter equal to the line width … drawn around the point where the two segments meet,
/// connecting the outer edges of the strokes for the two segments": in the quadrant, the
/// quarter disc of radius 4 about `(50, 50)`, `4π`. Its arc is flattened, so the quadrant's
/// ink may fall short of it by the strip §10.7.2 lets a chord cut off (arc length times
/// `FLATTEN_TOLERANCE`, in device pixels) and exceed it by nothing but rounding; and no
/// pixel of the quadrant wholly outside the disc holds any ink.
#[test]
fn a_round_join_where_an_arc_meets_a_line_is_its_quarter_disc() {
    let quarter = std::f32::consts::PI * 16.0 / 4.0;
    let arc = std::f32::consts::FRAC_PI_2 * 4.0;
    for (back, path) in [(false, arc_then_line()), (true, reversed(&arc_then_line()))] {
        for s in RUNGS {
            let m = mask(&path, stroke(8.0, LineCap::Butt, LineJoin::Round, 10.0), s);
            let (half, full) = (m.width / 2, m.height);
            let (mut ink, mut partial) = (0.0_f32, 0_u32);
            for y in half..full {
                for x in 0..half {
                    let byte = m.coverage[(y * m.width + x) as usize];
                    ink += f32::from(byte) / 255.0;
                    partial += u32::from(byte != 0 && byte != 255);
                    // The pixel's nearest point to the join's centre, in page units.
                    #[expect(clippy::cast_precision_loss)] // pixel indices below 800
                    let near = |a: u32, c: f32| (a as f32).max(c.min((a + 1) as f32));
                    let (cx, cy) = (50.0 * s, 50.0 * s);
                    let d = (near(x, cx) - cx).hypot(near(y, cy) - cy) / s;
                    assert!(
                        d < 4.0 || byte == 0,
                        "round join at {s}× (reversed: {back}): ink {byte} at ({x}, {y}), \
                         {d:.3} from the corner"
                    );
                }
            }
            let ink = ink / (s * s);
            #[expect(clippy::cast_precision_loss)] // a pixel count
            let rounding = partial as f32 / 510.0 / (s * s) + 1.0 / 16.0;
            let short = arc * FLATTEN_TOLERANCE / s;
            assert!(
                ink >= quarter - short - rounding && ink <= quarter + rounding,
                "round join at {s}× (reversed: {back}): {ink:.4} in the quadrant against the \
                 quarter disc's {quarter:.4}"
            );
        }
    }
}

/// **Where the stroker says its pieces tile the set, the fill's integral is the set's area**:
/// drawn with the question skipped, as [`fill_mask_settled`] does for such a stroke, every
/// shape here reads within a byte of the same pieces drawn with it asked — one straight
/// segment under each cap, a rectangle under each join, a circle of four arcs — at 1×–8×.
#[test]
fn a_stroke_whose_pieces_tile_draws_the_same_without_the_question() {
    let k = 0.552_284_8 * 20.0;
    let circle = vec![
        Segment::MoveTo(Point::new(70.0, 50.0)),
        Segment::CubicTo {
            c1: Point::new(70.0, 50.0 + k),
            c2: Point::new(50.0 + k, 70.0),
            to: Point::new(50.0, 70.0),
        },
        Segment::CubicTo {
            c1: Point::new(50.0 - k, 70.0),
            c2: Point::new(30.0, 50.0 + k),
            to: Point::new(30.0, 50.0),
        },
        Segment::CubicTo {
            c1: Point::new(30.0, 50.0 - k),
            c2: Point::new(50.0 - k, 30.0),
            to: Point::new(50.0, 30.0),
        },
        Segment::CubicTo {
            c1: Point::new(50.0 + k, 30.0),
            c2: Point::new(70.0, 50.0 - k),
            to: Point::new(70.0, 50.0),
        },
        Segment::Close,
    ];
    let segment = line_path(&[(20.3, 30.6), (70.1, 61.2)], false);
    let rectangle = line_path(
        &[(20.3, 20.6), (80.3, 20.6), (80.3, 60.6), (20.3, 60.6)],
        true,
    );
    let cases = [
        (&segment, stroke(6.0, LineCap::Round, LineJoin::Miter, 10.0)),
        (
            &segment,
            stroke(6.0, LineCap::Square, LineJoin::Miter, 10.0),
        ),
        (&segment, stroke(6.0, LineCap::Butt, LineJoin::Miter, 10.0)),
        (
            &rectangle,
            stroke(6.0, LineCap::Butt, LineJoin::Miter, 10.0),
        ),
        (
            &rectangle,
            stroke(6.0, LineCap::Butt, LineJoin::Round, 10.0),
        ),
        (
            &rectangle,
            stroke(6.0, LineCap::Butt, LineJoin::Bevel, 10.0),
        ),
        (&circle, stroke(3.0, LineCap::Butt, LineJoin::Miter, 10.0)),
    ];
    for (path, drawn) in cases {
        for back in [false, true] {
            let path = if back { reversed(path) } else { path.clone() };
            for s in RUNGS {
                let stroked =
                    stroke_pieces(&flatten_stroke(&path, scaled(s)), drawn, drawn.width * s);
                assert!(
                    stroked.tiles,
                    "{drawn:?} at {s}× (reversed: {back}) is not vouched for"
                );
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 800
                let size = (100.0 * s) as u32;
                let asked = fill_mask(&stroked.pieces, Rule::NonZero, 0, 0, size, size);
                let kept =
                    fill_mask_settled(&stroked.pieces, Rule::NonZero, (0, 0, size, size), true);
                let worst = asked
                    .coverage
                    .iter()
                    .zip(&kept.coverage)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap_or(0);
                assert!(
                    worst <= 1,
                    "{drawn:?} at {s}× (reversed: {back}): the integral differs from the set by \
                     {worst} at a pixel"
                );
            }
        }
    }
}

/// **The stroker vouches for nothing it has not built to tile**: two segments meeting at a
/// corner (a join and two bodies whose inner sides are cut, but whose far ends the stroker
/// has not compared), two subpaths, a concave outline, and a rectangle narrower than its own
/// stroke, whose corners are too tight to cut.
#[test]
fn a_stroke_that_may_overlap_itself_is_not_vouched_for() {
    let two_segments = line_path(&[(20.0, 20.0), (60.0, 20.0), (60.0, 60.0)], false);
    let mut two_subpaths = line_path(&[(20.0, 20.0), (60.0, 20.0)], false);
    two_subpaths.extend(line_path(&[(20.0, 40.0), (60.0, 40.0)], false));
    let concave = line_path(
        &[
            (20.0, 20.0),
            (80.0, 20.0),
            (50.0, 40.0),
            (80.0, 60.0),
            (20.0, 60.0),
        ],
        true,
    );
    let narrow = line_path(
        &[(20.0, 20.0), (80.0, 20.0), (80.0, 21.0), (20.0, 21.0)],
        true,
    );
    for (what, path) in [
        ("two segments", two_segments),
        ("two subpaths", two_subpaths),
        ("concave", concave),
        ("narrow", narrow),
    ] {
        let drawn = stroke(4.0, LineCap::Butt, LineJoin::Miter, 10.0);
        let stroked = stroke_pieces(&flatten_stroke(&path, scaled(1.0)), drawn, 4.0);
        assert!(!stroked.tiles, "{what}: vouched for");
    }
}
