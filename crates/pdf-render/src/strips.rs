//! Cutting a target into horizontal strips of roughly equal cost.
//!
//! A display list's commands are independent of one another — every one carries its own
//! absolute transform and clip, which is what [`crate::Command`]'s own doc comment calls the
//! property "that allows a backend to reorder or parallelise them". Cutting the target into
//! runs of rows and replaying the list into each is the decomposition that follows from it,
//! and it is shared here rather than owned by a backend because both of them band: the CPU
//! backend to parallelise (ADR 0137), the GPU backend because Vello's working buffers are
//! fixed constants a page can exceed (ADR 0127).
//!
//! # Why the strips are not of equal height
//!
//! Threads finish when the last one does, so what a split is judged by is its *worst* strip.
//! Equal heights are adequate on a uniformly inked page and useless on a page whose ink is
//! concentrated: `bug1721218_reduced.pdf` is one wide gradient over part of its height, and
//! equal heights hand one strip 72% of the work — a 1.4× ceiling on eight threads. Cutting
//! where the estimated *cost* is even takes the same page's worst strip to 12.8% against a
//! 12.5% ideal, and every page measured in ADR 0137 lands within 4% of ideal.
//!
//! # What the estimate is, and is not
//!
//! A row's cost is the summed width of every command that can mark it. That is an estimate
//! of blending work and it ignores edge building, which is proportional to a path's
//! complexity rather than to its bounding box; it also treats a diagonal hairline's bounding
//! box as though it were inked. Both errors apply to every row alike, which is the direction
//! that matters for choosing *where* to cut rather than for predicting a time.

use crate::{Command, DisplayList, Rect, TargetSpec, Transform};

/// The device extent of every command a backend draws, in target pixels, in painting order.
///
/// A group contributes its elements rather than itself, because those are what a backend
/// walks; `None` means "can mark anywhere", which is what an extent this cannot bound has to
/// say for the estimate to stay conservative.
#[must_use]
pub fn command_extents(list: &DisplayList, target: TargetSpec) -> Vec<Option<Rect>> {
    let mut extents = Vec::new();
    gather(list, list.commands(), target.transform, None, &mut extents);
    extents
}

/// Walks a command list, meeting each command's own extent with the clip in force.
fn gather(
    list: &DisplayList,
    commands: &[Command],
    to_device: Transform,
    inherited: Option<Rect>,
    into: &mut Vec<Option<Rect>>,
) {
    for command in commands {
        let clip = command
            .clip()
            .map_or(inherited, |id| meet(inherited, chain(list, id, to_device)));
        if let Command::Group { commands, .. } = command {
            gather(list, commands, to_device, clip, into);
        } else {
            into.push(meet(command.device_bounds(to_device), clip));
        }
    }
}

/// A clip chain's device extent: every clip in it, met.
fn chain(list: &DisplayList, leaf: crate::ClipId, to_device: Transform) -> Option<Rect> {
    let mut extent = None;
    let mut current = Some(leaf);
    let mut first = true;
    // The same bound `MaskCache` walks a chain under: a cycle cannot be built through the
    // public API, and a bound here costs nothing to state.
    for _ in 0..crate::MAX_GROUP_DEPTH {
        let Some(id) = current else { break };
        let Some(clip) = list.clip(id) else { break };
        let one = clip.path.bounds(clip.transform.then(to_device));
        extent = if first { one } else { meet(extent, one) };
        first = false;
        current = clip.parent;
    }
    extent
}

/// The intersection of two extents, where `None` on either side means "unbounded".
fn meet(left: Option<Rect>, right: Option<Rect>) -> Option<Rect> {
    match (left, right) {
        (Some(left), Some(right)) => {
            let met = Rect {
                min: crate::Point::new(left.min.x.max(right.min.x), left.min.y.max(right.min.y)),
                max: crate::Point::new(left.max.x.min(right.max.x), left.max.y.min(right.max.y)),
            };
            (met.min.x < met.max.x && met.min.y < met.max.y).then_some(met)
        }
        (Some(one), None) | (None, Some(one)) => Some(one),
        (None, None) => None,
    }
}

/// What each row of the target costs, estimated as the summed width of what can mark it.
///
/// One entry per target row. An extent of `None` can mark any row, so it contributes the
/// full width to all of them.
#[must_use]
pub fn row_costs(extents: &[Option<Rect>], target: TargetSpec) -> Vec<f64> {
    let rows = target.height as usize;
    let width = f64::from(target.width);
    let mut costs = vec![0.0_f64; rows];
    for extent in extents {
        let (from, to, span) = match *extent {
            Some(rect) => {
                let from = clamp_row(rect.min.y, rows);
                let to = clamp_row(rect.max.y.ceil(), rows).max(from.saturating_add(1));
                (from, to, f64::from(rect.width()).clamp(0.0, width))
            }
            None => (0, rows, width),
        };
        for cost in costs.get_mut(from..to.min(rows)).into_iter().flatten() {
            *cost += span;
        }
    }
    costs
}

/// A row count as a float, for comparing a device coordinate against it.
///
/// Exact: `MAX_EXTENT` bounds a target at 2^24 rows, far inside `f64`'s 53-bit mantissa.
#[expect(
    clippy::cast_precision_loss,
    reason = "a row count below MAX_EXTENT = 2^24, exact in f64"
)]
fn row_count(rows: usize) -> f64 {
    rows as f64
}

/// A device y coordinate as a row index inside the target.
fn clamp_row(y: f32, rows: usize) -> usize {
    if y <= 0.0 {
        0
    } else if f64::from(y) >= row_count(rows) {
        rows
    } else {
        // Inside `0..rows` by the two branches above, so the cast neither truncates
        // meaningfully nor loses a sign.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded to 0..rows by the branches above"
        )]
        let row = y as usize;
        row
    }
}

/// The rows at which to cut, so that each strip holds about the same estimated cost.
///
/// Returns `strips + 1` boundaries, the first `0` and the last the target's height — or
/// fewer, when the target has too few rows to divide. A single boundary pair means "do not
/// split", which is what a caller should treat as the serial case.
///
/// The split is a prefix sum and nothing cleverer: rows are atomic, the cost is already an
/// estimate, and an exactly optimal partition of a sequence would be a dynamic program
/// answering a question the estimate cannot pose that precisely.
#[must_use]
pub fn strip_boundaries(costs: &[f64], strips: u32) -> Vec<u32> {
    let rows = costs.len();
    let strips = strips.max(1) as usize;
    if strips == 1 || rows < strips {
        return vec![0, u32::try_from(rows).unwrap_or(u32::MAX)];
    }

    let total: f64 = costs.iter().sum();
    let mut boundaries = vec![0_u32];
    if total <= 0.0 {
        // Nothing is drawn, so any split is as good as any other and equal rows is the one
        // that needs no justification.
        for strip in 1..=strips {
            let row = rows.saturating_mul(strip).checked_div(strips).unwrap_or(0);
            boundaries.push(u32::try_from(row).unwrap_or(u32::MAX));
        }
        return boundaries;
    }

    let mut running = 0.0;
    for (row, cost) in costs.iter().enumerate() {
        running += *cost;
        let wanted = row_count(boundaries.len()) * total / row_count(strips);
        if running >= wanted && boundaries.len() < strips {
            boundaries.push(u32::try_from(row.saturating_add(1)).unwrap_or(u32::MAX));
        }
    }
    while boundaries.len() <= strips {
        boundaries.push(u32::try_from(rows).unwrap_or(u32::MAX));
    }
    boundaries
}

#[cfg(test)]
mod tests {
    use super::{row_costs, strip_boundaries};
    use crate::{Rect, TargetSpec, Transform};

    fn target(width: u32, height: u32) -> TargetSpec {
        TargetSpec {
            width,
            height,
            transform: Transform::IDENTITY,
        }
    }

    fn rect(top: f32, bottom: f32, width: f32) -> Rect {
        Rect::from_corners(
            crate::Point::new(0.0, top),
            crate::Point::new(width, bottom),
        )
    }

    /// The whole point of the module: a page whose ink sits in one place is cut so that the
    /// strips still cost the same. Equal heights would give the first strip everything.
    #[test]
    fn ink_in_one_place_is_still_divided_evenly() {
        let target = target(100, 800);
        let costs = row_costs(&[Some(rect(0.0, 200.0, 100.0))], target);
        let boundaries = strip_boundaries(&costs, 4);

        assert_eq!(boundaries.first().copied(), Some(0));
        assert_eq!(boundaries.last().copied(), Some(800));
        // Every cut is inside the inked 200 rows, not spread over the empty 800.
        for boundary in boundaries.iter().skip(1).take(3) {
            assert!(
                *boundary <= 201,
                "a cut at row {boundary} put empty rows in a strip that has work to do"
            );
        }
    }

    /// A uniformly inked page gets the split equal heights would have given it, which is
    /// what says the estimate is not doing something exotic on the ordinary case.
    #[test]
    fn uniform_ink_is_cut_at_equal_heights() {
        let target = target(100, 800);
        let costs = row_costs(&[Some(rect(0.0, 800.0, 100.0))], target);
        assert_eq!(strip_boundaries(&costs, 4), vec![0, 200, 400, 600, 800]);
    }

    /// An unbounded extent is what a command this cannot bound reports, and it must be
    /// counted everywhere rather than nowhere.
    #[test]
    fn an_unbounded_command_costs_every_row() {
        let target = target(100, 8);
        assert_eq!(row_costs(&[None], target), vec![100.0; 8]);
    }

    /// A target with fewer rows than strips cannot be divided, and saying so is how the
    /// caller knows to stay serial rather than making empty strips.
    #[test]
    fn a_target_too_short_to_divide_says_so() {
        assert_eq!(strip_boundaries(&[1.0, 1.0], 8), vec![0, 2]);
        assert_eq!(strip_boundaries(&[1.0; 8], 1), vec![0, 8]);
    }

    /// A blank page still divides, because the caller has already decided to split and an
    /// empty split would be a strip of zero rows.
    #[test]
    fn a_page_with_no_ink_is_cut_at_equal_heights() {
        assert_eq!(
            strip_boundaries(&[0.0; 800], 4),
            vec![0, 200, 400, 600, 800]
        );
    }
}
