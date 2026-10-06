//! A fill that is one convex polygon, and the area such fills hold inside a pixel — alone, and
//! intersected with one another (ADR 1582).
//!
//! ISO 32000-2 §10.7.4 makes a pixel's coverage the area of the set inside it, and a residue
//! clip's coverage, or the meet of a clipped mark with its clip, the area of an intersection of
//! sets. [`fill`](super::fill) and [`meet`](super::meet) answer that for any set: deposits and a
//! prefix sum, and bands cut wherever an edge starts, ends or crosses. **A set that is one closed
//! polyline turning one way and going round once is the intersection of the half-planes its edges
//! bound**, and so is the intersection of several such sets — so its area in a pixel is a convex
//! polygon's, the pixel cut by each edge in turn and measured by the shoelace. §8.5.3.3's two
//! rules agree on such a polygon, since every point inside it winds once and every point outside
//! winds nothing.
//!
//! The answer is the same area the general constructions compute, from the same `f32` points, in
//! `f64`: both are the exact area to far below a coverage level, and they can differ in a byte
//! only where that area lies within a few units in the last place of a rounding boundary.
//! [`Convex::of`] decides the shape **exactly or not at all** — a turn whose sign the arithmetic
//! cannot certify is a polygon this module declines — because a polygon cut by its own edges'
//! half-planes is the polygon only if it is convex: a reflex corner would be cut away.

use super::flatten::Polyline;

/// The relative error bound of a 2 × 2 determinant of `f64` differences, as Shewchuk's
/// `orient2d` states it: `(3 + 16ε)ε` with `ε = 2⁻⁵³`. A determinant larger than this times the
/// sum of its two products' magnitudes has the sign of the exact one.
const ORIENTATION_BOUND: f64 = (3.0 + 16.0 * f64::EPSILON / 2.0) * (f64::EPSILON / 2.0);

/// Which side of every edge a convex polyline's inside lies on: a polyline [`Convex::of`] has
/// established turns one way throughout and goes round once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Convex {
    /// The sign every turn's cross product takes, `+1` or `−1`: a point is inside an edge's
    /// half-plane where the edge's cross product with it, times this, is not negative.
    turn: f64,
}

impl Convex {
    /// The side `polylines` keep their inside on, where they are one closed polyline that is
    /// convex and goes round once, judged exactly; `None` for anything else — more than one
    /// subpath, fewer than three distinct points, every point on one line, a reflex corner, a
    /// turn back along an edge, a polygon going round more than once, or a turn whose sign the
    /// arithmetic cannot certify.
    ///
    /// Repeated consecutive points are edges of no length: they bound nothing and are passed
    /// over. A straight-on corner is allowed. "Goes round once" is read as
    /// [`flatten::convex`](super::flatten) reads it — each coordinate changing direction at most
    /// twice round the loop — which a polygon turning one way throughout meets exactly when its
    /// turns sum to one revolution.
    #[expect(clippy::arithmetic_side_effects)] // `(i + 1) % n` with `i < n`
    pub(crate) fn of(polylines: &[Polyline]) -> Option<Self> {
        let [polyline] = polylines else {
            return None;
        };
        let points = &polyline.points;
        let n = points.len();
        let mut walk = Walk::default();
        let mut first = None;
        for i in 0..n {
            let (p, q) = (points[i], points[(i + 1) % n]);
            let d = (
                f64::from(q.x) - f64::from(p.x),
                f64::from(q.y) - f64::from(p.y),
            );
            // Exact: a difference of two `f64`s is zero only where they are equal.
            if d.0 == 0.0 && d.1 == 0.0 {
                continue;
            }
            first.get_or_insert(d);
            walk.step(d)?;
        }
        // Round the corner back to the first step, so the last turn and the last change of
        // direction are counted too.
        walk.step(first?)?;
        let Walk {
            steps, turn, flips, ..
        } = walk;
        // `steps` counts the first step twice.
        (steps > 3 && turn != 0.0 && flips[0] <= 2 && flips[1] <= 2).then_some(Self { turn })
    }
}

/// The intersection of convex sets, as one convex polygon, to be measured pixel by pixel.
#[derive(Debug)]
pub(crate) struct ConvexMeet {
    polygon: Vec<(f64, f64)>,
}

impl ConvexMeet {
    /// The intersection of `sets`: the first set's corners, cut by every other set's edges in
    /// turn (Sutherland–Hodgman).
    ///
    /// Each cut keeps the part of the polygon on the inside of one edge's line, which for a
    /// convex set is the set's own half-plane, so what is left after every edge is the
    /// intersection. An edge of no length keeps everything.
    #[expect(clippy::arithmetic_side_effects)] // `(i + 1) % n` with `i < n`
    pub(crate) fn of(sets: &[(&Polyline, Convex)]) -> Self {
        let Some(((first, _), rest)) = sets.split_first() else {
            return Self {
                polygon: Vec::new(),
            };
        };
        let mut polygon: Vec<(f64, f64)> = first
            .points
            .iter()
            .map(|p| (f64::from(p.x), f64::from(p.y)))
            .collect();
        let mut spare = Vec::new();
        for (polyline, convex) in rest {
            let n = polyline.points.len();
            for i in 0..n {
                if polygon.is_empty() {
                    return Self { polygon };
                }
                let (a, b) = (polyline.points[i], polyline.points[(i + 1) % n]);
                let a = (f64::from(a.x), f64::from(a.y));
                let d = (f64::from(b.x) - a.0, f64::from(b.y) - a.1);
                if d.0 == 0.0 && d.1 == 0.0 {
                    continue;
                }
                let turn = convex.turn;
                cut(
                    &polygon,
                    |p| turn * (d.0 * (p.1 - a.1) - d.1 * (p.0 - a.0)),
                    &mut spare,
                );
                std::mem::swap(&mut polygon, &mut spare);
            }
        }
        Self { polygon }
    }

    /// The area, in `0 ..= 1`, of the intersection inside device pixel `(x, y)` —
    /// `[x, x + 1) × [y, y + 1)`, §10.7.4's pixel. `work` holds the cut pieces between calls.
    pub(crate) fn area_in_pixel(&self, x: i32, y: i32, work: &mut Work) -> f64 {
        let (xl, yt) = (f64::from(x), f64::from(y));
        strip(&self.polygon, Axis::Y, yt, &mut work.row, &mut work.spare);
        strip(&work.row, Axis::X, xl, &mut work.cell, &mut work.spare);
        shoelace(&work.cell, (xl, yt)).clamp(0.0, 1.0)
    }
}

/// The buffers [`ConvexMeet::area_in_pixel`] reuses across the pixels of one meet.
#[derive(Debug, Default)]
pub(crate) struct Work {
    row: Vec<(f64, f64)>,
    cell: Vec<(f64, f64)>,
    spare: Vec<(f64, f64)>,
}

/// What two consecutive steps of a polyline do at the corner between them, where the
/// arithmetic can say.
enum Turning {
    /// On along the same line, the same way.
    Straight,
    /// Back along the same line.
    Back,
    /// To one side: `+1` or `−1`, the sign of the cross product.
    Side(f64),
}

/// The turn from step `d` to step `e`, or `None` where the sign of their cross product is not
/// certain under [`ORIENTATION_BOUND`].
///
/// A cross product computed as exactly zero from two products each exactly zero is exactly
/// zero: a product of two `f64`s of `f32` origin cannot underflow, so it is zero only where a
/// factor is, and a difference is zero only where its operands are equal.
fn certain_turn(d: (f64, f64), e: (f64, f64)) -> Option<Turning> {
    let (left, right) = (d.0 * e.1, d.1 * e.0);
    let cross = left - right;
    let bound = ORIENTATION_BOUND * (left.abs() + right.abs());
    if cross.abs() > bound {
        return Some(Turning::Side(cross.signum()));
    }
    // Both products are exactly zero only where both steps run along one axis: a step with a
    // zero `x` has a non-zero `y`, so the other step's `x` is zero too, and the same the other
    // way round. Their signs along that axis say whether the second goes on or back.
    if left != 0.0 || right != 0.0 {
        return None;
    }
    let (now, then) = if d.0 == 0.0 { (d.1, e.1) } else { (d.0, e.0) };
    Some(if now.signum() == then.signum() {
        Turning::Straight
    } else {
        Turning::Back
    })
}

/// What [`Convex::of`] has seen of a polyline's steps so far: how many, the side every turn
/// took, and how often each coordinate's direction changed, passing over the steps that do not
/// move along it.
#[derive(Default)]
struct Walk {
    steps: usize,
    previous: Option<(f64, f64)>,
    turn: f64,
    signs: [f64; 2],
    flips: [usize; 2],
}

impl Walk {
    /// Take step `d`; `None` where the corner before it is not one a convex polygon turns.
    #[expect(clippy::float_cmp)] // signs are exactly `−1`, `0` or `+1`
    fn step(&mut self, d: (f64, f64)) -> Option<()> {
        if let Some(previous) = self.previous {
            match certain_turn(previous, d)? {
                Turning::Straight => {}
                Turning::Back => return None,
                Turning::Side(sign) => {
                    if self.turn != 0.0 && sign != self.turn {
                        return None;
                    }
                    self.turn = sign;
                }
            }
        }
        // The sign of an `f64` difference is exact, so these counts are.
        for (axis, value) in [d.0, d.1].into_iter().enumerate() {
            if value != 0.0 {
                let sign = value.signum();
                if self.signs[axis] != 0.0 && sign != self.signs[axis] {
                    self.flips[axis] = self.flips[axis].saturating_add(1);
                }
                self.signs[axis] = sign;
            }
        }
        self.previous = Some(d);
        self.steps = self.steps.saturating_add(1);
        Some(())
    }
}

/// The axis a cut is taken across.
#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

/// The convex `polygon` cut to the unit band `from ..= from + 1` across `axis`, into `out`,
/// with `spare` as the cut's scratch. The band's own sides are placed exactly: a corner made
/// on a side has that side's coordinate, not an interpolated one.
fn strip(
    polygon: &[(f64, f64)],
    axis: Axis,
    from: f64,
    out: &mut Vec<(f64, f64)>,
    spare: &mut Vec<(f64, f64)>,
) {
    let to = from + 1.0;
    match axis {
        Axis::X => {
            cut_at(polygon, |p| p.0 - from, (Axis::X, from), spare);
            cut_at(spare, |p| to - p.0, (Axis::X, to), out);
        }
        Axis::Y => {
            cut_at(polygon, |p| p.1 - from, (Axis::Y, from), spare);
            cut_at(spare, |p| to - p.1, (Axis::Y, to), out);
        }
    }
}

/// `polygon` cut to where `keep` is not negative, into `out`.
#[expect(clippy::arithmetic_side_effects)] // `(k + 1) % n` with `k < n`
fn cut(polygon: &[(f64, f64)], keep: impl Fn((f64, f64)) -> f64, out: &mut Vec<(f64, f64)>) {
    out.clear();
    let n = polygon.len();
    for (k, &here) in polygon.iter().enumerate() {
        let next = polygon[(k + 1) % n];
        let (sh, sn) = (keep(here), keep(next));
        if sh >= 0.0 {
            out.push(here);
        }
        if (sh >= 0.0) != (sn >= 0.0) {
            let t = sh / (sh - sn);
            out.push((
                here.0 + t * (next.0 - here.0),
                here.1 + t * (next.1 - here.1),
            ));
        }
    }
}

/// [`cut`] against an axis-aligned line, the corner made on it placed on it exactly.
#[expect(clippy::arithmetic_side_effects)] // `(k + 1) % n` with `k < n`
fn cut_at(
    polygon: &[(f64, f64)],
    keep: impl Fn((f64, f64)) -> f64,
    (axis, at): (Axis, f64),
    out: &mut Vec<(f64, f64)>,
) {
    out.clear();
    let n = polygon.len();
    for (k, &here) in polygon.iter().enumerate() {
        let next = polygon[(k + 1) % n];
        let (sh, sn) = (keep(here), keep(next));
        if sh >= 0.0 {
            out.push(here);
        }
        if (sh >= 0.0) != (sn >= 0.0) {
            let t = sh / (sh - sn);
            out.push(match axis {
                Axis::X => (at, here.1 + t * (next.1 - here.1)),
                Axis::Y => (here.0 + t * (next.0 - here.0), at),
            });
        }
    }
}

/// The area of `polygon`, measured from `origin` — a corner of the pixel it lies in, so the
/// products are of numbers below one and lose nothing to the page's coordinates.
#[expect(clippy::arithmetic_side_effects)] // `(k + 1) % n` with `k < n`
fn shoelace(polygon: &[(f64, f64)], (ox, oy): (f64, f64)) -> f64 {
    let n = polygon.len();
    let twice: f64 = (0..n)
        .map(|k| {
            let (a, b) = (polygon[k], polygon[(k + 1) % n]);
            (a.0 - ox) * (b.1 - oy) - (b.0 - ox) * (a.1 - oy)
        })
        .sum();
    twice.abs() * 0.5
}

#[cfg(test)]
mod tests;
