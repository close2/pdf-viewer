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

use std::collections::{HashMap, HashSet};

use crate::{ClipId, Command, DisplayList, Rect, TargetSpec, Transform};

/// The device extent of every command a backend draws, in target pixels, in painting order.
///
/// A group contributes its elements rather than itself, because those are what a backend
/// walks; `None` means "can mark anywhere", which is what an extent this cannot bound has to
/// say for the estimate to stay conservative.
#[must_use]
pub fn command_extents(list: &DisplayList, target: TargetSpec) -> Vec<Option<Rect>> {
    let mut extents = Vec::new();
    let mut memo = HashMap::new();
    gather(
        list,
        list.commands(),
        target.transform,
        None,
        &mut extents,
        &mut memo,
    );
    extents
}

/// Walks a command list, meeting each command's own extent with the clip in force.
fn gather(
    list: &DisplayList,
    commands: &[Command],
    to_device: Transform,
    inherited: Option<Rect>,
    into: &mut Vec<Option<Rect>>,
    memo: &mut HashMap<ClipId, Option<Rect>>,
) {
    for command in commands {
        let clip = command.clip().map_or(inherited, |id| {
            meet(inherited, chain(list, id, to_device, memo))
        });
        if let Command::Group { commands, .. } = command {
            gather(list, commands, to_device, clip, into, memo);
        } else {
            into.push(meet(command.device_bounds(to_device), clip));
        }
    }
}

/// A clip chain's device extent: every clip in it, met.
///
/// Memoised per identifier, and that is not a micro-optimisation. A chain's extent is its own
/// clip met with its parent's chain, so walking each command's chain from the leaf costs the
/// depth *per command*: on `bug1721218_reduced.pdf` — 7050 commands over 3608 chains — the
/// un-memoised form took **606 ms**, six times the whole page's rasterisation, and this takes
/// 5. Once the planner runs on the drawing path rather than in an example, that is the
/// difference between a decision and a regression.
fn chain(
    list: &DisplayList,
    leaf: ClipId,
    to_device: Transform,
    memo: &mut HashMap<ClipId, Option<Rect>>,
) -> Option<Rect> {
    if let Some(held) = memo.get(&leaf) {
        return *held;
    }
    // Walked up to the first clip already known, then unwound so that every clip on the way
    // is recorded. The same bound `MaskCache` walks a chain under: a cycle cannot be built
    // through the public API, and a bound here costs nothing to state.
    let mut ancestry = Vec::new();
    let mut current = Some(leaf);
    let mut extent = None;
    let mut complete = false;
    for _ in 0..crate::MAX_GROUP_DEPTH {
        let Some(id) = current else {
            complete = true;
            break;
        };
        if let Some(held) = memo.get(&id) {
            extent = *held;
            complete = true;
            break;
        }
        let Some(clip) = list.clip(id) else {
            complete = true;
            break;
        };
        ancestry.push((id, clip.path.bounds(clip.transform.then(to_device))));
        current = clip.parent;
    }
    for (id, own) in ancestry.into_iter().rev() {
        extent = meet(extent, own);
        // A chain truncated by the depth bound is not this clip's whole chain, so recording
        // it would answer a later question with a partial walk.
        if complete {
            let _ = memo.insert(id, extent);
        }
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

/// The rows of the target a strip may not begin at.
///
/// One entry per target row, `true` where some segment that a cut would re-state — a curve, an
/// oblique edge, anything a stroker touches — spans that row, whether it belongs to a fill, a
/// stroke, a clip path or a soft mask's group. Cutting there chops the segment against the
/// strip's edge and changes the coverage of an edge pixel by up to a quarter of a channel
/// (ADR 0138 measured the consequence; ADR 0139 measured the rule, and
/// [`crate::Path::oblique_spans`] is where the three cases are tabulated).
///
/// # What it is conservative about
///
/// A curve's span comes from its control hull, which is at least the curve; a *stroke* counts
/// its whole extent whatever its geometry, for the two reasons stated where it is counted; and
/// a clip's segments are counted wherever the chain is used rather than where it admits
/// anything. Every one of those errs towards refusing a cut, which is the direction that keeps
/// the strips exact.
#[must_use]
pub fn unsplittable_rows(list: &DisplayList, target: TargetSpec) -> Vec<bool> {
    let mut rows = vec![false; target.height as usize];
    let mut clips = HashSet::new();
    segments(
        list,
        list.commands(),
        target.transform,
        &mut rows,
        &mut clips,
    );
    rows
}

/// Walks a command list, marking the rows every segment it can reach would be re-stated in.
fn segments(
    list: &DisplayList,
    commands: &[Command],
    to_device: Transform,
    rows: &mut [bool],
    seen: &mut HashSet<ClipId>,
) {
    for command in commands {
        if let Some(id) = command.clip() {
            clip_segments(list, id, to_device, rows, seen);
        }
        if let Some(mask) = command.mask().and_then(|id| list.soft_mask(id)) {
            segments(list, &mask.commands, to_device, rows, seen);
        }
        match command {
            Command::Fill {
                path, transform, ..
            } => path.oblique_spans(transform.then(to_device), |top, bottom| {
                mark(rows, top, bottom);
            }),
            // A stroke's whole extent, whatever its geometry. Two reasons, and the second is
            // why the first is not enough: a stroker turns a path into an outline, so a round
            // cap or join puts a curve on a straight path; and a stroke thin enough to be a
            // hairline is not turned into an outline at all but scan-converted by
            // `tiny-skia`'s own hairline path, which clips the *line* against the target and
            // so re-states its endpoints. Distinguishing the cases would be a claim about a
            // dependency's internals, and strokes are a small share of the rows on every page
            // measured — page 101 of ISO 32000-2 loses 3% of its legal rows to this.
            Command::Stroke { .. } => {
                if let Some(extent) = command.device_bounds(to_device) {
                    mark(rows, extent.min.y, extent.max.y);
                } else {
                    rows.fill(true);
                }
            }
            // An image is drawn into the parallelogram its transform names, whose edges are
            // straight, and its samples are read at absolute device positions.
            Command::Image { .. } => {}
            Command::Group { commands, .. } => segments(list, commands, to_device, rows, seen),
        }
    }
}

/// Marks the rows every clip in a chain re-states a segment in, once per chain leaf met.
fn clip_segments(
    list: &DisplayList,
    leaf: ClipId,
    to_device: Transform,
    rows: &mut [bool],
    seen: &mut HashSet<ClipId>,
) {
    // Stops at the first clip already walked: marking a chain marks all of its ancestors, so
    // one clip's segments are counted once however many chains end below it.
    let mut current = Some(leaf);
    for _ in 0..crate::MAX_GROUP_DEPTH {
        let Some(id) = current else { break };
        if !seen.insert(id) {
            break;
        }
        let Some(clip) = list.clip(id) else { break };
        clip.path
            .oblique_spans(clip.transform.then(to_device), |top, bottom| {
                mark(rows, top, bottom);
            });
        current = clip.parent;
    }
}

/// Marks the rows a boundary may not fall on, given a segment spanning `top..bottom`.
///
/// A boundary at row `r` puts rows `r..` in the next strip, so it cuts the target along the
/// line `y = r`: the segment is chopped exactly when `top < r < bottom`. That is rows
/// `floor(top) + 1` up to `ceil(bottom) - 1`, and no more — probed at every row either side of
/// a curve's extent, where 100 and 970 are exact and 101 and 969 are not (ADR 0139).
fn mark(rows: &mut [bool], top: f32, bottom: f32) {
    let count = rows.len();
    let from = clamp_row(top.floor() + 1.0, count);
    let to = clamp_row(bottom.ceil(), count);
    for row in rows.get_mut(from..to).into_iter().flatten() {
        *row = true;
    }
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

/// How many times a split replays the command list, as a multiple of the list itself.
///
/// A strip replays every command whose extent reaches its rows, and a command replayed pays
/// again for the work that does not shrink with the band — building a path, taking its bounds,
/// compiling a raster pipeline, converting an image's samples. ADR 0137 measured that at 19%
/// of a dense text page's rasterisation and counted this ratio at **1.01 to 1.13** over eight
/// strips on four pages, which is what said the duplication was not the problem.
///
/// It is not always. `issue12841_reduced.pdf` is **two** commands, each covering the page, so
/// sixteen strips replay both sixteen times: a ratio of 16, and the page took 105 ms serially
/// and 166 ms split. So the ratio is computed per page rather than trusted per corpus, and
/// [`strip_boundaries_avoiding`]'s caller declines a split whose replay costs more than the
/// division saves.
///
/// One where nothing is drawn, so a caller comparing against a threshold need not special-case
/// an empty page.
#[must_use]
pub fn replay_ratio(extents: &[Option<Rect>], boundaries: &[u32]) -> f64 {
    let strips = boundaries.len().saturating_sub(1);
    let mut touches = 0_u64;
    let mut commands = 0_u64;
    for extent in extents.iter().flatten() {
        commands = commands.saturating_add(1);
        for strip in 0..strips {
            let top = boundaries.get(strip).copied().unwrap_or(0);
            let bottom = boundaries
                .get(strip.saturating_add(1))
                .copied()
                .unwrap_or(0);
            if extent.min.y < row_edge(bottom) && extent.max.y > row_edge(top) {
                touches = touches.saturating_add(1);
            }
        }
    }
    if commands == 0 {
        return 1.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts of one page's commands, far inside f64's exact integer range"
    )]
    let ratio = touches as f64 / commands as f64;
    ratio
}

/// A row index as a device coordinate, for comparing an extent against a strip's edges.
#[expect(
    clippy::cast_precision_loss,
    reason = "a row index below MAX_EXTENT = 2^24, exact in f32"
)]
fn row_edge(row: u32) -> f32 {
    row as f32
}

/// The rows at which to cut, balanced by cost as [`strip_boundaries`] is but choosing only
/// among rows [`curved_rows`] permits.
///
/// Returns at most `strips + 1` boundaries and possibly fewer: where no legal row separates two
/// cuts the strip is not made, because an illegal cut is not a slower answer but a different
/// picture. A result of `[0, height]` means the target cannot be divided at all, which is what
/// a page stating one curve down its whole height gives.
///
/// # Why this one is optimal and [`strip_boundaries`] is not
///
/// A prefix sum over every row lands within 4% of a perfect split (ADR 0137), so nothing
/// cleverer was worth writing. With the cut rows restricted it is not: page 101 of ISO 32000-2
/// grants 27% of its rows, and snapping each prefix-sum boundary to the nearest of them gives a
/// worst strip of 24.5% against a 12.5% ideal. So the split is the smallest achievable maximum
/// instead — a binary search on that maximum with a greedy feasibility test, which is
/// `O(strips · log rows)` per probe and exact for the estimate it is given.
#[must_use]
pub fn strip_boundaries_avoiding(
    costs: &[f64],
    unsplittable: &[bool],
    strips: u32,
    least: u32,
) -> Vec<u32> {
    let rows = costs.len();
    let strips = strips.max(1) as usize;
    let least = least.max(1) as usize;
    let height = u32::try_from(rows).unwrap_or(u32::MAX);
    if strips == 1 || rows < strips.saturating_mul(least) {
        return vec![0, height];
    }

    // `prefix[row]` is what rows `0..row` cost, so a strip's cost is one subtraction.
    let mut prefix = Vec::with_capacity(rows.saturating_add(1));
    let mut running = 0.0_f64;
    prefix.push(running);
    for cost in costs {
        running += *cost;
        prefix.push(running);
    }
    // `previous[row]` is the largest legal cut at or below `row`, so the greedy walk can step
    // back from the furthest row a limit allows to the furthest it may actually cut at.
    let mut previous = Vec::with_capacity(rows.saturating_add(1));
    let mut held = 0_u32;
    for row in 0..=rows {
        if row == 0 || row == rows || unsplittable.get(row).is_some_and(|held| !*held) {
            held = u32::try_from(row).unwrap_or(u32::MAX);
        }
        previous.push(held);
    }

    let total = running;
    let (mut low, mut high) = (total / row_count(strips), total);
    // A limit is a cost, and costs here are sums of pixel widths: forty halvings take the
    // interval below any difference that could move a boundary.
    for _ in 0..40 {
        let middle = f64::midpoint(low, high);
        if feasible(&prefix, &previous, strips, least, middle).is_some() {
            high = middle;
        } else {
            low = middle;
        }
    }
    feasible(&prefix, &previous, strips, least, high).unwrap_or_else(|| vec![0, height])
}

/// The greedy split under a cost limit: every strip reaches as far as the limit and the legal
/// rows allow. `None` if that needs more than `strips` of them.
///
/// Reaching as far as possible is optimal for feasibility — a shorter first strip leaves a
/// longer remainder, which no later choice can improve on — so a limit this refuses is a limit
/// no split achieves.
fn feasible(
    prefix: &[f64],
    previous: &[u32],
    strips: usize,
    least: usize,
    limit: f64,
) -> Option<Vec<u32>> {
    let rows = prefix.len().saturating_sub(1);
    let mut boundaries = vec![0_u32];
    let mut start = 0_usize;
    while start < rows {
        // The furthest row whose strip cost stays inside the limit, then the furthest legal
        // cut at or below it.
        let ceiling = prefix.get(start).copied().unwrap_or(0.0) + limit;
        let reach = prefix
            .partition_point(|at| *at <= ceiling)
            .saturating_sub(1);
        let cut = previous.get(reach.max(start))?;
        let cut = usize::try_from(*cut).unwrap_or(rows);
        if cut < start.saturating_add(least) {
            return None;
        }
        // A tail too short to be a strip is not left as one: `tiny-skia` refuses a hairline
        // stroke into a target under three rows tall, and a strip of a handful of rows is not
        // worth a thread in any case.
        let cut = if rows.saturating_sub(cut) < least {
            rows
        } else {
            cut
        };
        boundaries.push(u32::try_from(cut).unwrap_or(u32::MAX));
        start = cut;
        if boundaries.len().saturating_sub(1) > strips {
            return None;
        }
    }
    (boundaries.len() > 1).then_some(boundaries)
}

#[cfg(test)]
mod tests {
    use super::{row_costs, strip_boundaries, strip_boundaries_avoiding};
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

    /// The rule ADR 0139 measured: a cut lands on a row no curve crosses. Asserted as a
    /// property rather than as a list, because the list is what the search is free to change
    /// and the two things that must hold are that no cut is illegal and that the split is
    /// still balanced.
    #[test]
    fn every_cut_lands_where_no_curve_crosses() {
        let mut curved = vec![false; 800];
        // Every row costs the same, so the unconstrained cuts are 200, 400 and 600. Forbid a
        // window around two of them.
        for row in curved.get_mut(195..=203).into_iter().flatten() {
            *row = true;
        }
        for row in curved.get_mut(398..=412).into_iter().flatten() {
            *row = true;
        }
        let costs = vec![1.0_f64; 800];
        let boundaries = strip_boundaries_avoiding(&costs, &curved, 4, 1);

        assert_eq!(boundaries.len(), 5, "{boundaries:?} is not four strips");
        for cut in boundaries.iter().skip(1).take(3) {
            assert!(
                !curved[*cut as usize],
                "cut at row {cut}, which a curve crosses"
            );
        }
        let worst = boundaries
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .max()
            .unwrap_or(0);
        // 202 is the smallest achievable maximum: a strip of 201 rows forces the second cut
        // below 396 and the third above 599, which the forbidden window at 398..=412 refuses.
        assert_eq!(worst, 202);
    }

    /// A page whose curves cross every row cannot be cut at all, and saying so is how a
    /// backend knows to draw it serially rather than to draw it differently.
    #[test]
    fn a_page_curved_everywhere_is_not_divided() {
        let costs = vec![1.0_f64; 800];
        assert_eq!(
            strip_boundaries_avoiding(&costs, &[true; 800], 4, 1),
            vec![0, 800]
        );
    }

    /// Two cuts may not collapse onto one row: a strip of no rows is not a strip, and the
    /// count returned has to be what the caller can use.
    #[test]
    fn cuts_that_cannot_be_separated_become_fewer_strips() {
        let mut curved = vec![true; 800];
        curved[400] = false;
        let costs = vec![1.0_f64; 800];
        assert_eq!(
            strip_boundaries_avoiding(&costs, &curved, 4, 1),
            vec![0, 400, 800]
        );
    }
}
