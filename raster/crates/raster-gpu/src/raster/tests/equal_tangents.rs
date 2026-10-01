//! §8.4.3.4: **segments that meet with equal tangents meet at no angle**, so no join style
//! shapes the point between them (ADR 1468).
//!
//! > Join styles shall be significant only at points where consecutive segments of a path
//! > connect at an angle
//!
//! Where two curves meet along one tangent — the four cubics of a circle, an S-curve's
//! inflection — or a line leaves a curve along its tangent, the path does not turn; only the
//! chords of its flattening do, by the flattening's own angle, exactly as they do at every
//! point inside a curve. What a stroke paints there is §8.4.3.2's set:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from the
//! > path in user space is less than or equal to half the line width
//!
//! which round a point between two chords is the disc of the half-width about it, whatever
//! join the stroke names. A miter between the chords stands `hw · (sec(θ / 2) − 1)` past it and
//! a bevel leaves the disc's segment out; on a circle of radius 8 stroked 24 wide the three
//! drew 8 pixels apart, by up to 3 levels above the disc and 7 below. The expected value is
//! therefore that the three styles draw the same bytes, that the bytes' ink is the set's, and
//! that a corner the arithmetic can resolve, however small, keeps its style.

use raster_scene::{LineCap, LineJoin, Point, Segment, Stroke};

use super::IDENTITY;
use crate::raster::flatten::FLATTEN_TOLERANCE;
use crate::raster::{Rule, fill_mask_settled, flatten_stroke, stroke_pieces};

/// The region drawn.
const SIZE: (u32, u32) = (140, 140);

fn stroke(join: LineJoin, hw: f32) -> Stroke {
    Stroke {
        width: 2.0 * hw,
        adjust: false,
        cap: LineCap::Butt,
        join,
        miter_limit: 10.0,
    }
}

/// `path` stroked at half-width `hw` under `join`, as the encoder fills a stroke's pieces.
fn drawn(path: &[Segment], join: LineJoin, hw: f32) -> Vec<u8> {
    let stroked = stroke_pieces(&flatten_stroke(path, IDENTITY), stroke(join, hw), 2.0 * hw);
    fill_mask_settled(
        &stroked.pieces,
        Rule::NonZero,
        (0, 0, SIZE.0, SIZE.1),
        stroked.tiles,
    )
    .coverage
}

fn p(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// A circle as four cubics, each meeting the next along one tangent, off the pixel grid.
fn circle(r: f32) -> Vec<Segment> {
    let (cx, cy, k) = (60.3, 60.7, 0.552_284_8 * r);
    vec![
        Segment::MoveTo(p(cx + r, cy)),
        Segment::CubicTo {
            c1: p(cx + r, cy + k),
            c2: p(cx + k, cy + r),
            to: p(cx, cy + r),
        },
        Segment::CubicTo {
            c1: p(cx - k, cy + r),
            c2: p(cx - r, cy + k),
            to: p(cx - r, cy),
        },
        Segment::CubicTo {
            c1: p(cx - r, cy - k),
            c2: p(cx - k, cy - r),
            to: p(cx, cy - r),
        },
        Segment::CubicTo {
            c1: p(cx + k, cy - r),
            c2: p(cx + r, cy - k),
            to: p(cx + r, cy),
        },
        Segment::Close,
    ]
}

/// Two cubics meeting at an inflection along one tangent: `(60, 50) − (50, 10)` is
/// `(70, 90) − (60, 50)`.
fn s_curve() -> Vec<Segment> {
    vec![
        Segment::MoveTo(p(10.4, 60.4)),
        Segment::CubicTo {
            c1: p(30.4, 10.4),
            c2: p(50.4, 10.4),
            to: p(60.4, 50.4),
        },
        Segment::CubicTo {
            c1: p(70.4, 90.4),
            c2: p(90.4, 90.4),
            to: p(110.4, 40.4),
        },
    ]
}

/// A quarter arc of radius 12 that leaves along a line continuing its end tangent, and a
/// second arc entered from a line along its start tangent.
fn arc_into_line() -> Vec<Segment> {
    let k = 0.552_284_8 * 12.0;
    vec![
        Segment::MoveTo(p(40.3, 30.6)),
        Segment::CubicTo {
            c1: p(40.3, 30.6 + k),
            c2: p(28.3 + k, 42.6),
            to: p(28.3, 42.6),
        },
        Segment::LineTo(p(10.3, 42.6)),
        Segment::MoveTo(p(70.3, 100.6)),
        Segment::LineTo(p(70.3, 80.6)),
        Segment::CubicTo {
            c1: p(70.3, 80.6 - k),
            c2: p(82.3 - k, 68.6),
            to: p(82.3, 68.6),
        },
    ]
}

/// The flattened centre lines of `path`, as the stroker receives them.
fn chords(path: &[Segment]) -> Vec<(Vec<(f64, f64)>, bool)> {
    flatten_stroke(path, IDENTITY)
        .into_iter()
        .map(|line| {
            let points = line
                .points
                .iter()
                .map(|q| (f64::from(q.x), f64::from(q.y)))
                .collect();
            (points, line.closed)
        })
        .collect()
}

/// The distance from `(x, y)` to the nearest point of the chords.
fn distance(lines: &[(Vec<(f64, f64)>, bool)], x: f64, y: f64) -> f64 {
    let mut best = f64::INFINITY;
    for (points, closed) in lines {
        let n = points.len();
        let segments = if *closed { n } else { n - 1 };
        for i in 0..segments {
            let ((ax, ay), (bx, by)) = (points[i], points[(i + 1) % n]);
            let (dx, dy) = (bx - ax, by - ay);
            let length = dx * dx + dy * dy;
            let t = if length > 0.0 {
                (((x - ax) * dx + (y - ay) * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            best = best.min((x - ax - t * dx).hypot(y - ay - t * dy));
        }
    }
    best
}

/// The share of pixel `(column, row)` within `hw` of the chords, past their two open ends
/// cut square — the set a butt-capped stroke of these chords paints — on a 16 × 16 grid of
/// sample centres; `None` for a pixel no part of which is near a join.
fn near_set(lines: &[(Vec<(f64, f64)>, bool)], hw: f64, column: u32, row: u32) -> f64 {
    const N: u32 = 16;
    let inside = (0..N * N)
        .filter(|k| {
            let x = f64::from(column) + (f64::from(k % N) + 0.5) / f64::from(N);
            let y = f64::from(row) + (f64::from(k / N) + 0.5) / f64::from(N);
            distance(lines, x, y) <= hw
        })
        .count();
    #[expect(clippy::cast_precision_loss)] // at most 256
    let share = inside as f64 / f64::from(N * N);
    share
}

/// **The three join styles draw the same bytes where segments meet with equal tangents**, on
/// a circle of four cubics at three widths, an S-curve's inflection and a curve continued by a
/// line along its tangent — the clause makes the style insignificant there, so any byte that
/// differs is a style drawn where none applies. Before ADR 1468 the circle of radius 8 at
/// half-width 12 drew 8 pixels apart under miter and under bevel.
#[test]
fn equal_tangents_are_no_corner_and_every_join_style_draws_the_same() {
    let paths = [
        ("circle r30", circle(30.0)),
        ("circle r8", circle(8.0)),
        ("s-curve", s_curve()),
        ("arc into a line", arc_into_line()),
    ];
    for (name, path) in &paths {
        for hw in [0.5_f32, 3.0, 12.0, 25.0] {
            let round = drawn(path, LineJoin::Round, hw);
            for join in [LineJoin::Miter, LineJoin::Bevel] {
                let other = drawn(path, join, hw);
                let apart: Vec<(usize, u8, u8)> = round
                    .iter()
                    .zip(&other)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, (a, b))| (i, *a, *b))
                    .take(4)
                    .collect();
                assert!(
                    apart.is_empty(),
                    "{name} at half-width {hw}: {join:?} draws apart from round at {apart:?}"
                );
            }
        }
    }
}

/// **And the bytes are the set's**: the round join's ink round the circle is the chords'
/// distance set's to within a level of 255 per pixel of its rim — the same allowance
/// `inside_a_curve.rs` holds a hairpin to — and no pixel is off by more than the arcs'
/// flattening can move it.
#[test]
fn the_point_between_two_curves_is_the_sets_disc() {
    for (r, hw) in [(30.0_f32, 12.0_f32), (8.0, 12.0)] {
        let path = circle(r);
        let lines = chords(&path);
        let bytes = drawn(&path, LineJoin::Miter, hw);
        let (mut ink, mut set, mut rim, mut worst) = (0.0, 0.0, 0_u32, 0.0_f64);
        for row in 0..SIZE.1 {
            for column in 0..SIZE.0 {
                let share = 255.0 * near_set(&lines, f64::from(hw), column, row);
                let byte = f64::from(bytes[(row * SIZE.0 + column) as usize]);
                ink += byte;
                set += share;
                if share > 0.0 && share < 255.0 {
                    rim += 1;
                }
                worst = worst.max((byte - share).abs());
            }
        }
        assert!(
            (ink - set).abs() <= f64::from(rim),
            "circle r{r} hw {hw}: ink {ink} against the set's {set}, rim {rim}"
        );
        let bound = 255.0 * f64::from(FLATTEN_TOLERANCE) * 1.5;
        assert!(
            worst <= bound,
            "circle r{r} hw {hw}: a pixel {worst} from the set"
        );
    }
}

/// **A corner the arithmetic resolves keeps its style, however small**: two lines turning by
/// a tenth of a degree, stroked 400 wide, are a corner of the path, and a miter there stands
/// `hw · (sec(θ / 2) − 1)` past the disc — sub-pixel, but not equal — so the test asks the
/// join's own pieces rather than the bytes: the miter and the round join must be different
/// polygons.
#[test]
fn a_small_corner_between_two_lines_keeps_its_join() {
    let turn = 0.1_f32.to_radians();
    let path = vec![
        Segment::MoveTo(p(0.0, 70.0)),
        Segment::LineTo(p(70.0, 70.0)),
        Segment::LineTo(p(70.0 + 70.0 * turn.cos(), 70.0 + 70.0 * turn.sin())),
    ];
    let pieces = |join| {
        stroke_pieces(&flatten_stroke(&path, IDENTITY), stroke(join, 200.0), 400.0)
            .pieces
            .into_iter()
            .map(|piece| piece.points)
            .collect::<Vec<_>>()
    };
    assert_ne!(
        pieces(LineJoin::Miter),
        pieces(LineJoin::Round),
        "a turn of a tenth of a degree is an angle and is mitered"
    );
}
