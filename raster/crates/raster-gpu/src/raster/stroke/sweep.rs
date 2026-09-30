//! Which of a stroke's pieces can share area, found by a sweep along `x` over their boxes
//! rather than by asking every pair (ADR 1421, ADR 1431).
//!
//! Two pieces whose boxes share no area share none either, so whether two subpaths' pieces
//! overlap is asked only of pairs whose boxes meet — a stroke of many subpaths, a table's
//! rules drawn as one path, has far more pairs than meetings. Sorted by their left edges, the
//! boxes that can meet a box are the ones still open when it opens, and the sweep looks at
//! those alone, up to a stated number of comparisons.
//!
//! **Not used for the tiling's own scan over earlier pieces**, and that was measured: within
//! one tight bend nearly every box shares its `x` range with every other, the scan's box test
//! is a handful of instructions, and a sweep that found only the pairs whose boxes meet cost
//! more in its sorting and bookkeeping than the tests it saved (ADR 1431).

use std::ops::ControlFlow;

use raster_scene::Point;

/// An axis-aligned box round a piece, for skipping the pairs that cannot meet.
#[derive(Debug, Clone, Copy)]
pub(super) struct Bounds {
    pub(super) min: Point,
    pub(super) max: Point,
}

impl Bounds {
    /// The box round `points`, or an empty box (one that meets nothing) where there are none.
    pub(super) fn of(points: &[Point]) -> Self {
        let mut bounds = Self::EMPTY;
        for p in points {
            bounds.min = Point::new(bounds.min.x.min(p.x), bounds.min.y.min(p.y));
            bounds.max = Point::new(bounds.max.x.max(p.x), bounds.max.y.max(p.y));
        }
        bounds
    }

    /// The box that meets nothing, and that every box contains.
    const EMPTY: Self = Self {
        min: Point::new(f32::INFINITY, f32::INFINITY),
        max: Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// The box round `boxes`, or an empty box where there are none.
    pub(super) fn round(boxes: impl Iterator<Item = Self>) -> Self {
        boxes.fold(Self::EMPTY, |a, b| Self {
            min: Point::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y)),
            max: Point::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y)),
        })
    }

    /// Whether the two boxes share any area. Boxes that only touch along an edge share
    /// none, which is how pieces that already meet edge to edge are passed over.
    pub(super) fn meets(self, other: Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// Hands `visit` every pair of `boxes` that share area, as `(opened earlier, opened later)`
/// in the sweep's order — the box with the lower left edge first.
///
/// Each box is compared with the boxes still open when it opens, those whose right edge
/// lies past its left edge; `budget` bounds how many such comparisons are made, and past
/// it the sweep stops with `Break`, as it does where `visit` asks it to.
#[expect(clippy::arithmetic_side_effects)] // a count of comparisons, at most `budget` plus a length
pub(super) fn meeting(
    boxes: &[Bounds],
    budget: usize,
    mut visit: impl FnMut(usize, usize) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_unstable_by(|&a, &b| boxes[a].min.x.total_cmp(&boxes[b].min.x));
    let (mut open, mut compared) = (Vec::<usize>::new(), 0_usize);
    for k in order {
        open.retain(|&m| boxes[m].max.x > boxes[k].min.x);
        compared += open.len();
        if compared > budget {
            return ControlFlow::Break(());
        }
        for &m in &open {
            if boxes[m].meets(boxes[k]) {
                visit(m, k)?;
            }
        }
        open.push(k);
    }
    ControlFlow::Continue(())
}
