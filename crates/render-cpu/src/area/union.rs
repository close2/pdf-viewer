//! The boundary of the set a path's portions fill together under the non-zero rule — ISO 32000-2
//! §8.5.3.3.2 and §10.7.4 — for a mark stated as many portions overlapping one another.
//!
//! # What it is for
//!
//! ADR 1348 states a stroke that bends more tightly than its half-width as a union of convex
//! pieces wound one way, each overlapping the ones beside it. [`super::fill`]'s winding integral
//! reads such a mark as the integral of the winding number, which is not the area of the filled
//! set wherever one pixel holds both a point outside the mark and a point two pieces cover, and its
//! second measurement settles every such row pair by pair, which is quadratic in pieces that all
//! overlap: `inks.pdf` a hundredfold (ADR 2106 §3).
//!
//! What a pixel is owed is the area of the set §8.5.3.3.2's rule declares inside, and that set's
//! **boundary** is stretches of the pieces' edges, cut where other edges cross them. So the
//! boundary is found once for the whole mark and measured once by the accumulator: it winds once
//! round every point of the set and round no other point, so its integral over a pixel is the
//! set's area there and nothing is left for a second measurement to settle. ADR 2122.
//!
//! # The construction
//!
//! The winding numbers on the two sides of an edge differ by exactly one, and along the edge both
//! change only where another edge crosses it. So an edge bounds the filled set over the stretches
//! where one side winds zero and the other does not, and every stretch is found by walking each
//! closed contour once, carrying the winding beside it unchanged through every vertex and changing
//! it by one at every crossing. Where the walk starts is a winding known from somewhere else: a
//! crossing with a contour already walked, which fixes the four windings round the crossing point,
//! or for the first contour of a cluster that crosses nothing walked, a count of the edges a ray
//! from outside the mark crosses on its way to the contour. The boundary is every such stretch,
//! turned so that the filled side is the side its direction encloses.
//!
//! # Why the predicates are exact, and how a tie is broken
//!
//! Which side of a line a point lies on decides everything above, and a wrong answer is not a
//! small error: a crossing counted on the wrong edge, or a walk started from a wrong winding,
//! changes the winding along the rest of a contour. So the questions are asked of the device
//! coordinates on a grid of `2^-21` of a pixel, where the determinant is an exact integer, and a
//! determinant of zero — a vertex on another piece's edge, which the pieces' own construction
//! makes common — is answered by Edelsbrunner and Mücke's simulation of simplicity: every vertex
//! is moved by its own infinitesimal, in an order fixed by its index, so that every question has
//! an answer and every answer is about one configuration.
//!
//! # A piece stated twice
//!
//! A path that retraces itself states some of its pieces twice, vertex for vertex, and every point
//! of such a piece's edges lies on its twin's, so no ray can start a walk beside one and two of
//! their edges may cross along a common line. Under the non-zero rule a contour stated `k` times
//! is one contour whose edges each change the winding by `k`, so where the walk declines a mark for
//! either reason the twins are taken as one contour of that **weight** and the mark is tried again:
//! the winding is the same at every point off the edges, so the set and its boundary are too. A
//! mark the walk measures at the first attempt is measured as it was, twins or none (ADR 2281).
//!
//! # What it declines
//!
//! Two inputs the construction cannot place, each degenerate, go to the converter that drew a
//! mark before this existed, and each is said: [`UnionCondition`] names it, and `scan::mask_fill`
//! keeps it for the draw's caller (ADR 2280). Where two edges cross along a common line that no
//! twin explains, the crossing has no position. And a contour may have no point to start from:
//! every point tried on every edge lies on another edge as well — an upright edge's from a
//! horizontal ray, and where none is free a horizontal edge's from a vertical one, which is what
//! enters a thin piece whose upright sides lie along its neighbours' (ADR 2281). A mark whose walk
//! does not return to the winding it started from is declined too, and that one no consistent
//! arithmetic allows, so a build with debug assertions stops on it.
//!
//! Where along an edge a crossing lies is a ratio of two determinants, each rounded once to
//! `f64`, so two crossings closer than a part in about `2^50` of the edge may be taken in the
//! wrong order. That costs the stretch between them and nothing else: the walk passes both either
//! way. It would cost a whole contour if the walk *started* between them, so a crossing is a
//! starting point only where no other crossing of either edge lies within [`SEPARATION`] of it.
//!
//! The boundary's own coordinates are the device coordinates the path was traced to — a vertex
//! where the path states it, a crossing at one point computed once and shared by its two edges —
//! so the boundary closes to the bit.

use tiny_skia::Point;

/// Bits of a device pixel below the binary point on the grid the predicates are exact on.
///
/// One more than the twenty the grid resolves, so that the midpoint of two vertices is a grid
/// point too. A mark inside `scan::MEASURED_LIMIT` spans less than `2^16` pixels, so a coordinate
/// is under `2^37`, a difference under `2^38` and a determinant under `2^77`, inside an `i128`.
const GRID_BITS: i32 = 21;

/// The widest a coordinate may be on the grid, which keeps every determinant inside an `i128`.
const GRID_LIMIT: i64 = 1 << 38;

/// The most work one mark may cost, in pairs of edges compared.
///
/// A cost guard rather than a condition: past it the mark is drawn by the converter that drew it
/// before this existed, and the guard is the mark's own — nothing in it depends on the surface.
const UNION_WORK: usize = 1 << 25;

/// Consecutive edges of a contour filed under the grid as one, so that a pair of pieces is found
/// once rather than once for every pair of their edges.
const CHUNK: usize = 8;

/// How far apart, as a share of an edge, two crossings must lie on it before one is a walk's
/// starting point: far past the `2^-50` a parameter is rounded to, and far below any stretch that
/// carries area.
const SEPARATION: f64 = 1e-9;

/// The finest a ray's point is placed along an edge, in binary places: the point is tried at
/// `1/2`, then `1/4` and `3/4`, and so on, a coordinate under `2^38` growing to under `2^46` and
/// a determinant to under `2^93`.
const RAY_DEPTH: u32 = 8;

/// One vertex of a contour: where it is on the grid and where the path put it.
#[derive(Clone, Copy, Debug)]
struct Vertex {
    /// Its `x` on the grid.
    x: i64,
    /// Its `y` on the grid.
    y: i64,
    /// Its device coordinates.
    at: Point,
}

/// A point a predicate is asked about: a vertex, or a ray's two ends.
#[derive(Clone, Copy, Debug)]
struct Site {
    /// Its `x` on the grid.
    x: i64,
    /// Its `y` on the grid.
    y: i64,
    /// Its index, which orders its infinitesimal: a vertex's own, and past every vertex for a
    /// ray's ends.
    index: usize,
}

/// A bounding box on the grid.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    /// The smallest `x`.
    left: i64,
    /// The smallest `y`.
    top: i64,
    /// The largest `x`.
    right: i64,
    /// The largest `y`.
    bottom: i64,
}

impl Bounds {
    /// The box of two sites.
    fn of(a: Site, b: Site) -> Self {
        Self {
            left: a.x.min(b.x),
            top: a.y.min(b.y),
            right: a.x.max(b.x),
            bottom: a.y.max(b.y),
        }
    }

    /// The smallest box holding both.
    fn and(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// Whether the two meet, edges included.
    fn meets(self, other: Self) -> bool {
        self.left <= other.right
            && other.left <= self.right
            && self.top <= other.bottom
            && other.top <= self.bottom
    }
}

/// One edge: its two ends, and the contour it belongs to.
#[derive(Clone, Copy, Debug)]
struct Edge {
    /// Where it starts.
    from: Site,
    /// Where it ends.
    to: Site,
    /// Its box.
    bounds: Bounds,
    /// Its contour.
    contour: u32,
    /// How many times the path states its contour: what crossing it changes the winding by.
    weight: i8,
}

/// Consecutive edges of one contour, filed together.
#[derive(Clone, Copy, Debug)]
struct Chunk {
    /// Its first edge.
    first: u32,
    /// The edge after its last.
    end: u32,
    /// The box of all of them.
    bounds: Bounds,
    /// The cells it reaches: first column and row, last column and row.
    cells: [u32; 4],
}

/// A crossing as one of its two edges sees it.
#[derive(Clone, Copy, Debug)]
struct Crossing {
    /// The edge it lies on.
    edge: u32,
    /// The other edge.
    other: u32,
    /// Where along the edge, from `0` at its start to `1` at its end.
    t: f64,
    /// What it adds to the winding on either side of the edge, walking the edge's way: the other
    /// edge's weight, signed.
    step: i8,
    /// Where it is, computed once for both edges.
    at: Point,
}

/// The working state, kept for the length of a band and started afresh per mark.
#[derive(Debug, Default)]
pub(super) struct Union {
    /// The traced edges, each subpath's together.
    lines: Vec<[Point; 2]>,
    /// Where each subpath's edges begin in `lines`.
    subpaths: Vec<usize>,
    /// The contours' vertices; the edge from a vertex to the next one of its contour has that
    /// vertex's index.
    vertices: Vec<Vertex>,
    /// Every edge.
    edges: Vec<Edge>,
    /// Where each contour's edges begin and end.
    contours: Vec<(usize, usize)>,
    /// The chunks.
    chunks: Vec<Chunk>,
    /// Every cell's chunks, cell by cell.
    filed: Vec<u32>,
    /// Where each cell's chunks begin in `filed`.
    cell_starts: Vec<usize>,
    /// The ray a chunk was last looked at for, so that a chunk filed in several cells is counted
    /// once.
    stamps: Vec<u32>,
    /// Every crossing, twice, in the order found.
    found: Vec<Crossing>,
    /// The same, edge by edge and in order along each.
    crossings: Vec<Crossing>,
    /// Where each edge's crossings begin in `crossings`.
    crossing_starts: Vec<usize>,
    /// Per contour, the edge a walk starts at and the winding beside its start, once known.
    entries: Vec<Option<(usize, i32)>>,
    /// The contours whose entry is known and whose walk is owed.
    queue: Vec<usize>,
    /// The last ray cast, which is what [`Union::stamps`] are marked with.
    ray: u32,
    /// Per vertex count and sums of coordinates, the last contour with them, while twins are
    /// merged: what a contour is compared with before its vertices are.
    twin_keys: std::collections::HashMap<(usize, i64, i64), u32>,
    /// Per contour, the one before it with the same key, while twins are merged.
    same_key: Vec<Option<u32>>,
    /// The boundary, as directed device-space edges.
    pub(super) boundary: Vec<[Point; 2]>,
}

impl super::Edges for Union {
    fn line(&mut self, from: Point, to: Point) {
        self.lines.push([from, to]);
    }

    fn subpath(&mut self) {
        self.subpaths.push(self.lines.len());
    }
}

impl Union {
    /// Forgets the last mark.
    pub(super) fn start(&mut self) {
        self.lines.clear();
        self.subpaths.clear();
        self.subpaths.push(0);
        self.boundary.clear();
    }

    /// Replaces the traced edges by the boundary of the set they fill under the non-zero rule,
    /// left in [`Union::boundary`]; the condition the mark is declined for where it is — see the
    /// module's comment.
    pub(super) fn settle(&mut self) -> Result<(), UnionCondition> {
        match self.settle_as(Twins::Apart) {
            // Only where the walk could not place the mark, so that a mark it measures stays
            // measured to the bit: a twin is what both of these are where a path retraces itself.
            Err(UnionCondition::NoStartingPoint | UnionCondition::CrossingAlongALine)
                if self.has_twins() =>
            {
                self.settle_as(Twins::Merged)
            }
            settled => settled,
        }
    }

    /// The pieces the traced path states: its subpaths that hold an edge.
    pub(super) fn pieces(&self) -> usize {
        let spans = self
            .subpaths
            .windows(2)
            .filter(|pair| matches!(pair, [from, until] if from < until))
            .count();
        let open = self
            .subpaths
            .last()
            .is_some_and(|&last| last < self.lines.len());
        spans.saturating_add(usize::from(open))
    }

    /// [`Union::settle`], with twin contours apart or merged.
    fn settle_as(&mut self, twins: Twins) -> Result<(), UnionCondition> {
        self.boundary.clear();
        self.contours_on_the_grid(twins)?;
        let Some(grid) = self.chunk() else {
            return Ok(());
        };
        if !self.file(&grid) {
            return Err(UnionCondition::PastTheBudget);
        }
        self.cross(&grid)?;
        self.order_crossings();
        self.entries.clear();
        self.entries.resize(self.contours.len(), None);
        self.stamps.clear();
        self.stamps.resize(self.chunks.len(), 0);
        self.ray = 0;
        for seed in 0..self.contours.len() {
            if self.entries.get(seed).is_some_and(Option::is_some) {
                continue;
            }
            match self.ray_entry(seed, &grid) {
                Entry::Found(edge, winding) => {
                    if let Some(entry) = self.entries.get_mut(seed) {
                        *entry = Some((edge, winding));
                    }
                }
                // A contour of horizontal edges alone encloses nothing.
                Entry::Nothing => continue,
                Entry::Degenerate => return Err(UnionCondition::NoStartingPoint),
            }
            self.queue.clear();
            self.queue.push(seed);
            while let Some(contour) = self.queue.pop() {
                let closes = self.walk(contour);
                debug_assert!(
                    closes,
                    "the walk of contour {contour} does not close (ADR 2268)"
                );
                if !closes {
                    return Err(UnionCondition::WalkDoesNotReturn);
                }
            }
        }
        Ok(())
    }

    /// Whether two of the contours found are the same vertices in the same order round.
    fn has_twins(&mut self) -> bool {
        self.twin_keys.clear();
        self.same_key.clear();
        (0..self.contours.len()).any(|contour| self.twin_of(contour).is_some())
    }

    /// The contour before `contour` that is its twin, filing `contour` under its key where none
    /// is: the same number of vertices, the same sums of their coordinates, and then the same
    /// vertices from one of its own.
    fn twin_of(&mut self, contour: usize) -> Option<usize> {
        let &(first, end) = self.contours.get(contour)?;
        let here = self.vertices.get(first..end)?;
        let key = here
            .iter()
            .fold((here.len(), 0_i64, 0_i64), |(n, x, y), v| {
                (n, x.wrapping_add(v.x), y.wrapping_add(v.y))
            });
        let mut candidate = self.twin_keys.get(&key).copied();
        while let Some(earlier) = candidate {
            if let Some(&(from, until)) = self.contours.get(earlier as usize)
                && let Some(there) = self.vertices.get(from..until)
                && same_round(here, there)
            {
                return Some(earlier as usize);
            }
            candidate = self.same_key.get(earlier as usize).copied().flatten();
        }
        let filed = u32::try_from(contour).ok()?;
        let before = self.twin_keys.insert(key, filed);
        if self.same_key.len() <= contour {
            self.same_key.resize(contour.saturating_add(1), None);
        }
        if let Some(slot) = self.same_key.get_mut(contour) {
            *slot = before;
        }
        None
    }

    /// The traced subpaths as closed contours of vertices on the grid, each vertex once: a vertex
    /// the grid cannot tell from the one before it is that one, and a contour of fewer than three
    /// encloses nothing and changes the winding of no point off a line. A coordinate that is not
    /// finite or is off the grid declines the mark.
    ///
    /// With [`Twins::Merged`] a contour that is the twin of one before it is not kept, and the
    /// earlier one's weight counts it instead — up to `i8::MAX`, past which a twin stays a contour
    /// of its own (see the module's comment).
    fn contours_on_the_grid(&mut self, twins: Twins) -> Result<(), UnionCondition> {
        self.vertices.clear();
        self.edges.clear();
        self.contours.clear();
        self.twin_keys.clear();
        self.same_key.clear();
        let (mut left, mut top) = (f32::INFINITY, f32::INFINITY);
        for &[from, to] in &self.lines {
            left = left.min(from.x).min(to.x);
            top = top.min(from.y).min(to.y);
        }
        if !(left.is_finite() && top.is_finite()) {
            return if self.lines.is_empty() {
                Ok(())
            } else {
                Err(UnionCondition::OffTheGrid)
            };
        }
        let origin = (left.floor(), top.floor());
        self.subpaths.push(self.lines.len());
        for index in 0..self.subpaths.len().saturating_sub(1) {
            let (Some(&from), Some(&until)) = (
                self.subpaths.get(index),
                self.subpaths.get(index.saturating_add(1)),
            ) else {
                continue;
            };
            let first = self.vertices.len();
            for line in self.lines.get(from..until).unwrap_or_default() {
                let Some(vertex) = on_grid(line[0], origin) else {
                    return Err(UnionCondition::OffTheGrid);
                };
                let repeated = self
                    .vertices
                    .get(first..)
                    .and_then(<[Vertex]>::last)
                    .is_some_and(|last| last.x == vertex.x && last.y == vertex.y);
                if !repeated {
                    self.vertices.push(vertex);
                }
            }
            // The closing edge ends at the first vertex, so a last vertex the grid puts there is
            // the first one.
            while self.vertices.len() > first.saturating_add(1)
                && self
                    .vertices
                    .last()
                    .zip(self.vertices.get(first))
                    .is_some_and(|(last, start)| last.x == start.x && last.y == start.y)
            {
                self.vertices.pop();
            }
            let end = self.vertices.len();
            if end.saturating_sub(first) < 3 {
                self.vertices.truncate(first);
                continue;
            }
            let Ok(contour) = u32::try_from(self.contours.len()) else {
                return Err(UnionCondition::PastTheBudget);
            };
            if twins == Twins::Merged {
                self.contours.push((first, end));
                let twin = self.twin_of(self.contours.len().saturating_sub(1));
                self.contours.pop();
                if let Some(twin) = twin
                    && self.add_weight(twin)
                {
                    self.vertices.truncate(first);
                    continue;
                }
            }
            for vertex in first..end {
                let after = if vertex.saturating_add(1) == end {
                    first
                } else {
                    vertex.saturating_add(1)
                };
                let (Some(a), Some(b)) = (self.vertices.get(vertex), self.vertices.get(after))
                else {
                    return Err(UnionCondition::OffTheGrid);
                };
                let from = Site {
                    x: a.x,
                    y: a.y,
                    index: vertex,
                };
                let to = Site {
                    x: b.x,
                    y: b.y,
                    index: after,
                };
                self.edges.push(Edge {
                    from,
                    to,
                    bounds: Bounds::of(from, to),
                    contour,
                    weight: 1,
                });
            }
            self.contours.push((first, end));
        }
        Ok(())
    }

    /// Counts one more statement of `contour` in its edges' weight; `false` where that is past
    /// `i8::MAX`.
    fn add_weight(&mut self, contour: usize) -> bool {
        let Some(&(first, end)) = self.contours.get(contour) else {
            return false;
        };
        let Some(edges) = self.edges.get_mut(first..end) else {
            return false;
        };
        if edges.iter().any(|edge| edge.weight == i8::MAX) {
            return false;
        }
        for edge in edges {
            edge.weight = edge.weight.saturating_add(1);
        }
        true
    }

    /// Cuts every contour into chunks of at most [`CHUNK`] consecutive edges, and lays the grid
    /// over them; `None` for a mark with no contour.
    fn chunk(&mut self) -> Option<Grid> {
        self.chunks.clear();
        for &(first, end) in &self.contours {
            let mut start = first;
            while start < end {
                let stop = start.saturating_add(CHUNK).min(end);
                let bounds = self
                    .edges
                    .get(start..stop)
                    .unwrap_or_default()
                    .iter()
                    .map(|edge| edge.bounds)
                    .reduce(Bounds::and)?;
                self.chunks.push(Chunk {
                    first: u32::try_from(start).ok()?,
                    end: u32::try_from(stop).ok()?,
                    bounds,
                    cells: [0; 4],
                });
                start = stop;
            }
        }
        let grid = Grid::over(&self.chunks)?;
        for chunk in &mut self.chunks {
            let (left, top) = grid.at(chunk.bounds.left, chunk.bounds.top);
            let (right, bottom) = grid.at(chunk.bounds.right, chunk.bounds.bottom);
            chunk.cells = [left, top, right, bottom];
        }
        Some(grid)
    }

    /// Files every chunk under the cells its box reaches.
    fn file(&mut self, grid: &Grid) -> bool {
        self.cell_starts.clear();
        self.cell_starts.resize(grid.cells().saturating_add(1), 0);
        let mut filed = 0_usize;
        for chunk in &self.chunks {
            let [left, top, right, bottom] = chunk.cells;
            for row in top..=bottom {
                for column in left..=right {
                    if let Some(count) = self
                        .cell_starts
                        .get_mut(grid.cell(column as usize, row as usize).saturating_add(1))
                    {
                        *count = count.saturating_add(1);
                    }
                    filed = filed.saturating_add(1);
                }
            }
        }
        if filed > UNION_WORK {
            return false;
        }
        let mut running = 0_usize;
        for start in &mut self.cell_starts {
            running = running.saturating_add(*start);
            *start = running;
        }
        self.filed.clear();
        self.filed.resize(running, 0);
        // Each cell's start is its cursor while the chunks are placed, and ends the pass as the
        // next cell's start, which the rotation below puts back.
        for (index, chunk) in self.chunks.iter().enumerate() {
            let Ok(index) = u32::try_from(index) else {
                return false;
            };
            let [left, top, right, bottom] = chunk.cells;
            for row in top..=bottom {
                for column in left..=right {
                    let cell = grid.cell(column as usize, row as usize);
                    if let Some(cursor) = self.cell_starts.get_mut(cell) {
                        if let Some(slot) = self.filed.get_mut(*cursor) {
                            *slot = index;
                        }
                        *cursor = cursor.saturating_add(1);
                    }
                }
            }
        }
        self.cell_starts.rotate_right(1);
        if let Some(first) = self.cell_starts.first_mut() {
            *first = 0;
        }
        true
    }

    /// Finds every crossing of two edges that do not share a vertex. Two chunks are compared in
    /// the one cell where both their reaches begin, so a pair filed together in several cells is
    /// compared once; the edges of one chunk are compared with one another once.
    fn cross(&mut self, grid: &Grid) -> Result<(), UnionCondition> {
        let Self {
            vertices,
            edges,
            chunks,
            filed,
            cell_starts,
            found,
            ..
        } = self;
        found.clear();
        let mut work = 0_usize;
        let mut pairs = Pairs {
            vertices,
            edges,
            found,
            work: &mut work,
        };
        for chunk in chunks.iter() {
            pairs.compare(chunk, chunk)?;
        }
        for (cell, reach) in cell_starts.windows(2).enumerate() {
            let &[from, until] = reach else {
                continue;
            };
            let here = filed.get(from..until).unwrap_or_default();
            for (rank, &one) in here.iter().enumerate() {
                let Some(a) = chunks.get(one as usize) else {
                    continue;
                };
                for &other in here.get(rank.saturating_add(1)..).unwrap_or_default() {
                    let Some(b) = chunks.get(other as usize) else {
                        continue;
                    };
                    if !a.bounds.meets(b.bounds)
                        || grid.cell(
                            a.cells[0].max(b.cells[0]) as usize,
                            a.cells[1].max(b.cells[1]) as usize,
                        ) != cell
                    {
                        continue;
                    }
                    pairs.compare(a, b)?;
                }
            }
        }
        Ok(())
    }

    /// Takes the crossings found into [`Union::crossings`], edge by edge and in order along each.
    fn order_crossings(&mut self) {
        self.crossing_starts.clear();
        self.crossing_starts
            .resize(self.edges.len().saturating_add(1), 0);
        for crossing in &self.found {
            if let Some(count) = self
                .crossing_starts
                .get_mut((crossing.edge as usize).saturating_add(1))
            {
                *count = count.saturating_add(1);
            }
        }
        let mut running = 0_usize;
        for start in &mut self.crossing_starts {
            running = running.saturating_add(*start);
            *start = running;
        }
        self.crossings.clear();
        self.crossings.extend_from_slice(&self.found);
        for crossing in &self.found {
            if let Some(cursor) = self.crossing_starts.get_mut(crossing.edge as usize) {
                if let Some(slot) = self.crossings.get_mut(*cursor) {
                    *slot = *crossing;
                }
                *cursor = cursor.saturating_add(1);
            }
        }
        self.crossing_starts.rotate_right(1);
        if let Some(first) = self.crossing_starts.first_mut() {
            *first = 0;
        }
        for edge in 0..self.edges.len() {
            if let (Some(&from), Some(&until)) = (
                self.crossing_starts.get(edge),
                self.crossing_starts.get(edge.saturating_add(1)),
            ) && let Some(list) = self.crossings.get_mut(from..until)
            {
                in_order(list);
            }
        }
    }

    /// Where one edge's crossings begin and end in [`Union::crossings`].
    fn crossings_of(&self, edge: usize) -> (usize, usize) {
        match (
            self.crossing_starts.get(edge),
            self.crossing_starts.get(edge.saturating_add(1)),
        ) {
            (Some(&from), Some(&until)) => (from, until),
            _ => (0, 0),
        }
    }

    /// Where a walk of `contour` starts, from a ray to a point on one of its edges that is not
    /// horizontal: the winding beside the point, counted along a horizontal ray from outside the
    /// mark, less every crossing between the edge's start and the point.
    ///
    /// The ray's own end is the point moved by the smallest infinitesimal of all, so it lies
    /// beside the edge, and the winding it reads is that side's only where no other edge passes
    /// through the unmoved point. So the point is tried at the edge's midpoint and then at its
    /// quarters, eighths and so on to [`RAY_DEPTH`], on a grid that much finer, where two pieces'
    /// edges along one line — which the pieces of a straight stretch are — leave the point on
    /// one of them; and then at the next edge.
    ///
    /// A piece whose every upright edge lies along other pieces' edges — a thin one between two
    /// neighbours — has no such point on them, so its horizontal edges are tried after them, each
    /// from a vertical ray, in the same way (ADR 2281). Only after: a mark the horizontal rays
    /// enter is entered as it was.
    fn ray_entry(&mut self, contour: usize, grid: &Grid) -> Entry {
        let Some(&(first, end)) = self.contours.get(contour) else {
            return Entry::Nothing;
        };
        let upright = |edge: &Edge| edge.from.y != edge.to.y;
        if !self
            .edges
            .get(first..end)
            .unwrap_or_default()
            .iter()
            .any(upright)
        {
            return Entry::Nothing;
        }
        for across in [Ray::Across, Ray::Down] {
            for reference in first..end {
                let Some(&e) = self.edges.get(reference) else {
                    continue;
                };
                // A ray is never along the edge it reads the winding beside.
                if upright(&e) != (across == Ray::Across) {
                    continue;
                }
                if let Some(entry) = self.entry_beside((reference, e), across, grid) {
                    return entry;
                }
            }
        }
        Entry::Degenerate
    }

    /// [`Union::ray_entry`] on one edge, from rays `across` it: the first point tried that no other
    /// edge passes through.
    fn entry_beside(
        &mut self,
        (reference, e): (usize, Edge),
        across: Ray,
        grid: &Grid,
    ) -> Option<Entry> {
        for depth in 1..=RAY_DEPTH {
            let denominator = 1_i64 << depth;
            for numerator in (1..denominator).step_by(2) {
                let along = (numerator, denominator);
                let point = Site {
                    x: e.from
                        .x
                        .wrapping_mul(denominator.wrapping_sub(numerator))
                        .wrapping_add(e.to.x.wrapping_mul(numerator)),
                    y: e.from
                        .y
                        .wrapping_mul(denominator.wrapping_sub(numerator))
                        .wrapping_add(e.to.y.wrapping_mul(numerator)),
                    index: self.vertices.len(),
                };
                let Some(beside) = self.winding_at((point, depth), (reference, across), grid)
                else {
                    continue;
                };
                let (from, to) = (finer(e.from, depth), finer(e.to, depth));
                // The negative side of an edge winds its weight more than its positive side.
                let mut winding = if positive(from, to, point) {
                    beside
                } else {
                    beside.saturating_sub(i32::from(e.weight))
                };
                let (start, until) = self.crossings_of(reference);
                for index in start..until {
                    let Some(&crossing) = self.crossings.get(index) else {
                        continue;
                    };
                    let Some(&f) = self.edges.get(crossing.other as usize) else {
                        continue;
                    };
                    if before_the_point(e, f, (point, depth), along) {
                        winding = winding.saturating_sub(i32::from(crossing.step));
                    }
                }
                return Some(Entry::Found(reference, winding));
            }
        }
        None
    }

    /// The winding at `point`, on the grid `depth` binary places finer, moved by the smallest
    /// infinitesimal: the crossings of a ray `across` the mark to it from outside, from the nearer
    /// side; `None` where an edge other than `reference` passes through the unmoved point.
    fn winding_at(
        &mut self,
        (point, depth): (Site, u32),
        (reference, across): (usize, Ray),
        grid: &Grid,
    ) -> Option<i32> {
        // A ray of its own for every attempt, so that one declined at a point another edge passes
        // through leaves no chunk marked as counted for the next.
        self.ray = self.ray.saturating_add(1);
        let ray = self.ray;
        let coarse = (point.x >> depth, point.y >> depth);
        let side = Side::nearer(across, coarse, grid);
        let outside = match side {
            Side::Left => Site {
                x: (grid.left.wrapping_sub(2)) << depth,
                ..point
            },
            Side::Right => Site {
                x: (grid.right.wrapping_add(2)) << depth,
                ..point
            },
            Side::Top => Site {
                y: (grid.top.wrapping_sub(2)) << depth,
                ..point
            },
            Side::Bottom => Site {
                y: (grid.bottom.wrapping_add(2)) << depth,
                ..point
            },
        };
        let outside = Site {
            index: point.index.saturating_add(1),
            ..outside
        };
        let (column, row) = grid.at(coarse.0, coarse.1);
        let (column, row) = (column as usize, row as usize);
        let reached = match side {
            Side::Left => column.saturating_add(1),
            Side::Right => grid.columns.saturating_sub(column),
            Side::Top => row.saturating_add(1),
            Side::Bottom => grid.rows.saturating_sub(row),
        };
        let alone = Bounds::of(point, point);
        let mut winding = 0_i32;
        for step in 0..reached {
            let cell = match side {
                Side::Left => grid.cell(column.saturating_sub(step), row),
                Side::Right => grid.cell(column.saturating_add(step), row),
                Side::Top => grid.cell(column, row.saturating_sub(step)),
                Side::Bottom => grid.cell(column, row.saturating_add(step)),
            };
            let (Some(&from), Some(&until)) = (
                self.cell_starts.get(cell),
                self.cell_starts.get(cell.saturating_add(1)),
            ) else {
                continue;
            };
            for slot in from..until {
                let Some(&index) = self.filed.get(slot) else {
                    continue;
                };
                let Some(stamp) = self.stamps.get_mut(index as usize) else {
                    continue;
                };
                if *stamp == ray {
                    continue;
                }
                *stamp = ray;
                let Some(&chunk) = self.chunks.get(index as usize) else {
                    continue;
                };
                if !reaches(chunk.bounds, coarse, side) {
                    continue;
                }
                for edge in chunk.first..chunk.end {
                    let Some(&f) = self.edges.get(edge as usize) else {
                        continue;
                    };
                    if !reaches(f.bounds, coarse, side) {
                        continue;
                    }
                    let (f0, f1) = (finer(f.from, depth), finer(f.to, depth));
                    if edge as usize != reference
                        && Bounds::of(f0, f1).meets(alone)
                        && determinant(f0, f1, point) == 0
                    {
                        return None;
                    }
                    if positive(outside, point, f0) == positive(outside, point, f1) {
                        continue;
                    }
                    let (from_outside, at_point) =
                        (positive(f0, f1, outside), positive(f0, f1, point));
                    if from_outside != at_point {
                        let weight = i32::from(f.weight);
                        winding = if from_outside {
                            winding.saturating_add(weight)
                        } else {
                            winding.saturating_sub(weight)
                        };
                    }
                }
            }
        }
        Some(winding)
    }

    /// Walks one contour from its entry, leaving the stretches of its edges that bound the filled
    /// set in [`Union::boundary`] and the entries of the contours it crosses; `false` where the
    /// winding does not come back to itself round the contour.
    fn walk(&mut self, contour: usize) -> bool {
        let Some(&Some((entry, start))) = self.entries.get(contour) else {
            return false;
        };
        let Some(&(first, end)) = self.contours.get(contour) else {
            return false;
        };
        let count = end.saturating_sub(first);
        let mut winding = start;
        for offset in 0..count {
            let edge = first.saturating_add(
                entry
                    .saturating_sub(first)
                    .saturating_add(offset)
                    .checked_rem(count)
                    .unwrap_or(0),
            );
            winding = self.bound(edge, winding);
        }
        winding == start
    }

    /// Deposits the stretches of `edge` that bound the set, from the winding on its positive side
    /// at its start, and gives every contour it crosses whose entry is not yet known the one this
    /// crossing fixes; the winding at its end.
    fn bound(&mut self, edge: usize, mut winding: i32) -> i32 {
        let Some(&e) = self.edges.get(edge) else {
            return winding;
        };
        let (Some(from), Some(to)) = (
            self.vertices.get(e.from.index).map(|v| v.at),
            self.vertices.get(e.to.index).map(|v| v.at),
        ) else {
            return winding;
        };
        let (first, until) = self.crossings_of(edge);
        let mut stretch_from = from;
        let mut side = side_of(winding, e.weight);
        for index in first..until {
            let Some(&crossing) = self.crossings.get(index) else {
                continue;
            };
            self.enter_across(crossing, (index, first, until), winding);
            winding = winding.saturating_add(i32::from(crossing.step));
            let now = side_of(winding, e.weight);
            if now != side {
                deposit(&mut self.boundary, side, stretch_from, crossing.at);
                stretch_from = crossing.at;
                side = now;
            }
        }
        deposit(&mut self.boundary, side, stretch_from, to);
        winding
    }

    /// Gives the contour of `crossing.other` its entry, where it has none and the crossing is
    /// separated from its neighbours on both edges: `winding` is the positive side's just before
    /// the crossing, walking `crossing.edge`.
    ///
    /// Round the crossing point the two edges make four windings, and the negative side of each
    /// edge winds its weight more than its positive side, so the one beside the walked edge just
    /// before the crossing fixes all four, and with them the winding beside the other edge just
    /// before the crossing; every crossing earlier on the other edge is then taken back off.
    fn enter_across(
        &mut self,
        crossing: Crossing,
        (index, first, until): (usize, usize, usize),
        winding: i32,
    ) {
        let Some(&f) = self.edges.get(crossing.other as usize) else {
            return;
        };
        let contour = f.contour as usize;
        if !self.entries.get(contour).is_some_and(Option::is_none)
            || !separated(&self.crossings, (index, first, until))
        {
            return;
        }
        let (from, until_f) = self.crossings_of(crossing.other as usize);
        let Some(twin) = (from..until_f).find(|&slot| {
            self.crossings
                .get(slot)
                .is_some_and(|other| other.other == crossing.edge)
        }) else {
            return;
        };
        if !separated(&self.crossings, (twin, from, until_f)) {
            return;
        }
        let Some(&twin_crossing) = self.crossings.get(twin) else {
            return;
        };
        // The walked edge's start lies on the other edge's positive side exactly where crossing
        // the other edge adds to the winding, and likewise the other edge's start and the walked
        // edge; what a crossing adds is the crossed edge's weight.
        let both_positive = if crossing.step > 0 {
            winding
        } else {
            winding.saturating_add(i32::from(crossing.step))
        };
        let mut beside = if twin_crossing.step > 0 {
            both_positive
        } else {
            both_positive.saturating_sub(i32::from(twin_crossing.step))
        };
        for slot in from..twin {
            if let Some(earlier) = self.crossings.get(slot) {
                beside = beside.saturating_sub(i32::from(earlier.step));
            }
        }
        if let Some(entry) = self.entries.get_mut(contour) {
            *entry = Some((crossing.other as usize, beside));
            self.queue.push(contour);
        }
    }
}

/// What [`Union::cross`] compares edges with: the mark's vertices and edges, and where the
/// crossings it finds go.
struct Pairs<'a> {
    /// The contours' vertices.
    vertices: &'a [Vertex],
    /// Every edge.
    edges: &'a [Edge],
    /// The crossings found so far.
    found: &'a mut Vec<Crossing>,
    /// The pairs compared so far; see [`UNION_WORK`].
    work: &'a mut usize,
}

impl Pairs<'_> {
    /// Records every crossing between an edge of `a` and an edge of `b` — of one chunk with
    /// itself where the two are the same, each pair once; declined where [`UNION_WORK`] is spent
    /// or two edges cross along one line.
    fn compare(&mut self, a: &Chunk, b: &Chunk) -> Result<(), UnionCondition> {
        let same = a.first == b.first;
        let Some(theirs) = self.edges.get(b.first as usize..b.end as usize) else {
            return Ok(());
        };
        for one in a.first..a.end {
            let Some(e) = self.edges.get(one as usize) else {
                continue;
            };
            if !e.bounds.meets(b.bounds) {
                continue;
            }
            let skip = if same {
                one.saturating_add(1).saturating_sub(b.first) as usize
            } else {
                0
            };
            for (offset, f) in theirs.iter().enumerate().skip(skip) {
                *self.work = self.work.saturating_add(1);
                if *self.work > UNION_WORK {
                    return Err(UnionCondition::PastTheBudget);
                }
                if !e.bounds.meets(f.bounds) || shares_a_vertex(e, f) {
                    continue;
                }
                let Ok(offset) = u32::try_from(offset) else {
                    continue;
                };
                match self.crossing((one, e), (b.first.saturating_add(offset), f)) {
                    Meeting::None => {}
                    Meeting::Collinear => return Err(UnionCondition::CrossingAlongALine),
                    Meeting::At(on_one, on_other) => {
                        self.found.push(on_one);
                        self.found.push(on_other);
                    }
                }
            }
        }
        Ok(())
    }

    /// Whether the edges `one` and `other` cross, and where, as each of them sees it.
    fn crossing(&self, (one, e): (u32, &Edge), (other, f): (u32, &Edge)) -> Meeting {
        // The exact determinant answers the side wherever it is not zero, and the simulation is
        // asked only where it is, which is what `positive` does in that order too.
        let (d1, d2) = (
            determinant(e.from, e.to, f.from),
            determinant(e.from, e.to, f.to),
        );
        let s1 = side(d1, (e.from, e.to, f.from));
        if s1 == side(d2, (e.from, e.to, f.to)) {
            return Meeting::None;
        }
        let (d3, d4) = (
            determinant(f.from, f.to, e.from),
            determinant(f.from, f.to, e.to),
        );
        let s3 = side(d3, (f.from, f.to, e.from));
        if s3 == side(d4, (f.from, f.to, e.to)) {
            return Meeting::None;
        }
        let (Some(t_one), Some(t_other)) = (ratio(d3, d4), ratio(d1, d2)) else {
            return Meeting::Collinear;
        };
        let (Some(from), Some(to)) = (
            self.vertices.get(e.from.index),
            self.vertices.get(e.to.index),
        ) else {
            return Meeting::None;
        };
        let at = between(from.at, to.at, t_one);
        Meeting::At(
            Crossing {
                edge: one,
                other,
                t: t_one,
                step: if s3 {
                    f.weight
                } else {
                    f.weight.saturating_neg()
                },
                at,
            },
            Crossing {
                edge: other,
                other: one,
                t: t_other,
                step: if s1 {
                    e.weight
                } else {
                    e.weight.saturating_neg()
                },
                at,
            },
        )
    }
}

/// Sorts one edge's crossings along it. An edge holds a handful, for which an insertion sort is
/// a third of what the general sort spends on them: `inks.pdf`'s three draws sort 742 000
/// crossings in groups of seven on average, and the general sort's small-slice path was 90
/// million instructions of them, measured under callgrind.
fn in_order(list: &mut [Crossing]) {
    const BY_INSERTION: usize = 24;
    if list.len() > BY_INSERTION {
        list.sort_unstable_by(|a, b| a.t.total_cmp(&b.t));
        return;
    }
    for next in 1..list.len() {
        let mut at = next;
        while at > 0
            && list
                .get(at.saturating_sub(1))
                .zip(list.get(at))
                .is_some_and(|(before, here)| before.t > here.t)
        {
            list.swap(at.saturating_sub(1), at);
            at = at.saturating_sub(1);
        }
    }
}

/// Whether contours stated twice are walked apart or as one of twice the weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Twins {
    /// Every contour as the path states it.
    Apart,
    /// A contour stated again, vertex for vertex, counted in the weight of the first.
    Merged,
}

/// Why the union of a mark's pieces declined it — what [`crate::UnionDecline`] names (ADR 2280).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnionCondition {
    /// A coordinate is not finite, or lies past the grid the predicates are exact on.
    OffTheGrid,
    /// The mark costs more than `UNION_WORK` pairs of edges, or more cells than an accumulator
    /// states: a cost guard, not a condition of the mark.
    PastTheBudget,
    /// Two edges cross along a common line, where the crossing has no position.
    CrossingAlongALine,
    /// A contour has no point to start its walk from: every point tried on every one of its edges
    /// lies on another edge as well.
    NoStartingPoint,
    /// A walk does not return to the winding it started from, which no consistent arithmetic
    /// allows: a defect, and a build with debug assertions stops on it.
    WalkDoesNotReturn,
    /// A row of the boundary's accumulator does not sum to nothing, which a boundary the walk
    /// returns cannot do: a defect, and a build with debug assertions stops on it.
    RowDoesNotClose,
}

impl UnionCondition {
    /// Whether no input should reach it: the walk's arithmetic has gone wrong, and the mark is
    /// declined only because a release build draws it rather than stop.
    #[must_use]
    pub fn is_defect(self) -> bool {
        matches!(self, Self::WalkDoesNotReturn | Self::RowDoesNotClose)
    }
}

impl std::fmt::Display for UnionCondition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::OffTheGrid => "a coordinate lies off the grid the union is exact on",
            Self::PastTheBudget => "it costs more than the union's budget",
            Self::CrossingAlongALine => "two of its edges cross along a common line",
            Self::NoStartingPoint => {
                "every edge of one of its pieces lies along another's, so its walk has no start"
            }
            Self::WalkDoesNotReturn => {
                "a walk did not return to its winding, which is a defect of the union"
            }
            Self::RowDoesNotClose => {
                "a row of its boundary did not close, which is a defect of the union"
            }
        })
    }
}

/// Whether `here` and `there` are the same vertices in the same order round, from any start.
fn same_round(here: &[Vertex], there: &[Vertex]) -> bool {
    let same = |a: &Vertex, b: &Vertex| a.x == b.x && a.y == b.y;
    let (Some(start), true) = (here.first(), here.len() == there.len()) else {
        return false;
    };
    (0..there.len()).any(|offset| {
        there.get(offset).is_some_and(|v| same(v, start))
            && here.iter().enumerate().all(|(index, v)| {
                there
                    .get(
                        offset
                            .saturating_add(index)
                            .checked_rem(there.len())
                            .unwrap_or(0),
                    )
                    .is_some_and(|w| same(v, w))
            })
    })
}

/// How a walk of a contour is started.
enum Entry {
    /// From this edge, with this winding beside its start.
    Found(usize, i32),
    /// Not at all: the contour encloses nothing.
    Nothing,
    /// Not at all: every midpoint lies on another edge.
    Degenerate,
}

/// Whether the crossing at `index` lies more than [`SEPARATION`] from the crossings beside it on
/// its edge, whose crossings are `first..until`.
fn separated(crossings: &[Crossing], (index, first, until): (usize, usize, usize)) -> bool {
    let Some(here) = crossings.get(index).map(|crossing| crossing.t) else {
        return false;
    };
    let before = index > first
        && crossings
            .get(index.saturating_sub(1))
            .is_some_and(|crossing| here - crossing.t <= SEPARATION);
    let after = index.saturating_add(1) < until
        && crossings
            .get(index.saturating_add(1))
            .is_some_and(|crossing| crossing.t - here <= SEPARATION);
    !(before || after)
}

/// Whether a box reaches a ray from `(x, y)` out to `side`.
fn reaches(bounds: Bounds, (x, y): (i64, i64), side: Side) -> bool {
    match side {
        Side::Left | Side::Right => {
            bounds.top <= y
                && bounds.bottom >= y
                && if side == Side::Left {
                    bounds.left <= x
                } else {
                    bounds.right >= x
                }
        }
        Side::Top | Side::Bottom => {
            bounds.left <= x
                && bounds.right >= x
                && if side == Side::Top {
                    bounds.top <= y
                } else {
                    bounds.bottom >= y
                }
        }
    }
}

/// Which way a ray runs to the point it reads the winding at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ray {
    /// Horizontally, beside an edge that is not horizontal.
    Across,
    /// Vertically, beside a horizontal edge.
    Down,
}

/// The side of the mark a ray comes in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// From the left, horizontally.
    Left,
    /// From the right, horizontally.
    Right,
    /// From above, vertically.
    Top,
    /// From below, vertically.
    Bottom,
}

impl Side {
    /// The nearer side for a ray `across` to the grid point `(x, y)`, which crosses fewer cells.
    fn nearer(across: Ray, (x, y): (i64, i64), grid: &Grid) -> Self {
        match across {
            Ray::Across if x.wrapping_sub(grid.left) <= grid.right.wrapping_sub(x) => Self::Left,
            Ray::Across => Self::Right,
            Ray::Down if y.wrapping_sub(grid.top) <= grid.bottom.wrapping_sub(y) => Self::Top,
            Ray::Down => Self::Bottom,
        }
    }
}

/// What a stretch of an edge of `weight` is to the filled set, from the winding on its positive
/// side, the negative side winding `weight` more: `1` where the positive side winds zero, `-1`
/// where the negative side does, and `0` where neither does and the edge bounds nothing.
fn side_of(positive_side: i32, weight: i8) -> i8 {
    if positive_side == 0 {
        1
    } else if positive_side == i32::from(weight).saturating_neg() {
        -1
    } else {
        0
    }
}

/// The stretch from `from` to `to` as a boundary edge directed so that the filled side is the
/// side it encloses, or nothing.
fn deposit(boundary: &mut Vec<[Point; 2]>, side: i8, from: Point, to: Point) {
    match side {
        1 => boundary.push([from, to]),
        -1 => boundary.push([to, from]),
        _ => {}
    }
}

/// Whether the edge `f` crosses the edge `e` before `point`, which lies `along = (p, q)` of the
/// way along `e` on the grid `depth` places finer, walking `e` from its start — exactly, a
/// crossing at the point itself decided by which side of `f` the moved point lies on.
fn before_the_point(e: Edge, f: Edge, (point, depth): (Site, u32), (p, q): (i64, i64)) -> bool {
    let (o0, o1) = (
        determinant(f.from, f.to, e.from),
        determinant(f.from, f.to, e.to),
    );
    // The crossing is at `o0 / (o0 - o1)` along `e`, against `p / q`; the sign of the
    // denominator decides which way the comparison runs. `q` is at most 2^RAY_DEPTH, so the
    // products stay inside 2^86.
    let denominator = o0.wrapping_sub(o1);
    let (scaled, against) = (
        o0.wrapping_mul(i128::from(q)),
        denominator.wrapping_mul(i128::from(p)),
    );
    let order = if denominator > 0 {
        scaled.cmp(&against)
    } else {
        against.cmp(&scaled)
    };
    match order {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            let (f0, f1) = (finer(f.from, depth), finer(f.to, depth));
            positive(f0, f1, point) != positive(f0, f1, finer(e.from, depth))
        }
    }
}

/// A site on the grid `depth` binary places finer, which every predicate answers as it answers
/// the site itself: the determinants are scaled by a positive power of two and the coordinate
/// differences keep their signs.
fn finer(site: Site, depth: u32) -> Site {
    Site {
        x: site.x << depth,
        y: site.y << depth,
        index: site.index,
    }
}

/// What two edges' meeting is.
enum Meeting {
    /// They do not cross.
    None,
    /// They cross along a common line, where the crossing has no position.
    Collinear,
    /// They cross, as the first and as the second sees it.
    At(Crossing, Crossing),
}

/// `numerator / (numerator - other)`, the parameter at which a line between two sides of another
/// meets it; `None` where both are zero, which is the two edges along one line.
fn ratio(numerator: i128, other: i128) -> Option<f64> {
    let denominator = numerator.wrapping_sub(other);
    (denominator != 0).then(|| (approximate(numerator) / approximate(denominator)).clamp(0.0, 1.0))
}

/// A determinant as an `f64`, from two parts: two conversions the processor makes in one
/// instruction each, where the whole `i128` is a call into the compiler's runtime — 67 million
/// instructions of `inks.pdf`'s three draws, measured under callgrind.
///
/// The parts are split at bit 32, so that each converts exactly whatever the value's sign — the
/// high part of a value under `2^78` is under `2^46` and the low part is under `2^32` — and the
/// sum is the one rounding. Split at bit 64, a small negative value's low part is `2^64` less its
/// magnitude, which rounds by up to `2^10` before `2^64` is taken off again: two crossings of one
/// point read `2.3e-9` apart, past [`SEPARATION`], and a contour's walk started from the wrong
/// winding (ADR 2268).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the high part of a value under 2^78 is under 2^46 after the shift, which an i64 \
              holds and an f64 holds exactly, and the low part is the value's own low 32 bits"
)]
fn approximate(value: i128) -> f64 {
    const LOW: f64 = 4_294_967_296.0;
    // A multiplication and an addition rather than `mul_add`, which is a call into the platform's
    // maths library on a target without the fused instruction; the product is exact, being a
    // power of two times an integer under 2^53.
    ((value >> 32) as i64 as f64) * LOW + f64::from(value as u32)
}

/// The side of a line a point lies on, from the line's exact determinant with it where that is
/// not zero, and from the simulation of simplicity where it is.
fn side(determinant: i128, (from, to, point): (Site, Site, Site)) -> bool {
    if determinant == 0 {
        positive(from, to, point)
    } else {
        determinant > 0
    }
}

/// The point `t` of the way from `from` to `to`, in `f64` and rounded once.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a device coordinate between two device coordinates, rounded once to the f32 the \
              accumulator reads"
)]
fn between(from: Point, to: Point, t: f64) -> Point {
    let (x0, y0) = (f64::from(from.x), f64::from(from.y));
    let (x1, y1) = (f64::from(to.x), f64::from(to.y));
    Point::from_xy((x0 + (x1 - x0) * t) as f32, (y0 + (y1 - y0) * t) as f32)
}

/// Whether two edges share a vertex, which is where a contour continues and not a crossing.
fn shares_a_vertex(e: &Edge, f: &Edge) -> bool {
    e.from.index == f.from.index
        || e.from.index == f.to.index
        || e.to.index == f.from.index
        || e.to.index == f.to.index
}

/// `(b - a) × (c - a)`, exactly: twice the signed area of the triangle.
fn determinant(a: Site, b: Site, c: Site) -> i128 {
    // Every coordinate is inside GRID_LIMIT = 2^38, so each difference is inside 2^39 and each
    // product inside 2^78: the arithmetic is exact and wraps nowhere.
    let (bx, by) = (
        i128::from(b.x.wrapping_sub(a.x)),
        i128::from(b.y.wrapping_sub(a.y)),
    );
    let (cx, cy) = (
        i128::from(c.x.wrapping_sub(a.x)),
        i128::from(c.y.wrapping_sub(a.y)),
    );
    bx.wrapping_mul(cy).wrapping_sub(by.wrapping_mul(cx))
}

/// Whether `c` lies on the positive side of the line from `a` to `b`, with every point moved by
/// its own infinitesimal — Edelsbrunner and Mücke's simulation of simplicity — so that the answer
/// is never "on it".
///
/// A point's `x` moves by `ε^(2^(2i))` and its `y` by `ε^(2^(2i+1))`, `i` its index. With the
/// three sorted by index, `i < j < k`, the determinant's expansion in `ε` has its terms in the
/// order of their exponents, and the sign of the first that does not vanish is the answer: the
/// determinant itself, then the coefficients of `x_i`, `y_i` and `x_j` — `y_j − y_k`, `x_k − x_j`
/// and `y_k − y_i` — and then that of `y_i x_j`, which is `−1`.
fn positive(from: Site, to: Site, point: Site) -> bool {
    let (mut low, mut middle, mut high) = (from, to, point);
    let mut flipped = false;
    if low.index > middle.index {
        std::mem::swap(&mut low, &mut middle);
        flipped = !flipped;
    }
    if middle.index > high.index {
        std::mem::swap(&mut middle, &mut high);
        flipped = !flipped;
    }
    if low.index > middle.index {
        std::mem::swap(&mut low, &mut middle);
        flipped = !flipped;
    }
    let exact = determinant(low, middle, high);
    let sign = if exact != 0 {
        exact > 0
    } else if middle.y != high.y {
        middle.y > high.y
    } else if high.x != middle.x {
        high.x > middle.x
    } else if high.y != low.y {
        high.y > low.y
    } else {
        false
    };
    sign != flipped
}

/// A device point on the grid, against `origin`; `None` where it is not finite or off the grid.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is checked against GRID_LIMIT before the cast, far inside i64"
)]
fn on_grid(at: Point, origin: (f32, f32)) -> Option<Vertex> {
    let half = f64::from(1_u32 << GRID_BITS.saturating_sub(1));
    // Rounded to an even grid point, so that the midpoint of two is a grid point too.
    let x = ((f64::from(at.x) - f64::from(origin.0)) * half).round() * 2.0;
    let y = ((f64::from(at.y) - f64::from(origin.1)) * half).round() * 2.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "GRID_LIMIT is a power of two, exact in f64"
    )]
    let limit = GRID_LIMIT as f64;
    (x.is_finite() && y.is_finite() && x.abs() < limit && y.abs() < limit).then_some(Vertex {
        x: x as i64,
        y: y as i64,
        at,
    })
}

/// The uniform grid the chunks are filed under, over their own extent.
#[derive(Clone, Copy, Debug)]
struct Grid {
    /// The smallest `x`.
    left: i64,
    /// The smallest `y`.
    top: i64,
    /// The largest `x`.
    right: i64,
    /// The largest `y`.
    bottom: i64,
    /// A cell's side.
    side: i64,
    /// Cells across.
    columns: usize,
    /// Cells down.
    rows: usize,
}

impl Grid {
    /// A grid whose cell is a chunk's typical extent, with at most a few cells per chunk; `None`
    /// for no chunks.
    ///
    /// A cell the size of what is filed under it is the side at which one cell's comparisons are
    /// fewest: smaller, and each chunk is filed, and compared, in more cells; larger, and each
    /// cell holds more chunks that do not meet.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "an extent inside GRID_LIMIT and a count of chunks, both far inside f64's exact \
                  integers, and a side of at least one"
    )]
    fn over(chunks: &[Chunk]) -> Option<Self> {
        let bounds = chunks
            .iter()
            .map(|chunk| chunk.bounds)
            .reduce(Bounds::and)?;
        let (width, height) = (
            bounds.right.wrapping_sub(bounds.left).max(1) as f64,
            bounds.bottom.wrapping_sub(bounds.top).max(1) as f64,
        );
        let count = chunks.len().max(1) as f64;
        let typical = chunks
            .iter()
            .map(|chunk| {
                (chunk.bounds.right.wrapping_sub(chunk.bounds.left))
                    .max(chunk.bounds.bottom.wrapping_sub(chunk.bounds.top)) as f64
            })
            .sum::<f64>()
            / count;
        let side = (typical
            .max((width * height / (4.0 * count)).sqrt())
            .max(width.max(height) / (4.0 * count))
            .ceil() as i64)
            .max(1);
        let columns = usize::try_from(bounds.right.wrapping_sub(bounds.left).checked_div(side)?)
            .ok()?
            .saturating_add(1);
        let rows = usize::try_from(bounds.bottom.wrapping_sub(bounds.top).checked_div(side)?)
            .ok()?
            .saturating_add(1);
        Some(Self {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
            side,
            columns,
            rows,
        })
    }

    /// How many cells it has.
    fn cells(self) -> usize {
        self.columns.saturating_mul(self.rows)
    }

    /// The cell at a column and a row.
    fn cell(self, column: usize, row: usize) -> usize {
        row.saturating_mul(self.columns).saturating_add(column)
    }

    /// The column and row a grid point falls in, held inside the grid.
    fn at(self, x: i64, y: i64) -> (u32, u32) {
        let last = |count: usize| u32::try_from(count.saturating_sub(1)).unwrap_or(u32::MAX);
        let column = x
            .wrapping_sub(self.left)
            .checked_div(self.side)
            .and_then(|column| u32::try_from(column).ok())
            .unwrap_or(0);
        let row = y
            .wrapping_sub(self.top)
            .checked_div(self.side)
            .and_then(|row| u32::try_from(row).ok())
            .unwrap_or(0);
        (column.min(last(self.columns)), row.min(last(self.rows)))
    }
}

#[cfg(test)]
mod tests {
    use super::{Twins, Union, UnionCondition, approximate};

    /// A square's `(left, top, right, bottom)`, and whether it is wound clockwise.
    type Square = ((f32, f32, f32, f32), bool);

    /// Squares as one path, each `(left, top, right, bottom)` wound the way it is written: from
    /// its top left corner along the top where `clockwise`, down the left side otherwise.
    fn squares(squares: &[Square]) -> tiny_skia::Path {
        let mut builder = tiny_skia::PathBuilder::new();
        for &((left, top, right, bottom), clockwise) in squares {
            builder.move_to(left, top);
            if clockwise {
                builder.line_to(right, top);
                builder.line_to(right, bottom);
                builder.line_to(left, bottom);
            } else {
                builder.line_to(left, bottom);
                builder.line_to(right, bottom);
                builder.line_to(right, top);
            }
            builder.close();
        }
        builder.finish().expect("squares")
    }

    /// The union's coverage of `path` on a 16 by 16 surface, or why it declined.
    fn measured(path: &tiny_skia::Path) -> Result<Vec<u8>, super::super::UnionDecline> {
        let at = tiny_skia::Transform::identity();
        let region = super::super::region(path, at, (16, 16), (0, 0)).expect("a region");
        let mut target = vec![0_u8; 16 * 16];
        super::super::fill_union(
            &mut super::super::Buffers::default(),
            &mut target,
            (16, region),
            path,
            at,
        )
        .map(|()| target)
    }

    /// A piece the path states twice is measured as one piece of twice the weight (ADR 2281):
    /// §8.5.3.3.2's non-zero rule counts a contour's crossings once for each time the path
    /// states it, so `A`, `A` and a square `L` inside it wound the other way winds `2 − 1` inside
    /// `L` and fills it, where `A` and `L` alone wind `0` there and leave it out. Every pixel of
    /// the first is `A`'s alone, to the bit, which a twin merely left out would not be; and with
    /// the twins walked apart the mark has no point to start from, which is the decline this
    /// replaces.
    #[test]
    fn a_piece_stated_twice_is_one_piece_of_twice_the_weight() {
        let outer = ((2.25, 2.5, 12.75, 11.5), true);
        let inner = ((5.5, 5.25, 8.75, 8.5), false);
        let alone = measured(&squares(&[outer])).expect("a square");
        let holed = measured(&squares(&[outer, inner])).expect("a square with a hole");
        assert_ne!(alone, holed);
        assert_eq!(measured(&squares(&[outer, outer, inner])), Ok(alone));
        let mut union = Union::default();
        union.start();
        super::super::trace(
            &mut union,
            &squares(&[outer, outer, inner]),
            tiny_skia::Transform::identity(),
        );
        assert_eq!(
            union.settle_as(Twins::Apart),
            Err(UnionCondition::NoStartingPoint)
        );
        assert_eq!(union.settle(), Ok(()));
        assert_eq!(union.pieces(), 3);
    }

    /// A thin piece between two others, its upright sides along theirs — `inks.pdf`'s at 1x, a
    /// piece four and a half pixels long and a hundredth tall — is entered from a vertical ray
    /// beside its horizontal edge, the only point no other edge passes through (ADR 2281). The
    /// three meet only along their sides, so §8.5.3.3.2's set covers a pixel by the sum of the
    /// three rectangles' areas in it, and §10.7.4's anti-aliasing paints it at that area, rounded
    /// to the nearest of 255 levels.
    #[test]
    fn a_thin_piece_between_two_others_is_entered_from_above() {
        // The thin one first, so that its walk is the first and no crossing with a contour already
        // walked can enter it, as none entered `inks.pdf`'s.
        let rectangles = [
            (4.5, 5.375, 9.75, 5.625),
            (1.25, 2.5, 4.5, 10.75),
            (9.75, 2.5, 12.5, 10.75),
        ];
        let path = squares(&rectangles.map(|rectangle| (rectangle, true)));
        let mut union = Union::default();
        union.start();
        super::super::trace(&mut union, &path, tiny_skia::Transform::identity());
        assert_eq!(union.settle(), Ok(()));
        let drawn = measured(&path).expect("the three are measured");
        for row in 0..16_u32 {
            for column in 0..16_u32 {
                let (x, y) = (f64::from(column), f64::from(row));
                let area: f64 = rectangles
                    .iter()
                    .map(|&(left, top, right, bottom)| {
                        let across =
                            (f64::from(right).min(x + 1.0) - f64::from(left).max(x)).max(0.0);
                        let down =
                            (f64::from(bottom).min(y + 1.0) - f64::from(top).max(y)).max(0.0);
                        across * down
                    })
                    .sum();
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "an area between 0 and 1 times 255, rounded"
                )]
                let owed = (area.min(1.0) * 255.0).round() as u8;
                assert_eq!(
                    drawn[(row * 16 + column) as usize],
                    owed,
                    "({column}, {row})"
                );
            }
        }
    }

    /// A determinant converts to the nearest `f64` whatever its sign, which is what the `as`
    /// conversion of the whole `i128` gives; the negative values are the ones a split at bit 64
    /// rounded by up to `2^10` (ADR 2268), the first of them a determinant of the mark
    /// `three_pieces_meeting_at_one_point_leave_a_closed_boundary` holds.
    #[test]
    fn a_determinant_converts_to_the_nearest_f64_whatever_its_sign() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the reference: Rust's conversion of an i128 rounds to the nearest f64"
        )]
        let nearest = |value: i128| value as f64;
        let mut values = vec![
            -652_948_884_480,
            -13_453_138_944,
            -1,
            0,
            1,
            (1 << 53) + 1,
            -(1 << 53) - 1,
            (1 << 77) - 1,
            -(1 << 77) + 1,
        ];
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..10_000 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let magnitude = i128::from(state >> (state % 40));
            values.push(if state & 1 == 0 {
                magnitude
            } else {
                -magnitude
            });
        }
        for value in values {
            assert_eq!(
                approximate(value).to_bits(),
                nearest(value).to_bits(),
                "{value}"
            );
        }
    }
}
