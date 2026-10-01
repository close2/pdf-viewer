//! §10.7.2 and ADR 0044: **how finely a curve becomes chords, and what that costs its ink.**
//!
//! Every case here is a statement about the flatness bound. Two of the three read it out of
//! [`fill`](crate::raster::fill)'s coverage bytes, because ink is the only place a chord
//! that left the curve becomes visible — the bytes are the instrument, the bound is the
//! subject.

use raster_scene::{Point, Segment};

use crate::raster::flatten::{DeviceTransform, FLATTEN_TOLERANCE, cubic_tolerance};
use crate::raster::{Rule, fill_mask, flatten};

use super::IDENTITY;

/// `4(√2 − 1)/3`: the control offset, in units of the radius, of the four-cubic
/// circle — the construction the caller's shared crate uses for §8.5.3.2's dot, and
/// the one the tolerance below is stated against.
const CIRCLE_K: f32 = 0.552_284_8;

/// A circle of radius `r` about `(cx, cy)`, as the four cubics a document draws it
/// with.
fn circle_path(cx: f32, cy: f32, r: f32) -> Vec<Segment> {
    let k = CIRCLE_K * r;
    let p = |x: f32, y: f32| Point::new(cx + x, cy + y);
    vec![
        Segment::MoveTo(p(r, 0.0)),
        Segment::CubicTo {
            c1: p(r, k),
            c2: p(k, r),
            to: p(0.0, r),
        },
        Segment::CubicTo {
            c1: p(-k, r),
            c2: p(-r, k),
            to: p(-r, 0.0),
        },
        Segment::CubicTo {
            c1: p(-r, -k),
            c2: p(-k, -r),
            to: p(0.0, -r),
        },
        Segment::CubicTo {
            c1: p(k, -r),
            c2: p(r, -k),
            to: p(r, 0.0),
        },
        Segment::Close,
    ]
}

/// The area a closed path of lines and cubics encloses, in closed form, `f64`: Green's
/// theorem, where a cubic from `p0` contributes `(3/20)·(p1×p2 + p1×p3 + 2·p2×p3)` measured
/// from `p0` plus the triangle `p0×p3 / 2` (ADR 1443).
fn enclosed_area(path: &[Segment], t: DeviceTransform) -> f64 {
    let at = |p: Point| {
        (
            f64::from(t.a) * f64::from(p.x) + f64::from(t.c) * f64::from(p.y) + f64::from(t.e),
            f64::from(t.b) * f64::from(p.x) + f64::from(t.d) * f64::from(p.y) + f64::from(t.f),
        )
    };
    let cross = |a: (f64, f64), b: (f64, f64)| a.0 * b.1 - a.1 * b.0;
    let less = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0, a.1 - b.1);
    let (mut sum, mut start, mut current) = (0.0, (0.0, 0.0), (0.0, 0.0));
    for segment in path {
        match *segment {
            Segment::MoveTo(p) => {
                start = at(p);
                current = start;
            }
            Segment::LineTo(p) => {
                let to = at(p);
                sum += cross(current, to) / 2.0;
                current = to;
            }
            Segment::CubicTo { c1, c2, to } => {
                let (p1, p2, p3) = (at(c1), at(c2), at(to));
                let (a, b, c) = (less(p1, current), less(p2, current), less(p3, current));
                sum += 0.15 * (cross(a, b) + cross(a, c) + 2.0 * cross(b, c))
                    + cross(current, p3) / 2.0;
                current = p3;
            }
            Segment::Close => {
                sum += cross(current, start) / 2.0;
                current = start;
            }
        }
    }
    sum
}

/// A flattening's own polygon area and perimeter, `f64`, no rasteriser.
fn polygon_area_and_length(path: &[Segment], t: DeviceTransform) -> (f64, f64) {
    let (mut area, mut length) = (0.0, 0.0);
    for polyline in flatten(path, t) {
        let p = &polyline.points;
        for i in 0..p.len() {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            area += (f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y)) / 2.0;
            length += (f64::from(b.x) - f64::from(a.x)).hypot(f64::from(b.y) - f64::from(a.y));
        }
    }
    (area, length)
}

/// **A flattened curve encloses its own area** (ADR 1443), summed with no rasteriser.
///
/// §10.7.4: "[t]he area covered by painted pixels shall always be at least as large as the
/// area of the original shape." A chord between two points of a curve lies on its concave
/// side, so an inscribed flattening takes two thirds of each piece's height times its length
/// out of the rim — 2.4 levels of 255 a rim pixel on a circle of radius 0.75 and 19.7 on one
/// of radius 24, at this tolerance. The two chords through the midpoint of each piece's inner
/// controls enclose the piece's own area to second order; the bound held here is **half a
/// coverage level per unit of rim**, `0.5/255` times the perimeter, which is the rounding a
/// coverage byte already costs a rim pixel. Circles from a quarter pixel to 96, a sheared
/// ellipse, and a closed S whose cubic inflects, each drawn both ways round.
#[test]
fn a_flattened_curve_encloses_its_own_area() {
    let sheared = DeviceTransform {
        a: 1.4,
        b: 1.0,
        c: -0.5,
        d: 0.7,
        e: 30.0,
        f: 10.0,
    };
    let s_curve = vec![
        Segment::MoveTo(Point::new(10.0, 10.0)),
        Segment::CubicTo {
            c1: Point::new(30.0, -5.0),
            c2: Point::new(20.0, 35.0),
            to: Point::new(40.0, 20.0),
        },
        Segment::LineTo(Point::new(40.0, 40.0)),
        Segment::Close,
    ];
    let mut cases: Vec<(String, Vec<Segment>, DeviceTransform)> =
        [0.25_f32, 0.75, 1.5, 3.0, 6.0, 24.0, 96.0]
            .iter()
            .map(|&r| {
                (
                    format!("circle of radius {r}"),
                    circle_path(100.3, 100.7, r),
                    IDENTITY,
                )
            })
            .collect();
    cases.push((
        "sheared ellipse".into(),
        circle_path(20.0, 20.0, 3.75),
        sheared,
    ));
    cases.push(("closed S".into(), s_curve, IDENTITY));
    for (what, path, t) in cases {
        for drawn in [path.clone(), super::stroke_set::reversed(&path)] {
            let want = enclosed_area(&drawn, t);
            let (got, rim) = polygon_area_and_length(&drawn, t);
            let per_rim = 255.0 * (got - want) / rim;
            assert!(
                per_rim.abs() <= 0.5,
                "{what}: the flattening encloses {got:.6} against the curve's {want:.6}, \
                 {per_rim:+.3} levels a unit of rim"
            );
        }
    }
}

/// A cubic with collinear control points is a straight line: flattening must not
/// bend it, so the fill equals the `LineTo` version byte for byte.
#[test]
fn collinear_cubic_equals_the_line() {
    let with_cubic = vec![
        Segment::MoveTo(Point::new(0.0, 0.0)),
        Segment::LineTo(Point::new(6.0, 0.0)),
        Segment::CubicTo {
            c1: Point::new(6.0, 2.0),
            c2: Point::new(6.0, 4.0),
            to: Point::new(6.0, 6.0),
        },
        Segment::LineTo(Point::new(0.0, 6.0)),
        Segment::Close,
    ];
    let with_line = vec![
        Segment::MoveTo(Point::new(0.0, 0.0)),
        Segment::LineTo(Point::new(6.0, 0.0)),
        Segment::LineTo(Point::new(6.0, 6.0)),
        Segment::LineTo(Point::new(0.0, 6.0)),
        Segment::Close,
    ];
    let a = fill_mask(&flatten(&with_cubic, IDENTITY), Rule::NonZero, 0, 0, 7, 7);
    let b = fill_mask(&flatten(&with_line, IDENTITY), Rule::NonZero, 0, 0, 7, 7);
    assert_eq!(a.coverage, b.coverage);
}

/// **A circle deposits its own area at every size, including the sub-pixel ones.**
///
/// The caller's `QUORRA_FEEDBACK.md` section 21.2: at diameters 0.5, 1.0 and 2.0 device
/// pixels this rasteriser deposited 36.1 %, 36.1 % and 10.1 % less ink than
/// `π·r²` — the inscribed square, the inscribed square, and the inscribed octagon,
/// which is what a quarter-pixel flatness bound admits when a whole curve is a
/// pixel across. ISO 32000-2 §10.7.2's NOTE 2 says what the bound is for: "the
/// purpose of the flatness tolerance is to control the precision of curve
/// rendering, not to draw inscribed polygons".
///
/// The area compared against is the four cubics' own, in closed form ([`enclosed_area`]),
/// and the bound is one half of a coverage step for each pixel the circle can touch,
/// either way, because coverage is quantised by `round(cov × 255)` at every one of them:
/// each piece's two chords enclose its own area (ADR 1443), so the flattening leaves no
/// shortfall of its own to allow for.
#[test]
fn a_circle_deposits_its_own_area_at_every_size() {
    // A pixel centre, so a mark of any diameter is centred in the grid rather than
    // straddling it: the touched-pixel count below is then the honest one.
    const C: f32 = 4.5;
    for diameter in [0.5_f32, 1.0, 2.0] {
        let r = diameter / 2.0;
        let mask = fill_mask(
            &flatten(&circle_path(C, C, r), IDENTITY),
            Rule::NonZero,
            0,
            0,
            9,
            9,
        );
        let ink: f32 = mask.coverage.iter().map(|b| f32::from(*b) / 255.0).sum();
        #[expect(clippy::cast_possible_truncation)] // an area of a few pixels
        let area = enclosed_area(&circle_path(C, C, r), IDENTITY) as f32;

        // Only pixels the circle reaches carry a rounding error; the rest are an
        // exact zero. `[floor(c − r), ceil(c + r))` is §10.7.4's half-open pixel
        // rule applied to the mark's own bounds.
        let touched = (C + r).ceil() - (C - r).floor();
        let quantum = touched * touched * 0.5 / 255.0;

        assert!(
            (ink - area).abs() <= quantum,
            "a circle of diameter {diameter} drew {ink:.4} against its area {area:.4}, \
             past the {quantum:.4} rounding allows"
        );
    }
}

/// **The relative bound is inert on anything big, and that is a perf statement.**
///
/// `RELATIVE_FLATTEN_TOLERANCE` enters through a `min`, so it can only ever add
/// segments, and it adds none once `extent/32 ≥ 0.25` — a control polygon 8 device
/// pixels across, which for a circle's quarter-arc cubic (diagonal `r√2`) is a
/// radius of 5.66. The population that pays for ADR 0044 is therefore bounded by
/// arithmetic rather than by hope, and the counts below pin it.
///
/// A quarter-arc of half-angle `α` puts its controls `(4/3)·r·tan(α/2)·sin(α)` from
/// its chord, so at `r = 20` the successive depths are 7.81, 2.03, 0.51 and 0.128
/// device pixels: three splits, eight pieces a quarter, **32 a turn**, each piece two
/// chords (ADR 1443). The polyline carries one point more than its 64 chords because it
/// opens on the `MoveTo` and closes by returning to it.
#[test]
fn a_large_curve_keeps_the_segment_count_it_had() {
    let big = flatten(&circle_path(40.0, 40.0, 20.0), IDENTITY);
    assert_eq!(big.len(), 1, "one subpath");
    assert_eq!(
        big[0].points.len(),
        65,
        "32 pieces a turn at r = 20, two chords each"
    );

    // The `min` is not binding anywhere on that curve: every one of its four cubics
    // spans `r√2 = 28.3` device pixels, and 28.3/32 is past a quarter pixel.
    for segment in circle_path(40.0, 40.0, 20.0) {
        if let Segment::CubicTo { c1, c2, to } = segment {
            let from = Point::new(40.0 + 20.0, 40.0);
            assert!(
                cubic_tolerance(from, c1, c2, to) >= FLATTEN_TOLERANCE,
                "the relative bound must not tighten a 28-pixel cubic"
            );
        }
    }

    // And the small ones are where it does bind: 16 pieces a turn, at every size.
    for diameter in [0.5_f32, 1.0, 2.0, 4.0] {
        let small = flatten(&circle_path(8.0, 8.0, diameter / 2.0), IDENTITY);
        assert_eq!(
            small[0].points.len(),
            33,
            "a circle of diameter {diameter} flattens to 16 pieces, two chords each"
        );
    }
}
