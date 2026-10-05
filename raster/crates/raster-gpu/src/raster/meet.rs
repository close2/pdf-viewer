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

/// A horizontal edge: it crosses no horizontal ray, but where it runs across a pixel it
/// bounds a band — a set's inside changes across it — so its height is a cut.
#[derive(Debug, Clone, Copy)]
struct Level {
    y: f64,
    lo: f64,
    hi: f64,
}

/// Every horizontal edge of `polylines` at a height strictly inside one of the rows
/// `first .. last` (both whole numbers), closed as a fill closes each subpath (§8.5.3.1), by
/// row, as `(row, level)`.
#[expect(clippy::arithmetic_side_effects)] // `(i + 1) % n` with `i < n`
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a height inside the rows
fn levels_between(polylines: &[Polyline], first: f64, last: f64) -> Vec<(u32, Level)> {
    let mut levels = Vec::new();
    for polyline in polylines {
        let count = polyline.points.len();
        for i in 0..count {
            let (from, to) = (polyline.points[i], polyline.points[(i + 1) % count]);
            let (y, xa, xb) = (f64::from(from.y), f64::from(from.x), f64::from(to.x));
            #[expect(clippy::float_cmp)] // exact: a horizontal edge is one whose ends are equal
            let horizontal = from.y == to.y;
            // A level on a row's own top edge is already a band's end there.
            let inside_a_row = y > first && y < last && y.fract() != 0.0;
            if horizontal && inside_a_row && xa.is_finite() && xb.is_finite() {
                let row = (y - first).floor() as u32;
                levels.push((
                    row,
                    Level {
                        y,
                        lo: xa.min(xb),
                        hi: xa.max(xb),
                    },
                ));
            }
        }
    }
    levels
}

/// The part of row `y .. y + 1` an edge spans.
fn row_span(edge: &Edge, y: f64) -> (f64, f64) {
    (edge.y0.max(y), edge.y1.min(y + 1.0))
}

/// How wide across its row a run may grow: a pixel a run reaches reads the run edge by edge,
/// so a run wider than a pixel or two would hand a pixel edges that stand well beside it.
const RUN_WIDTH: f64 = 1.0;

/// Every edge's part in each row it reaches, gathered into runs ([`Entry`]) and listed as
/// `(row, run)` in the order the runs were begun; `span` gives the rows an edge reaches, and
/// `pairs` — the edge-row pairs, an upper bound on the runs — sizes the list.
///
/// An edge continues the run its row last began when it follows that run's last edge in the
/// set's edges ([`Entry::continued_by`]); the set's edges are each subpath's in path order, so
/// a curve flattened into short pieces is gathered row by row as the walk crosses it.
#[expect(clippy::cast_possible_truncation)] // an edge index below the edge count, a `u32`
#[expect(clippy::arithmetic_side_effects)] // a run's count is at most the edge count
fn runs_by_row(
    edges: &[Edge],
    first: f64,
    rows: u32,
    pairs: usize,
    span: impl Fn(&Edge) -> (u32, u32),
) -> Vec<(u32, Entry)> {
    let mut runs: Vec<(u32, Entry)> = Vec::with_capacity(pairs);
    // Per row, the run it last began, or `usize::MAX` before its first.
    let mut open = vec![usize::MAX; rows as usize];
    for (index, edge) in edges.iter().enumerate() {
        let (from, to) = span(edge);
        for r in from..to {
            let (ya, yb) = row_span(edge, first + f64::from(r));
            let (xa, xb) = (edge.x_at(ya), edge.x_at(yb));
            let (lo, hi) = (xa.min(xb), xa.max(xb));
            let last = &mut open[r as usize];
            match runs.get_mut(*last) {
                Some((_, run))
                    if run.continued_by(edges, index, (ya, yb))
                        && run.hi.max(hi) - run.lo.min(lo) <= RUN_WIDTH =>
                {
                    run.count += 1;
                    run.lo = run.lo.min(lo);
                    run.hi = run.hi.max(hi);
                    run.ya = run.ya.min(ya);
                    run.yb = run.yb.max(yb);
                }
                _ => {
                    *last = runs.len();
                    runs.push((
                        r,
                        Entry {
                            edge: index as u32,
                            count: 1,
                            lo,
                            hi,
                            ya,
                            yb,
                        },
                    ));
                }
            }
        }
    }
    // Bucketed by row, each row's runs in the order they were begun.
    runs.sort_by_key(|&(row, _)| row);
    runs
}

/// `starts[r] .. starts[r + 1]` indexes the runs of row `r` in `runs`, which are sorted by row.
#[expect(clippy::cast_possible_truncation)] // a run count below the edge-row pairs, a `u32`
fn row_starts(runs: &[(u32, Entry)], rows: u32) -> Vec<u32> {
    (0..=rows)
        .map(|r| runs.partition_point(|&(row, _)| row < r) as u32)
        .collect()
}

/// One set's edges bucketed by device row: every edge that reaches into a row is listed
/// under it, so a ray along that row meets no edge the bucket does not hold.
///
/// Each row's entries are sorted by where their edge starts across the row, and beside them
/// is the furthest any entry so far reaches: so a pixel finds the run of edges that can touch
/// it by two searches, and the edges wholly left of that run add their winding from a prefix
/// sum rather than one at a time — a row of a clip holding thousands of glyph edges is not
/// walked again for every pixel a mark cuts in it.
#[derive(Debug, Clone)]
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
    /// `level_starts[r] .. level_starts[r + 1]` indexes `levels` for row `top + r`.
    level_starts: Vec<u32>,
    levels: Vec<Level>,
    /// `charged[r]` is what the bound in [`RowEdges::of`] counts for the rows `top .. top + r`:
    /// each edge once for every one of them it reaches, and each level in them. Empty unless
    /// the set was built by [`RowEdges::of_charged`].
    charged: Vec<u64>,
}

/// A run of edges as one row lists it: `count` edges from `edge` on, consecutive in the set's
/// edges, of one direction, whose parts in the row abut end to start — so between them they
/// span `ya .. yb` of the row with no gap and no overlap — and how far across the row they
/// reach, `lo ..= hi`.
///
/// **A run adds to a ray beside it what one edge over its span would** (ADR 1503). A ray at
/// height `ym` meets an edge where `ya ≤ ym < yb` (§8.5.3.3.2's count, half-open as a ray
/// through a vertex counts one of its two edges), and abutting half-open spans partition their
/// union, so a ray at any height of the union meets exactly one of the run's edges. A curve
/// flattened into short pieces crosses a row as one run, and where that run spans the whole
/// row its winding is the row's prefix sum rather than a partial edge counted per band.
#[derive(Debug, Clone, Copy)]
struct Entry {
    edge: u32,
    count: u32,
    lo: f64,
    hi: f64,
    ya: f64,
    yb: f64,
}

impl Entry {
    /// Whether `edge`, the set's edge at `index` spanning `ya .. yb` of this run's row,
    /// continues this run: it follows the run's last edge, runs the same way, and starts its
    /// part of the row where the run's part ends, in the path's own direction.
    #[expect(clippy::float_cmp)] // exact: two edges meet at one `f32` vertex or they do not
    #[expect(clippy::arithmetic_side_effects)] // an edge index below the edge count, a `u32`
    fn continued_by(&self, edges: &[Edge], index: usize, (ya, yb): (f64, f64)) -> bool {
        let last = &edges[self.edge as usize];
        let follows = self.edge as usize + self.count as usize == index;
        follows
            && last.dir == edges[index].dir
            && if last.dir > 0 {
                self.yb == ya
            } else {
                yb == self.ya
            }
    }
}

/// One row of a [`RowEdges`].
struct Row<'a> {
    edges: &'a [Edge],
    entries: &'a [Entry],
    reach: &'a [f64],
    prefix: &'a [i32],
    partials: &'a [u32],
    levels: &'a [Level],
}

impl RowEdges {
    /// The edges of `polylines`, filled under `rule`, over the rows `top .. top + rows`.
    ///
    /// Every subpath is closed, as a fill closes it (§8.5.3.1). `None` where the buckets
    /// would hold more than `limit` entries — a page-tall clip of many long edges — which
    /// the caller answers by keeping its bound rather than by allocating past it
    /// (principle 3).
    pub(crate) fn of(
        polylines: &[Polyline],
        rule: Rule,
        top: i32,
        rows: u32,
        limit: usize,
    ) -> Option<Self> {
        Self::build(polylines, rule, (top, rows), limit, false)
    }

    /// [`RowEdges::of`], keeping beside the buckets what its bound counts row by row, so that
    /// [`RowEdges::charge`] can answer for any run of these rows (ADR 1513).
    ///
    /// Kept only where it is asked for: the per-row count is a pass over every edge part, and
    /// a set built for one meet never asks it.
    pub(crate) fn of_charged(
        polylines: &[Polyline],
        rule: Rule,
        top: i32,
        rows: u32,
        limit: usize,
    ) -> Option<Self> {
        Self::build(polylines, rule, (top, rows), limit, true)
    }

    /// [`RowEdges::of`] and [`RowEdges::of_charged`]: `charges` says which.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // rows are clamped
    // into `0 .. rows`, a tile's height, and an entry count below `u32::MAX` is a tile's
    #[expect(clippy::arithmetic_side_effects)] // indices inside the row range
    fn build(
        polylines: &[Polyline],
        rule: Rule,
        (top, rows): (i32, u32),
        limit: usize,
        charges: bool,
    ) -> Option<Self> {
        let first = f64::from(top);
        let last = first + f64::from(rows);
        let edges = edges_between(polylines, first, last);
        let span = |edge: &Edge| -> (u32, u32) {
            let from = (edge.y0.max(first) - first).floor() as u32;
            let to = ((edge.y1.min(last) - first).ceil() as u32).min(rows);
            (from, to)
        };
        // The bound counts an edge once for every row it reaches, whether or not a run
        // gathers it with its neighbours: what a set may cost is decided before its runs are.
        // Kept row by row as well, so that a meet over some of these rows asks the bound a
        // build over those rows alone would have asked ([`RowEdges::charge`]).
        let mut total = 0_usize;
        let mut reached = if charges {
            vec![0_u64; rows as usize + 1]
        } else {
            Vec::new()
        };
        for edge in &edges {
            let (from, to) = span(edge);
            total = total.saturating_add(to.saturating_sub(from) as usize);
            if total > limit {
                return None;
            }
            if let Some(row) = reached.get_mut(from as usize..to as usize) {
                for count in row {
                    *count += 1;
                }
            }
        }
        let mut levels = levels_between(polylines, first, last);
        total = total.saturating_add(levels.len());
        if total > limit {
            return None;
        }
        levels.sort_by_key(|&(row, _)| row);
        let mut by_row = Vec::new();
        if charges {
            for &(row, _) in &levels {
                reached[row as usize] += 1;
            }
            by_row.reserve(rows as usize + 1);
            let mut running = 0_u64;
            for count in &reached[..rows as usize] {
                by_row.push(running);
                running += count;
            }
            by_row.push(running);
        }
        let mut level_starts = Vec::with_capacity(rows as usize + 1);
        let mut at = 0_usize;
        for r in 0..=rows {
            level_starts.push(at as u32);
            while levels.get(at).is_some_and(|&(row, _)| row == r) {
                at += 1;
            }
        }
        let levels: Vec<Level> = levels.into_iter().map(|(_, level)| level).collect();
        let total = total - levels.len();
        let entries = runs_by_row(&edges, first, rows, total, span);
        let starts = row_starts(&entries, rows);
        let entries: Vec<Entry> = entries.into_iter().map(|(_, entry)| entry).collect();
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
            level_starts,
            levels,
            charged: by_row,
        };
        sets.order_rows(first, rows);
        Some(sets)
    }

    /// What [`RowEdges::of`]'s bound counts for the device rows `top .. top + rows`, or `None`
    /// where they are not all bucketed here.
    ///
    /// **A build over more rows than a meet asks holds that meet's rows exactly**: a row's
    /// bucket lists every edge part reaching it, gathered into runs of edges that follow one
    /// another in the set, and an edge between two such edges reaches the row too, so it is in
    /// either build's list (ADR 1503's runs, ADR 1513). What a narrower build adds is only this
    /// bound's question, and the per-row counts answer it: the narrower build is `None`
    /// exactly where this charge passes its limit.
    pub(crate) fn charge(&self, top: i32, rows: u32) -> Option<u64> {
        let start = usize::try_from(top.checked_sub(self.top)?).ok()?;
        let end = start.checked_add(usize::try_from(rows).ok()?)?;
        self.charged
            .get(end)?
            .checked_sub(*self.charged.get(start)?)
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
                furthest = furthest.max(entry.hi);
                self.reach[from + k] = furthest;
                #[expect(clippy::float_cmp)] // exact: the run spans the row or it does not
                if entry.ya == y && entry.yb == y + 1.0 {
                    sum += self.edges[entry.edge as usize].dir;
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
            .saturating_add(self.level_starts.len())
            .saturating_mul(size_of::<u32>());
        let wide = self
            .edges
            .len()
            .saturating_mul(size_of::<Edge>())
            .saturating_add(self.entries.len().saturating_mul(size_of::<Entry>()))
            .saturating_add(self.reach.len().saturating_mul(size_of::<f64>()))
            .saturating_add(self.levels.len().saturating_mul(size_of::<Level>()))
            .saturating_add(self.charged.len().saturating_mul(size_of::<u64>()));
        words.saturating_add(wide) as u64
    }

    /// Device row `y`, empty outside the bucketed rows.
    #[expect(clippy::arithmetic_side_effects)] // `r + 1` is a row of `starts`
    fn row(&self, y: i32) -> Row<'_> {
        let bounds = |r: usize| -> Option<(usize, usize, usize, usize, usize, usize)> {
            Some((
                *self.starts.get(r)? as usize,
                *self.starts.get(r + 1)? as usize,
                *self.partial_starts.get(r)? as usize,
                *self.partial_starts.get(r + 1)? as usize,
                *self.level_starts.get(r)? as usize,
                *self.level_starts.get(r + 1)? as usize,
            ))
        };
        match usize::try_from(y.saturating_sub(self.top))
            .ok()
            .and_then(|r| Some((r, bounds(r)?)))
        {
            Some((r, (from, to, p_from, p_to, l_from, l_to))) => Row {
                edges: &self.edges,
                entries: &self.entries[from..to],
                reach: &self.reach[from..to],
                prefix: &self.prefix[from + r..=to + r],
                partials: &self.partials[p_from..p_to],
                levels: &self.levels[l_from..l_to],
            },
            None => Row {
                edges: &self.edges,
                entries: &[],
                reach: &[],
                prefix: &[0],
                partials: &[],
                levels: &[],
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

/// An edge that spans only part of its row on the side the winding is counted from: it adds
/// its winding to a ray at the heights it spans — negated when the count is taken from the
/// right, where an edge through the pixel is one of them too (ADR 1491).
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
    /// Per set, the winding entering the pixel from the edges beside it that span its whole
    /// row.
    left: Vec<i32>,
    windings: Vec<i32>,
    /// Each band's middle height, in order.
    middles: Vec<f64>,
    /// Per band and set, the winding of the partial edges beside the pixel
    /// ([`partial_windings`]).
    band_partials: Vec<i32>,
}

/// The area, in `0 ..= 1`, of the intersection of every set in `sets` inside device pixel
/// `(x, y)` — `[x, x + 1) × [y, y + 1)`, §10.7.4's pixel.
///
/// One pass over the row's edges sorts them three ways: through the pixel (the bands'
/// boundaries), beside it across the whole row (a constant winding for every band), and
/// beside it across part of the row (a winding for the bands they span) — beside it on the
/// left, or on the right where fewer partial edges stand there ([`sort_row`]).
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
        // A horizontal edge running across the pixel changes this set's inside at its height,
        // whether or not an edge through the pixel starts or ends there.
        for level in row.levels {
            if level.y > yt && level.y < yb && level.lo <= xr && level.hi >= xl {
                work.cuts.push(level.y);
            }
        }
        sort_row(index, &row, (xl, xr), (yt, yb), work);
    }
    crossings(&work.through, &mut work.cuts, (yt, yb));
    work.cuts.sort_by(f64::total_cmp);
    work.cuts.dedup();

    partial_windings(sets.len(), work);

    let mut area = 0.0;
    let mut band = 0;
    for k in 1..work.cuts.len() {
        let (ya, yz) = (work.cuts[k - 1], work.cuts[k]);
        if yz <= ya {
            continue;
        }
        area += band_area(sets, (xl, xr), (ya, yz), band, work);
        band += 1;
    }
    area.clamp(0.0, 1.0)
}

/// Sort one set's row for pixel `[xl, xr) × [yt, yb)` into `work`: its edges through the
/// pixel, and the winding entering the pixel from the left, counted from whichever side holds
/// fewer partial edges.
#[expect(clippy::arithmetic_side_effects)] // windings count edges of one row
fn sort_row(
    index: usize,
    row: &Row<'_>,
    (xl, xr): (f64, f64),
    (yt, yb): (f64, f64),
    work: &mut Work,
) {
    // Every entry before `wholly_left` ends left of the pixel, and every entry from
    // `right_start` on starts right of it.
    let wholly_left = row.reach.partition_point(|&furthest| furthest < xl);
    let right_start = row.entries.partition_point(|entry| entry.lo <= xr);
    // `partials` lists its entries by position in the row, so the ones left of the pixel
    // are a prefix of it and the ones right of it a suffix: a row of a long curved clip is
    // almost all partial edges (ADR 1479).
    let left_partials = row
        .partials
        .partition_point(|&k| (k as usize) < wholly_left);
    let right_partials = row
        .partials
        .partition_point(|&k| (k as usize) < right_start);
    // **The winding entering the pixel is counted from whichever side holds fewer partial
    // edges** (ADR 1491). Along a horizontal line every closed subpath's crossings sum to
    // zero, so the winding left of the pixel is minus the crossings through it and right
    // of it; both counts are integers, so the side changes nothing but the work.
    let from_right = row.partials.len() - right_partials < left_partials;
    let sign = if from_right { -1 } else { 1 };
    let (prefix, partials) = if from_right {
        let whole = row.prefix[row.entries.len()] - row.prefix[right_start];
        (whole, &row.partials[right_partials..])
    } else {
        (row.prefix[wholly_left], &row.partials[..left_partials])
    };
    work.left[index] += sign * prefix;
    for &k in partials {
        let run = &row.entries[k as usize];
        work.partial.push(Partial {
            ya: run.ya,
            yb: run.yb,
            set: index,
            dir: sign * row.edges[run.edge as usize].dir,
        });
    }
    // A run that reaches the pixel is read edge by edge: its edges beside the pixel count as
    // the side they stand on, and the ones through it bound its bands.
    let near = row.entries[wholly_left..right_start]
        .iter()
        .flat_map(|run| &row.edges[run.edge as usize..(run.edge + run.count) as usize]);
    for edge in near {
        let (ya, yz) = row_span(edge, yt);
        let (xa, xz) = (edge.x_at(ya), edge.x_at(yz));
        let beside = if xa.max(xz) < xl {
            !from_right
        } else if xa.min(xz) > xr {
            from_right
        } else {
            false
        };
        if beside {
            let dir = sign * edge.dir;
            #[expect(clippy::float_cmp)] // exact: the edge spans the row or it does not
            if ya == yt && yz == yb {
                work.left[index] += dir;
            } else {
                work.partial.push(Partial {
                    ya,
                    yb: yz,
                    set: index,
                    dir,
                });
            }
            continue;
        }
        if xa.max(xz) < xl || xa.min(xz) > xr {
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
        // Counted from the right, an edge through the pixel is crossed on the way in.
        if from_right {
            work.partial.push(Partial {
                ya,
                yb: yz,
                set: index,
                dir: -edge.dir,
            });
        }
    }
}

/// Each band's winding from the partial edges beside the pixel, per set, into
/// `work.band_partials` — band `b`'s windings at `b × sets .. (b + 1) × sets`.
///
/// A partial edge adds its winding to a band whose middle height `ym` has
/// `ya ≤ ym < yb` (half-open, as a ray through a vertex counts one of its two edges). The
/// middles rise with the bands, so those bands are one run, found by two searches, and the
/// run is added as a difference: each edge costs two searches rather than one test per band.
/// The windings are integers, so the order they are summed in changes nothing.
#[expect(clippy::arithmetic_side_effects)] // band and set indices inside the buffers
fn partial_windings(sets: usize, work: &mut Work) {
    work.middles.clear();
    for k in 1..work.cuts.len() {
        let (ya, yz) = (work.cuts[k - 1], work.cuts[k]);
        if yz > ya {
            work.middles.push(0.5 * (ya + yz));
        }
    }
    let bands = work.middles.len();
    work.band_partials.clear();
    work.band_partials.resize((bands + 1) * sets, 0);
    for partial in &work.partial {
        let first = work.middles.partition_point(|&ym| ym < partial.ya);
        let last = work.middles.partition_point(|&ym| ym < partial.yb);
        if first < last {
            work.band_partials[first * sets + partial.set] += partial.dir;
            work.band_partials[last * sets + partial.set] -= partial.dir;
        }
    }
    for b in 1..bands {
        for set in 0..sets {
            work.band_partials[b * sets + set] += work.band_partials[(b - 1) * sets + set];
        }
    }
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
    band: usize,
    work: &mut Work,
) -> f64 {
    let ym = 0.5 * (ya + yz);
    work.windings.clone_from(&work.left);
    let partials = &work.band_partials[band * sets.len()..(band + 1) * sets.len()];
    for (winding, partial) in work.windings.iter_mut().zip(partials) {
        *winding += partial;
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

    /// **The winding entering the pixel is the same counted from either side** (ADR 1491):
    /// a wedge `x ≥ 2 + 0.5·(y − 2)` cuts pixel `(2, 2)` by a trapezoid of area `0.75`, and
    /// eleven small squares spanning part of the row — partial edges — stand left of the pixel
    /// in one arm, so the count is taken from the right, and right of it in the other, so it is
    /// taken from the left. Wound both ways round, each square adds and removes nothing, and
    /// both arms read the wedge's area, under either rule.
    #[test]
    fn the_winding_into_a_pixel_is_counted_from_the_side_with_fewer_partial_edges() {
        let mut work = Work::default();
        for left_of_pixel in [true, false] {
            for rule in [Rule::NonZero, Rule::EvenOdd] {
                let mut polylines =
                    vec![polygon(&[(2.0, 2.0), (9.0, 2.0), (9.0, 12.0), (7.0, 12.0)])];
                for k in 0..11_u8 {
                    let x = if left_of_pixel {
                        -10.0 + f32::from(k)
                    } else {
                        3.5 + f32::from(k) * 0.3
                    };
                    let square = [(x, 2.2), (x + 0.2, 2.2), (x + 0.2, 2.6), (x, 2.6)];
                    let wound: Vec<(f32, f32)> = if k % 2 == 0 {
                        square.to_vec()
                    } else {
                        square.iter().rev().copied().collect()
                    };
                    polylines.push(polygon(&wound));
                }
                let wedge = set(&polylines, rule);
                let area = area_in_pixel(2, 2, &[&wedge], &mut work);
                assert!(
                    (area - 0.75).abs() < 1e-12,
                    "{rule:?}, squares {}: {area}",
                    if left_of_pixel { "left" } else { "right" }
                );
            }
        }
    }

    /// A set whose horizontal edge crosses the pixel between two sides that lie outside it:
    /// the rectangle `[-3, 9] × [2.2, 9]` holds the pixel `(2, 2)` below `y = 2.2` only, and
    /// the half-plane `x ≤ 2.6` holds it left of `x = 2.6`, so the two meet in
    /// `[2, 2.6] × [2.2, 3]` — `0.6 × 0.8 = 0.48`. The rectangle's winding along the pixel's
    /// left side changes at `y = 2.2`, where its left side ends, and no edge through the
    /// pixel starts there: the band must still be cut at that height, or the rectangle is
    /// read as holding the whole of it (`0.6`).
    #[test]
    fn a_horizontal_edge_through_the_pixel_bounds_a_band() {
        let mut work = Work::default();
        let rectangle = set(
            &[polygon(&[(-3.0, 2.2), (9.0, 2.2), (9.0, 9.0), (-3.0, 9.0)])],
            Rule::NonZero,
        );
        let half = set(
            &[polygon(&[
                (-3.0, -3.0),
                (2.6, -3.0),
                (2.6, 9.0),
                (-3.0, 9.0),
            ])],
            Rule::NonZero,
        );
        let area = area_in_pixel(2, 2, &[&rectangle, &half], &mut work);
        assert!((area - 0.48).abs() < 1e-6, "{area} against 0.48");
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

    /// **A set built over more rows than a meet asks meets each of those rows to the same
    /// area, and charges them what a build over them alone counts** (ADR 1513): a curve
    /// flattened into short pieces crossing the band's boundaries — so that runs begin and
    /// end at them, and a level and a vertex sit on a whole row — met against a slanted
    /// half-plane, every pixel of every narrower band read from the wide build and from its
    /// own, to the bit; and the narrower build is `None` exactly where the charge passes its
    /// limit.
    #[test]
    fn a_wider_build_meets_and_charges_a_band_as_the_band_alone() {
        let mut points = Vec::new();
        for k in 0..=96_u8 {
            let t = f32::from(k) / 96.0 * std::f32::consts::TAU;
            points.push((6.0 + 5.5 * t.cos(), 6.0 + 5.0 * t.sin()));
        }
        points.extend([(3.0, 4.0), (1.5, 4.0), (1.5, 9.0)]);
        let curve = vec![polygon(&points)];
        let plane = set(
            &[polygon(&[
                (-3.0, -3.0),
                (4.0, -3.0),
                (9.0, 13.0),
                (-3.0, 13.0),
            ])],
            Rule::NonZero,
        );
        let wide = RowEdges::of_charged(&curve, Rule::EvenOdd, -4, 20, usize::MAX)
            .expect("within the limit");
        let mut work = Work::default();
        for top in -2..12 {
            for rows in 1..6_u32 {
                let charge = wide.charge(top, rows).expect("inside the wide build");
                let alone = RowEdges::of(&curve, Rule::EvenOdd, top, rows, usize::MAX)
                    .expect("within the limit");
                for y in top..top + rows.cast_signed() {
                    for x in -1..13 {
                        let from_wide = area_in_pixel(x, y, &[&wide, &plane], &mut work);
                        let from_alone = area_in_pixel(x, y, &[&alone, &plane], &mut work);
                        assert_eq!(
                            from_wide.to_bits(),
                            from_alone.to_bits(),
                            "pixel ({x}, {y}) of the rows {top} + {rows}"
                        );
                    }
                }
                let limit = usize::try_from(charge).expect("a small charge");
                assert!(RowEdges::of(&curve, Rule::EvenOdd, top, rows, limit).is_some());
                if let Some(under) = limit.checked_sub(1) {
                    assert!(
                        RowEdges::of(&curve, Rule::EvenOdd, top, rows, under).is_none(),
                        "the rows {top} + {rows} charge {charge}"
                    );
                }
            }
        }
        assert_eq!(wide.charge(-5, 2), None, "a row above the build");
        assert_eq!(wide.charge(15, 2), None, "a row below it");
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

    /// The convex polygon `polygon` cut to where `keep` is not negative (Sutherland–Hodgman,
    /// one side).
    fn clip_convex(polygon: &[(f64, f64)], keep: impl Fn((f64, f64)) -> f64) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for (k, &here) in polygon.iter().enumerate() {
            let next = polygon[(k + 1) % polygon.len()];
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
        out
    }

    /// **A curve flattened into short pieces meets as its pieces do** (ADR 1503): a 3000-gon
    /// of radius 6, whose pieces each span part of a row and gather into runs — tall ones
    /// where the curve is steep, ones cut at a pixel's width where it is shallow — met with
    /// the half-plane `x ≤ 6.3`, wound both ways and under both rules. The closed form is the
    /// convex polygon cut to the half-plane and to the pixel, its shoelace area; every pixel of
    /// the disc's bounding rows is asked, so runs stand left of, right of and through each.
    #[test]
    fn a_finely_flattened_curve_meets_as_its_pieces_do() {
        const N: u16 = 3000;
        let ring: Vec<(f32, f32)> = (0..N)
            .map(|k| {
                let a = std::f32::consts::TAU * f32::from(k) / f32::from(N);
                (6.1 + 6.0 * a.cos(), 6.2 + 6.0 * a.sin())
            })
            .collect();
        let half = polygon(&[(-3.0, -3.0), (6.3, -3.0), (6.3, 16.0), (-3.0, 16.0)]);
        let mut work = Work::default();
        for reversed in [false, true] {
            let wound: Vec<(f32, f32)> = if reversed {
                ring.iter().rev().copied().collect()
            } else {
                ring.clone()
            };
            let exact_ring: Vec<(f64, f64)> = ring
                .iter()
                .map(|&(x, y)| (f64::from(x), f64::from(y)))
                .collect();
            for rule in [Rule::NonZero, Rule::EvenOdd] {
                let disc = set(&[polygon(&wound)], rule);
                let plane = set(std::slice::from_ref(&half), Rule::NonZero);
                let mut worst = 0.0_f64;
                for py in 0..12 {
                    for px in 0..12 {
                        let (x0, y0) = (f64::from(px), f64::from(py));
                        let mut cut = clip_convex(&exact_ring, |p| f64::from(6.3_f32) - p.0);
                        for keep in [
                            &(|p: (f64, f64)| p.0 - x0) as &dyn Fn((f64, f64)) -> f64,
                            &|p: (f64, f64)| x0 + 1.0 - p.0,
                            &|p: (f64, f64)| p.1 - y0,
                            &|p: (f64, f64)| y0 + 1.0 - p.1,
                        ] {
                            cut = clip_convex(&cut, keep);
                        }
                        let closed: f64 = (0..cut.len())
                            .map(|k| {
                                let (a, b) = (cut[k], cut[(k + 1) % cut.len()]);
                                a.0 * b.1 - b.0 * a.1
                            })
                            .sum::<f64>()
                            .abs()
                            * 0.5;
                        let area = area_in_pixel(px, py, &[&disc, &plane], &mut work);
                        worst = worst.max((area - closed).abs());
                    }
                }
                assert!(worst < 1e-9, "{rule:?}, reversed {reversed}: {worst}");
            }
        }
    }

    /// A limit the buckets would pass declines rather than allocates.
    #[test]
    fn buckets_past_their_limit_are_declined() {
        let tall = polygon(&[(0.0, -4.0), (1.0, -4.0), (1.0, 12.0), (0.0, 12.0)]);
        assert!(RowEdges::of(std::slice::from_ref(&tall), Rule::NonZero, -4, 16, 31).is_none());
        assert!(RowEdges::of(&[tall], Rule::NonZero, -4, 16, 32).is_some());
    }
}
