//! §8.7.3.1: which sites of a tiling's lattice a path's bounds touch, in the pattern's own space,
//! and how far one site sits from the site its cell was drawn at (ADR 2081).

use pdf_render::{Path, PathCommand, Point, Transform};

use crate::content::Interpreter;

use super::tiling::Tiling;

/// Where one site of a tiling sits relative to the site its cell was drawn at, in page space.
///
/// ISO 32000-2 §8.7.3.1 puts site (i, j) at i × `/XStep` and j × `/YStep` in *pattern* space, so
/// the displacement between two sites is that difference carried through the pattern matrix —
/// and through its linear part alone, because the matrix's own translation is common to both
/// sites and cancels. The result is therefore a pure translation of page space whatever the
/// pattern matrix rotates or shears, which is what [`pdf_render::Cell::repeat`] wants.
pub(super) fn displacement(tiling: &Tiling, columns: i32, rows: i32) -> Transform {
    let to_page = tiling.to_page;
    let x = tiling.step.0 * as_f32(columns);
    let y = tiling.step.1 * as_f32(rows);
    Transform::translate(
        to_page.a.mul_add(x, to_page.c * y),
        to_page.b.mul_add(x, to_page.d * y),
    )
}

/// The bounding box of a path once transformed, as `(min_x, min_y, max_x, max_y)`.
pub(super) fn bounds_of(path: &Path, transform: Transform) -> Option<(f32, f32, f32, f32)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    let mut visit = |point: Point| {
        let at = transform.apply(point);
        if !at.x.is_finite() || !at.y.is_finite() {
            return;
        }
        bounds = Some(match bounds {
            None => (at.x, at.y, at.x, at.y),
            Some((x0, y0, x1, y1)) => (x0.min(at.x), y0.min(at.y), x1.max(at.x), y1.max(at.y)),
        });
    };
    for command in path.commands() {
        match command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => visit(*point),
            // A curve stays inside the hull of its control points, so those bound it —
            // loosely, which only ever draws tiles that turn out to be clipped away.
            PathCommand::CurveTo(a, b, c) => {
                visit(*a);
                visit(*b);
                visit(*c);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// Which tiles of a pattern the given bounds in pattern space touch, by column and by row.
///
/// Where the cell itself is matters, which is [`span`]'s subject. Table 74 makes `/BBox`
/// required, and a pattern that states none is tiled as though its cell began at the origin.
pub(super) fn spans(tiling: &Tiling, bounds: (f32, f32, f32, f32)) -> ((i32, i32), (i32, i32)) {
    let cell = tiling
        .bbox
        .unwrap_or([0.0, 0.0, tiling.step.0, tiling.step.1]);
    (
        span(
            bounds.0,
            bounds.2,
            tiling.step.0,
            cell[0].min(cell[2]),
            cell[0].max(cell[2]),
        ),
        span(
            bounds.1,
            bounds.3,
            tiling.step.1,
            cell[1].min(cell[3]),
            cell[1].max(cell[3]),
        ),
    )
}

/// How many columns or rows an inclusive index range spans, and never fewer than one.
pub(super) fn extent(first: i32, last: i32) -> usize {
    usize::try_from(last.saturating_sub(first).saturating_add(1))
        .unwrap_or(usize::MAX)
        .max(1)
}

/// The range of tile indices covering an interval, given a step and where the cell itself sits.
///
/// §8.7.3.1 places the pattern cell where its content stream draws it and replicates that at
/// multiples of `/XStep` and `/YStep` — so the offsets needed to cover `low..high` are measured
/// from the **cell's own extent**, not from the pattern space's origin. Tile `k` covers
/// `cell + k × step`, so it is wanted when `cell_low + k × step <= high` and
/// `cell_high + k × step >= low`.
///
/// **Measuring from the origin instead is nearly invisible**, because Table 74's `/BBox` is
/// nearly always at the pattern's origin: the ±1 of slack `floor` and `ceil` give covers a cell
/// within one step of it. `issue13561_reduced.pdf` states `/BBox [35.4 396.6 287.4 588]` against
/// a `/YStep` of 191.4 — two steps out — which lands every tile two rows below the page from
/// the origin (ADR 0169).
fn span(low: f32, high: f32, step: f32, cell_low: f32, cell_high: f32) -> (i32, i32) {
    /// Bounds the index range so a huge path or a tiny step cannot overflow.
    const LIMIT: f32 = 1e6;

    let first = ((low - cell_high) / step).floor().clamp(-LIMIT, LIMIT);
    let last = ((high - cell_low) / step).ceil().clamp(-LIMIT, LIMIT);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both are clamped to a million, well inside i32"
    )]
    {
        (first as i32, last as i32)
    }
}

/// Widens a tile index for arithmetic in pattern space.
pub(super) fn as_f32(index: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "tile indices are clamped to a million, exact in f32"
    )]
    {
        index as f32
    }
}

/// Returns a step only if it is usable as one.
///
/// A zero step would place every cell on top of the last, which is an infinite loop rather
/// than a pattern; the specification forbids it. A negative one is legal and tiles in the
/// other direction, so only its magnitude matters here.
pub(super) fn non_zero(step: f32) -> Option<f32> {
    let step = step.abs();
    (step.is_finite() && step > 0.0).then_some(step)
}

/// The width or height of a pattern cell's bounding box, as a fallback step.
pub(super) fn cell_extent(bbox: &[f32], axis: usize) -> Option<f32> {
    let low = bbox.get(axis)?;
    let high = bbox.get(axis.checked_add(2)?)?;
    non_zero(high - low)
}

impl Interpreter<'_> {
    /// Table 74's `/TilingType` applied to one pattern's matrix (§8.7.3.1).
    ///
    /// Codes 1 and 3 ask for cells "spaced consistently - that is, by a multiple of a device
    /// pixel"; [`pdf_render::snap_lattice`] is that sentence's arithmetic and states what each
    /// code gets. This function is the other half of the answer — **where a device pixel comes
    /// from** — and there is exactly one place in this tree that knows: `ViewState`'s
    /// magnification, in device pixels per default user space unit, which §12.5.3's `NoZoom` put
    /// there for the same reason. A caller that has stated none is drawing at no particular
    /// resolution, so there is no grid to snap to and the tiling is placed at the geometry the
    /// file states — which is code 2's treatment, and which is what every caller that never
    /// mentions a magnification has always had.
    ///
    /// A page whose lattice the magnification can move is view-dependent in the sense
    /// [`Interpretation::view_dependent`](crate::content::Interpretation::view_dependent) already
    /// carries, so [`Self::magnified_tiling`] is set and a zoom re-interprets rather than
    /// re-rasterising. It is set whenever a constant-spacing tiling is placed under a stated
    /// magnification, and not only where the snap was taken: whether it is taken is itself a
    /// function of the magnification, so a page where it was declined would otherwise keep a list
    /// that a different magnification would have snapped. ADR 1080.
    pub(super) fn lattice(
        &mut self,
        tiling_type: Option<pdf_render::TilingType>,
        stated: Transform,
        step: (f32, f32),
        cell: Option<[f32; 4]>,
    ) -> Transform {
        if !tiling_type.is_some_and(pdf_render::TilingType::constant_spacing) {
            return stated;
        }
        let Some(pixels_per_unit) = self.view.magnification() else {
            return stated;
        };
        self.magnified_tiling = true;
        // What the distortion is measured across. Table 74 makes `/BBox` required and a pattern
        // stating none is tiled as though its cell were one step across, which is the same
        // fallback `spans` uses.
        let extent = cell.map_or((step.0.abs(), step.1.abs()), |corners| {
            (
                (corners[2] - corners[0]).abs(),
                (corners[3] - corners[1]).abs(),
            )
        });
        pdf_render::snap_lattice(stated, step, extent, pixels_per_unit)
            .map_or(stated, |lattice| lattice.to_page)
    }
}
