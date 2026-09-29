//! §8.5.3.3 with §10.7.4: **a pixel is covered by the area of the set the rule declares
//! inside**, where the path's own portions overlap (ADR 1389).
//!
//! §8.5.3.3.2 and §8.5.3.3.3 decide insideness point by point, and §10.7.4 scan-converts
//! once "all 'insideness' computations have been performed". So wherever the path is one
//! boundary the integrated winding is the answer, and wherever two of its boundaries meet
//! in a pixel the answer is still the inside set's area there — never the sum of two
//! portions, and never their difference. Every expected value below is that area in
//! closed form: inclusion–exclusion over axis-aligned rectangles for the squares, and a
//! polygon clipped to each pixel for the star, computed in `f64` with no rasteriser.
//! Each is checked at every pixel, at 1×, 2×, 4× and 8×, to the nearest coverage byte.

// A pixel's `(i, j)`, its corner's `(x, y)` and an edge's `(p, q)` are the letters the
// clauses and the closed forms use.
#![allow(clippy::many_single_char_names)]

use raster_scene::{Point, Segment};

use crate::raster::{DeviceTransform, Rule, fill_mask, flatten};

const RUNGS: [f64; 4] = [1.0, 2.0, 4.0, 8.0];

/// Half a coverage step, the byte's own rounding, and a margin for `f32`: every pixel
/// below is the nearest byte to its closed form.
const ROUNDING: f64 = 0.6;

/// A closed subpath through `points`, in the order given.
fn subpath(points: &[(f64, f64)]) -> Vec<Segment> {
    #[expect(clippy::cast_possible_truncation)] // page coordinates under 100
    let at = |(x, y): (f64, f64)| Point::new(x as f32, y as f32);
    let mut path = vec![Segment::MoveTo(at(points[0]))];
    path.extend(points[1..].iter().map(|&p| Segment::LineTo(at(p))));
    path.push(Segment::Close);
    path
}

/// A rectangle `[x0, x1] × [y0, y1]` as a subpath, wound clockwise on the page (as the
/// rows run down) when `forward`, and the other way round when not.
fn square(x0: f64, y0: f64, x1: f64, y1: f64, forward: bool) -> Vec<Segment> {
    let mut corners = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    if !forward {
        corners.reverse();
    }
    subpath(&corners)
}

/// The length of `[a0, a1] ∩ [b0, b1]`.
fn meet(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

/// The area of rectangle `r` inside pixel `(i, j)`.
fn in_pixel(r: [f64; 4], i: usize, j: usize) -> f64 {
    #[expect(clippy::cast_precision_loss)] // pixel indices under 1000
    let (x, y) = (i as f64, j as f64);
    meet(r[0], r[2], x, x + 1.0) * meet(r[1], r[3], y, y + 1.0)
}

/// Rasterise `path` at scale `s` under `rule`, and compare every pixel against `want`.
/// Returns the worst difference in coverage steps.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // sizes under 1000
fn worst(
    path: &[Segment],
    rule: Rule,
    s: f64,
    size: f64,
    want: impl Fn(usize, usize) -> f64,
) -> f64 {
    #[expect(clippy::cast_possible_truncation)] // rung scales are small integers
    let transform = DeviceTransform {
        a: s as f32,
        b: 0.0,
        c: 0.0,
        d: s as f32,
        e: 0.0,
        f: 0.0,
    };
    let n = (size * s) as u32;
    let mask = fill_mask(&flatten(path, transform), rule, 0, 0, n, n);
    let mut worst = 0.0_f64;
    for j in 0..n as usize {
        for i in 0..n as usize {
            let got = f64::from(mask.coverage[j * n as usize + i]);
            worst = worst.max((got - want(i, j).clamp(0.0, 1.0) * 255.0).abs());
        }
    }
    worst
}

/// `QUORRA_FEEDBACK.md` section 45's reproduction: one fill whose path states the band
/// `[0, 0.75] × [0, 4]` twice. Same way round, the set is the band once — three whole
/// pixels of ink, the boundary column 0.75 and not 1. Opposed, non-zero winds it to zero.
#[test]
fn a_rectangle_stated_twice_is_the_rectangle_once() {
    for forward in [true, false] {
        let mut path = square(0.0, 0.0, 0.75, 4.0, true);
        path.extend(square(0.0, 0.0, 0.75, 4.0, forward));
        for s in RUNGS {
            let band = |i: usize, j: usize| {
                let area = in_pixel([0.0, 0.0, 0.75 * s, 4.0 * s], i, j);
                if forward { area } else { 0.0 }
            };
            let off = worst(&path, Rule::NonZero, s, 8.0, band);
            assert!(
                off <= ROUNDING,
                "stated twice (same way: {forward}) at {s}x: {off} steps off"
            );
        }
    }
}

/// Two squares in one path, `A = [1, 5.3]²` and `B = [3.6, 7.9] × [2.4, 6.7]`, every edge
/// off the pixel grid so the overlap's rim shares pixels with both. Per pixel, by
/// inclusion–exclusion:
///
/// - wound the same way, non-zero's set is `A ∪ B`: `|A| + |B| − |A ∩ B|`;
/// - wound the same way, even-odd's is the symmetric difference, `|A| + |B| − 2 |A ∩ B|`
///   (the overlap winds twice);
/// - wound against each other, the overlap winds to zero and each square alone to `±1`,
///   so **both** rules' set is the symmetric difference — and in a pixel that holds some
///   of `A` alone and some of `B` alone, the winding `+1` beside `−1` is inside on both
///   sides, where an integral of the two cancels.
#[test]
fn two_overlapping_squares_are_their_set_under_both_rules() {
    let (a, b): ([f64; 4], [f64; 4]) = ([1.0, 1.0, 5.3, 5.3], [3.6, 2.4, 7.9, 6.7]);
    let both = [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ];
    for (forward, rule, twice) in [
        (true, Rule::NonZero, 1.0),
        (true, Rule::EvenOdd, 2.0),
        (false, Rule::NonZero, 2.0),
        (false, Rule::EvenOdd, 2.0),
    ] {
        let mut path = square(a[0], a[1], a[2], a[3], true);
        path.extend(square(b[0], b[1], b[2], b[3], forward));
        for s in RUNGS {
            let scaled = |r: [f64; 4]| r.map(|v| v * s);
            let set = |i: usize, j: usize| {
                in_pixel(scaled(a), i, j) + in_pixel(scaled(b), i, j)
                    - twice * in_pixel(scaled(both), i, j)
            };
            let off = worst(&path, rule, s, 9.0, set);
            assert!(
                off <= ROUNDING,
                "squares (same way: {forward}) under {rule:?} at {s}x: {off} steps off"
            );
        }
    }
}

/// The area of the simple polygon `poly` inside pixel `(i, j)`: Sutherland–Hodgman against
/// the pixel's four sides, then the shoelace. Exact for a concave polygon against a convex
/// window, which is all the star needs.
fn polygon_in_pixel(poly: &[(f64, f64)], i: usize, j: usize) -> f64 {
    #[expect(clippy::cast_precision_loss)] // pixel indices under 1000
    let (x0, y0) = (i as f64, j as f64);
    let mut out: Vec<(f64, f64)> = poly.to_vec();
    // Each side as (inside test, intersection) along one axis.
    for (axis, bound, keep_above) in [
        (0, x0, true),
        (0, x0 + 1.0, false),
        (1, y0, true),
        (1, y0 + 1.0, false),
    ] {
        let coord = |p: (f64, f64)| if axis == 0 { p.0 } else { p.1 };
        let inside = |p: (f64, f64)| {
            if keep_above {
                coord(p) >= bound
            } else {
                coord(p) <= bound
            }
        };
        let input = std::mem::take(&mut out);
        for k in 0..input.len() {
            let (p, q) = (input[k], input[(k + 1) % input.len()]);
            if inside(p) {
                out.push(p);
            }
            if inside(p) != inside(q) {
                let t = (bound - coord(p)) / (coord(q) - coord(p));
                out.push((p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t));
            }
        }
        if out.is_empty() {
            return 0.0;
        }
    }
    let mut twice = 0.0;
    for k in 0..out.len() {
        let (p, q) = (out[k], out[(k + 1) % out.len()]);
        twice += p.0 * q.1 - q.0 * p.1;
    }
    (twice * 0.5).abs()
}

/// §8.5.3.3.3's own example, Figure 19's five-pointed star, drawn as one subpath through
/// its five points in star order: under non-zero the set is the whole star, points and
/// centre (the pentagon winds twice); under even-odd, "the rule considers the triangular
/// points to be inside the path, but not the pentagon in the centre". Expected per pixel:
/// the star's ten-sided outline clipped to the pixel, less, for even-odd, the pentagon.
#[test]
fn the_five_pointed_star_is_its_set_under_both_rules() {
    let (c, r) = (20.0_f64, 15.3_f64);
    let at = |radius: f64, degrees: f64| {
        let a = degrees.to_radians();
        (c + radius * a.cos(), c + radius * a.sin())
    };
    // The pentagon's corners sit where the star's edges cross: radius
    // `r · cos 72° / cos 36°`, half-way round between the points.
    let inner = r * 72.0_f64.to_radians().cos() / 36.0_f64.to_radians().cos();
    let star_order: Vec<(f64, f64)> = (0..5)
        .map(|k| at(r, -90.0 + 144.0 * f64::from(k)))
        .collect();
    let outline: Vec<(f64, f64)> = (0..10)
        .map(|k| {
            let (radius, degrees) = if k % 2 == 0 { (r, 0.0) } else { (inner, 36.0) };
            at(radius, -90.0 + degrees + 36.0 * f64::from(k - k % 2))
        })
        .collect();
    let pentagon: Vec<(f64, f64)> = (0..5)
        .map(|k| at(inner, -54.0 + 72.0 * f64::from(k)))
        .collect();
    let path = subpath(&star_order);
    for (rule, hole) in [(Rule::NonZero, 0.0), (Rule::EvenOdd, 1.0)] {
        for s in RUNGS {
            let scale = |poly: &[(f64, f64)]| {
                poly.iter()
                    .map(|&(x, y)| (x * s, y * s))
                    .collect::<Vec<_>>()
            };
            let (outline, pentagon) = (scale(&outline), scale(&pentagon));
            let set = |i: usize, j: usize| {
                polygon_in_pixel(&outline, i, j) - hole * polygon_in_pixel(&pentagon, i, j)
            };
            let off = worst(&path, rule, s, 40.0, set);
            assert!(
                off <= ROUNDING,
                "the star under {rule:?} at {s}x: {off} steps off"
            );
        }
    }
}

/// ADR 0049's window rule holds for the recomputed pixels too: a tile cut out of the
/// star's region, its edges crossing both of the tile's sides, reads what the whole
/// region reads over the same pixels — the set's boundary deposits at the border exactly
/// as an edge does.
#[test]
fn a_tile_of_the_star_is_the_crop_of_the_whole() {
    let (c, r) = (20.0_f64, 15.3_f64);
    let points: Vec<(f64, f64)> = (0..5)
        .map(|k| {
            let a = (-90.0 + 144.0 * f64::from(k)).to_radians();
            (c + r * a.cos(), c + r * a.sin())
        })
        .collect();
    let transform = DeviceTransform {
        a: 4.0,
        b: 0.0,
        c: 0.0,
        d: 4.0,
        e: 0.0,
        f: 0.0,
    };
    let polylines = flatten(&subpath(&points), transform);
    for rule in [Rule::NonZero, Rule::EvenOdd] {
        let whole = fill_mask(&polylines, rule, 0, 0, 160, 160);
        let tile = fill_mask(&polylines, rule, 53, 61, 47, 29);
        let crop = whole.crop(53, 61, 47, 29);
        let worst = tile
            .coverage
            .iter()
            .zip(&crop.coverage)
            .map(|(a, b)| a.abs_diff(*b))
            .max();
        assert!(
            worst <= Some(1),
            "{rule:?}: the tile differs from the crop by {worst:?}"
        );
    }
}

/// Two subpaths that neither cross nor nest can still share a pixel, and wound against
/// each other they are `doc/QUORRA_FEEDBACK.md` section 45's cancellation without any
/// overlap: bands `[1.2, 1.5] × [1, 7]` wound one way and `[1.7, 2.9] × [1, 7]` the other. The winding is `+1` in the first,
/// `−1` in the second, and both are inside under both rules — `0.3 + 0.3` of pixel column 1,
/// where an integral of the two reads `0.3 − 0.3`.
///
/// And a square held by a square wound the same way, `[1.2, 6.8]²` round `[1.6, 6.4]²`:
/// non-zero's set is the outer square, even-odd's the ring, and the pixels along the
/// ring's inside hold windings `0`, `1` and `2` at once.
#[test]
fn subpaths_apart_or_nested_are_their_set_under_both_rules() {
    let (a, b) = ([1.2, 1.0, 1.5, 7.0], [1.7, 1.0, 2.9, 7.0]);
    let mut apart = square(a[0], a[1], a[2], a[3], true);
    apart.extend(square(b[0], b[1], b[2], b[3], false));
    let (outer, inner) = ([1.2, 1.2, 6.8, 6.8], [1.6, 1.6, 6.4, 6.4]);
    let mut nested = square(outer[0], outer[1], outer[2], outer[3], true);
    nested.extend(square(inner[0], inner[1], inner[2], inner[3], true));
    for s in RUNGS {
        let scaled = |r: [f64; 4]| r.map(|v| v * s);
        for rule in [Rule::NonZero, Rule::EvenOdd] {
            let set = |i: usize, j: usize| in_pixel(scaled(a), i, j) + in_pixel(scaled(b), i, j);
            let off = worst(&apart, rule, s, 8.0, set);
            assert!(
                off <= ROUNDING,
                "apart, opposed, under {rule:?} at {s}x: {off} steps off"
            );
        }
        let held = |i: usize, j: usize| in_pixel(scaled(outer), i, j);
        let off = worst(&nested, Rule::NonZero, s, 8.0, held);
        assert!(
            off <= ROUNDING,
            "nested, same way, non-zero at {s}x: {off} steps off"
        );
        let ring =
            |i: usize, j: usize| in_pixel(scaled(outer), i, j) - in_pixel(scaled(inner), i, j);
        let off = worst(&nested, Rule::EvenOdd, s, 8.0, ring);
        assert!(
            off <= ROUNDING,
            "nested, same way, even-odd at {s}x: {off} steps off"
        );
    }
}

/// **A hairline far from the origin is asked about in `f64`**: a `0.12`-wide segment with
/// round caps at a page coordinate of a thousand — a hatch line of `issue12810.pdf`, where
/// 28 553 strokes of this shape were walked pixel by pixel — is its body and two caps, which
/// neither cross nor nest, so the fill winds two values. The cap's inside point is stepped a
/// thousandth of its longest edge, `6e-5`, from that edge, below the `f32` spacing there;
/// rounded to `f32` it lands on the edge the body shares, and the body was read as holding
/// the cap (ADR 1397).
#[test]
fn a_hairline_far_from_the_origin_winds_two_values() {
    use raster_scene::{LineCap, LineJoin, Stroke};
    let hatch = [
        Segment::MoveTo(Point::new(1024.86, 606.0)),
        Segment::LineTo(Point::new(1028.4, 602.46)),
    ];
    let hairline = Stroke {
        width: 0.12,
        adjust: false,
        cap: LineCap::Round,
        join: LineJoin::Round,
        miter_limit: 10.0,
    };
    let pieces = crate::raster::stroke_polylines(
        &crate::raster::flatten_stroke(&hatch, super::IDENTITY),
        hairline,
        hairline.width,
    );
    assert_eq!(pieces.len(), 3, "the body and two caps");
    assert_eq!(crate::raster::winds_two_values(&pieces), Some(true));
}
