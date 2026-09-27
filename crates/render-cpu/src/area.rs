//! A path's coverage measured as **area**, with nothing between the geometry and the level —
//! ISO 32000-2 §10.7.4.
//!
//! # What this is for
//!
//! `doc/todo/_scan-conversion.md`'s departure (1) is that both backends anti-alias, which
//! replaces the clause's "paint the pixel" with coverage proportional to area. The departure's
//! own second half is *how finely* that area is measured, and for everything that is not
//! axis-aligned rectangles this backend measured it to a sixteenth of a pixel: `tiny-skia`'s
//! anti-aliased path converter supersamples four times per pixel row and quantises a run along
//! `x` to quarter-pixel steps, so the only coverages it can state for a pixel one edge crosses
//! are the sixteen multiples of a sixteenth. §10.7.4's first paragraph is what a lattice is on
//! the wrong side of:
//!
//! > Its coordinates are mapped into device space but not rounded to device pixel boundaries.
//!
//! and its third sentence is what a lattice *reached by rounding* breaks in the direction the
//! `shall` forbids:
//!
//! > The area covered by painted pixels shall always be at least as large as the area of the
//! > original shape.
//!
//! ADR 0476 gave an axis-aligned rectangle its own exact coverage, ADR 0583 a path stating
//! several, ADR 0226 a shape thinner than the quantum that measures it. This module is the
//! general case those three left: a glyph, a curve, a diagonal, a stroke's outline. ADR 1082.
//!
//! # The construction, derived from the clause's own definition of a pixel
//!
//! §10.7.4 identifies a pixel by flooring a point and gives it the region
//! `[i, i+1) × [j, j+1)`, and §8.5.3.3's two fill rules define insideness by a **winding
//! number**. So the coverage a path gives pixel `(i, j)` is the integral of its winding number
//! over that square, taken through whichever rule is in force. Written as an integral it needs
//! no sampling at all.
//!
//! Take one directed edge and one pixel row `[j, j+1)`. A point `(px, py)` is enclosed by that
//! edge exactly when the edge crosses the horizontal line `y = py` at some `x` below `px`, and
//! it contributes `+1` or `−1` there according to the edge's direction. So, with `X(y)` the
//! edge's `x` at height `y` and `w = ±1` its direction, the edge's share of the integral over
//! pixel `(i, j)` is
//!
//! ```text
//!   w · ∫ clamp(i + 1 − X(y), 0, 1) dy
//! ```
//!
//! over the part of the row the edge spans — the inner `clamp` being the width of the part of
//! the column that lies to the right of the edge at that height. Call that quantity `S(i)`.
//! `S` is zero for every column left of the edge and `w · dy` for every column right of it, so
//! the **differences** `S(i) − S(i−1)` are non-zero only on the columns the edge actually
//! touches: accumulate those differences for every edge, then run a prefix sum along the row and
//! the sum at column `i` is the whole path's winding integral there. That is the one
//! multiplication per touched column this module is made of, and it is exact rather than
//! sampled.
//!
//! The integral itself is elementary. Over one row the edge is a straight segment from `x₀` to
//! `x₁`, so substituting `w = u − x` turns it into the antiderivative of a clamped ramp,
//! [`ramp_integral`], evaluated twice — which is [`shadow`], and which is exact at every
//! placement including a vertical edge, where the two evaluations coincide and the clamp is the
//! answer on its own.
//!
//! # The fill rule is applied to the sum, which is where §8.5.3.3 enters
//!
//! The prefix sum is a *signed* winding integral, so the two rules of §8.5.3.3 are two ways of
//! reading it: the non-zero rule takes its magnitude, capped at the whole pixel, and the
//! even-odd rule folds it into `0..=1` with period two. Both are exact wherever the winding stays
//! inside `0..=1` or `-1..=0`, which is every simple path. A row where it leaves them — portions of
//! the path overlapping, or wound against one another — is measured a second way: cut at every
//! height where two edges cross, the edges stand in one order in each piece of the row, and only
//! those where the rule's answer changes are deposited, at a unit weight. The sum over that
//! boundary is the area of the filled set itself (ADR 1341).
//!
//! # What it declines, and why each
//!
//! - **An overlapping row past [`SET_WORK`].** The second measurement is quadratic in the edges
//!   one row holds where they all overlap in `x`, so a mark that would spend more is left to the
//!   library's converter, which applies the rule per sample. A cost guard and not a condition.
//! - **A region past [`CELL_BUDGET`].** The accumulator is one `f32` per pixel of the mark's own
//!   device extent, so a mark larger than the budget is left to the library's converter rather
//!   than given a buffer nobody bounded. It is a cost guard and not a condition.
//! - **A mark outside the raster, or a transform that states no bounds.** There is nothing to
//!   measure.
//! - **Anything the caller has already withdrawn anti-aliasing from**, which is `scan`'s range
//!   rule: outside `SUPERSAMPLED_LIMIT` the aliased converter draws, and this module is not it.

use tiny_skia::{FillRule, Path, PathSegment, Point, Transform};

/// The largest accumulator this module will build, in cells.
///
/// Four million is a page of ISO 32000-2 at 150 dpi with room over, and sixteen megabytes of
/// `f32` — the buffer is kept for the length of a band and grows to the largest mark that band
/// holds, so the figure bounds the buffer rather than a per-mark allocation. A mark past it is
/// drawn by `tiny-skia`'s converter, which is what drew every mark before this module existed.
const CELL_BUDGET: usize = 1 << 22;

/// How far a flattened curve may lie from the curve it stands for, in device pixels.
///
/// The clause states no number — §10.7.4 says only that "curves have been flattened to sequences
/// of straight lines" by the time its rules apply, and §10.7.2's flatness tolerance is about a
/// *marking* device and is a permission this tree already declines to need. So the number is
/// chosen, and it is chosen against the raster's own depth rather than against a renderer: a
/// chord lying `d` pixels from its curve moves a boundary pixel's coverage by at most `d`, so a
/// sixteenth of the smallest level an eight-bit mask can hold is `1/256` of a pixel. Below the
/// level the raster can show, in other words, which is the point at which finer flattening stops
/// being visible arithmetic.
const FLATNESS: f32 = 1.0 / 256.0;

/// The most line segments one curve is flattened into.
///
/// A cost guard rather than a condition, and the geometry reaches it nowhere: `scan`'s
/// `SUPERSAMPLED_LIMIT` bounds a device coordinate at 8191, which bounds a cubic's second
/// difference by the same figure and [`steps_for`] by 1256.
const STEPS_PER_CURVE: u32 = 4096;

/// The device pixels a mark can reach, as the accumulator's own extent.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Region {
    /// The first column, and the `x` every device coordinate is stated against.
    left: u32,
    /// The first row, and the `y` every device coordinate is stated against.
    top: u32,
    /// Columns in the region.
    width: u32,
    /// Rows in the region.
    height: u32,
}

impl Region {
    /// Cells the accumulator needs: one per pixel, plus the column a clamped edge is folded on to.
    fn cells(self) -> Option<usize> {
        let columns = (self.width as usize).checked_add(1)?;
        columns.checked_mul(self.height as usize)
    }
}

/// The pixels `path` drawn under `at` can reach on a `width` by `height` surface.
///
/// Rounded outwards by a whole pixel on every side, which is `scan::reached_pixels`' own margin
/// and for its reason: the extent comes from control points and the coverage from this converter,
/// and a mark must not lose ink to a rectangle. `None` where the transform states no rectangle,
/// where the mark falls outside the surface, or where the accumulator would be past
/// [`CELL_BUDGET`].
pub(crate) fn region(path: &Path, at: Transform, (width, height): (u32, u32)) -> Option<Region> {
    let bounds = path.bounds().transform(at)?;
    let left = clamped(bounds.left() - 1.0, width);
    let top = clamped(bounds.top() - 1.0, height);
    let right = clamped(bounds.right() + 1.0, width);
    let bottom = clamped(bounds.bottom() + 1.0, height);
    if left >= right || top >= bottom {
        return None;
    }
    let region = Region {
        left,
        top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    };
    region.cells().filter(|cells| *cells <= CELL_BUDGET)?;
    Some(region)
}

/// A device coordinate as a pixel index, rounded outwards and held inside `0..=limit`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 0.0..=limit before the cast, and a surface's extent is \
              bounded by `rasterize`'s MAX_EXTENT = 2^24, every integer below which is exact in \
              f32"
)]
fn clamped(value: f32, limit: u32) -> u32 {
    if value.is_nan() {
        return 0;
    }
    value.max(0.0).min(f32_of(limit)) as u32
}

/// Writes `path`'s exact coverage into `target`, one byte per pixel of a `width` by `height`
/// mask — ISO 32000-2 §10.7.4.
///
/// `cells` is the accumulator, kept by the caller and grown to the largest mark it has been asked
/// for; it is cleared here, so nothing is carried between marks. `region` must be the one
/// [`region`] returned for the same path, transform and extent.
///
/// It takes the **larger** of what is there and what it writes, which is `scan::mask_rectangle`'s
/// contract and holds for the same reason: every caller in this crate fills a region that is
/// already clear, so the two are the same value there.
///
/// # Why the level is a plain rounding and carries no floor
///
/// `pdf_render::expressible_coverage` states a positive coverage under one level *at* one level,
/// and every other exact construction in this backend applies it. This one does not, and the
/// difference is what the coverage is made of. That function exists for a mark whose area was
/// deliberately moved into the paint's alpha by a substitution (ADR 0419), where a coverage
/// rounding to nothing is the substitution losing the mark it was built to keep. Here the
/// coverage is the shape's own area, measured where it lies; a shape too small for the raster to
/// show is the business of `pdf_render::collapsed`, `sub_pixel_bands` and `point_mark`, each of
/// which runs *before* this converter is reached and each of which answers §10.7.4's "no shape
/// ever disappears" for the shapes it names. Rounding to nearest therefore keeps anything above
/// half a level, which is every shape this converter is the right instrument for — and it keeps
/// the accumulator's own residue, of the order of `2^-24` per cell summed along a row, an order
/// of magnitude below the first level rather than lifted on to it.
pub(crate) fn fill(
    buffers: &mut Buffers,
    target: &mut [u8],
    (width, region): (u32, Region),
    (path, overlapping): (&Path, bool),
    fill_rule: FillRule,
    at: Transform,
) -> bool {
    let Some(count) = region.cells() else {
        return false;
    };
    let Buffers { cells, set } = buffers;
    cells.clear();
    cells.resize(count, 0.0);
    let Some(columns) = (region.width as usize).checked_add(1) else {
        return false;
    };
    let mut accumulator = Accumulator {
        values: cells,
        columns,
        rows: region.height as usize,
        origin: (f32_of(region.left), f32_of(region.top)),
        limit: f32_of(region.width),
    };
    trace(&mut accumulator, path, at);
    if !accumulator.measure_overlapping_rows((path, overlapping), at, fill_rule, set) {
        // Nothing has been written yet, so there is nothing to put back: the accumulator is this
        // module's own buffer and the target is still the clear region the caller established.
        return false;
    }
    read_off(&accumulator, target, (width, region), fill_rule);
    true
}

/// What [`fill`] measures in, kept by its caller for the length of a band and cleared per mark,
/// never allocated per mark: the accumulator, and what [`Accumulator::measure_overlapping_rows`]
/// walks a suspect mark with.
#[derive(Debug, Default)]
pub(crate) struct Buffers {
    /// The accumulator's cells.
    cells: Vec<f32>,
    /// The second measurement's buffers.
    set: SetBuffers,
}

/// The buffers the second measurement is made in; see [`Buffers`].
#[derive(Debug, Default)]
struct SetBuffers {
    /// The mark's edges as the walk hands them over.
    lines: Vec<[Point; 2]>,
    /// The same, taken downwards and sorted — [`Edge::sort_into`].
    edges: Vec<Edge>,
    /// One row's working state.
    rows: RowScratch,
}

/// A pixel index as the coordinate of its own lower corner, which is what §10.7.4's `i` and `j`
/// are. Exact for every raster this backend can allocate.
#[expect(
    clippy::cast_precision_loss,
    reason = "a raster's own pixel index, bounded by `rasterize`'s MAX_EXTENT = 2^24 and far \
              inside f32's exactly-represented integers"
)]
fn f32_of(index: u32) -> f32 {
    index as f32
}

/// The same, for an index already held inside a region's own extent.
#[expect(
    clippy::cast_precision_loss,
    reason = "see `f32_of`: the value is a pixel index inside a region bounded by MAX_EXTENT"
)]
fn index_as_f32(index: usize) -> f32 {
    index as f32
}

/// A coverage in `0.0..=1.0` as the level an eight-bit mask holds, rounded to nearest.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a coverage clamped to 0..=1 multiplied by 255, so the cast is in range \
              by construction"
)]
fn level_of(coverage: f32) -> u8 {
    (coverage.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// What a path's device-space edges are handed to: the accumulator, or the list
/// [`Accumulator::measure_overlapping_rows`] measures a row's filled set from.
///
/// One walk and one flattening for both, so that the two constructions are made of the same
/// straight lines to the bit.
trait Edges {
    /// One straight edge, directed from `from` to `to`, in device space.
    fn line(&mut self, from: Point, to: Point);
}

impl Edges for Vec<[Point; 2]> {
    fn line(&mut self, from: Point, to: Point) {
        self.push([from, to]);
    }
}

impl Edges for Accumulator<'_> {
    fn line(&mut self, from: Point, to: Point) {
        self.accumulate(from, to);
    }
}

/// Walks `path` under `at`, handing every edge of every subpath to `into`.
///
/// A fill closes each subpath — §8.5.3.3 says an open subpath "shall be closed implicitly" — so
/// the segment back to the subpath's first point is emitted whether or not `h` was written.
fn trace(into: &mut impl Edges, path: &Path, at: Transform) {
    let mut start = Point::zero();
    let mut current = Point::zero();
    let mut open = false;
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                if open {
                    into.line(current, start);
                }
                let point = mapped(point, at);
                start = point;
                current = point;
                open = true;
            }
            PathSegment::LineTo(point) => {
                let point = mapped(point, at);
                into.line(current, point);
                current = point;
            }
            PathSegment::QuadTo(control, point) => {
                let (control, point) = (mapped(control, at), mapped(point, at));
                quadratic(into, current, control, point);
                current = point;
            }
            PathSegment::CubicTo(first, second, point) => {
                let (first, second, point) =
                    (mapped(first, at), mapped(second, at), mapped(point, at));
                cubic(into, current, first, second, point);
                current = point;
            }
            PathSegment::Close => {
                into.line(current, start);
                current = start;
                open = false;
            }
        }
    }
    if open {
        into.line(current, start);
    }
}

/// `point` in device space.
fn mapped(point: Point, at: Transform) -> Point {
    let mut point = point;
    at.map_point(&mut point);
    point
}

/// How many chords a curve needs for its polyline to lie within [`FLATNESS`] of it.
///
/// The uniform polyline through `n + 1` points of a Bézier lies within `max|B″| / (8n²)` of the
/// curve, so `n` is the square root of `max|B″| / (8 · FLATNESS)`; the caller supplies that
/// numerator, because the second derivative's bound is a different multiple of the control
/// polygon's second difference for a quadratic than for a cubic.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a square root clamped to 1.0..=STEPS_PER_CURVE before the cast"
)]
fn steps_for(numerator: f32) -> u32 {
    if !numerator.is_finite() || numerator <= 0.0 {
        return 1;
    }
    let steps = (numerator / FLATNESS).sqrt().ceil();
    if !steps.is_finite() {
        return STEPS_PER_CURVE;
    }
    steps.clamp(1.0, f32::from(u16::MAX)) as u32
}

/// Step `step` of `steps` as a parameter in `0.0..=1.0`, exactly 1.0 at the last so that a
/// flattened curve ends where the curve does.
fn fraction(step: u32, steps: u32) -> f32 {
    if step >= steps {
        return 1.0;
    }
    f32_of(step) / f32_of(steps)
}

/// Flattens one quadratic and hands its chords to the accumulator.
///
/// `B″` is the constant `2(p₀ − 2p₁ + p₂)`, so the numerator [`steps_for`] wants is that second
/// difference divided by four.
fn quadratic(into: &mut impl Edges, from: Point, control: Point, to: Point) {
    let deviation = (from.x - 2.0 * control.x + to.x).hypot(from.y - 2.0 * control.y + to.y);
    let steps = steps_for(deviation * 0.25).min(STEPS_PER_CURVE);
    let mut previous = from;
    for step in 1..=steps {
        let t = fraction(step, steps);
        let u = 1.0 - t;
        let point = Point::from_xy(
            u * u * from.x + 2.0 * u * t * control.x + t * t * to.x,
            u * u * from.y + 2.0 * u * t * control.y + t * t * to.y,
        );
        into.line(previous, point);
        previous = point;
    }
}

/// Flattens one cubic and hands its chords to the accumulator.
///
/// `B″(t) = 6[(1 − t)(p₀ − 2p₁ + p₂) + t(p₁ − 2p₂ + p₃)]`, whose magnitude is bounded by six
/// times the larger of the two second differences, so the numerator [`steps_for`] wants is three
/// quarters of that larger difference.
fn cubic(into: &mut impl Edges, from: Point, first: Point, second: Point, to: Point) {
    let one = (from.x - 2.0 * first.x + second.x).hypot(from.y - 2.0 * first.y + second.y);
    let two = (first.x - 2.0 * second.x + to.x).hypot(first.y - 2.0 * second.y + to.y);
    let steps = steps_for(one.max(two) * 0.75).min(STEPS_PER_CURVE);
    let mut previous = from;
    for step in 1..=steps {
        let t = fraction(step, steps);
        let u = 1.0 - t;
        let weights = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        let point = Point::from_xy(
            weights.0 * from.x + weights.1 * first.x + weights.2 * second.x + weights.3 * to.x,
            weights.0 * from.y + weights.1 * first.y + weights.2 * second.y + weights.3 * to.y,
        );
        into.line(previous, point);
        previous = point;
    }
}

/// The differences of §10.7.4's winding integral, one cell per pixel of the mark's own extent.
///
/// See the module comment for what a cell holds and why a prefix sum along a row reads the
/// coverage off it.
struct Accumulator<'a> {
    /// `columns` values per row, `rows` rows.
    values: &'a mut [f32],
    /// The region's width plus the column a clamped edge is folded on to.
    columns: usize,
    /// The region's height.
    rows: usize,
    /// The region's own corner, subtracted from every device coordinate.
    origin: (f32, f32),
    /// The region's width, which is the last `x` a deposit may land on.
    limit: f32,
}

impl Accumulator<'_> {
    /// Accumulates one device-space edge, row by row.
    ///
    /// The edge is taken in increasing `y` with its direction carried in the weight, because the
    /// winding integral's sign is the edge's direction and everything below is then about a
    /// positive height.
    fn accumulate(&mut self, from: Point, to: Point) {
        if !(from.x.is_finite() && from.y.is_finite() && to.x.is_finite() && to.y.is_finite()) {
            return;
        }
        let (weight, upper, lower) = if from.y < to.y {
            (1.0_f32, from, to)
        } else {
            (-1.0_f32, to, from)
        };
        let rise = lower.y - upper.y;
        if rise <= 0.0 {
            // A horizontal edge encloses no point at any height: its integral is zero.
            return;
        }
        let (first, height) = (self.origin.1, index_as_f32(self.rows));
        let (enters, leaves) = (upper.y.max(first), lower.y.min(first + height));
        if enters >= leaves {
            return;
        }
        let slope = (lower.x - upper.x) / rise;
        let mut row = floored(enters - first);
        while row < self.rows {
            // Each row's two `x` are evaluated from the edge's **device** coordinates rather than
            // carried forward from the row above, and that is what makes a strip's answer the
            // whole page's: `first + row` is the device row whichever region the surface was cut
            // into, so the same two expressions are evaluated either way and give the same bits.
            // Accumulating `x` row by row instead put a strip's first row a rounding step off, and
            // `pdf-model/tests/strip_parallelism.rs` is what caught it.
            let top = first + index_as_f32(row);
            let (above, below) = (top.max(enters), (top + 1.0).min(leaves));
            let rows_height = below - above;
            if rows_height > 0.0 {
                let left = upper.x + (above - upper.y) * slope - self.origin.0;
                let right = upper.x + (below - upper.y) * slope - self.origin.0;
                self.crossing(row, weight * rows_height, left, right);
            }
            if below >= leaves {
                break;
            }
            row = row.saturating_add(1);
        }
    }

    /// One row's crossing, with whatever left the region's columns folded on to the side it left
    /// through.
    ///
    /// Folding is exact for every column the region holds: a portion of the edge left of column
    /// zero encloses those columns whatever its `x` is, and a portion right of the last encloses
    /// none of them. The split is by the share of the row's height each portion spans, which is
    /// the parameter at which the straight crossing meets the boundary.
    fn crossing(&mut self, row: usize, weight: f32, a: f32, b: f32) {
        let (low, high) = (a.min(b), a.max(b));
        if high <= 0.0 {
            self.span(row, weight, 0.0, 0.0);
            return;
        }
        if low >= self.limit {
            self.span(row, weight, self.limit, self.limit);
            return;
        }
        if low >= 0.0 && high <= self.limit {
            self.span(row, weight, low, high);
            return;
        }
        let reach = high - low;
        let before = ((0.0 - low) / reach).clamp(0.0, 1.0);
        let after = ((self.limit - low) / reach).clamp(0.0, 1.0);
        if before > 0.0 {
            self.span(row, weight * before, 0.0, 0.0);
        }
        if after > before {
            self.span(
                row,
                weight * (after - before),
                low.max(0.0),
                high.min(self.limit),
            );
        }
        if after < 1.0 {
            self.span(row, weight * (1.0 - after), self.limit, self.limit);
        }
    }

    /// Deposits the differences of `S` for a crossing that runs from `low` to `high` inside the
    /// region's columns — the module comment's closed form, one subtraction per touched column.
    fn span(&mut self, row: usize, weight: f32, low: f32, high: f32) {
        if weight == 0.0 {
            return;
        }
        let first = floored(low);
        // `S` is `weight` for every column at or past `ceil(high)`, so that column is the last
        // whose difference can be non-zero.
        let last = ceiled(high).min(self.columns.saturating_sub(1));
        let mut previous = 0.0_f32;
        let mut column = first;
        while column <= last {
            let next = if column == last {
                1.0
            } else {
                shadow(index_as_f32(column) + 1.0, low, high)
            };
            self.add(row, column, weight * (next - previous));
            previous = next;
            column = column.saturating_add(1);
        }
    }

    /// Adds `value` to one cell, ignoring a cell the region does not hold.
    fn add(&mut self, row: usize, column: usize, value: f32) {
        if let Some(cell) = row
            .checked_mul(self.columns)
            .and_then(|start| start.checked_add(column))
            .and_then(|index| self.values.get_mut(index))
        {
            *cell += value;
        }
    }

    /// One row's cells, as the slice [`read_off`] and [`overlaps`] read.
    fn row_cells(&mut self, row: usize) -> Option<&mut [f32]> {
        let from = row.checked_mul(self.columns)?;
        let until = from.checked_add(self.columns)?;
        self.values.get_mut(from..until)
    }

    /// Re-measures, as the area of the filled **set**, every row whose winding integral cannot be
    /// read as that area — ISO 32000-2 §10.7.4, §8.5.3.3, §11.6.2.
    ///
    /// Two steps, because the question is cheap to suspect and dear to settle. [`overlaps`] reads
    /// each row's own sums for a sign that the winding left `0..=1` or `-1..=0` somewhere, and a
    /// mark with no such row is left exactly as it was accumulated. A mark with one is **suspect as
    /// a whole** — an overlap confined to pixels its path only partly covers shows no sign in its
    /// own row, and the rows beside it are what give it away — so every row of it is walked by
    /// [`Accumulator::measure_row`], which settles the question per row and rewrites only a row
    /// whose winding did leave those ranges. A row that did not keeps the accumulator's bits.
    ///
    /// `false` only where [`SET_WORK`] was spent first, in which case nothing has been written to
    /// the caller's target and the library's converter draws the mark.
    ///
    /// # What it costs, and what it buys
    ///
    /// `callgrind_rasterise`, `RAYON_NUM_THREADS=1`, five rasterisations, against the tree that left
    /// every such mark to `tiny-skia`'s supersampled converter: ISO 32000-2's page 101 **−2.7%**
    /// (the per-deposit test this replaced was on every glyph), `issue12295.pdf` −0.03% (65 859
    /// strokes, each one straight segment), `issue20232.pdf` **+9.3%**, and `issue19802.pdf` and
    /// `issue14415.pdf` **+50%** and **+47%** — the two pages dense with stroked paths whose joins
    /// overlap, where each row with more than two edges is now walked. What it buys is the page's
    /// own ink: `render-raster/examples/ink_ladder` reads `issue20232.pdf` at 17 932 at 1× against
    /// its own 17 866 at 8×, where the supersampled converter read 19 324 and the integral alone
    /// 23 722, and every page it moved lands on its own 2× figure. ADR 1341.
    fn measure_overlapping_rows(
        &mut self,
        (path, overlapping): (&Path, bool),
        at: Transform,
        rule: FillRule,
        set: &mut SetBuffers,
    ) -> bool {
        let suspect = overlapping
            || (0..self.rows).any(|row| self.row_cells(row).is_some_and(|cells| overlaps(cells)));
        if !suspect {
            return true;
        }
        let SetBuffers { lines, edges, rows } = set;
        lines.clear();
        trace(lines, path, at);
        Edge::sort_into(lines, edges);
        rows.start();
        for row in 0..self.rows {
            if !self.measure_row(row, edges, rule, rows) {
                return false;
            }
        }
        true
    }

    /// Settles one row of a suspect mark: walks the boundary of the filled set and, where the
    /// row's winding left `0..=1` or `-1..=0`, writes its cells from that boundary instead of from
    /// the path's edges. `false` where [`SET_WORK`] is spent.
    ///
    /// §10.7.4 applies its rules to a shape whose inside is already decided:
    ///
    /// > At this level, curves have been flattened to sequences of straight lines, and all
    /// > "insideness" computations have been performed.
    ///
    /// So the quantity a pixel is covered by is the area of the set §8.5.3.3's rule declares
    /// inside — not the integral of the winding number that set was decided from. The two part
    /// only where the winding leaves those ranges, which is portions of one path overlapping or
    /// wound against one another.
    ///
    /// The row is cut into horizontal sub-strips at every height where an edge begins, ends or
    /// crosses another. Inside one sub-strip no two edges cross, so they stand in one left-to-right
    /// order, and walking that order with the running winding number finds exactly where the
    /// rule's answer changes. Each such edge is deposited with a **unit** weight — `+1` where the
    /// set is entered, `−1` where it is left — and every other edge not at all. The prefix sum
    /// [`read_off`] runs is then the integral of the set's own indicator, which is its area,
    /// exactly: the same closed form the whole module is made of, over a boundary that no longer
    /// overlaps itself.
    fn measure_row(
        &mut self,
        row: usize,
        edges: &[Edge],
        rule: FillRule,
        scratch: &mut RowScratch,
    ) -> bool {
        let top = self.origin.1 + index_as_f32(row);
        scratch.gather(edges, (top, top + 1.0));
        // A row one edge crosses has a winding of zero and one other value, and a row two edges of
        // opposite directions cross without meeting has zero and one sign: neither can leave the
        // ranges, so neither is walked. Most rows of a stroke's outline are the second.
        match scratch.pieces.as_slice() {
            [] | [_] => return true,
            [a, b] if a.direction != b.direction && crossing_height(a, b).is_none() => return true,
            _ => {}
        }
        if !scratch.cut() {
            return false;
        }
        let RowScratch {
            pieces,
            heights,
            order,
            boundary,
            work,
            live,
            ..
        } = scratch;
        boundary.clear();
        // The sub-strips are taken downwards, so the pieces spanning one are a window over the
        // pieces in order of where they enter the row — which is the order [`RowScratch::gather`]
        // collected them in, since its edges are sorted by their upper ends. Every end is a cut,
        // so a piece that has entered and not yet left spans the whole sub-strip.
        live.clear();
        let mut entered = 0_usize;
        // The one sign a winding may take in this row and still be read exactly, once met.
        let mut sign = 0_i32;
        let mut leaves_the_range = false;
        for pair in heights.windows(2) {
            let &[above, below] = pair else {
                continue;
            };
            if below <= above {
                continue;
            }
            let middle = 0.5 * (above + below);
            while let Some(piece) = pieces.get(entered) {
                if piece.from > above {
                    break;
                }
                live.push(entered);
                entered = entered.saturating_add(1);
            }
            live.retain(|&index| pieces.get(index).is_some_and(|piece| piece.to > above));
            order.clear();
            order.extend(
                live.iter()
                    .filter_map(|&index| Some((pieces.get(index)?.x_at(middle), index))),
            );
            *work = work.saturating_add(order.len());
            if *work > SET_WORK {
                return false;
            }
            order.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0_i32;
            let mut inside = false;
            for &(_, index) in order.iter() {
                let Some(piece) = pieces.get(index) else {
                    continue;
                };
                winding = winding.saturating_add(piece.direction);
                if winding != 0 {
                    if winding.abs() > 1 || (sign != 0 && winding != sign) {
                        leaves_the_range = true;
                    }
                    sign = winding;
                }
                let now = match rule {
                    FillRule::Winding => winding != 0,
                    FillRule::EvenOdd => winding & 1 == 1,
                };
                if now != inside {
                    let height = if now { below - above } else { above - below };
                    boundary.push((height, piece.x_at(above), piece.x_at(below)));
                    inside = now;
                }
            }
        }
        if leaves_the_range {
            if let Some(cells) = self.row_cells(row) {
                cells.fill(0.0);
            }
            for &(height, a, b) in boundary.iter() {
                self.crossing(row, height, a - self.origin.0, b - self.origin.0);
            }
        }
        true
    }
}

/// Whether a row's winding integral may not be read as the area of the filled set, which is where
/// [`Accumulator::measure_row`] measures the set itself.
///
/// Three signs, each the arithmetic's own. A **cell** past one whole winding is two edges of one
/// direction crossing one pixel, since one edge changes the integral by at most one. A **pixel**
/// whose integral is past one has a winding of two somewhere in it. And a row holding both a
/// positive and a negative integral has portions wound against one another, which can cancel in a
/// pixel they share. None of the three is met by a path whose winding stays inside `0..=1` or
/// `-1..=0`, so nothing that reads exactly is re-measured.
fn overlaps(cells: &[f32]) -> bool {
    let mut running = 0.0_f32;
    let (mut positive, mut negative) = (false, false);
    for &cell in cells {
        running += cell;
        if cell.abs() > OVERLAPPED || running.abs() > OVERLAPPED {
            return true;
        }
        positive |= running > RESIDUE;
        negative |= running < -RESIDUE;
    }
    positive && negative
}

/// One straight edge of a suspect mark, taken downwards with its direction carried beside it.
#[derive(Clone, Copy, Debug)]
struct Edge {
    /// The upper end, in device space.
    upper: Point,
    /// The height of the lower end.
    lower: f32,
    /// The change in `x` per unit of `y`.
    slope: f32,
    /// `+1` for an edge running down the device, `−1` for one running up: §8.5.3.3.2's count.
    direction: i32,
}

impl Edge {
    /// The edges that enclose anything — every one with a height and finite ends — in order of
    /// their upper end, which is the order the rows meet them in.
    fn sort_into(lines: &[[Point; 2]], edges: &mut Vec<Self>) {
        edges.clear();
        edges.extend(
            lines
                .iter()
                .filter(|[from, to]| {
                    from.x.is_finite() && from.y.is_finite() && to.x.is_finite() && to.y.is_finite()
                })
                .filter_map(|&[from, to]| {
                    let (direction, upper, lower) = if from.y < to.y {
                        (1_i32, from, to)
                    } else {
                        (-1_i32, to, from)
                    };
                    let rise = lower.y - upper.y;
                    (rise > 0.0).then(|| Self {
                        upper,
                        lower: lower.y,
                        slope: (lower.x - upper.x) / rise,
                        direction,
                    })
                }),
        );
        edges.sort_unstable_by(|a, b| a.upper.y.total_cmp(&b.upper.y));
    }
}

/// One edge's part of one pixel row, for [`Accumulator::measure_row`].
#[derive(Clone, Copy, Debug)]
struct Piece {
    /// The edge's upper end, in device space.
    upper: Point,
    /// Its change in `x` per unit of `y`.
    slope: f32,
    /// `+1` for an edge running down the device, `−1` for one running up: §8.5.3.3.2's count.
    direction: i32,
    /// The height at which it enters the row.
    from: f32,
    /// The height at which it leaves the row.
    to: f32,
    /// The smaller of its two `x` inside the row.
    left: f32,
    /// The larger.
    right: f32,
}

impl Piece {
    /// The edge's `x` at height `y`, from its device coordinates — the expression
    /// [`Accumulator::accumulate`] evaluates, so a strip's answer is the whole page's.
    fn x_at(&self, y: f32) -> f32 {
        self.upper.x + (y - self.upper.y) * self.slope
    }

    /// One edge's part of the row `[enters, leaves)`.
    fn of(edge: &Edge, (enters, leaves): (f32, f32)) -> Self {
        let mut piece = Self {
            upper: edge.upper,
            slope: edge.slope,
            direction: edge.direction,
            from: enters,
            to: leaves,
            left: 0.0,
            right: 0.0,
        };
        let (a, b) = (piece.x_at(enters), piece.x_at(leaves));
        (piece.left, piece.right) = (a.min(b), a.max(b));
        piece
    }
}

/// The height strictly inside both pieces' common span at which they cross, if they do.
fn crossing_height(a: &Piece, b: &Piece) -> Option<f32> {
    let (from, to) = (a.from.max(b.from), a.to.min(b.to));
    if to <= from {
        return None;
    }
    let (first, last) = (a.x_at(from) - b.x_at(from), a.x_at(to) - b.x_at(to));
    if (first < 0.0 && last > 0.0) || (first > 0.0 && last < 0.0) {
        let height = from + (to - from) * (first / (first - last));
        (height > from && height < to).then_some(height)
    } else {
        None
    }
}

/// [`Accumulator::measure_row`]'s buffers, kept for the length of a band and started afresh per
/// mark.
#[derive(Debug, Default)]
struct RowScratch {
    /// The edges' parts inside the row.
    pieces: Vec<Piece>,
    /// Every height the row is cut at, sorted.
    heights: Vec<f32>,
    /// One sub-strip's pieces, by `x` at its middle.
    order: Vec<(f32, usize)>,
    /// The filled set's boundary in one row: each edge where the rule's answer changes, as the
    /// signed height of its sub-strip and its `x` at the sub-strip's top and bottom.
    boundary: Vec<(f32, f32, f32)>,
    /// What the mark has spent so far, in pair tests and pieces placed; see [`SET_WORK`].
    work: usize,
    /// The edges the current row may meet, as indices into [`Edge::sort_into`]'s order.
    active: Vec<usize>,
    /// The row's pieces, as indices, in order of their leftmost `x`.
    by_left: Vec<usize>,
    /// The pieces spanning the current sub-strip, as indices.
    live: Vec<usize>,
    /// The first edge in that order no row has met yet.
    next: usize,
}

impl RowScratch {
    /// Forgets the last mark: no work spent, no edge met.
    fn start(&mut self) {
        self.work = 0;
        self.active.clear();
        self.next = 0;
    }

    /// Collects every edge's part of the device row `[top, bottom)`.
    ///
    /// Rows are asked in increasing order, so the edges a row meets are kept as a window over
    /// [`Edge::sort_into`]'s order: one that begins above the row's bottom joins it, and one that ends
    /// at or above the row's top leaves it. Each edge is looked at in the rows it spans and not in
    /// the others.
    fn gather(&mut self, edges: &[Edge], (top, bottom): (f32, f32)) {
        self.pieces.clear();
        self.heights.clear();
        while let Some(edge) = edges.get(self.next) {
            if edge.upper.y >= bottom {
                break;
            }
            self.active.push(self.next);
            self.next = self.next.saturating_add(1);
        }
        self.active
            .retain(|&index| edges.get(index).is_some_and(|edge| edge.lower > top));
        for &index in &self.active {
            let Some(edge) = edges.get(index) else {
                continue;
            };
            let (enters, leaves) = (edge.upper.y.max(top), edge.lower.min(bottom));
            if enters >= leaves {
                continue;
            }
            self.pieces.push(Piece::of(edge, (enters, leaves)));
            self.heights.push(enters);
            self.heights.push(leaves);
        }
    }

    /// Adds every height at which two pieces cross, and sorts the cuts — `false` where the mark's
    /// [`SET_WORK`] is spent.
    ///
    /// Pieces are taken in order of their leftmost `x`, so that a pair whose reaches do not
    /// overlap is never tested: two pieces with no `x` in common cannot cross.
    fn cut(&mut self) -> bool {
        let pieces = &self.pieces;
        self.by_left.clear();
        self.by_left.extend(0..pieces.len());
        self.by_left.sort_unstable_by(|&a, &b| {
            let left = |index: usize| pieces.get(index).map_or(f32::INFINITY, |piece| piece.left);
            left(a).total_cmp(&left(b))
        });
        for (rank, &index) in self.by_left.iter().enumerate() {
            let Some(piece) = pieces.get(index) else {
                continue;
            };
            for &other in self
                .by_left
                .get(rank.saturating_add(1)..)
                .unwrap_or_default()
            {
                let Some(other) = pieces.get(other) else {
                    continue;
                };
                if other.left > piece.right {
                    break;
                }
                self.work = self.work.saturating_add(1);
                if self.work > SET_WORK {
                    return false;
                }
                if let Some(height) = crossing_height(piece, other) {
                    self.heights.push(height);
                }
            }
        }
        self.heights.sort_unstable_by(f32::total_cmp);
        self.heights.dedup();
        true
    }
}

/// The most work [`Accumulator::measure_row`] spends on one mark, in pair tests and pieces placed.
///
/// A cost guard rather than a condition: the construction is quadratic in the edges one row holds
/// where they all overlap in `x`, and a mark past this is drawn by the library's supersampled
/// converter, which applies the fill rule per sample and so has the right set at a coarser
/// measure. Sixteen million is of the order of a tenth of a second of this loop.
const SET_WORK: usize = 1 << 24;

/// A running integral this close to zero is the arithmetic's own residue rather than a winding —
/// see [`OVERLAPPED`], whose thousandth it is.
const RESIDUE: f32 = 0.001;

/// A non-negative coordinate as the index of the pixel holding it — §10.7.4's `floor`.
///
/// The cast truncates towards zero, which *is* the floor for a value at or above it, so this is
/// the clause's arithmetic and not an approximation of it. It is written as a cast rather than as
/// `f32::floor` because that method is a call into the platform's maths library on any target
/// without SSE 4.1, and this converter asks for it three times per edge per row: on
/// `issue840.pdf` page 1 the two rounding calls between them were **3.4%** of the whole
/// rasterisation, measured under callgrind.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "every caller passes a value already held at or above zero and below a region \
              extent, which `rasterize`'s MAX_EXTENT bounds at 2^24"
)]
fn floored(value: f32) -> usize {
    // Ordered this way round rather than negated, so that a NaN takes the same branch as a
    // negative one: there is no pixel at a coordinate that is not a number.
    if value > 0.0 { value as usize } else { 0 }
}

/// The smallest pixel index at or above a non-negative coordinate — `ceil`, by [`floored`]'s
/// route and for its reason.
fn ceiled(value: f32) -> usize {
    let whole = floored(value);
    if value > index_as_f32(whole) {
        whole.saturating_add(1)
    } else {
        whole
    }
}

/// `∫₀ᵛ clamp(w, 0, 1) dw` — the antiderivative [`shadow`] is written from.
fn ramp_integral(v: f32) -> f32 {
    if v <= 0.0 {
        0.0
    } else if v < 1.0 {
        0.5 * v * v
    } else {
        v - 0.5
    }
}

/// `S(u)` for one row's crossing, per unit of the row's height: the mean over the crossing of the
/// width of `[u − 1, u)` lying to the right of it.
///
/// With the crossing's `x` linear from `low` to `high`, substituting `w = u − x` turns the mean
/// into a difference of [`ramp_integral`]s; where the crossing is vertical the two coincide and
/// the clamped ramp is the answer directly.
fn shadow(u: f32, low: f32, high: f32) -> f32 {
    let span = high - low;
    if span <= 0.0 {
        return (u - low).clamp(0.0, 1.0);
    }
    ((ramp_integral(u - low) - ramp_integral(u - high)) / span).clamp(0.0, 1.0)
}

/// Runs the prefix sum along every row and writes the level each pixel's coverage takes.
///
/// §8.5.3.3's two rules are two readings of the same signed integral — see the module comment.
///
/// # A row whose portions overlap has been re-measured before this runs, and §11.6.2 is why
///
/// The sum is the integral of the **winding number**, which is the filled set's own indicator only
/// while that number stays inside `0..=1` or `-1..=0`. Where two portions of one path wound the
/// same way cover one region, the number there is two, and a pixel on the *boundary* of that region
/// would read twice the area it covers; where two wound against one another share a pixel, their
/// integrals cancel there. The first is portions of an object composited with one another, and
/// §11.6.2 forbids exactly that:
///
/// > Portions of an object shall not be composited with one another, even if they are described in
/// > a way that would seem to cause overlaps (such as a self-intersecting path, combined fill and
/// > stroke of a path, or a shading pattern containing an overlap or fold-over).
///
/// So [`overlaps`] names every row where the reading may part from the set, and
/// [`Accumulator::measure_row`] has already rewritten that row's cells from the set's own boundary:
/// by the time this runs, every sum it reads is an area. **The condition is the arithmetic's own**:
/// a row whose winding stays inside one of the two ranges above never meets it, so nothing that
/// reads exactly is re-measured. What it does not see is an overlap confined to pixels its path
/// only partly covers, in a row with no whole winding of two and no sign against another, where the
/// error is bounded by the overlap's own area. ADR 1341.
///
/// `pdf-model/tests/glyph_clip_direction.rs` is §9.3.6's scene for it — text rendering mode 7
/// accumulates a glyph's outline into the clipping path, so a word set twice in one place is one
/// path stating every outline twice — and `render-cpu/tests/overlapping_portions.rs` states the
/// closed form: two squares overlapping at a corner, under both rules.
fn read_off(
    accumulator: &Accumulator<'_>,
    target: &mut [u8],
    (stride, region): (u32, Region),
    fill_rule: FillRule,
) {
    let stride = stride as usize;
    let width = region.width as usize;
    for row in 0..accumulator.rows {
        let Some(cells) = row
            .checked_mul(accumulator.columns)
            .and_then(|from| Some((from, from.checked_add(width)?)))
            .and_then(|(from, until)| accumulator.values.get(from..until))
        else {
            continue;
        };
        let Some(start) = (region.top as usize)
            .checked_add(row)
            .and_then(|line| line.checked_mul(stride))
            .and_then(|line| line.checked_add(region.left as usize))
        else {
            continue;
        };
        let Some(scanline) = target.get_mut(start..start.saturating_add(width)) else {
            continue;
        };
        let mut running = 0.0_f32;
        for (cell, byte) in cells.iter().zip(scanline) {
            running += *cell;
            let covered = match fill_rule {
                FillRule::Winding => running.abs().min(1.0),
                FillRule::EvenOdd => {
                    let folded = running.rem_euclid(2.0);
                    if folded > 1.0 { 2.0 - folded } else { folded }
                }
            };
            if covered > 0.0 {
                *byte = (*byte).max(level_of(covered));
            }
        }
    }
}

/// The largest change one edge can make to a pixel's winding integral.
///
/// One whole winding, plus a thousandth for the arithmetic's own residue — of the order of
/// `2^-24` per deposit, which a thousandth clears by three orders of magnitude while staying far
/// below the second whole winding a crossing adds. See [`overlaps`].
const OVERLAPPED: f32 = 1.001;

#[cfg(test)]
mod tests {
    #![expect(
        clippy::arithmetic_side_effects,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::indexing_slicing,
        reason = "a test module: every expected value below is arithmetic on constants written in \
                  the test itself, and an index that is out of range is the test failing"
    )]

    use super::{Region, fill, region};

    /// The coverage of a `width` by `height` mask, as levels.
    fn levels(
        path: &tiny_skia::Path,
        fill_rule: tiny_skia::FillRule,
        extent: (u32, u32),
    ) -> Vec<u8> {
        levels_of(path, fill_rule, extent, false)
    }

    /// [`levels`], saying whether the path is known to overlap itself.
    fn levels_of(
        path: &tiny_skia::Path,
        fill_rule: tiny_skia::FillRule,
        (width, height): (u32, u32),
        overlapping: bool,
    ) -> Vec<u8> {
        let mut target = vec![0_u8; (width as usize) * (height as usize)];
        let mut cells = super::Buffers::default();
        let region: Region = region(path, tiny_skia::Transform::identity(), (width, height))
            .expect("the path reaches the surface");
        assert!(fill(
            &mut cells,
            &mut target,
            (width, region),
            (path, overlapping),
            fill_rule,
            tiny_skia::Transform::identity(),
        ));
        target
    }

    fn rectangle(ltrb: (f32, f32, f32, f32)) -> tiny_skia::Path {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(ltrb.0, ltrb.1);
        builder.line_to(ltrb.2, ltrb.1);
        builder.line_to(ltrb.2, ltrb.3);
        builder.line_to(ltrb.0, ltrb.3);
        builder.close();
        builder.finish().expect("a rectangle")
    }

    /// §10.7.4's closed form for an axis-aligned rectangle is the product of two overlaps, and
    /// this converter has to reproduce it exactly — the one shape whose answer is known
    /// independently (ADR 0476).
    #[test]
    fn a_rectangles_edge_is_the_product_of_its_two_overlaps() {
        for step in 0_u8..=16 {
            let edge = 1.0 + f32::from(step) / 16.0;
            let path = rectangle((1.0, 1.0, edge, 3.0));
            let levels = levels(&path, tiny_skia::FillRule::Winding, (5, 5));
            let expected = (255.0 * (edge - 1.0).min(1.0)).round() as u8;
            assert_eq!(
                levels[5 + 1],
                expected,
                "a rectangle from x = 1 to x = {edge} covers pixel (1, 1) by its own overlap"
            );
        }
    }

    /// The quantum this module replaces: a coverage between two sixteenths is a coverage
    /// `tiny-skia`'s supersampled converter cannot state, and this one has to.
    #[test]
    fn a_coverage_between_two_sixteenths_is_stated() {
        let path = rectangle((1.03, 1.0, 4.0, 3.0));
        let levels = levels(&path, tiny_skia::FillRule::Winding, (5, 5));
        assert_eq!(levels[5 + 1], (255.0_f32 * 0.97).round() as u8);
    }

    /// A square inside a square, wound the *other* way: winding one in the ring and zero in the
    /// hole, which both of §8.5.3.3's rules read the same way.
    #[test]
    fn a_ring_is_a_hole_under_either_rule() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(6.0, 0.0);
        builder.line_to(6.0, 6.0);
        builder.line_to(0.0, 6.0);
        builder.close();
        builder.move_to(2.0, 2.0);
        builder.line_to(2.0, 4.0);
        builder.line_to(4.0, 4.0);
        builder.line_to(4.0, 2.0);
        builder.close();
        let path = builder.finish().expect("a ring");
        for rule in [tiny_skia::FillRule::Winding, tiny_skia::FillRule::EvenOdd] {
            let levels = levels(&path, rule, (6, 6));
            assert_eq!(levels[3 * 6 + 3], 0, "{rule:?}: the hole is a hole");
            assert_eq!(levels[0], 255, "{rule:?}: the ring is solid");
        }
    }

    /// Two portions of one path wound the **same** way over one region: the non-zero rule's set is
    /// the one square, and §11.6.2 forbids compositing the two portions with one another — so a
    /// boundary pixel is covered by the square's own area, not twice it.
    #[test]
    fn two_same_wound_portions_are_measured_as_the_one_region_they_fill() {
        let mut builder = tiny_skia::PathBuilder::new();
        for _ in 0..2 {
            builder.move_to(1.0, 1.0);
            builder.line_to(4.5, 1.0);
            builder.line_to(4.5, 4.5);
            builder.line_to(1.0, 4.5);
            builder.close();
        }
        let path = builder.finish().expect("one square stated twice");
        let levels = levels(&path, tiny_skia::FillRule::Winding, (6, 6));
        assert_eq!(levels[6 + 4], 128, "half of pixel (4, 1), not all of it");
        assert_eq!(levels[4 * 6 + 4], 64, "a quarter of pixel (4, 4)");
        assert_eq!(levels[2 * 6 + 2], 255, "the interior is covered once");
    }

    /// Two squares overlapping at a corner, both wound the same way, measured at pixel `(3, 2)`,
    /// where the first reaches `a` of the pixel's width and the second `1 − b` of its height.
    ///
    /// §8.5.3.3.2's set is their union and §8.5.3.3.3's is the union less the overlap, so the
    /// clause's areas are `a + (1 − b) − a(1 − b)` and `a + (1 − b) − 2a(1 − b)`. The winding
    /// integral reads `a + (1 − b)` under both, which is what this separates.
    #[test]
    fn a_corner_overlap_is_the_union_under_one_rule_and_the_difference_under_the_other() {
        for (a, b) in [(0.5_f32, 0.5_f32), (0.3, 0.6), (0.7, 0.15)] {
            let mut builder = tiny_skia::PathBuilder::new();
            for (left, top, right, bottom) in
                [(1.0, 1.0, 3.0 + a, 3.0 + a), (2.0 + b, 2.0 + b, 5.0, 5.0)]
            {
                builder.move_to(left, top);
                builder.line_to(right, top);
                builder.line_to(right, bottom);
                builder.line_to(left, bottom);
                builder.close();
            }
            let path = builder.finish().expect("two squares");
            let overlap = a * (1.0 - b);
            let union = a + (1.0 - b) - overlap;
            for (rule, area) in [
                (tiny_skia::FillRule::Winding, union),
                (tiny_skia::FillRule::EvenOdd, union - overlap),
            ] {
                let levels = levels(&path, rule, (6, 6));
                assert_eq!(
                    levels[2 * 6 + 3],
                    (255.0 * area).round() as u8,
                    "{rule:?} at a = {a}, b = {b}: pixel (3, 2) is the set's own area"
                );
                // Pixel (2, 2) lies wholly inside the first square and the second reaches
                // (1 − b)² of it: the union covers it all and the difference leaves the rest.
                let inner = (1.0 - b) * (1.0 - b);
                let expected = match rule {
                    tiny_skia::FillRule::Winding => 1.0,
                    tiny_skia::FillRule::EvenOdd => 1.0 - inner,
                };
                assert_eq!(
                    levels[2 * 6 + 2],
                    (255.0 * expected).round() as u8,
                    "{rule:?} at a = {a}, b = {b}: pixel (2, 2)"
                );
            }
        }
    }

    /// Two portions wound against one another that share a pixel and nothing else: their winding
    /// integrals cancel there, and the non-zero rule's set covers both.
    #[test]
    fn opposed_portions_sharing_a_pixel_do_not_cancel() {
        let mut builder = tiny_skia::PathBuilder::new();
        // Clockwise on the device, from x = 1 to 2.4.
        builder.move_to(1.0, 1.0);
        builder.line_to(2.4, 1.0);
        builder.line_to(2.4, 3.0);
        builder.line_to(1.0, 3.0);
        builder.close();
        // Anticlockwise, from x = 2.6 to 4.
        builder.move_to(2.6, 1.0);
        builder.line_to(2.6, 3.0);
        builder.line_to(4.0, 3.0);
        builder.line_to(4.0, 1.0);
        builder.close();
        let path = builder.finish().expect("two opposed rectangles");
        for rule in [tiny_skia::FillRule::Winding, tiny_skia::FillRule::EvenOdd] {
            let levels = levels(&path, rule, (5, 5));
            assert_eq!(
                levels[5 + 2],
                (255.0_f32 * 0.8).round() as u8,
                "{rule:?}: 0.4 of pixel (2, 1) from each, not 0.4 − 0.4"
            );
        }
    }

    /// Two slivers wound the same way that overlap inside one pixel and nowhere else: no row shows
    /// a sign, because no pixel's integral reaches one, so only a mark known to overlap itself —
    /// a stroker's outline — is measured as the set. The union covers 0.6 of the pixel and the
    /// integral 0.8; the second reading is the residue a mark not known to overlap keeps, and it is
    /// bounded by the overlap's own area.
    #[test]
    fn an_overlap_inside_one_pixel_is_the_set_where_the_mark_is_known_to_overlap() {
        let mut builder = tiny_skia::PathBuilder::new();
        for (left, right) in [(1.1_f32, 1.5_f32), (1.3, 1.7)] {
            builder.move_to(left, 1.0);
            builder.line_to(right, 1.0);
            builder.line_to(right, 2.0);
            builder.line_to(left, 2.0);
            builder.close();
        }
        let path = builder.finish().expect("two slivers");
        let known = levels_of(&path, tiny_skia::FillRule::Winding, (3, 3), true);
        assert_eq!(known[3 + 1], (255.0_f32 * 0.6).round() as u8, "the union");
        let unknown = levels_of(&path, tiny_skia::FillRule::Winding, (3, 3), false);
        assert_eq!(
            unknown[3 + 1],
            (255.0_f32 * 0.8).round() as u8,
            "the integral, which is the residue"
        );
    }

    /// A shape whose whole extent is a fraction of one pixel is painted at that fraction, where
    /// the supersampled converter states nothing at all below an eighth.
    #[test]
    fn a_sliver_of_a_pixel_keeps_its_own_area() {
        let path = rectangle((1.0, 1.0, 1.2, 1.5));
        let levels = levels(&path, tiny_skia::FillRule::Winding, (4, 4));
        assert_eq!(levels[4 + 1], (255.0_f32 * 0.2 * 0.5).round() as u8);
    }

    /// A mark reaching past the raster keeps the ink that falls on it: the part outside is folded
    /// on to the boundary it left through, which changes no column the region holds.
    #[test]
    fn a_mark_running_off_the_raster_keeps_what_falls_on_it() {
        let path = rectangle((-10.0, -10.0, 2.5, 2.0));
        let levels = levels(&path, tiny_skia::FillRule::Winding, (4, 4));
        assert_eq!(levels[0], 255);
        assert_eq!(levels[2], (255.0_f32 * 0.5).round() as u8);
        assert_eq!(levels[2 * 4 + 2], 0);
    }

    /// A triangle's diagonal: the coverage of the pixel its hypotenuse cuts in half is a half,
    /// which is a value halfway between two of the lattice's steps.
    #[test]
    fn a_diagonal_cuts_a_pixel_by_its_own_area() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(1.0, 1.0);
        builder.line_to(2.0, 1.0);
        builder.line_to(2.0, 2.0);
        builder.close();
        let path = builder.finish().expect("a triangle");
        let levels = levels(&path, tiny_skia::FillRule::Winding, (4, 4));
        assert_eq!(
            levels[4 + 1],
            128,
            "half of one pixel, to the nearest level"
        );
    }
}
