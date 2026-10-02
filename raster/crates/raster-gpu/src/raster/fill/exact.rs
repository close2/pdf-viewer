//! The area of the set a fill rule declares inside, in the pixels that need it (ADR 1389).
//!
//! ISO 32000-2 §8.5.3.3.2 decides insideness point by point:
//!
//! > Starting with a count of 0, the rule adds 1 each time a path segment crosses the ray
//! > from left to right and subtracts 1 each time a segment crosses from right to left.
//! > After counting all the crossings, if the result is 0, the point is outside the path;
//! > otherwise, it is inside.
//!
//! and §8.5.3.3.3 by parity:
//!
//! > If this number is odd, the point is inside; if even, the point is outside.
//!
//! §10.7.4 scan-converts only once "all 'insideness' computations have been performed",
//! so a pixel is covered by the area of the inside set within it. This module computes
//! that area for one pixel from the edges that pass through it and nothing else:
//!
//! 1. **Bands.** The pixel's row is cut at every height where one of those edges starts,
//!    ends or crosses another, so inside a band every edge spans it and their
//!    left-to-right order is fixed; the winding is constant between two neighbours.
//! 2. **The winding up to a constant.** Down a vertical line through the pixel that no
//!    vertex lies on, the winding changes only where an edge crosses the line, and every
//!    such edge passes through the pixel. So the winding in every band is known up to one
//!    integer `K`, the same for the whole pixel.
//! 3. **The constant from the integral.** The fill's own accumulation already holds the
//!    pixel's average winding, and it is `K` plus the average of the relative winding —
//!    so `K` is their difference, rounded, and a difference that is not near a whole
//!    number declines the pixel rather than guessing.
//! 4. **The set.** Walking each band from the left with the winding now absolute, the
//!    inside intervals are bounded by the edges where the rule changes its answer; those
//!    boundaries are deposited by the fill's own trapezoid arithmetic
//!    ([`deposit_slab`](super::deposit_slab)) into a one-pixel window, a left boundary
//!    adding one and a right one taking it away, so what the pixel holds is the inside
//!    area and never more than the pixel.

use super::overlap::Complex;
use super::{Polyline, Real, Rule, deposit_slab, local_edge};

/// Edges through one pixel past which it keeps its integral: the band construction is
/// quadratic in them. A pixel crossed by more than this many edges of one path is past
/// anything the corpus draws; what it then shows is the integral, which is what every
/// pixel showed before ADR 1389.
const MAX_LOCAL: usize = 32;

/// How far the pixel's integral may sit from a whole winding before the pixel is declined:
/// the accumulation's own rounding is several orders below it.
const WHOLE: f32 = 0.25;

/// One edge through the pixel, in the region's coordinates, as the fill closed it.
#[derive(Debug, Clone, Copy)]
struct Local<R> {
    x0: R,
    y0: R,
    x1: R,
    y1: R,
}

// Floating-point arithmetic throughout, which cannot wrap; named because the type is generic.
#[expect(clippy::arithmetic_side_effects)]
impl<R: Real> Local<R> {
    /// `+1` for an edge running down the rows, `−1` for one running up, `0` flat.
    fn dir(self) -> R {
        if self.y1 > self.y0 {
            R::ONE
        } else if self.y1 < self.y0 {
            -R::ONE
        } else {
            R::ZERO
        }
    }

    /// The edge's `x` at height `y`; only asked of an edge that is not flat.
    fn x_at(self, y: R) -> R {
        self.x0 + (y - self.y0) * ((self.x1 - self.x0) / (self.y1 - self.y0))
    }

    /// Whether the edge spans the whole of `ya .. yb` — exactly: a flat edge spans no height
    /// at all.
    fn spans(self, ya: R, yb: R) -> bool {
        self.y0.min(self.y1) <= ya && self.y0.max(self.y1) >= yb && self.y0 != self.y1
    }
}

/// Rewrite the complex pixels of `coverage` with their set's area under `rule`.
///
/// `averages` is the accumulation grid after its prefix sum — each pixel's average
/// winding at `row · (width + 1) + column` — and `size` the region's `(width, height)`. The
/// area is computed in the accumulation's own precision (ADR 1491).
#[expect(clippy::arithmetic_side_effects)] // indices inside the region
pub(super) fn correct<R: Real>(
    complex: &Complex,
    polylines: &[Polyline],
    rule: Rule,
    origin: (i32, i32),
    size: (usize, usize),
    averages: &[R],
    coverage: &mut [u8],
) {
    let (w, _) = size;
    let mut local = Vec::new();
    let mut work = Work::default();
    for run in &complex.runs {
        local.clear();
        local.extend(
            complex.edges[run.edges.clone()]
                .iter()
                .filter_map(|&(s, e)| {
                    let polyline = polylines.get(s as usize)?;
                    let (x0, y0, x1, y1) = local_edge(polyline, e as usize, origin.0, origin.1);
                    Some(Local { x0, y0, x1, y1 })
                }),
        );
        if local.len() > MAX_LOCAL {
            continue;
        }
        let row = run.row as usize;
        for x in run.from as usize..=run.to as usize {
            let average = averages[row * (w + 1) + x];
            if let Some(area) = pixel_area(&local, (x, row), average, rule, &mut work) {
                coverage[row * w + x] = (area.clamp(R::ZERO, R::ONE) * R::LEVELS).round().byte();
            }
        }
    }
}

/// Buffers every pixel of one fill reuses.
#[derive(Debug, Default)]
struct Work<R> {
    heights: Vec<R>,
    turns: Vec<(R, R)>,
    active: Vec<(R, R, R)>,
}

/// The inside area of pixel `(i, j)`, whose average winding is `average`, from the edges
/// through it; `None` where the edges and the average disagree about the winding.
#[expect(clippy::arithmetic_side_effects)]
fn pixel_area<R: Real>(
    local: &[Local<R>],
    (i, j): (usize, usize),
    average: R,
    rule: Rule,
    work: &mut Work<R>,
) -> Option<R> {
    let (fi, fj) = (R::of_count(i), R::of_count(j));
    let xi = reference_line(local, fi);
    let Work {
        heights,
        turns,
        active,
    } = work;
    // Step 2's turns: where each edge crosses `x = xi` inside the row, and what crossing
    // it downwards does to the winding — an edge heading `+x` takes one away, whatever
    // its slope, flat edges included (the ray may be taken in any direction).
    turns.clear();
    for e in local {
        if (e.x0 - xi) * (e.x1 - xi) < R::ZERO {
            let y = e.y0 + (xi - e.x0) * (e.y1 - e.y0) / (e.x1 - e.x0);
            if y > fj && y < fj + R::ONE {
                turns.push((y, if e.x1 > e.x0 { -R::ONE } else { R::ONE }));
            }
        }
    }
    // Step 1's bands.
    heights.clear();
    heights.extend([fj, fj + R::ONE]);
    for (k, a) in local.iter().enumerate() {
        for y in [a.y0, a.y1] {
            if y > fj && y < fj + R::ONE {
                heights.push(y);
            }
        }
        if a.dir() == R::ZERO {
            continue;
        }
        for b in local[k + 1..].iter().filter(|b| b.dir() != R::ZERO) {
            if let Some(y) = crossing(*a, *b).filter(|&y| y > fj && y < fj + R::ONE) {
                heights.push(y);
            }
        }
    }
    heights.sort_unstable_by(R::total_cmp);
    heights.dedup();
    // Steps 2 and 3: the relative winding's integral, then the constant.
    let mut relative = [R::ZERO; 2];
    for pair in heights.windows(2) {
        let (ya, yb) = (pair[0], pair[1]);
        let Some(far_left) = band(local, turns, (ya, yb), xi, active) else {
            continue;
        };
        relative[0] += far_left * (yb - ya);
        for &(xa, xb, dir) in active.iter() {
            deposit_slab(&mut relative, R::ONE, dir, xa - fi, ya, xb - fi, yb);
        }
    }
    let constant = (average - relative[0]).round();
    if (average - relative[0] - constant).abs() > R::of(WHOLE) {
        return None;
    }
    // Step 4: the set.
    let mut inside_area = [R::ZERO; 2];
    for pair in heights.windows(2) {
        let (ya, yb) = (pair[0], pair[1]);
        let Some(far_left) = band(local, turns, (ya, yb), xi, active) else {
            continue;
        };
        let mut winding = constant + far_left;
        if inside(rule, winding) {
            inside_area[0] += yb - ya;
        }
        for &(xa, xb, dir) in active.iter() {
            let before = inside(rule, winding);
            winding += dir;
            let after = inside(rule, winding);
            if before != after {
                let step = if after { R::ONE } else { -R::ONE };
                deposit_slab(&mut inside_area, R::ONE, step, xa - fi, ya, xb - fi, yb);
            }
        }
    }
    Some(inside_area[0])
}

/// The edges spanning band `ya .. yb` into `active`, left to right, and the relative
/// winding to the left of all of them; `None` for a band of no height.
#[expect(clippy::arithmetic_side_effects)] // floating-point, which cannot wrap
fn band<R: Real>(
    local: &[Local<R>],
    turns: &[(R, R)],
    (ya, yb): (R, R),
    xi: R,
    active: &mut Vec<(R, R, R)>,
) -> Option<R> {
    if yb <= ya {
        return None;
    }
    let middle = R::HALF * (ya + yb);
    active.clear();
    active.extend(
        local
            .iter()
            .filter(|e| e.spans(ya, yb))
            .map(|e| (e.x_at(ya), e.x_at(yb), e.dir())),
    );
    active.sort_unstable_by(|a, b| (a.0 + a.1).total_cmp(&(b.0 + b.1)));
    // The winding on the line at this band, relative to the row's top; then back across
    // the edges that lie between the line and the band's left.
    let on_line: R = turns.iter().filter(|t| t.0 < middle).map(|t| t.1).sum();
    let behind: R = active
        .iter()
        .filter(|a| R::HALF * (a.0 + a.1) < xi)
        .map(|a| a.2)
        .sum();
    Some(on_line - behind)
}

/// A vertical line through the pixel that no edge's end point lies on, so that the
/// winding changes along it only where an edge crosses it.
///
/// The comparisons are exact: a vertex on the line is what is avoided.
#[expect(clippy::arithmetic_side_effects)] // floating-point, which cannot wrap
fn reference_line<R: Real>(local: &[Local<R>], fi: R) -> R {
    let clear = |x: R| local.iter().all(|e| e.x0 != x && e.x1 != x);
    [0.5_f32, 0.375, 0.625, 0.3125, 0.6875, 0.4375, 0.5625]
        .into_iter()
        .map(|f| fi + R::of(f))
        .find(|&x| clear(x))
        .unwrap_or(fi + R::of(0.531_25))
}

/// The height at which two edges that are not flat cross, where they do.
#[expect(clippy::arithmetic_side_effects)] // floating-point, which cannot wrap
fn crossing<R: Real>(a: Local<R>, b: Local<R>) -> Option<R> {
    let (ya, yb) = (
        a.y0.min(a.y1).max(b.y0.min(b.y1)),
        a.y0.max(a.y1).min(b.y0.max(b.y1)),
    );
    if yb <= ya {
        return None;
    }
    let (top, bottom) = (b.x_at(ya) - a.x_at(ya), b.x_at(yb) - a.x_at(yb));
    if top * bottom >= R::ZERO {
        return None;
    }
    let y = ya + (yb - ya) * (top / (top - bottom));
    y.is_finite().then_some(y)
}

/// §8.5.3.3's two rules, on a whole winding number.
#[expect(clippy::arithmetic_side_effects)] // floating-point, which cannot wrap
fn inside<R: Real>(rule: Rule, winding: R) -> bool {
    match rule {
        Rule::NonZero => winding != R::ZERO,
        Rule::EvenOdd => winding.rem_euclid(R::ONE + R::ONE) != R::ZERO,
    }
}
