//! §8.4.3.2: **a stroke is a set of points, and the set does not depend on which way the
//! path runs.**
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from the
//! > path in user space is less than or equal to half the line width
//!
//! Under round joins and caps that sentence is the whole shape; under miter and bevel joins
//! Table 54 adds or removes a stated polygon at each corner. Neither mentions the path's
//! direction, so every fixture here is drawn **both ways** and the two masks must agree to a
//! sixteenth of a pixel at every pixel, at every rung of a 1×, 2×, 4×, 8× ladder — the
//! caller's `ink_ladder` run as a gate, so that a stroker that turns one way better than the
//! other fails here rather than on a page. ADR 1361 is the construction these hold.
//!
//! The expected ink is the set's area, derived from the clause and not from this rasteriser:
//! in closed form where the figure is polygons and discs, and for the one curve whose set has
//! no closed form (the hook) by a Python sampling of the distance set on a 1/64-unit grid from
//! the control points alone, whose script is quoted where the number is used. Where a curve
//! or an arc is involved the ink may fall short of the set by the strip §10.7.2 lets a chord
//! cut off — the set's rim times `FLATTEN_TOLERANCE`, in device pixels — and by nothing more;
//! each such bound is stated beside its fixture.

use raster_scene::{LineCap, LineJoin, Point, Segment, Stroke};

use crate::raster::flatten::FLATTEN_TOLERANCE;
use crate::raster::{
    CoverageMask, DeviceTransform, Rule, fill_mask, fill_mask_settled, flatten_stroke,
    stroke_pieces, stroke_polylines,
};

/// The rungs of the ladder: the caller's `ink_ladder` scales.
pub(super) const RUNGS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

/// A sixteenth of a pixel, in coverage bytes: the CPU oracle's converter quantum (ADR 1348),
/// and the most two drawings of one set may differ by at a pixel.
const SIXTEENTH: u8 = 16;

pub(super) fn scaled(s: f32) -> DeviceTransform {
    DeviceTransform {
        a: s,
        b: 0.0,
        c: 0.0,
        d: s,
        e: 0.0,
        f: 0.0,
    }
}

pub(super) fn stroke(width: f32, cap: LineCap, join: LineJoin, miter_limit: f32) -> Stroke {
    Stroke {
        width,
        adjust: false,
        cap,
        join,
        miter_limit,
    }
}

/// The mask of `path` stroked on a 100 × 100 unit page at scale `s`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 800 at most
pub(super) fn mask(path: &[Segment], stroke: Stroke, s: f32) -> CoverageMask {
    let size = (100.0 * s) as u32;
    let pieces = stroke_polylines(&flatten_stroke(path, scaled(s)), stroke, stroke.width * s);
    fill_mask(&pieces, Rule::NonZero, 0, 0, size, size)
}

/// Ink in page units: the coverage summed and divided by the scale squared, as
/// `ink_ladder` normalises its rungs.
fn ink(mask: &CoverageMask, s: f32) -> f32 {
    mask.coverage
        .iter()
        .map(|b| f32::from(*b) / 255.0)
        .sum::<f32>()
        / (s * s)
}

/// A path run the other way: each subpath's segments in reverse order and each one
/// reversed, the subpaths themselves in their own order.
pub(super) fn reversed(path: &[Segment]) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < path.len() {
        let end = path[start + 1..]
            .iter()
            .position(|segment| matches!(segment, Segment::MoveTo(_)))
            .map_or(path.len(), |at| start + 1 + at);
        out.extend(reversed_subpath(&path[start..end]));
        start = end;
    }
    out
}

fn reversed_subpath(subpath: &[Segment]) -> Vec<Segment> {
    let mut points = Vec::new();
    let mut closed = false;
    for segment in subpath {
        match *segment {
            Segment::MoveTo(p) | Segment::LineTo(p) => points.push((p, None)),
            Segment::CubicTo { c1, c2, to } => points.push((to, Some((c1, c2)))),
            Segment::Close => closed = true,
        }
    }
    let mut out = vec![Segment::MoveTo(points[points.len() - 1].0)];
    for i in (1..points.len()).rev() {
        let to = points[i - 1].0;
        out.push(match points[i].1 {
            None => Segment::LineTo(to),
            Some((c1, c2)) => Segment::CubicTo { c1: c2, c2: c1, to },
        });
    }
    if closed {
        out.push(Segment::Close);
    }
    out
}

/// One rung's reading: the scale, the forward drawing's ink, and how far rounding each
/// pixel to a byte can have moved that ink — half a step, `1/510`, at every pixel that is
/// neither empty nor full, in page units.
struct Rung {
    s: f32,
    ink: f32,
    rounding: f32,
}

/// Draws `path` both ways at every rung, asserts the two masks agree to a sixteenth at
/// every pixel, and hands back each rung's reading of the forward drawing.
fn both_ways(path: &[Segment], stroke: Stroke, what: &str) -> Vec<Rung> {
    let back = reversed(path);
    RUNGS
        .iter()
        .map(|&s| {
            let (a, b) = (mask(path, stroke, s), mask(&back, stroke, s));
            let worst = a
                .coverage
                .iter()
                .zip(&b.coverage)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            assert!(
                worst <= SIXTEENTH,
                "{what} at {s}×: the two directions differ by {worst}/255 at a pixel"
            );
            #[expect(clippy::cast_precision_loss)] // a pixel count
            let partial = a.coverage.iter().filter(|b| **b != 0 && **b != 255).count() as f32;
            Rung {
                s,
                ink: ink(&a, s),
                rounding: partial / 510.0 / (s * s),
            }
        })
        .collect()
}

pub(super) fn line_path(points: &[(f32, f32)], closed: bool) -> Vec<Segment> {
    let mut path = vec![Segment::MoveTo(Point::new(points[0].0, points[0].1))];
    path.extend(
        points[1..]
            .iter()
            .map(|&(x, y)| Segment::LineTo(Point::new(x, y))),
    );
    if closed {
        path.push(Segment::Close);
    }
    path
}

/// A rung's ink against the set's area `want`: short of it by no more than the strip a
/// flattening may cut off (`short`), and on either side by no more than the rung's byte
/// rounding and a sixteenth of a pixel besides — the slack for a rim pixel whose coverage
/// was within half a step of empty or full and so is not counted as partial.
fn within(rung: &Rung, want: f32, short: f32, what: &str) {
    let slack = rung.rounding + 1.0 / 16.0;
    assert!(
        rung.ink >= want - short - slack && rung.ink <= want + slack,
        "{what} at {}×: {:.4} against the set's {want:.4} (chords {short:.4}, rounding {:.4})",
        rung.s,
        rung.ink,
        rung.rounding
    );
}

/// **The hook `20 20 m 20 32 32 32 32 20 c` at `16 w`, round caps and joins**: the path
/// `QUORRA_FEEDBACK.md` section 54 found drawn with radial slivers one way and solid the other.
///
/// The half-width, 8, is larger than the curve's radius of curvature at its bend, so every
/// flattened vertex's join reaches across its neighbours — the case where a join wound against
/// the body cancelled it. The set's area is 565.50, and its rim 87.47 units long, both from
/// the control points alone:
///
/// ```text
/// b = cubic([(20,20),(20,32),(32,32),(32,20)], 1201 points)
/// area = h² · #{grid points p : min distance from p to the chords of b ≤ 8},  h = 1/64
/// rim  = (π/4) · h · (row and column transitions of that grid),  h = 1/32   (Crofton)
/// ```
///
/// A pixel whose centre is within `8 − √2/2 − 1/4` device units of the curve (scaled) lies
/// wholly inside the set even after flattening, so it is fully covered; one beyond
/// `8 + √2/2` is wholly outside it. Those two are checked at every pixel, from the curve's
/// own points, and a sliver or a hole fails the first.
#[test]
fn the_hook_is_one_set_drawn_either_way() {
    const AREA: f32 = 565.50;
    const RIM: f32 = 87.47;
    let p = [(20.0, 20.0), (20.0, 32.0), (32.0, 32.0), (32.0, 20.0)].map(|(x, y)| Point::new(x, y));
    let path = [
        Segment::MoveTo(p[0]),
        Segment::CubicTo {
            c1: p[1],
            c2: p[2],
            to: p[3],
        },
    ];
    let round = stroke(16.0, LineCap::Round, LineJoin::Round, 10.0);
    for rung in both_ways(&path, round, "the hook") {
        within(&rung, AREA, RIM * FLATTEN_TOLERANCE / rung.s, "the hook");
    }

    // The curve itself, densely, for the per-pixel statement.
    let curve: Vec<Point> = (0..=2000)
        .map(|i| {
            #[expect(clippy::cast_precision_loss)] // 2000
            let t = i as f32 / 2000.0;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            Point::new(
                w.iter().zip(&p).map(|(w, p)| w * p.x).sum(),
                w.iter().zip(&p).map(|(w, p)| w * p.y).sum(),
            )
        })
        .collect();
    let s = 4.0;
    for back in [false, true] {
        let drawn = if back { reversed(&path) } else { path.to_vec() };
        let m = mask(&drawn, round, s);
        let mut holes = 0;
        for y in 0..m.height {
            for x in 0..m.width {
                #[expect(clippy::cast_precision_loss)] // 400
                let (cx, cy) = ((x as f32 + 0.5) / s, (y as f32 + 0.5) / s);
                let d = curve
                    .iter()
                    .map(|q| (q.x - cx).hypot(q.y - cy))
                    .fold(f32::INFINITY, f32::min)
                    * s;
                let byte = m.coverage[(y * m.width + x) as usize];
                // The sampled curve overstates a distance by at most half its spacing,
                // 24 units / 2000 / 2 = 0.006 units, 0.024 device pixels here: that makes the
                // inner test stricter and needs a twentieth of a pixel on the outer one.
                let inner = 8.0 * s - std::f32::consts::FRAC_1_SQRT_2 - FLATTEN_TOLERANCE;
                let outer = 8.0 * s + std::f32::consts::FRAC_1_SQRT_2 + 0.05;
                if d <= inner && byte != 255 {
                    holes += 1;
                }
                assert!(
                    d < outer || byte == 0,
                    "ink at ({x}, {y}), {d:.2} from the curve"
                );
            }
        }
        assert_eq!(
            holes, 0,
            "pixels inside the set left uncovered (reversed: {back})"
        );
    }
}

/// **A circle of radius 30 at `1 w`, drawn anticlockwise and clockwise**: section 54's
/// 188.94 against 187.40, where the set is one annulus.
///
/// The path is the usual four cubics with `k = 4(√2 − 1)/3`, whose length is 188.522
/// (sampled at 400 001 points per quarter); the half-width, 0.5, is far below its radius
/// of curvature, so the set is the tube round it and its area is length × width exactly
/// (the tube formula) — 188.522, against the true circle's 60π = 188.496.
///
/// A chord that falls `δ` inside an arc of radius `R` is shorter than it by a fraction
/// `δ / 3R` to first order, and a thin ring loses length, not depth — its inner rim gains
/// what its outer loses — so the ink may fall short by `188.522 · δ / 90` with
/// `δ = FLATTEN_TOLERANCE / rung.s`: a sixteenth of a pixel at the 8× rung.
#[test]
fn a_thin_circle_is_one_annulus_drawn_either_way() {
    const AREA: f32 = 188.522;
    let k = 4.0 * (2f32.sqrt() - 1.0) / 3.0;
    let at = |x: f32, y: f32| Point::new(50.0 + 30.0 * x, 50.0 + 30.0 * y);
    let path = [
        Segment::MoveTo(at(1.0, 0.0)),
        Segment::CubicTo {
            c1: at(1.0, k),
            c2: at(k, 1.0),
            to: at(0.0, 1.0),
        },
        Segment::CubicTo {
            c1: at(-k, 1.0),
            c2: at(-1.0, k),
            to: at(-1.0, 0.0),
        },
        Segment::CubicTo {
            c1: at(-1.0, -k),
            c2: at(-k, -1.0),
            to: at(0.0, -1.0),
        },
        Segment::CubicTo {
            c1: at(k, -1.0),
            c2: at(1.0, -k),
            to: at(1.0, 0.0),
        },
        Segment::Close,
    ];
    let thin = stroke(1.0, LineCap::Round, LineJoin::Round, 10.0);
    for rung in both_ways(&path, thin, "the circle") {
        within(
            &rung,
            AREA,
            AREA * FLATTEN_TOLERANCE / rung.s / 90.0,
            "the circle",
        );
    }
}

/// **The same circle as a regular 64-gon**, where nothing is flattened and the set has a
/// closed form: under round joins a convex polygon's stroke is its outer parallel body
/// less its inner one, `(A + P·r + πr²) − N·tan(π/N)·(a − r)²` with apothem `a`, perimeter
/// `P = 2N·a·tan(π/N)` and `r` the half-width, which is `P·w + r²·(π − N·tan(π/N))`.
/// Every one of its 64 vertices is a join, and each join's pieces must add to the set,
/// neither cancelling (section 54) nor counting a rim pixel twice (ADR 1361).
#[test]
fn a_polygon_ring_is_its_closed_form_drawn_either_way() {
    const N: usize = 64;
    let (radius, w) = (30.0f64, 1.0f64);
    let r = w / 2.0;
    #[expect(clippy::cast_precision_loss)] // 64
    let n = N as f64;
    let tan = (std::f64::consts::PI / n).tan();
    let apothem = radius * (std::f64::consts::PI / n).cos();
    let perimeter = 2.0 * n * apothem * tan;
    #[expect(clippy::cast_possible_truncation)] // a page-sized area
    let area = (perimeter * w + r * r * (std::f64::consts::PI - n * tan)) as f32;
    let points: Vec<(f32, f32)> = (0..N)
        .map(|i| {
            #[expect(clippy::cast_precision_loss)]
            let a = std::f32::consts::TAU * i as f32 / N as f32;
            (50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
        })
        .collect();
    let thin = stroke(1.0, LineCap::Butt, LineJoin::Round, 10.0);
    // The joins' arcs are the only chords: 64 of them, turning 2π in all, at radius `r`.
    let arcs = std::f32::consts::TAU * 0.5;
    for rung in both_ways(&line_path(&points, true), thin, "the 64-gon") {
        within(&rung, area, arcs * FLATTEN_TOLERANCE / rung.s, "the 64-gon");
    }
}

/// The L the joins below are drawn on: two arms of 40 at a right angle, `8 w`, placed off
/// the pixel grid so that every edge of every piece splits pixels. Its two butt-ended
/// rectangles cover `2 · 40 · 8 − 4 · 4 = 624`, the square they share at the inner corner
/// counted once.
fn ell() -> Vec<Segment> {
    line_path(&[(20.3, 20.6), (60.3, 20.6), (60.3, 60.6)], false)
}

/// **Table 54's three joins at a right angle, each drawn both ways**: the miter adds the
/// outer corner's whole `4 × 4` square, the round join a quarter disc `π · 4² / 4`, the bevel
/// the half of the square its triangle is — 640, 636.566 and 632.
#[test]
fn each_join_adds_what_table_54_draws_either_way() {
    let quarter_disc = std::f32::consts::PI * 16.0 / 4.0;
    for (join, area, arc) in [
        (LineJoin::Miter, 640.0, 0.0),
        (
            LineJoin::Round,
            624.0 + quarter_disc,
            std::f32::consts::FRAC_PI_2 * 4.0,
        ),
        (LineJoin::Bevel, 632.0, 0.0),
    ] {
        let what = format!("{join:?}");
        for rung in both_ways(&ell(), stroke(8.0, LineCap::Butt, join, 10.0), &what) {
            within(&rung, area, arc * FLATTEN_TOLERANCE / rung.s, &what);
        }
    }
}

/// **§8.4.3.5's own example**, which gives 1.414 as the limit that converts miters to bevels
/// below 90 degrees, with the clause's rule: "When the limit is exceeded, the join is
/// converted from a miter to a bevel." At exactly 90° the ratio is `1 / sin 45° = 1.41421…`,
/// which exceeds 1.414 — so that limit draws the bevel's 632 — and does not exceed 1.415,
/// which draws the miter's 640.
#[test]
fn a_miter_past_its_limit_is_a_bevel_either_way() {
    for (limit, area) in [(1.414, 632.0), (1.415, 640.0)] {
        let what = format!("miter limit {limit}");
        let mitered = stroke(8.0, LineCap::Butt, LineJoin::Miter, limit);
        for rung in both_ways(&ell(), mitered, &what) {
            within(&rung, area, 0.0, &what);
        }
    }
}

/// **A path that turns straight back**, `20 40 m 60 40 l 30 40 l` at `8 w` with butt caps:
/// the points within 4 of it are the 40 × 8 rectangle and, round the vertex, a half disc —
/// `320 + 8π` under a round join. Under a miter the ratio is unbounded, so §8.4.3.5 makes it
/// a bevel, whose triangle at a reversal has no area: 320.
#[test]
fn a_reversal_is_a_half_disc_under_a_round_join() {
    let path = line_path(&[(20.0, 40.0), (60.0, 40.0), (30.0, 40.0)], false);
    let half_turn = std::f32::consts::PI * 4.0;
    let round = stroke(8.0, LineCap::Butt, LineJoin::Round, 10.0);
    for rung in both_ways(&path, round, "round reversal") {
        within(
            &rung,
            320.0 + 8.0 * std::f32::consts::PI,
            half_turn * FLATTEN_TOLERANCE / rung.s,
            "round reversal",
        );
    }
    let miter = stroke(8.0, LineCap::Butt, LineJoin::Miter, 10.0);
    for rung in both_ways(&path, miter, "mitred reversal") {
        within(&rung, 320.0, 0.0, "mitred reversal");
    }
}

/// **A dashed L with round caps and joins** (§8.4.3.6): "[t]he ends of each dash shall be
/// treated with the current line cap style, and corners within dashes shall be treated with
/// the current line join style". Dashing is the caller's (the module comment of
/// [`stroke`](crate::raster::stroke)); what arrives is the dashes as open subpaths, so the
/// fixture states them.
///
/// The path `10 20 m 50 20 l 50 60 l` under `[25 10] 0 d` is 80 long with its corner at 40:
/// on 0–25, off 25–35, on 35–60 (5 along the first arm, the corner, 20 along the second),
/// off 60–70, on 70–80. At `6 w` each dash is its length times 6 and a disc of radius 3 from
/// its two caps; the middle one loses the `3 × 3` square its arms share and gains its round
/// join's quarter disc. `25·6 + 25·6 − 9 + 10·6 + 3·9π + 9π/4 = 351 + 29.25π`.
#[test]
fn every_dash_has_its_caps_and_its_corner_either_way() {
    let dashes = [
        line_path(&[(10.0, 20.0), (35.0, 20.0)], false),
        line_path(&[(45.0, 20.0), (50.0, 20.0), (50.0, 40.0)], false),
        line_path(&[(50.0, 50.0), (50.0, 60.0)], false),
    ]
    .concat();
    let round = stroke(6.0, LineCap::Round, LineJoin::Round, 10.0);
    // Six half-turn caps and one quarter-turn join, at radius 3.
    let arcs = (6.0 * std::f32::consts::PI + std::f32::consts::FRAC_PI_2) * 3.0;
    for rung in both_ways(&dashes, round, "the dashes") {
        within(
            &rung,
            351.0 + 29.25 * std::f32::consts::PI,
            arcs * FLATTEN_TOLERANCE / rung.s,
            "the dashes",
        );
    }
}

/// A piece's signed area, the shoelace sum halved in `f64`.
fn signed_area(piece: &crate::raster::Polyline) -> f64 {
    let p = &piece.points;
    (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y)
        })
        .sum::<f64>()
        / 2.0
}

/// **The pieces add up to the set, each counted once and all wound one way** — asked of
/// the expansion itself, with no rasteriser in between, because rounding each pixel to a
/// byte hides an overlap of a fraction of a pixel in a total of hundreds.
///
/// Where the half-width is small beside the segments, ADR 1361's pieces tile: the signed
/// areas of the polygons [`stroke_polylines`] returns must then all have one sign and sum
/// to the set's area. The fixtures are the two whose area is exact under miter and bevel
/// joins — the L above (640 and 632), and the 64-gon of circumradius 30 under miters, whose
/// set is the outer 64-gon of apothem `a + r` less the inner one of `a − r`, which is
/// `4N·tan(π/N)·a·r = P·w`. A piece wound against the others (section 54), or two pieces
/// overlapping, moves the sum by the area involved.
#[test]
fn the_pieces_of_a_thin_stroke_tile_its_set() {
    const N: usize = 64;
    let check = |path: &[Segment], stroke: Stroke, want: f64, what: &str| {
        for drawn in [path.to_vec(), reversed(path)] {
            let pieces =
                stroke_polylines(&flatten_stroke(&drawn, scaled(1.0)), stroke, stroke.width);
            let areas: Vec<f64> = pieces.iter().map(signed_area).collect();
            let negative = areas.iter().filter(|a| **a < -1e-9).count();
            let positive = areas.iter().filter(|a| **a > 1e-9).count();
            assert!(
                negative == 0 || positive == 0,
                "{what}: {negative} pieces wound one way and {positive} the other"
            );
            let sum = areas.iter().sum::<f64>().abs();
            assert!(
                (sum - want).abs() < 1e-3,
                "{what}: the pieces sum to {sum:.5} against the set's {want:.5}"
            );
        }
    };
    check(
        &ell(),
        stroke(8.0, LineCap::Butt, LineJoin::Miter, 10.0),
        640.0,
        "mitred L",
    );
    check(
        &ell(),
        stroke(8.0, LineCap::Butt, LineJoin::Bevel, 10.0),
        632.0,
        "bevelled L",
    );
    // A path that turns straight back runs along its own rectangle: `10 20.3 m 50 20.3 l s`
    // and `10 20.3 m 50 20.3 l 30 20.3 l` are both the 40 × 3 rectangle, 120, under a miter
    // (§8.4.3.5 makes a reversal's miter a bevel, and a reversal's bevel has no area).
    let back = stroke(3.0, LineCap::Butt, LineJoin::Miter, 10.0);
    let out_and_back = line_path(&[(10.0, 20.3), (50.0, 20.3)], true);
    check(&out_and_back, back, 120.0, "out and back, closed");
    let part_way = line_path(&[(10.0, 20.3), (50.0, 20.3), (30.0, 20.3)], false);
    check(&part_way, back, 120.0, "out and part way back");

    #[expect(clippy::cast_precision_loss)] // 64
    let n = N as f64;
    let apothem = 30.0 * (std::f64::consts::PI / n).cos();
    let ring = 4.0 * n * (std::f64::consts::PI / n).tan() * apothem * 0.5;
    let points: Vec<(f32, f32)> = (0..N)
        .map(|i| {
            #[expect(clippy::cast_precision_loss)]
            let a = std::f32::consts::TAU * i as f32 / N as f32;
            (50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
        })
        .collect();
    let mitred = stroke(1.0, LineCap::Butt, LineJoin::Miter, 10.0);
    check(&line_path(&points, true), mitred, ring, "mitred 64-gon");
}

/// The L of arms 2 long at `8 w`, placed off the pixel grid: a bend far tighter than the
/// half-width, since the cut [`inner_cut`](crate::raster::stroke) would take, `4 · tan 45° =
/// 4` back along each arm, is past both arms' halves. Its pieces are the two rectangles
/// §8.4.3.2 sweeps, `A = [20.3, 22.3] × [16.6, 24.6]` and `B = [18.3, 26.3] × [20.6, 22.6]`,
/// and on the outer side of the turn Table 54's join — under a miter the square
/// `M = [22.3, 26.3] × [16.6, 20.6]`. `A` and `B` overlap in `[20.3, 22.3] × [20.6, 22.6]`,
/// and that square reaches the set's rim at three reflex corners, `(20.3, 20.6)`,
/// `(20.3, 22.6)` and `(22.3, 22.6)`, where the edge of one rectangle crosses the edge of
/// the other.
fn tight_ell() -> Vec<Segment> {
    line_path(&[(20.3, 20.6), (22.3, 20.6), (22.3, 22.6)], false)
}

/// The area of `[x0, x1] × [y0, y1]` inside another such rectangle.
fn overlap(a: [f64; 4], b: [f64; 4]) -> f64 {
    let w = (a[1].min(b[1]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[2].max(b[2])).max(0.0);
    w * h
}

/// **A bend tighter than the half-width counts its rim once, either way** (ADR 1375).
///
/// The set is `A ∪ B ∪ M` of [`tight_ell`], so its area is `16 + 16 − 4 + 16 = 44` and the
/// area of it inside any pixel is exactly `|P∩A| + |P∩B| + |P∩M| − |P∩A∩B|`, the three
/// rectangles meeting `M` only along edges. Every pixel at every rung, drawn both ways, is
/// held to that within one coverage step. A fill that integrates winding and clamps it
/// (trap 58) reads the overlap twice in the three pixels that hold a reflex corner, a
/// quarter of a pixel or more apart from the set's own coverage at each. The tiling re-cuts
/// every piece of the L, so the stroker vouches for it and the encoder keeps the integral
/// without asking (ADR 1421): that mask is held to the set too.
#[test]
fn a_tight_bend_counts_its_rim_once_either_way() {
    let mitred = stroke(8.0, LineCap::Butt, LineJoin::Miter, 10.0);
    let (a, b, m) = (
        [20.3, 22.3, 16.6, 24.6],
        [18.3, 26.3, 20.6, 22.6],
        [22.3, 26.3, 16.6, 20.6],
    );
    let both = [20.3, 22.3, 20.6, 22.6];
    for rung in both_ways(&tight_ell(), mitred, "the tight L") {
        within(&rung, 44.0, 0.0, "the tight L");
    }
    for s in RUNGS {
        for drawn in [tight_ell(), reversed(&tight_ell())] {
            let asked = mask(&drawn, mitred, s);
            let stroked = stroke_pieces(&flatten_stroke(&drawn, scaled(s)), mitred, 8.0 * s);
            assert!(stroked.tiles, "the tight L at {s}× is not vouched for");
            let kept = fill_mask_settled(
                &stroked.pieces,
                Rule::NonZero,
                (0, 0, asked.width, asked.height),
                true,
            );
            for mask in [asked, kept] {
                let scale = f64::from(s);
                for y in 0..mask.height {
                    for x in 0..mask.width {
                        let pixel = [
                            f64::from(x) / scale,
                            f64::from(x + 1) / scale,
                            f64::from(y) / scale,
                            f64::from(y + 1) / scale,
                        ];
                        let set = (overlap(pixel, a) + overlap(pixel, b) + overlap(pixel, m)
                            - overlap(pixel, both))
                            * scale
                            * scale;
                        let byte = f64::from(mask.coverage[(y * mask.width + x) as usize]);
                        assert!(
                            (byte - 255.0 * set).abs() <= 1.0,
                            "the tight L at {s}×, pixel ({x}, {y}): {byte} against the set's {:.2}",
                            255.0 * set
                        );
                    }
                }
            }
        }
    }
}

/// **The pieces of a tight bend tile its set** — the same question as
/// [`the_pieces_of_a_thin_stroke_tile_its_set`] asked where the rectangles cannot be cut at
/// their join: summed with no rasteriser, the pieces [`stroke_polylines`] returns for
/// [`tight_ell`] must all have one sign and meet the set's 44 under a miter and, with the
/// bevel's triangle `8` in place of the square, 36. Pieces that overlap sum to 48 and 40.
#[test]
fn the_pieces_of_a_tight_bend_tile_its_set() {
    for (join, want) in [(LineJoin::Miter, 44.0), (LineJoin::Bevel, 36.0)] {
        for drawn in [tight_ell(), reversed(&tight_ell())] {
            let pieces = stroke_polylines(
                &flatten_stroke(&drawn, scaled(1.0)),
                stroke(8.0, LineCap::Butt, join, 10.0),
                8.0,
            );
            let areas: Vec<f64> = pieces.iter().map(signed_area).collect();
            let negative = areas.iter().filter(|a| **a < -1e-9).count();
            let positive = areas.iter().filter(|a| **a > 1e-9).count();
            assert!(
                negative == 0 || positive == 0,
                "{join:?}: {negative} pieces wound one way and {positive} the other"
            );
            let sum = areas.iter().sum::<f64>().abs();
            assert!(
                (sum - want).abs() < 1e-3,
                "{join:?}: the pieces sum to {sum:.5} against the set's {want:.5}"
            );
        }
    }
}

/// Two subpaths drawn across each other at `6 w` under butt caps, off the pixel grid:
/// `20.3 50.6 m 80.3 50.6 l` and `50.3 20.6 m 50.3 80.6 l`. Each subpath's set is its own
/// rectangle, `H = [20.3, 80.3] × [47.6, 53.6]` and `V = [47.3, 53.3] × [20.6, 80.6]`, and
/// the two share the square `[47.3, 53.3] × [47.6, 53.6]`.
fn cross() -> (Vec<Segment>, [f64; 4], [f64; 4]) {
    let mut path = line_path(&[(20.3, 50.6), (80.3, 50.6)], false);
    path.extend(line_path(&[(50.3, 20.6), (50.3, 80.6)], false));
    (path, [20.3, 80.3, 47.6, 53.6], [47.3, 53.3, 20.6, 80.6])
}

/// The pieces' signed areas summed with no rasteriser: all of one sign but for rounding's
/// slivers, and their total.
fn pieces_sum(pieces: &[crate::raster::Polyline], what: &str) -> f64 {
    let areas: Vec<f64> = pieces.iter().map(signed_area).collect();
    // A fragment cut between two crossing points that round to nearly one `f32` point can
    // hold an area of that rounding's order, of either sign: a sliver of 1e-8 against a
    // piece's tens. Its sign says nothing about the winding; its area still counts.
    let negative = areas.iter().filter(|a| **a < -1e-6).count();
    let positive = areas.iter().filter(|a| **a > 1e-6).count();
    assert!(
        negative == 0 || positive == 0,
        "{what}: {negative} pieces wound one way and {positive} the other"
    );
    areas.iter().sum::<f64>().abs()
}

/// **Two subpaths are one set** (ADR 1431). §8.4.3.2 paints "all points whose perpendicular
/// distance from the path in user space is less than or equal to half the line width", and
/// the path is every subpath of it, so where two subpaths' strokes overlap the overlap is
/// painted once.
///
/// The set of [`cross`] is `H ∪ V`, whose area in any pixel `P` is exactly
/// `|P∩H| + |P∩V| − |P∩H∩V|`, and `360 + 360 − 36 = 684` in all. The pieces the stroker
/// returns must sum to 684, not 720; the stroker must vouch for them; and every pixel at every
/// rung, each subpath drawn either way and the two in either order, is held to the set within
/// one coverage step — with the fill's set question asked, and with the integral kept as the
/// encoder keeps it for a stroke the stroker vouches for. Vouched for without the tiling
/// across the two, the pieces sum to 720 and the rings below to 1 534.33 against 1 342.54.
#[test]
fn two_subpaths_that_cross_are_one_set() {
    let (path, h, v) = cross();
    let both = [
        h[0].max(v[0]),
        h[1].min(v[1]),
        h[2].max(v[2]),
        h[3].min(v[3]),
    ];
    let butt = stroke(6.0, LineCap::Butt, LineJoin::Miter, 10.0);
    let swapped = {
        let mut swapped = path[2..].to_vec();
        swapped.extend_from_slice(&path[..2]);
        swapped
    };
    for drawn in [path.clone(), reversed(&path), swapped] {
        let at_one = stroke_pieces(&flatten_stroke(&drawn, scaled(1.0)), butt, 6.0);
        let sum = pieces_sum(&at_one.pieces, "the cross");
        assert!(
            (sum - 684.0).abs() < 1e-3,
            "the cross: the pieces sum to {sum:.5} against the set's 684"
        );
        for s in RUNGS {
            let asked = mask(&drawn, butt, s);
            let stroked = stroke_pieces(&flatten_stroke(&drawn, scaled(s)), butt, 6.0 * s);
            assert!(stroked.tiles, "the cross at {s}× is not vouched for");
            let kept = fill_mask_settled(
                &stroked.pieces,
                Rule::NonZero,
                (0, 0, asked.width, asked.height),
                true,
            );
            let scale = f64::from(s);
            for mask in [asked, kept] {
                for y in 0..mask.height {
                    for x in 0..mask.width {
                        let pixel = [
                            f64::from(x) / scale,
                            f64::from(x + 1) / scale,
                            f64::from(y) / scale,
                            f64::from(y + 1) / scale,
                        ];
                        let set = (overlap(pixel, h) + overlap(pixel, v) - overlap(pixel, both))
                            * scale
                            * scale;
                        let byte = f64::from(mask.coverage[(y * mask.width + x) as usize]);
                        assert!(
                            (byte - 255.0 * set).abs() <= 1.0,
                            "the cross at {s}×, pixel ({x}, {y}): {byte} against the set's {:.2}",
                            255.0 * set
                        );
                    }
                }
            }
        }
    }
}

/// **Two concentric rings whose strokes overlap are one ring** (ADR 1431): regular 64-gons
/// about `(50.3, 50.6)` of apothems `a₁ = 29` and `a₂ = 32`, each its own closed subpath, at
/// `4 w` under miters (§8.4.3.5's ratio at each corner is `1 / cos(π/64)`, far under the limit
/// of 10). A convex polygon stroked under miters is its outer parallel polygon, of apothem
/// `a + r`, less its inner one, of apothem `a − r`; the two rings are 3 apart and each is 4
/// wide, so between them nothing is left out and the set is the polygon of apothem `a₂ + r`
/// less the polygon of apothem `a₁ − r`, `N·tan(π/N)·((a₂ + r)² − (a₁ − r)²)`. Nothing is
/// flattened, so that is exact; in each pixel the set is the outer polygon's area there less
/// the inner one's ([`area_in_pixel`](super::curve_join::area_in_pixel)), which every pixel is
/// held to at 1× and 2× — asked and kept — and the pieces' sum is held to the total.
#[test]
fn two_concentric_rings_whose_strokes_overlap_are_one_ring() {
    const N: usize = 64;
    let (centre, r) = ((50.3_f64, 50.6_f64), 2.0_f64);
    #[expect(clippy::cast_precision_loss)] // 64
    let n = N as f64;
    let (tan, cos) = (
        (std::f64::consts::PI / n).tan(),
        (std::f64::consts::PI / n).cos(),
    );
    let polygon = |apothem: f64| -> Vec<(f64, f64)> {
        (0..N)
            .map(|i| {
                #[expect(clippy::cast_precision_loss)] // below 64
                let angle = std::f64::consts::TAU * i as f64 / n;
                let radius = apothem / cos;
                (
                    centre.0 + radius * angle.cos(),
                    centre.1 + radius * angle.sin(),
                )
            })
            .collect()
    };
    #[expect(clippy::cast_possible_truncation)] // page coordinates, well inside `f32`
    let ring = |apothem: f64| -> Vec<(f32, f32)> {
        polygon(apothem)
            .into_iter()
            .map(|(x, y)| (x as f32, y as f32))
            .collect()
    };
    let mut path = line_path(&ring(29.0), true);
    path.extend(line_path(&ring(32.0), true));
    let mitred = stroke(4.0, LineCap::Butt, LineJoin::Miter, 10.0);
    let want = n * tan * ((32.0 + r).powi(2) - (29.0 - r).powi(2));
    for drawn in [path.clone(), reversed(&path)] {
        let at_one = stroke_pieces(&flatten_stroke(&drawn, scaled(1.0)), mitred, 4.0);
        let sum = pieces_sum(&at_one.pieces, "the rings");
        assert!(
            (sum - want).abs() < 1e-2,
            "the rings: the pieces sum to {sum:.5} against the set's {want:.5}"
        );
        for s in [1.0_f32, 2.0] {
            let scale = f64::from(s);
            let scaled_polygon = |apothem: f64| -> Vec<(f64, f64)> {
                polygon(apothem)
                    .into_iter()
                    .map(|(x, y)| (x * scale, y * scale))
                    .collect()
            };
            let (outer, inner) = (scaled_polygon(32.0 + r), scaled_polygon(29.0 - r));
            let asked = mask(&drawn, mitred, s);
            let stroked = stroke_pieces(&flatten_stroke(&drawn, scaled(s)), mitred, 4.0 * s);
            assert!(stroked.tiles, "the rings at {s}× are not vouched for");
            let kept = fill_mask_settled(
                &stroked.pieces,
                Rule::NonZero,
                (0, 0, asked.width, asked.height),
                true,
            );
            for mask in [asked, kept] {
                for y in 0..mask.height {
                    for x in 0..mask.width {
                        let (column, row) = (f64::from(x), f64::from(y));
                        let set = super::curve_join::area_in_pixel(&outer, column, row)
                            - super::curve_join::area_in_pixel(&inner, column, row);
                        let byte = f64::from(mask.coverage[(y * mask.width + x) as usize]);
                        assert!(
                            (byte - 255.0 * set).abs() <= 1.0,
                            "the rings at {s}×, pixel ({x}, {y}): {byte} against the set's {:.2}",
                            255.0 * set
                        );
                    }
                }
            }
        }
    }
}

/// **A round-capped dot is its disc, to within the flatness bound's own ceiling.**
///
/// A subpath of length `ε = 1/64` at `4 w` with round caps is, by Table 53, a disc of radius
/// 2 split by a `4 × ε` rectangle: `4π + 4ε`. §10.7.2 bounds how far a chord may fall inside
/// its arc, and at a radius of two device pixels that bound, a quarter pixel, would admit a
/// hexagon — the inscribed polygon §10.7.2's NOTE 2 says the tolerance is not for. ADR 0044's
/// relative bound is this tree's reading of that note: at least sixteen chords a turn, so a
/// disc may fall short of its area by `1 − (8/π)·sin(π/8) = 2.55%` and no more. Held at 1×
/// (radius 2 device pixels) and 4× (radius 8), both ways, since an arc's step at a small
/// radius is exactly where the distance bound alone is too loose (ADR 1375).
#[test]
fn a_round_dot_is_its_disc_within_the_relative_bound() {
    let eps = 1.0 / 64.0;
    let path = line_path(&[(50.3, 50.6), (50.3 + eps, 50.6)], false);
    let dot = stroke(4.0, LineCap::Round, LineJoin::Round, 10.0);
    let disc = 4.0 * std::f32::consts::PI + 4.0 * eps;
    let ceiling = 1.0 - (8.0 / std::f32::consts::PI) * (std::f32::consts::PI / 8.0).sin();
    // The rungs are the exact constants of `RUNGS`.
    #[expect(clippy::float_cmp)]
    for rung in both_ways(&path, dot, "the dot")
        .into_iter()
        .filter(|rung| rung.s == 1.0 || rung.s == 4.0)
    {
        within(&rung, disc, disc * ceiling, "the dot");
    }
}
