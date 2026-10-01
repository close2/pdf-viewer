//! §8.4.3.4: **a point inside a curve is no corner of the path**, so a stroke's join style does
//! not shape it (ADR 1455).
//!
//! > Join styles shall be significant only at points where consecutive segments of a path
//! > connect at an angle
//!
//! A curve is one segment, and the points its flattening adds are where the chords standing for
//! it meet, not where two segments do. What a stroke paints there is §8.4.3.2's set:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from the
//! > path in user space is less than or equal to half the line width
//!
//! which, round a point between two chords, is the disc of the half-width about it. Where a curve
//! turns back on itself more tightly than the half-width, a miter at such a point stands
//! `hw / cos(θ / 2)` from it — under the default limit of 10, past twice the half-width — and
//! paints a wedge that no point of the path is near. The fixture is the tail of a curve where
//! `inks.pdf` drew that wedge at 4×, its points as the stroker received them; the expected
//! coverage of every pixel is the share of it within the half-width of the chords, a predicate
//! on each sample point and nothing of this rasteriser.

use raster_scene::{LineCap, LineJoin, Point, Stroke};

use crate::raster::flatten::{FLATTEN_TOLERANCE, Polyline, Tangent};
use crate::raster::{Rule, fill_mask, stroke_polylines};

/// Half the stroke's device width.
const HW: f32 = 10.0;

/// The region drawn: every pixel the set or a miter at the hairpin can reach.
const SIZE: (u32, u32) = (56, 40);

/// The last four points of one flattened curve in `inks.pdf` at 4×, moved near the origin: the
/// curve runs left, turns back through 127° within less than a pixel, and leaves to the right.
const TAIL: [(f32, f32); 4] = [
    (32.34, 19.50),
    (30.65, 20.31),
    (30.15, 20.96),
    (32.94, 21.12),
];

fn points() -> Vec<Point> {
    TAIL.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

/// The tail as one curve: a direction leaving its first point and one arriving at its last, each
/// the chord's own to the bit so that the caps are square to the chords and the set is the
/// chords' alone; the two points between are the curve's flattening.
fn as_one_curve() -> Polyline {
    let p = points();
    let along = |a: Point, b: Point| Point::new(b.x - a.x, b.y - a.y);
    Polyline {
        tangents: vec![
            Tangent {
                at: 0,
                arriving: None,
                leaving: Some(along(p[0], p[1])),
            },
            Tangent {
                at: 3,
                arriving: Some(along(p[2], p[3])),
                leaving: None,
            },
        ],
        points: p,
        closed: false,
    }
}

/// The same points as three line segments: the hairpin is then a corner of the path.
fn as_three_lines() -> Polyline {
    Polyline {
        points: points(),
        closed: false,
        tangents: Vec::new(),
    }
}

/// Mitered at the miter limit's initial value, 10.0 (Table 51), round caps — so that the
/// set at the two ends is the discs about them and the whole set is the chords' distance set.
fn mitered() -> Stroke {
    Stroke {
        width: 2.0 * HW,
        adjust: false,
        cap: LineCap::Round,
        join: LineJoin::Miter,
        miter_limit: 10.0,
    }
}

/// The distance from `(x, y)` to the nearest point of the chords.
fn distance_to_chords(x: f64, y: f64) -> f64 {
    TAIL.windows(2)
        .map(|w| {
            let (ax, ay) = (f64::from(w[0].0), f64::from(w[0].1));
            let (dx, dy) = (f64::from(w[1].0) - ax, f64::from(w[1].1) - ay);
            let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (x - ax - t * dx).hypot(y - ay - t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The share of the pixel at `(column, row)` within the half-width of the chords, on a 32 × 32
/// grid of sample centres.
fn set_coverage(column: u32, row: u32) -> f64 {
    const N: u32 = 32;
    let inside = (0..N * N)
        .filter(|k| {
            let x = f64::from(column) + (f64::from(k % N) + 0.5) / f64::from(N);
            let y = f64::from(row) + (f64::from(k / N) + 0.5) / f64::from(N);
            distance_to_chords(x, y) <= f64::from(HW)
        })
        .count();
    #[expect(clippy::cast_precision_loss)] // at most 1024
    let share = inside as f64 / f64::from(N * N);
    share
}

/// The pixels of `polyline` stroked, beside the set's coverage of each in levels of 255.
fn drawn(polyline: &Polyline) -> Vec<(u32, u32, f64, f64)> {
    let pieces = stroke_polylines(std::slice::from_ref(polyline), mitered(), 2.0 * HW);
    let mask = fill_mask(&pieces, Rule::NonZero, 0, 0, SIZE.0, SIZE.1);
    let mut out = Vec::new();
    for row in 0..SIZE.1 {
        for column in 0..SIZE.0 {
            let byte = mask.coverage[(row * SIZE.0 + column) as usize];
            out.push((
                column,
                row,
                f64::from(byte),
                255.0 * set_coverage(column, row),
            ));
        }
    }
    out
}

/// Whether every point of the pixel at `(column, row)` is further from the chords than the
/// half-width and §10.7.2's tolerance together: no ink of the stroke may reach it.
fn beyond_the_set(column: u32, row: u32) -> bool {
    let reach = f64::from(HW + FLATTEN_TOLERANCE) + std::f64::consts::FRAC_1_SQRT_2;
    distance_to_chords(f64::from(column) + 0.5, f64::from(row) + 0.5) > reach
}

#[test]
fn a_hairpin_inside_a_curve_paints_the_set_and_no_miter() {
    let pixels = drawn(&as_one_curve());
    let outside: Vec<_> = pixels
        .iter()
        .filter(|&&(column, row, byte, _)| byte > 0.0 && beyond_the_set(column, row))
        .collect();
    assert!(outside.is_empty(), "ink beyond the set: {outside:?}");
    // Each pixel within what the arcs' flattening may move it: the round join and caps are
    // polygons within the tolerance of their circles on either side (ADR 1443), which in one
    // pixel is at most the tolerance times the arc's length there, under one and a half.
    let bound = 255.0 * f64::from(FLATTEN_TOLERANCE) * 1.5;
    let worst = pixels
        .iter()
        .map(|&(column, row, byte, set)| ((byte - set).abs(), column, row))
        .fold((0.0, 0, 0), |a, b| if b.0 > a.0 { b } else { a });
    assert!(worst.0 <= bound, "pixel {worst:?} past {bound}");
    // And the ink is the set's, to a level of 255 a pixel across its rim.
    let (ink, set): (f64, f64) = pixels
        .iter()
        .fold((0.0, 0.0), |(i, s), &(_, _, byte, area)| {
            (i + byte, s + area)
        });
    let rim = pixels
        .iter()
        .filter(|&&(_, _, _, area)| area > 0.0 && area < 255.0)
        .count();
    #[expect(clippy::cast_precision_loss)] // a few hundred pixels
    let allowance = rim as f64;
    assert!(
        (ink - set).abs() <= allowance,
        "ink {ink} against the set's {set}, rim {rim}"
    );
}

#[test]
fn the_same_hairpin_between_two_lines_is_a_corner_and_mitered() {
    // Table 54's miter: "[t]he outer edges of the strokes for the two segments shall be
    // extended until they meet at an angle". At 127° of turn the ratio is 1 / sin(26.5°),
    // about 2.2, under the limit of 10, so the corner reaches past the set.
    let pixels = drawn(&as_three_lines());
    let past = pixels
        .iter()
        .filter(|&&(column, row, byte, _)| byte >= 255.0 && beyond_the_set(column, row))
        .count();
    assert!(
        past > 20,
        "only {past} full pixels of the miter past the set"
    );
}
