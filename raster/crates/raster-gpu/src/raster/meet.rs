//! The area of several sets' intersection inside one pixel, each set a fill of its own
//! polylines under its own rule (ADR 1467).
//!
//! ISO 32000-2 §10.7.4:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the
//! > set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! A residue clip and a mark are each rasterised to a coverage byte, and two bytes say how
//! much of a pixel each set holds and not where; where both are fractional the
//! intersection's area is a function of where their edges run. This module computes it
//! from the edges, for the pixels that need it and no others:
//!
//! 1. **Bands.** The pixel is cut at every height where an edge through it starts, ends,
//!    crosses the pixel's left or right side, or crosses another edge through it — so
//!    inside a band every edge through the pixel spans the band, stays on one side of
//!    each of the pixel's sides, and keeps its left-to-right place.
//! 2. **The winding entering from the left.** Along the pixel's left side the winding of a
//!    set changes only where an edge crosses that side, and every such crossing is a band
//!    boundary; so the winding at the band's middle height, counted along a ray from the
//!    left (§8.5.3.3.2's count, taken over the edges of that row, which are all the edges
//!    the ray can cross), holds for the whole band.
//! 3. **The walk.** Left to right across the band, each edge through the pixel changes its
//!    own set's winding; between two consecutive edges every set's answer is fixed, and
//!    where every set says inside, the strip between them is a trapezoid whose area is
//!    exact.
//!
//! Every quantity is `f64` from the `f32` points the fill itself reads, so the sets are
//! the fill's sets to the bit and the arithmetic is far below a coverage level.

use super::fill::Rule;
use super::flatten::Polyline;

/// One edge as the meet reads it: ends ordered top to bottom, and which way the path ran.
#[derive(Debug, Clone, Copy)]
struct Edge {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    /// `+1` for an edge that ran down the rows, `−1` for one that ran up.
    dir: i32,
}

impl Edge {
    /// The edge's `x` at height `y`; only asked of an edge that spans `y`.
    fn x_at(&self, y: f64) -> f64 {
        self.x0 + (y - self.y0) * ((self.x1 - self.x0) / (self.y1 - self.y0))
    }
}

/// Every edge of `polylines` that crosses a height in `first .. last`, closed as a fill
/// closes each subpath (§8.5.3.1), its ends ordered top to bottom. A horizontal edge
/// crosses no horizontal ray and bounds no band's area, so it is left out.
#[expect(clippy::arithmetic_side_effects)] // `(i + 1) % n` with `i < n`
fn edges_between(polylines: &[Polyline], first: f64, last: f64) -> Vec<Edge> {
    let mut edges = Vec::new();
    for polyline in polylines {
        let count = polyline.points.len();
        for i in 0..count {
            let (from, to) = (polyline.points[i], polyline.points[(i + 1) % count]);
            let (a, b) = (
                (f64::from(from.x), f64::from(from.y)),
                (f64::from(to.x), f64::from(to.y)),
            );
            let ((top, bottom), dir) = match a.1.total_cmp(&b.1) {
                std::cmp::Ordering::Less => ((a, b), 1),
                std::cmp::Ordering::Greater => ((b, a), -1),
                std::cmp::Ordering::Equal => continue,
            };
            let edge = Edge {
                x0: top.0,
                y0: top.1,
                x1: bottom.0,
                y1: bottom.1,
                dir,
            };
            if edge.y1 > first && edge.y0 < last && edge.x0.is_finite() && edge.x1.is_finite() {
                edges.push(edge);
            }
        }
    }
    edges
}

/// The part of row `y .. y + 1` an edge spans.
fn row_span(edge: &Edge, y: f64) -> (f64, f64) {
    (edge.y0.max(y), edge.y1.min(y + 1.0))
}

/// One set's edges bucketed by device row: every edge that reaches into a row is listed
/// under it, so a ray along that row meets no edge the bucket does not hold.
///
/// Each row's entries are sorted by where their edge starts across the row, and beside them
/// is the furthest any entry so far reaches: so a pixel finds the run of edges that can touch
/// it by two searches, and the edges wholly left of that run add their winding from a prefix
/// sum rather than one at a time — a row of a clip holding thousands of glyph edges is not
/// walked again for every pixel a mark cuts in it.
#[derive(Debug)]
pub(crate) struct RowEdges {
    rule: Rule,
    /// The first row bucketed.
    top: i32,
    edges: Vec<Edge>,
    /// `starts[r] .. starts[r + 1]` indexes `entries` and `reach` for row `top + r`.
    starts: Vec<u32>,
    entries: Vec<Entry>,
    /// Per entry, the furthest right any entry of its row up to and including it reaches.
    reach: Vec<f64>,
    /// `starts[r] + r ..= starts[r + 1] + r` is row `r`'s prefix sum of the windings of its
    /// entries that span the whole row.
    prefix: Vec<i32>,
    /// `partial_starts[r] .. partial_starts[r + 1]` lists, by position in its row, the entries
    /// of row `r` whose edge spans only part of it.
    partial_starts: Vec<u32>,
    partials: Vec<u32>,
}

/// One edge as one row lists it: which edge, and where it starts across the row.
#[derive(Debug, Clone, Copy)]
struct Entry {
    edge: u32,
    lo: f64,
}

/// One row of a [`RowEdges`].
struct Row<'a> {
    edges: &'a [Edge],
    entries: &'a [Entry],
    reach: &'a [f64],
    prefix: &'a [i32],
    partials: &'a [u32],
}

impl RowEdges {
    /// The edges of `polylines`, filled under `rule`, over the rows `top .. top + rows`.
    ///
    /// Every subpath is closed, as a fill closes it (§8.5.3.1). `None` where the buckets
    /// would hold more than `limit` entries — a page-tall clip of many long edges — which
    /// the caller answers by keeping its bound rather than by allocating past it
    /// (principle 3).
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // rows are clamped
    // into `0 .. rows`, a tile's height, and an entry count below `u32::MAX` is a tile's
    #[expect(clippy::arithmetic_side_effects)] // indices inside the row range
    pub(crate) fn of(
        polylines: &[Polyline],
        rule: Rule,
        top: i32,
        rows: u32,
        limit: usize,
    ) -> Option<Self> {
        let first = f64::from(top);
        let last = first + f64::from(rows);
        let edges = edges_between(polylines, first, last);
        let span = |edge: &Edge| -> (u32, u32) {
            let from = (edge.y0.max(first) - first).floor() as u32;
            let to = ((edge.y1.min(last) - first).ceil() as u32).min(rows);
            (from, to)
        };
        let mut counts = vec![0_u32; rows as usize + 1];
        let mut total = 0_usize;
        for edge in &edges {
            let (from, to) = span(edge);
            total = total.saturating_add(to.saturating_sub(from) as usize);
            if total > limit {
                return None;
            }
            for count in &mut counts[from as usize..to as usize] {
                *count += 1;
            }
        }
        let mut starts = Vec::with_capacity(rows as usize + 1);
        let mut running = 0_u32;
        for count in &counts {
            starts.push(running);
            running += count;
        }
        let mut fill = starts.clone();
        let mut entries = vec![Entry { edge: 0, lo: 0.0 }; total];
        for (index, edge) in edges.iter().enumerate() {
            let (from, to) = span(edge);
            for r in from..to {
                let (ya, yb) = row_span(edge, first + f64::from(r));
                let slot = &mut fill[r as usize];
                entries[*slot as usize] = Entry {
                    edge: index as u32,
                    lo: edge.x_at(ya).min(edge.x_at(yb)),
                };
                *slot += 1;
            }
        }
        let mut sets = Self {
            rule,
            top,
            edges,
            starts,
            entries,
            reach: Vec::new(),
            prefix: Vec::new(),
            partial_starts: Vec::new(),
            partials: Vec::new(),
        };
        sets.order_rows(first, rows);
        Some(sets)
    }

    /// Sort each row by where its edges start across it, and record beside the entries the
    /// furthest reach so far, the prefix sums of the whole-row windings, and which entries
    /// span only part of their row.
    #[expect(clippy::cast_possible_truncation)] // an entry's place in its row is below the
    // row's length, a `u32`
    #[expect(clippy::arithmetic_side_effects)] // indices inside the row range, and windings
    // that count edges of one row
    fn order_rows(&mut self, first: f64, rows: u32) {
        let total = self.entries.len();
        self.reach = vec![0.0; total];
        self.prefix = Vec::with_capacity(total + rows as usize);
        self.partial_starts = Vec::with_capacity(rows as usize + 1);
        for r in 0..rows {
            let (from, to) = (
                self.starts[r as usize] as usize,
                self.starts[r as usize + 1] as usize,
            );
            let row = &mut self.entries[from..to];
            row.sort_by(|a, b| a.lo.total_cmp(&b.lo));
            let y = first + f64::from(r);
            let (mut furthest, mut sum) = (f64::NEG_INFINITY, 0_i32);
            self.prefix.push(0);
            self.partial_starts.push(self.partials.len() as u32);
            for (k, entry) in row.iter().enumerate() {
                let edge = &self.edges[entry.edge as usize];
                let (ya, yb) = row_span(edge, y);
                furthest = furthest.max(edge.x_at(ya).max(edge.x_at(yb)));
                self.reach[from + k] = furthest;
                #[expect(clippy::float_cmp)] // exact: the edge spans the row or it does not
                if ya == y && yb == y + 1.0 {
                    sum += edge.dir;
                } else {
                    self.partials.push(k as u32);
                }
                self.prefix.push(sum);
            }
        }
        self.partial_starts.push(self.partials.len() as u32);
    }

    /// The bytes these buckets hold, for the caller's budget.
    pub(crate) fn bytes(&self) -> u64 {
        let words = self
            .starts
            .len()
            .saturating_add(self.prefix.len())
            .saturating_add(self.partial_starts.len())
            .saturating_add(self.partials.len())
            .saturating_mul(size_of::<u32>());
        let wide = self
            .edges
            .len()
            .saturating_mul(size_of::<Edge>())
            .saturating_add(self.entries.len().saturating_mul(size_of::<Entry>()))
            .saturating_add(self.reach.len().saturating_mul(size_of::<f64>()));
        words.saturating_add(wide) as u64
    }

    /// Device row `y`, empty outside the bucketed rows.
    #[expect(clippy::arithmetic_side_effects)] // `r + 1` is a row of `starts`
    fn row(&self, y: i32) -> Row<'_> {
        let bounds = |r: usize| -> Option<(usize, usize, usize, usize)> {
            Some((
                *self.starts.get(r)? as usize,
                *self.starts.get(r + 1)? as usize,
                *self.partial_starts.get(r)? as usize,
                *self.partial_starts.get(r + 1)? as usize,
            ))
        };
        match usize::try_from(y.saturating_sub(self.top))
            .ok()
            .and_then(|r| Some((r, bounds(r)?)))
        {
            Some((r, (from, to, p_from, p_to))) => Row {
                edges: &self.edges,
                entries: &self.entries[from..to],
                reach: &self.reach[from..to],
                prefix: &self.prefix[from + r..=to + r],
                partials: &self.partials[p_from..p_to],
            },
            None => Row {
                edges: &self.edges,
                entries: &[],
                reach: &[],
                prefix: &[0],
                partials: &[],
            },
        }
    }

    /// Whether this set's rule calls a point of winding `w` inside (§8.5.3.3).
    fn inside(&self, w: i32) -> bool {
        match self.rule {
            Rule::NonZero => w != 0,
            Rule::EvenOdd => w & 1 == 1,
        }
    }
}

/// One edge through the pixel, cut to the pixel's rows, and the set it belongs to.
#[derive(Debug, Clone, Copy)]
struct Through {
    ya: f64,
    yb: f64,
    xa: f64,
    xb: f64,
    set: usize,
    dir: i32,
}

impl Through {
    /// The edge's `x` at height `y` inside its span.
    fn x_at(&self, y: f64) -> f64 {
        self.xa + (y - self.ya) * ((self.xb - self.xa) / (self.yb - self.ya))
    }
}

/// An edge left of the pixel that spans only part of its row: it adds its winding to a ray
/// at the heights it spans.
#[derive(Debug, Clone, Copy)]
struct Partial {
    ya: f64,
    yb: f64,
    set: usize,
    dir: i32,
}

/// A band's boundary: an edge through the pixel, its `x` at the band's middle and at both
/// ends, and the set whose winding it changes.
#[derive(Debug, Clone, Copy)]
struct Boundary {
    xm: f64,
    xa: f64,
    xb: f64,
    set: usize,
    dir: i32,
}

/// Buffers one meet reuses across its pixels.
#[derive(Debug, Default)]
pub(crate) struct Work {
    through: Vec<Through>,
    partial: Vec<Partial>,
    cuts: Vec<f64>,
    boundaries: Vec<Boundary>,
    /// Per set, the winding of the edges left of the pixel that span its whole row.
    left: Vec<i32>,
    windings: Vec<i32>,
}

/// The area, in `0 ..= 1`, of the intersection of every set in `sets` inside device pixel
/// `(x, y)` — `[x, x + 1) × [y, y + 1)`, §10.7.4's pixel.
///
/// One pass over the row's edges sorts them three ways: through the pixel (the bands'
/// boundaries), left of it across the whole row (a constant winding for every band), and
/// left of it across part of the row (a winding for the bands they span). An edge right of
/// the pixel crosses no ray that ends at it.
#[expect(clippy::arithmetic_side_effects)] // windings count edges of one row
pub(crate) fn area_in_pixel(x: i32, y: i32, sets: &[&RowEdges], work: &mut Work) -> f64 {
    let (xl, xr) = (f64::from(x), f64::from(x) + 1.0);
    let (yt, yb) = (f64::from(y), f64::from(y) + 1.0);
    work.through.clear();
    work.partial.clear();
    work.left.clear();
    work.left.resize(sets.len(), 0);
    work.cuts.clear();
    work.cuts.extend([yt, yb]);
    for (index, set) in sets.iter().enumerate() {
        let row = set.row(y);
        // Every entry before `wholly_left` ends left of the pixel: the whole-row ones add
        // their winding through the prefix sum, the partial ones one by one.
        let wholly_left = row.reach.partition_point(|&furthest| furthest < xl);
        work.left[index] += row.prefix[wholly_left];
        for &k in row.partials {
            let k = k as usize;
            if k < wholly_left {
                let edge = &row.edges[row.entries[k].edge as usize];
                let (ya, yz) = row_span(edge, yt);
                work.partial.push(Partial {
                    ya,
                    yb: yz,
                    set: index,
                    dir: edge.dir,
                });
            }
        }
        for entry in &row.entries[wholly_left..] {
            if entry.lo > xr {
                break;
            }
            let edge = &row.edges[entry.edge as usize];
            let (ya, yz) = row_span(edge, yt);
            let (xa, xz) = (edge.x_at(ya), edge.x_at(yz));
            if xa.max(xz) < xl {
                #[expect(clippy::float_cmp)] // exact: the edge spans the row or it does not
                if ya == yt && yz == yb {
                    work.left[index] += edge.dir;
                } else {
                    work.partial.push(Partial {
                        ya,
                        yb: yz,
                        set: index,
                        dir: edge.dir,
                    });
                }
                continue;
            }
            work.cuts.extend([ya, yz]);
            for side in [xl, xr] {
                if (xa - side) * (xz - side) < 0.0 {
                    work.cuts.push(ya + (side - xa) / (xz - xa) * (yz - ya));
                }
            }
            work.through.push(Through {
                ya,
                yb: yz,
                xa,
                xb: xz,
                set: index,
                dir: edge.dir,
            });
        }
    }
    crossings(&work.through, &mut work.cuts, (yt, yb));
    work.cuts.sort_by(f64::total_cmp);
    work.cuts.dedup();

    let mut area = 0.0;
    for k in 1..work.cuts.len() {
        let (ya, yz) = (work.cuts[k - 1], work.cuts[k]);
        if yz <= ya {
            continue;
        }
        area += band_area(sets, (xl, xr), (ya, yz), work);
    }
    area.clamp(0.0, 1.0)
}

/// The heights inside `rows` where two edges through the pixel cross.
#[expect(clippy::arithmetic_side_effects)] // `i + 1` is at most the slice's length
fn crossings(through: &[Through], cuts: &mut Vec<f64>, (yt, yb): (f64, f64)) {
    for (i, a) in through.iter().enumerate() {
        for b in &through[i + 1..] {
            let (lo, hi) = (a.ya.max(b.ya), a.yb.min(b.yb));
            if lo >= hi {
                continue;
            }
            let (d0, d1) = (a.x_at(lo) - b.x_at(lo), a.x_at(hi) - b.x_at(hi));
            if d0 * d1 < 0.0 {
                let y = lo + d0 / (d0 - d1) * (hi - lo);
                if y > yt && y < yb {
                    cuts.push(y);
                }
            }
        }
    }
}

/// The area of one band's strips that every set calls inside.
#[expect(clippy::arithmetic_side_effects)] // windings count edges of one row
fn band_area(
    sets: &[&RowEdges],
    (xl, xr): (f64, f64),
    (ya, yz): (f64, f64),
    work: &mut Work,
) -> f64 {
    let ym = 0.5 * (ya + yz);
    work.windings.clone_from(&work.left);
    // Half-open, as a ray through a vertex counts one of its two edges.
    for partial in &work.partial {
        if partial.ya <= ym && ym < partial.yb {
            work.windings[partial.set] += partial.dir;
        }
    }
    work.boundaries.clear();
    for edge in &work.through {
        if !(edge.ya <= ym && ym < edge.yb) {
            continue;
        }
        let xm = edge.x_at(ym);
        if xm < xl {
            work.windings[edge.set] += edge.dir;
        } else if xm <= xr {
            work.boundaries.push(Boundary {
                xm,
                xa: edge.x_at(ya).clamp(xl, xr),
                xb: edge.x_at(yz).clamp(xl, xr),
                set: edge.set,
                dir: edge.dir,
            });
        }
    }
    work.boundaries.sort_by(|a, b| a.xm.total_cmp(&b.xm));
    let inside = |windings: &[i32]| sets.iter().zip(windings).all(|(set, &w)| set.inside(w));
    let height = yz - ya;
    let mut area = 0.0;
    let mut previous = (xl, xl);
    let mut within = inside(&work.windings);
    for boundary in &work.boundaries {
        if within {
            area += height * ((boundary.xa + boundary.xb) - (previous.0 + previous.1)) * 0.5;
        }
        previous = (boundary.xa, boundary.xb);
        work.windings[boundary.set] += boundary.dir;
        within = inside(&work.windings);
    }
    if within {
        area += height * ((xr + xr) - (previous.0 + previous.1)) * 0.5;
    }
    area
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects)] // test arithmetic over a few literal shapes
mod tests {
    use super::{RowEdges, Work, area_in_pixel};
    use crate::raster::{Polyline, Rule};
    use raster_scene::Point;

    fn polygon(points: &[(f32, f32)]) -> Polyline {
        Polyline {
            points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            closed: true,
            tangents: Vec::new(),
        }
    }

    fn set(polylines: &[Polyline], rule: Rule) -> RowEdges {
        RowEdges::of(polylines, rule, -4, 16, usize::MAX).expect("within the limit")
    }

    /// Two half-planes `x ≤ 2.6` meet in a pixel at their common 0.6, and two that miss each
    /// other inside the pixel (`x ≤ 2.3` and `x ≥ 2.7`) meet in nothing — the two cases two
    /// bytes cannot tell apart from `0.6 × 0.6` and `0.3 × 0.3`.
    #[test]
    fn coincident_edges_keep_their_area_and_disjoint_ones_have_none() {
        let left = |r: f32| polygon(&[(-3.0, -3.0), (r, -3.0), (r, 9.0), (-3.0, 9.0)]);
        let right = |l: f32| polygon(&[(l, -3.0), (9.0, -3.0), (9.0, 9.0), (l, 9.0)]);
        let mut work = Work::default();
        let a = set(&[left(2.6)], Rule::NonZero);
        let b = set(&[left(2.6)], Rule::NonZero);
        assert!((area_in_pixel(2, 3, &[&a, &b], &mut work) - 0.6).abs() < 1e-6);
        let c = set(&[left(2.3)], Rule::NonZero);
        let d = set(&[right(2.7)], Rule::NonZero);
        assert!(area_in_pixel(2, 3, &[&c, &d], &mut work).abs() < 1e-12);
    }

    /// Two triangles crossing inside one pixel: the closed form is the overlap's own
    /// polygon, `(0, 0) (1, 0) (0.5, 0.5)` against `(0, 0) (0, 1) (0.5, 0.5)` reflected —
    /// the lower triangle under `y ≥ x` and the one under `y ≥ 1 − x` meet in the
    /// quarter `(0, 1) (0.5, 0.5) (1, 1)`, area 1/4.
    #[test]
    fn crossing_edges_inside_a_pixel_meet_in_their_shared_wedge() {
        let below_diagonal = polygon(&[(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let below_anti = polygon(&[(1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let mut work = Work::default();
        let a = set(&[below_diagonal], Rule::NonZero);
        let b = set(&[below_anti], Rule::EvenOdd);
        assert!((area_in_pixel(0, 0, &[&a, &b], &mut work) - 0.25).abs() < 1e-12);
    }

    /// A ring under even-odd — a square with a square hole wound the same way — leaves the
    /// hole out of the meet, which only the rule read per set does: the hole winds 2.
    #[test]
    fn each_set_answers_under_its_own_rule() {
        let outer = polygon(&[(-2.0, -2.0), (3.0, -2.0), (3.0, 3.0), (-2.0, 3.0)]);
        let hole = polygon(&[(0.25, -1.0), (0.75, -1.0), (0.75, 2.0), (0.25, 2.0)]);
        let ring = vec![outer.clone(), hole];
        let all = set(&[outer], Rule::NonZero);
        let mut work = Work::default();
        let even_odd = set(&ring, Rule::EvenOdd);
        assert!((area_in_pixel(0, 0, &[&all, &even_odd], &mut work) - 0.5).abs() < 1e-12);
        let non_zero = set(&ring, Rule::NonZero);
        assert!((area_in_pixel(0, 0, &[&all, &non_zero], &mut work) - 1.0).abs() < 1e-12);
    }

    /// The winding of `(x, y)` by §8.5.3.3.2's ray, counted over every edge of `polylines`.
    fn winding(polylines: &[Polyline], px: f64, py: f64) -> i32 {
        let mut count = 0;
        for line in polylines {
            let n = line.points.len();
            for i in 0..n {
                let (a, b) = (line.points[i], line.points[(i + 1) % n]);
                let (ax, ay, bx, by) = (
                    f64::from(a.x),
                    f64::from(a.y),
                    f64::from(b.x),
                    f64::from(b.y),
                );
                if (ay <= py) != (by <= py) {
                    let cx = ax + (py - ay) / (by - ay) * (bx - ax);
                    if cx < px {
                        count += if by > ay { 1 } else { -1 };
                    }
                }
            }
        }
        count
    }

    /// **The meet of two crossing, self-intersecting paths is the area their two rules
    /// declare inside together**, against the predicate itself sampled 128 × 128 per pixel:
    /// two seven-pointed stars, one under each rule, whose edges cross each other and
    /// themselves in many pixels — and a row of each holds every edge of its star, so the
    /// prefix sums and the run search are both exercised.
    #[test]
    fn two_crossing_stars_meet_as_their_rules_say() {
        const N: i32 = 128;
        let star = |cx: f32, cy: f32, r: f32, turn: f32| {
            let points = (0_u8..7)
                .map(|k| {
                    let a = turn + std::f32::consts::TAU * (3.0 * f32::from(k)) / 7.0;
                    Point::new(cx + r * a.cos(), cy + r * a.sin())
                })
                .collect();
            Polyline {
                points,
                closed: true,
                tangents: Vec::new(),
            }
        };
        let a = vec![star(6.3, 5.7, 4.6, 0.1)];
        let b = vec![star(5.1, 6.2, 4.9, 0.7)];
        let (sa, sb) = (
            RowEdges::of(&a, Rule::NonZero, -4, 20, usize::MAX).expect("within the limit"),
            RowEdges::of(&b, Rule::EvenOdd, -4, 20, usize::MAX).expect("within the limit"),
        );
        let mut work = Work::default();
        let mut worst = 0.0_f64;
        for py in 0..12 {
            for px in 0..12 {
                let exact = area_in_pixel(px, py, &[&sa, &sb], &mut work);
                let mut hits = 0;
                for k in 0..N * N {
                    let x = f64::from(px) + (f64::from(k % N) + 0.5) / f64::from(N);
                    let y = f64::from(py) + (f64::from(k / N) + 0.5) / f64::from(N);
                    if winding(&a, x, y) != 0 && winding(&b, x, y) % 2 != 0 {
                        hits += 1;
                    }
                }
                let sampled = f64::from(hits) / f64::from(N * N);
                worst = worst.max((exact - sampled).abs());
            }
        }
        assert!(worst < 0.01, "a pixel {worst} from the sampled predicate");
    }

    /// A limit the buckets would pass declines rather than allocates.
    #[test]
    fn buckets_past_their_limit_are_declined() {
        let tall = polygon(&[(0.0, -4.0), (1.0, -4.0), (1.0, 12.0), (0.0, 12.0)]);
        assert!(RowEdges::of(std::slice::from_ref(&tall), Rule::NonZero, -4, 16, 31).is_none());
        assert!(RowEdges::of(&[tall], Rule::NonZero, -4, 16, 32).is_some());
    }
}
