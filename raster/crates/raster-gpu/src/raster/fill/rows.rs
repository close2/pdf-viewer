//! A fill's edges and subpaths listed by the device rows they can reach, so that a small
//! region of a large fill reads the few edges that cross it rather than every edge the
//! fill has (ADR 1479).
//!
//! [`fill_mask`](super::fill_mask) walks every edge of its polylines and lets
//! [`Edge::cut`](super::Edge::cut) drop the ones outside the region's rows. That is the
//! right construction for a mark filled once over its own tile, and the wrong one for a
//! clip of a hundred thousand edges filled over three thousand tiles of two pixels each:
//! every tile pays every edge to learn that almost none of them reach it.
//!
//! **The index changes which edges are visited, never what a visited edge deposits, nor
//! the order deposits arrive in.** An edge running from height `lo` to height `hi` is listed
//! under every row `r` with `r < hi` and `r + 1 > lo`. [`Edge::cut`](super::Edge::cut) keeps
//! an edge for a region of rows `top .. top + h` only where `hi − top > 0` and
//! `lo − top < h` once rounded to `f32`, and rounding is monotone with `0` and `h` exact, so
//! it keeps only edges with `hi > top` and `lo < top + h` — which are listed under a row of
//! the region. The ones it would drop deposit nothing whether visited or not. A subpath is
//! listed under the rows its box spans and the one above, which holds every subpath the
//! closed box test of [`plainly_two_values`](super::topology::plainly_two_values) admits. The edges handed back are sorted by subpath and
//! then by position, which is the order the plain walk visits them in, so the accumulator's
//! `f32` sums are the same sums to the bit. An edge whose ends are not finite numbers is
//! listed for every region, which is where the plain walk would meet it too.

use super::super::flatten::{Polyline, polyline_bounds};

/// A box, `(left, top, right, bottom)`, as [`polyline_bounds`] returns one.
pub(super) type Bounds = (f32, f32, f32, f32);

/// One fill's edges and subpaths by device row.
#[derive(Debug)]
pub(crate) struct RowIndex {
    /// The first device row listed.
    top: i64,
    /// `edge_starts[r] .. edge_starts[r + 1]` indexes `edges` for row `top + r`.
    edge_starts: Vec<u32>,
    /// Each entry is `(subpath << 32) | edge`, so that integer order is the walk's order.
    edges: Vec<u64>,
    /// `subpath_starts[r] .. subpath_starts[r + 1]` indexes `subpaths` for row `top + r`.
    subpath_starts: Vec<u32>,
    subpaths: Vec<u32>,
    /// Each subpath's own box, as [`polyline_bounds`] states it for that subpath alone.
    bounds: Vec<Option<Bounds>>,
    /// Edges and subpaths whose extent is not a finite number of rows: listed everywhere.
    everywhere_edges: Vec<u64>,
    everywhere_subpaths: Vec<u32>,
}

/// The rows `from ..= to` a subpath whose box spans heights `top ..= bottom` is listed
/// under, or `None` for a box that is not finite.
#[expect(clippy::cast_possible_truncation)] // a finite `f32` floored fits `i64`
fn subpath_rows(top: f32, bottom: f32) -> Option<(i64, i64)> {
    (top.is_finite() && bottom.is_finite()).then(|| {
        (
            (top.floor() as i64).saturating_sub(1),
            bottom.floor() as i64,
        )
    })
}

/// Where an edge from height `lo` to height `hi` is listed.
enum EdgeRows {
    /// Under rows `from ..= to`.
    Listed(i64, i64),
    /// Nowhere: a horizontal edge deposits nothing in any region.
    Nowhere,
    /// Everywhere: an end that is not a finite number.
    Everywhere,
}

/// The rows `r` with `r < hi` and `r + 1 > lo` for an edge whose ends are at heights `a`
/// and `b`, `lo` the lower of the two and `hi` the higher.
#[expect(clippy::cast_possible_truncation)] // a finite `f32` rounded fits `i64`
#[expect(clippy::float_cmp)] // exact: a horizontal edge is one whose ends are equal
fn edge_rows(a: f32, b: f32) -> EdgeRows {
    if !(a.is_finite() && b.is_finite()) {
        EdgeRows::Everywhere
    } else if a == b {
        EdgeRows::Nowhere
    } else {
        let (lo, hi) = (a.min(b), a.max(b));
        EdgeRows::Listed(lo.floor() as i64, (hi.ceil() as i64).saturating_sub(1))
    }
}

impl RowIndex {
    /// The index of `polylines`, or `None` where its row lists would hold more than `limit`
    /// entries, or there is nothing to list.
    #[expect(clippy::arithmetic_side_effects)] // row offsets inside `first ..= last`
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // offsets below `rows`
    pub(crate) fn of(polylines: &[Polyline], limit: usize) -> Option<Self> {
        let bounds: Vec<Option<Bounds>> = polylines
            .iter()
            .map(|p| polyline_bounds(std::slice::from_ref(p)))
            .collect();
        let (mut first, mut last) = (i64::MAX, i64::MIN);
        for (lo, hi) in bounds
            .iter()
            .flatten()
            .filter_map(|b| subpath_rows(b.1, b.3))
        {
            first = first.min(lo);
            last = last.max(hi);
        }
        if first > last {
            return None;
        }
        let rows = usize::try_from(last - first + 1).ok()?;
        if rows > limit {
            return None;
        }
        let mut edge_counts = vec![0_u32; rows];
        let mut subpath_counts = vec![0_u32; rows];
        let mut total = 0_usize;
        let mut everywhere_edges = Vec::new();
        let mut everywhere_subpaths = Vec::new();
        let mut spans: Vec<(u64, i64, i64)> = Vec::new();
        for (index, polyline) in polylines.iter().enumerate() {
            let subpath = u32::try_from(index).ok()?;
            match bounds[index].and_then(|b| subpath_rows(b.1, b.3)) {
                Some((lo, hi)) => {
                    total = total.saturating_add((hi - lo + 1) as usize);
                    for count in &mut subpath_counts[(lo - first) as usize..=(hi - first) as usize]
                    {
                        *count += 1;
                    }
                }
                None if bounds[index].is_some() => everywhere_subpaths.push(subpath),
                None => {}
            }
            let n = polyline.points.len();
            for i in 0..n {
                let entry = (u64::from(subpath) << 32) | u64::try_from(i).ok()?;
                let (p, q) = (polyline.points[i], polyline.points[(i + 1) % n]);
                match edge_rows(p.y, q.y) {
                    EdgeRows::Listed(lo, hi) => {
                        total = total.saturating_add((hi - lo + 1) as usize);
                        if total > limit {
                            return None;
                        }
                        for count in &mut edge_counts[(lo - first) as usize..=(hi - first) as usize]
                        {
                            *count += 1;
                        }
                        spans.push((entry, lo, hi));
                    }
                    EdgeRows::Nowhere => {}
                    EdgeRows::Everywhere => everywhere_edges.push(entry),
                }
            }
            if total > limit {
                return None;
            }
        }
        let edge_starts = starts(&edge_counts)?;
        let subpath_starts = starts(&subpath_counts)?;
        let mut edges = vec![0_u64; *edge_starts.last()? as usize];
        let mut subpaths = vec![0_u32; *subpath_starts.last()? as usize];
        let mut slot: Vec<u32> = edge_starts.clone();
        for (entry, lo, hi) in spans {
            for row in lo..=hi {
                let at = &mut slot[(row - first) as usize];
                edges[*at as usize] = entry;
                *at += 1;
            }
        }
        let mut slot: Vec<u32> = subpath_starts.clone();
        for (index, b) in bounds.iter().enumerate() {
            if let Some((lo, hi)) = b.and_then(|b| subpath_rows(b.1, b.3)) {
                for row in lo..=hi {
                    let at = &mut slot[(row - first) as usize];
                    subpaths[*at as usize] = u32::try_from(index).ok()?;
                    *at += 1;
                }
            }
        }
        Some(Self {
            top: first,
            edge_starts,
            edges,
            subpath_starts,
            subpaths,
            bounds,
            everywhere_edges,
            everywhere_subpaths,
        })
    }

    /// The bytes the index holds.
    pub(crate) fn bytes(&self) -> u64 {
        let words = [
            self.edges.len().saturating_mul(2),
            self.subpaths.len(),
            self.edge_starts.len(),
            self.subpath_starts.len(),
            self.bounds.len().saturating_mul(5),
            self.everywhere_edges.len().saturating_mul(2),
            self.everywhere_subpaths.len(),
        ]
        .into_iter()
        .fold(0_usize, usize::saturating_add);
        u64::try_from(words).unwrap_or(u64::MAX).saturating_mul(4)
    }

    /// The row offsets of `top .. top + rows` that this index lists, as a range of
    /// positions, or `None` where the region holds every row listed — the plain walk is then
    /// the same visit with no list to sort.
    fn listed(&self, top: i32, rows: usize) -> Option<std::ops::Range<usize>> {
        let listed = self.edge_starts.len().saturating_sub(1);
        // Offsets past `i64` are past every row listed, so they clamp to the end.
        let offset = |row: i64| -> usize {
            usize::try_from(row.saturating_sub(self.top).max(0)).map_or(listed, |r| r.min(listed))
        };
        let from = offset(i64::from(top));
        let to = offset(i64::from(top).saturating_add(i64::try_from(rows).unwrap_or(i64::MAX)));
        (from > 0 || to < listed).then_some(from..to)
    }

    /// The edges that can reach rows `top .. top + rows`, as `(subpath, edge)` in the plain
    /// walk's order, into `out` — or `false` where that is every edge, and the caller walks
    /// them all.
    #[expect(clippy::arithmetic_side_effects)] // `r + 1` is a row of `edge_starts`
    pub(super) fn edges_meeting(
        &self,
        top: i32,
        rows: usize,
        out: &mut Vec<(usize, usize)>,
    ) -> bool {
        let Some(range) = self.listed(top, rows) else {
            return false;
        };
        let rows = range
            .map(|r| &self.edges[self.edge_starts[r] as usize..self.edge_starts[r + 1] as usize]);
        let found = merged(rows.chain(std::iter::once(self.everywhere_edges.as_slice())));
        out.clear();
        out.extend(
            found
                .into_iter()
                .map(|e| ((e >> 32) as usize, (e & 0xFFFF_FFFF) as usize)),
        );
        true
    }

    /// The subpaths whose boxes can meet rows `top .. top + rows`, in order, each with its
    /// box — or `None` where that is every subpath.
    #[expect(clippy::arithmetic_side_effects)] // `r + 1` is a row of `subpath_starts`
    pub(super) fn subpaths_meeting(
        &self,
        top: i32,
        rows: usize,
    ) -> Option<Vec<(usize, Option<Bounds>)>> {
        let range = self.listed(top, rows)?;
        let rows = range.map(|r| {
            &self.subpaths[self.subpath_starts[r] as usize..self.subpath_starts[r + 1] as usize]
        });
        let found = merged(rows.chain(std::iter::once(self.everywhere_subpaths.as_slice())));
        Some(
            found
                .into_iter()
                .map(|s| (s as usize, self.bounds[s as usize]))
                .collect(),
        )
    }
}

/// The union of `lists`, each ascending with no repeats, as one ascending list with no
/// repeats: each row's list is filled in the walk's order, so a merge keeps that order without
/// sorting what is already sorted.
fn merged<'a, T: Copy + Ord + 'a>(lists: impl Iterator<Item = &'a [T]>) -> Vec<T> {
    let mut union: Vec<T> = Vec::new();
    let mut next: Vec<T> = Vec::new();
    for list in lists {
        if list.is_empty() {
            continue;
        }
        if union.is_empty() {
            union.extend_from_slice(list);
            continue;
        }
        next.clear();
        next.reserve(union.len().saturating_add(list.len()));
        let (mut a, mut b) = (union.iter().peekable(), list.iter().peekable());
        loop {
            let take = match (a.peek(), b.peek()) {
                (Some(&&x), Some(&&y)) => match x.cmp(&y) {
                    std::cmp::Ordering::Less => a.next(),
                    std::cmp::Ordering::Greater => b.next(),
                    std::cmp::Ordering::Equal => {
                        b.next();
                        a.next()
                    }
                },
                (Some(_), None) => a.next(),
                (None, Some(_)) => b.next(),
                (None, None) => break,
            };
            if let Some(&value) = take {
                next.push(value);
            }
        }
        std::mem::swap(&mut union, &mut next);
    }
    union
}

/// Prefix offsets of `counts`, one longer than it.
fn starts(counts: &[u32]) -> Option<Vec<u32>> {
    let mut starts = Vec::with_capacity(counts.len().saturating_add(1));
    let mut running = 0_u32;
    for &count in counts {
        starts.push(running);
        running = running.checked_add(count)?;
    }
    starts.push(running);
    Some(starts)
}

#[cfg(test)]
mod tests {
    use super::super::{Rule, fill_mask, fill_mask_indexed};
    use super::RowIndex;
    use crate::raster::Polyline;
    use raster_scene::Point;

    /// A many-pointed star of `points` tips about `(cx, cy)`, wound one way, plus a ring of
    /// small squares wound the other: crossing edges, nested subpaths, edges of every slope
    /// and horizontal ones — the shapes the index must not change a byte of.
    fn shapes(cx: f32, cy: f32) -> Vec<Polyline> {
        let mut polylines = Vec::new();
        let points = 37_u16;
        let star: Vec<Point> = (0..points * 2)
            .map(|k| {
                let angle = f32::from(k) * std::f32::consts::PI / f32::from(points) * 3.0;
                let radius = if k % 2 == 0 { 40.3 } else { 9.7 };
                Point::new(cx + radius * angle.cos(), cy + radius * angle.sin())
            })
            .collect();
        polylines.push(Polyline::polygon(star));
        for k in 0..12_u16 {
            let (x, y) = (
                cx - 30.0 + 5.25 * f32::from(k),
                cy + 0.5 * f32::from(k) - 3.0,
            );
            polylines.push(Polyline::polygon(vec![
                Point::new(x, y),
                Point::new(x, y + 3.5),
                Point::new(x + 3.5, y + 3.5),
                Point::new(x + 3.5, y),
            ]));
        }
        polylines
    }

    /// **The index changes no byte** (ADR 1479): every tile of every size from one pixel
    /// to the whole shape, inside, astride and outside it, filled under both rules from the
    /// listed edges and from all of them, is the same coverage.
    #[test]
    fn a_tile_filled_from_its_rows_is_the_tile_filled_from_every_edge() {
        let polylines = shapes(60.25, 50.75);
        let index = RowIndex::of(&polylines, usize::MAX).expect("a bounded shape");
        let mut compared = 0;
        for rule in [Rule::NonZero, Rule::EvenOdd] {
            for (width, height) in [(1, 1), (2, 2), (3, 1), (7, 5), (16, 3), (90, 100)] {
                for top in (0..110).step_by(3) {
                    for left in (0..120).step_by(7) {
                        let plain = fill_mask(&polylines, rule, left, top, width, height);
                        let listed =
                            fill_mask_indexed(&polylines, &index, rule, left, top, width, height);
                        assert_eq!(
                            plain.coverage, listed.coverage,
                            "{rule:?} tile {width}x{height} at ({left}, {top})"
                        );
                        compared += 1;
                    }
                }
            }
        }
        assert!(compared > 2_000);
    }

    /// An index whose lists would pass the limit is not built, and the caller keeps the
    /// plain walk.
    #[test]
    fn an_index_past_its_limit_is_declined() {
        let polylines = shapes(60.25, 50.75);
        assert!(RowIndex::of(&polylines, 10).is_none());
    }

    /// The listed edges are visited in the plain walk's order: subpath, then position.
    #[test]
    fn the_listed_edges_come_in_the_walks_order() {
        let polylines = shapes(60.25, 50.75);
        let index = RowIndex::of(&polylines, usize::MAX).expect("a bounded shape");
        let mut listed = Vec::new();
        assert!(index.edges_meeting(48, 4, &mut listed));
        assert!(!listed.is_empty());
        assert!(listed.windows(2).all(|pair| pair[0] < pair[1]));
        let every = polylines.iter().map(|p| p.points.len()).sum::<usize>();
        assert!(
            listed.len() < every,
            "a band of four rows lists fewer than all"
        );
    }
}
