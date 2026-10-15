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

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use rayon::prelude::*;

use crate::{
    BlendMode, ClipId, Command, DisplayList, MarkBounds, Paint, Path, Rect, Stroke, TargetSpec,
    Transform,
};

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
        gather_one(
            list,
            command,
            to_device,
            inherited,
            OwnBound::Unknown,
            into,
            memo,
        );
    }
}

/// [`gather`] for one command: its own extent met with its clip, or a group's elements'.
///
/// `own` is the command's own [`Command::device_bounds`] where a caller already holds it —
/// [`strip_inputs`] reads a top-level command's from the list's [`MarkBounds`].
fn gather_one(
    list: &DisplayList,
    command: &Command,
    to_device: Transform,
    inherited: Option<Rect>,
    own: OwnBound,
    into: &mut Vec<Option<Rect>>,
    memo: &mut HashMap<ClipId, Option<Rect>>,
) {
    let clip = command.clip().map_or(inherited, |id| {
        meet(inherited, chain(list, id, to_device, memo))
    });
    if let Command::Group { commands, .. } = command {
        gather(list, commands, to_device, clip, into, memo);
    } else {
        into.push(meet(own.of(command, to_device), clip));
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

/// A command's own [`Command::device_bounds`], as the walk that needs it is handed it.
#[derive(Clone, Copy, Debug)]
enum OwnBound {
    /// Not measured yet: the walk measures it.
    Unknown,
    /// Measured already, under the walk's own transform: the function's answer, `None` included.
    Known(Option<Rect>),
}

impl OwnBound {
    /// The bound, measured now where it was not held.
    fn of(self, command: &Command, to_device: Transform) -> Option<Rect> {
        match self {
            Self::Unknown => command.device_bounds(to_device),
            Self::Known(bound) => bound,
        }
    }
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
        let (from, to, span) = rows_and_span(*extent, rows, width);
        for cost in costs.get_mut(from..to.min(rows)).into_iter().flatten() {
            *cost += span;
        }
    }
    costs
}

/// The rows `extent` adds its cost to, `from..to`, and the cost it adds to each: [`row_costs`]'s
/// arithmetic for one extent, which [`StripInputs::costs`] repeats per band of rows.
fn rows_and_span(extent: Option<Rect>, rows: usize, width: f64) -> (usize, usize, f64) {
    match extent {
        Some(rect) => {
            let from = clamp_row(rect.min.y, rows);
            let to = clamp_row(rect.max.y.ceil(), rows).max(from.saturating_add(1));
            (from, to, f64::from(rect.width()).clamp(0.0, width))
        }
        None => (0, rows, width),
    }
}

/// The rows of the target a strip may not begin at.
///
/// One entry per target row, `true` where some segment that a cut would re-state — a curve, an
/// oblique edge, anything a stroker touches — spans that row, whether it belongs to a fill, a
/// stroke, a clip path or a soft mask's group. Every stroke counts its whole extent here and every
/// fill its curves' and oblique edges' rows; a backend that can say which of its marks it draws a
/// row at a time asks [`unsplittable_rows_where`] instead. Cutting there chops the segment against the
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
    unsplittable_rows_where(list, target, &|_| false)
}

/// A fill as the planner meets it, for a backend to say whether it draws it a row at a time.
///
/// Stated in the target's own pixels, as [`PlannedStroke`] is and for its reason: none of the
/// fields is a strip's, so one answer stands for every division.
#[derive(Clone, Copy, Debug)]
pub struct PlannedFill<'a> {
    /// The path, in its own space.
    pub path: &'a Path,
    /// The path's own space onto the target's pixels — the page's, never a strip's.
    pub at: Transform,
    /// What it can mark on the target, as [`Command::device_bounds`] bounds it; `None` where that
    /// cannot say.
    pub extent: Option<Rect>,
    /// Whether it may be composited by anything but painting over what is below it, as
    /// [`PlannedStroke::replaces`] states it.
    pub replaces: bool,
}

/// A mark the planner would forbid rows for, put to the backend that draws it.
#[derive(Clone, Copy, Debug)]
pub enum PlannedMark<'a> {
    /// A fill, which forbids its curves' and oblique edges' rows unless the backend answers
    /// otherwise.
    Fill(PlannedFill<'a>),
    /// A stroke, which forbids its whole extent unless the backend answers otherwise.
    Stroke(PlannedStroke<'a>),
}

/// A stroke as the planner meets it, for a backend to say whether it draws it a row at a time.
///
/// Every field is a fact the backend's drawing of the stroke is decided by, stated in the target's
/// own pixels: none of them is a strip's, which is what lets one answer stand for every division.
#[derive(Clone, Copy, Debug)]
pub struct PlannedStroke<'a> {
    /// The path, in its own space.
    pub path: &'a Path,
    /// How it is stroked.
    pub stroke: &'a Stroke,
    /// How it is painted: a backend that states a mark's coverage in closed form asks it, as
    /// [`crate::thin_segment`] does (ADR 1847).
    pub paint: &'a Paint,
    /// How it combines with the backdrop, asked for the same reason as [`Self::paint`].
    pub blend: BlendMode,
    /// The path's own space onto the target's pixels — the page's, never a strip's.
    pub at: Transform,
    /// What it can mark on the target, as [`Command::device_bounds`] bounds it; `None` where that
    /// cannot say.
    pub extent: Option<Rect>,
    /// Whether it may be composited by anything but painting over what is below it: an element
    /// of a knockout group (§11.4.6), a half of a [`Command::Shaped`], or a mark in a soft mask's
    /// group, the last two counted here because the planner does not follow how they are drawn.
    pub replaces: bool,
}

/// [`unsplittable_rows`], with the marks `row_local` answers `true` for forbidding no row.
///
/// # Why a mark is the backend's question
///
/// A cut is exact where every mark crossing it draws the same bytes into the two strips as into
/// the whole page. ADR 0139 measured which marks do that under `tiny-skia`'s scan converter —
/// axis-aligned edges, never a curve or an oblique edge — and a stroke's outline is a stroker's
/// construction: a round cap or join puts a curve on a straight path. So under that converter a
/// stroke has to forbid the rows of everything it can put there, which [`unsplittable_rows`] takes
/// to be its whole extent.
///
/// A converter that evaluates each row from the edges' own device coordinates owes no row at
/// all: a row of the page and the same row of a strip are the same two evaluations, wherever the
/// strip begins. That is a fact about one backend's drawing, decided by its anti-aliasing, the
/// width it strokes at and the composition it uses, so the backend answers it per stroke and this
/// keeps the rows of everything else — clips, and the strokes it answers `false` for — exactly as
/// [`unsplittable_rows`] does. ADR 1819.
///
/// **A fill is the same question about the same converter**, and ADR 0139's geometric rule for it
/// — a curve's rows and an oblique edge's — is the same statement about a clipping scan converter
/// that the stroke rule was. So a fill that would forbid a row under that rule is asked too, as a
/// [`PlannedFill`]; one already inside forbidden rows is not asked, because no answer could
/// change them. ADR 1837.
///
/// A mark's clip chain and soft mask are counted whatever the answer: they are marks of their
/// own, drawn by their own rules.
#[must_use]
pub fn unsplittable_rows_where(
    list: &DisplayList,
    target: TargetSpec,
    row_local: &dyn Fn(PlannedMark<'_>) -> bool,
) -> Vec<bool> {
    let mut rows = vec![false; target.height as usize];
    let mut clips = HashSet::new();
    let walk = Walk {
        list,
        to_device: target.transform,
        row_local,
        freed: None,
    };
    walk.segments(list.commands(), false, &mut rows, &mut clips);
    rows
}

/// [`unsplittable_rows_where`], and beside it the rows the fills it freed would have forbidden by
/// ADR 0139's rule, from one walk of the list.
///
/// A freed row is a permission, and a division the cost estimate prefers through the freed rows
/// can still be one a caller would rather not take — it replays the fills it cuts, or it makes
/// fewer strips than the rows between the fills afford. The union of the two vectors is that
/// second division's rows, and walking the list once for both keeps the planner's serial work
/// where it was on the page that most needs the render divided (ADR 1837).
#[must_use]
pub fn unsplittable_rows_and_freed_fills(
    list: &DisplayList,
    target: TargetSpec,
    row_local: &dyn Fn(PlannedMark<'_>) -> bool,
) -> (Vec<bool>, Vec<bool>) {
    let mut rows = vec![false; target.height as usize];
    let mut clips = HashSet::new();
    let walk = Walk {
        list,
        to_device: target.transform,
        row_local,
        freed: Some(RefCell::new(vec![false; target.height as usize])),
    };
    walk.segments(list.commands(), false, &mut rows, &mut clips);
    let freed = walk.freed.map(RefCell::into_inner).unwrap_or_default();
    (rows, freed)
}

/// What a walk of [`unsplittable_rows_where`] carries unchanged into every level of it.
struct Walk<'a> {
    list: &'a DisplayList,
    to_device: Transform,
    row_local: &'a dyn Fn(PlannedMark<'_>) -> bool,
    /// Where the rows of the fills `row_local` frees are marked, for
    /// [`unsplittable_rows_and_freed_fills`]; `None` where nobody asked.
    freed: Option<RefCell<Vec<bool>>>,
}

impl Walk<'_> {
    /// Walks a command list, marking the rows every segment it can reach would be re-stated in.
    ///
    /// `replaces` is [`PlannedStroke::replaces`] for the commands of this list.
    fn segments(
        &self,
        commands: &[Command],
        replaces: bool,
        rows: &mut [bool],
        seen: &mut HashSet<ClipId>,
    ) {
        for command in commands {
            self.element(command, replaces, OwnBound::Unknown, rows, seen);
        }
    }

    /// [`Self::segments`] for one command: its clip chain's segments, its soft mask's, and its
    /// own.
    ///
    /// `own` is the command's own [`Command::device_bounds`] where a caller already holds it, as
    /// [`gather_one`]'s is.
    fn element(
        &self,
        command: &Command,
        replaces: bool,
        own: OwnBound,
        rows: &mut [bool],
        seen: &mut HashSet<ClipId>,
    ) {
        let (list, to_device) = (self.list, self.to_device);
        if let Some(id) = command.clip() {
            clip_segments(list, id, to_device, rows, seen);
        }
        if let Some(mask) = command.mask().and_then(|id| list.soft_mask(id)) {
            self.segments(&mask.commands, true, rows, seen);
        }
        self.command(command, replaces, own, rows, seen);
    }

    /// Marks the rows one command's own segments would be re-stated in.
    fn command(
        &self,
        command: &Command,
        replaces: bool,
        own: OwnBound,
        rows: &mut [bool],
        seen: &mut HashSet<ClipId>,
    ) {
        let to_device = self.to_device;
        match command {
            // A fill's curves and oblique edges, unless the backend draws it a row at a time:
            // ADR 0139's rule is a statement about a converter that clips a path at the strip's
            // edge, and the backend says which of its fills never meet one (ADR 1837). A fill
            // inside rows already forbidden is not asked, because no answer could change them.
            Command::Fill {
                path, transform, ..
            } => {
                let at = transform.then(to_device);
                if settled(rows, path, at) {
                    return;
                }
                let planned = PlannedFill {
                    path,
                    at,
                    extent: own.of(command, to_device),
                    replaces,
                };
                if !(self.row_local)(PlannedMark::Fill(planned)) {
                    path.oblique_spans(at, |top, bottom| mark(rows, top, bottom));
                } else if let Some(freed) = &self.freed {
                    let mut freed = freed.borrow_mut();
                    if !settled(&freed, path, at) {
                        path.oblique_spans(at, |top, bottom| mark(&mut freed, top, bottom));
                    }
                }
            }
            // A stroke's whole extent unless the backend draws it a row at a time. Two reasons
            // the extent is the default, and the second is why the first is not enough: a
            // stroker turns a path into an outline, so a round cap or join puts a curve on a
            // straight path; and a stroke thin enough to be a hairline is not turned into an
            // outline at all but scan-converted by `tiny-skia`'s own hairline path, which clips
            // the *line* against the target and so re-states its endpoints. Which of its strokes
            // escape both is the backend's to say, because it is a fact about its drawing
            // (ADR 1819).
            Command::Stroke {
                path,
                transform,
                stroke,
                paint,
                blend,
                ..
            } => {
                let extent = own.of(command, to_device);
                let planned = PlannedStroke {
                    path,
                    stroke,
                    paint,
                    blend: *blend,
                    at: transform.then(to_device),
                    extent,
                    replaces,
                };
                if (self.row_local)(PlannedMark::Stroke(planned)) {
                    return;
                }
                if let Some(extent) = extent {
                    mark(rows, extent.min.y, extent.max.y);
                } else {
                    rows.fill(true);
                }
            }
            // An image is drawn into the parallelogram its transform names, whose edges are
            // straight, and its samples are read at absolute device positions.
            Command::Image { .. } => {}
            Command::Group {
                commands, knockout, ..
            } => self.segments(commands, *knockout, rows, seen),
            // Both halves mark, and both are drawn: the shape empties the rows the object
            // then paints, so a strip boundary has to be legal for each.
            Command::Shaped { object, shape } => {
                self.segments(std::slice::from_ref(object), true, rows, seen);
                self.segments(std::slice::from_ref(shape), true, rows, seen);
            }
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
        let at = clip.transform.then(to_device);
        if !settled(rows, &clip.path, at) {
            clip.path
                .oblique_spans(at, |top, bottom| mark(rows, top, bottom));
        }
        current = clip.parent;
    }
}

/// Whether walking `path` under `at` could still change `rows`.
///
/// [`mark`] only ever *sets* a row, so a path every one of whose reachable rows is already
/// marked cannot change the answer, and the walk that would re-mark them is arithmetic with no
/// outcome. The rows it can reach are [`mark`]'s own range taken over the path's whole device
/// extent, which contains every one of its segments' — [`crate::Path::bounds`] is the control
/// hull, and [`crate::Path::oblique_spans`] reports a y range of the same control points — and
/// `clamp_row`, `floor` and `ceil` are all monotone, so a segment's range lies inside the
/// path's. **This skips arithmetic and never a mark**: the vector it produces is the one the
/// unconditional walk produces, row for row.
///
/// It is here because the walk is serial work in front of a parallel render, on the page that
/// most needs the render divided: a dense text page is thousands of glyph fills over a few
/// hundred rows, and once a line of text has marked its own rows every later glyph on that line
/// walks forty control points to mark them again. On page 101 of ISO 32000-2 it takes
/// `unsplittable_rows` from 8.28% of the page's rasterisation to under 2%, byte-identically
/// (ADR 0687).
///
/// A path that names no point reaches no row, so it settles nothing and falls through to a walk
/// that marks nothing either — the cheap honest answer rather than a second `None` branch.
fn settled(rows: &[bool], path: &Path, at: Transform) -> bool {
    let Some(extent) = path.bounds(at) else {
        return false;
    };
    let count = rows.len();
    let from = clamp_row(extent.min.y.floor() + 1.0, count);
    let to = clamp_row(extent.max.y.ceil(), count);
    rows.get(from..to)
        .is_none_or(|span| span.iter().all(|row| *row))
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

/// The fewest commands a piece of [`strip_inputs`] is given: below this a piece costs the pool
/// more to hand out than it saves, and a page of a few thousand commands is read as one piece.
const LEAST_PIECE: usize = 4_096;

/// How many pieces [`strip_inputs`] cuts a list into per thread of the pool, and how many bands
/// [`StripInputs::costs`] cuts the rows into: more than one, so that a piece of long curves beside
/// a piece of short lines, or a band of dense text beside a margin, does not leave a thread
/// waiting.
const SHARES_PER_THREAD: usize = 4;

/// The memory, in bytes, every piece's two row vectors may take together.
///
/// Each piece marks its own rows, a byte a row twice over, so the pieces of a target of very
/// many rows are fewer: the tallest target `rasterize` admits, 2^24 rows, is read in two.
const PIECE_ROWS_BUDGET: usize = 64 << 20;

/// What a strip planner reads of a list, measured on the pool and the same in every number as
/// the serial functions of this module measure it.
///
/// [`command_extents`], [`row_costs`], [`unsplittable_rows_and_freed_fills`] and
/// [`replay_ratio`] each walk every command once on the calling thread, and a planner asks all
/// four before a page is divided: on the owner's witness, 4 408 405 commands at six times the fit,
/// that was 231 to 351 ms of serial work in front of a 553 to 660 ms draw on twenty-four threads
/// (ADR 1882). The
/// list is cut into runs of commands, each run read on its own thread into its own extents and its
/// own rows, and the answers are put together so that **every one is the serial function's, bit for
/// bit**, which is what keeps a page's cuts — and so its pixels — where they were:
///
/// - **the extents** are the runs' in order, which is the list's order;
/// - **a row's cost** is summed over the extents in the list's order, each band of rows on a thread
///   of its own, because a sum of floating-point widths taken in another order can round to
///   another number and move a cut by a row;
/// - **the forbidden rows** are the union of the runs'. A row is only ever *set*, so the union is
///   the serial answer whatever the order; a run that cannot see the rows an earlier run forbade
///   walks some paths [`settled`] would have skipped, which is arithmetic and never a mark;
/// - **the rows between fills** — the forbidden rows and those the fills a backend draws a row at a
///   time would have forbidden — are a union too. The freed rows alone are not: a fill the serial
///   walk skips inside rows already forbidden marks its rows free in a run that has not seen them
///   forbidden, so only the union, which is all a planner reads, is stated (ADR 1837);
/// - **the replay** is two whole-number counts, summed over the runs.
///
/// A top-level command's own device bound is read from the list's [`MarkBounds`] where those were
/// measured under this very transform, because a backend measures them anyway before it draws
/// (ADR 1841) and the planner measuring them again was most of its work.
///
/// **And the extents are not copied out of them.** A top-level command's extent is its own bound
/// met with its clip chain's extent, which is the bound itself, every bit of it, for a command with
/// no clip and for one whose clip holds its bound; only a clip that narrows the bound, a degenerate
/// bound under a clip, or a group, whose extents are its elements', makes another. So a run read
/// from bounds measured where the target is keeps only those exceptions, beside the bounds it
/// borrows: the owner's witness has none among its 4 408 405 commands, and the 84.1 MiB copy of its
/// bounds the planner held beside the list's own is not made (ADR 1940).
#[derive(Debug)]
pub struct StripInputs<'a> {
    /// The runs of commands, in the list's order.
    pieces: Vec<Piece<'a>>,
    /// The target's rows.
    rows: usize,
    /// How many rows a band of [`Self::costs`] is, the last band perhaps fewer.
    band: usize,
    /// The target's width, the most a row's cost grows by for one extent.
    width: f64,
}

/// One run of a list's top-level commands, read by [`strip_inputs`].
#[derive(Debug)]
struct Piece<'a> {
    /// [`command_extents`] of these commands, in their order.
    extents: Extents<'a>,
    /// The rows these commands forbid, as [`unsplittable_rows_where`] marks them.
    forbidden: Vec<bool>,
    /// The rows the fills these commands free would have forbidden.
    freed: Vec<bool>,
    /// For each band of [`StripInputs::costs`], the keys ([`Extents::each`]) of the extents that
    /// add to a row of it, in order and each once: what lets a band sum only what reaches it.
    ///
    /// Thirty-two bits a key, because there are as many as there are extents and bands they reach:
    /// at eight bytes a key they were 59.7 MiB of the owner's witness's heap at its peak, and are
    /// 28.6 (ADR 1940). Empty where a key is past them, and every band then reads every extent of
    /// the piece.
    reaching: Vec<Vec<u32>>,
}

/// One run's [`command_extents`], in the list's order, held as the bounds it was read from allow.
#[derive(Debug)]
enum Extents<'a> {
    /// Every extent, one after another — a command one, a group its elements' — each keyed by
    /// its position.
    Stated(Vec<Option<Rect>>),
    /// Every command's extent is its own bound, borrowed from the list's [`MarkBounds`], but for
    /// the commands `exceptions` names; each extent is keyed by its command.
    Borrowed {
        /// The run's own bounds, one a command, measured under the target's own transform.
        own: &'a [Option<Rect>],
        /// The commands whose extents are not their own bound, in order, each with the index into
        /// `stated` its extents begin at; they end where the next one's begin.
        exceptions: Vec<(usize, usize)>,
        /// The excepted commands' extents, in order.
        stated: Vec<Option<Rect>>,
    },
}

/// How many of a run's commands may be exceptions to its borrowed bounds before its extents are
/// held in full, as a share of the run: one in four.
///
/// An exception costs its two indices, 16 bytes, beside its extents' 20 bytes each, so it is past
/// about one in two that the borrowed form holds more than the copy would; at one in four it still
/// holds under half of it, and the search [`Extents::of`] makes of the exceptions stays short. A
/// choice of memory against the search, made here, which no number the planner reads depends on:
/// the two forms state the same extents.
const MOST_EXCEPTIONS: usize = 4;

impl<'a> Extents<'a> {
    /// Extents to be read from `own` where the run has them, and stated where it has not.
    fn new(own: Option<&'a [Option<Rect>]>, commands: usize) -> Self {
        match own {
            Some(own) => Self::Borrowed {
                own,
                exceptions: Vec::new(),
                stated: Vec::new(),
            },
            None => Self::Stated(Vec::with_capacity(commands)),
        }
    }

    /// Adds `command`'s extents, as [`gather_one`] states them.
    fn push(&mut self, command: usize, extents: &[Option<Rect>]) {
        match self {
            Self::Stated(all) => all.extend_from_slice(extents),
            Self::Borrowed {
                own,
                exceptions,
                stated,
            } => {
                if let ([extent], Some(bound)) = (extents, own.get(command))
                    && same_bits(*extent, *bound)
                {
                    return;
                }
                exceptions.push((command, stated.len()));
                stated.extend_from_slice(extents);
            }
        }
    }

    /// These extents, stated in full where the exceptions are too many to keep beside the bounds
    /// ([`MOST_EXCEPTIONS`]).
    fn settled(self) -> Self {
        if let Self::Borrowed {
            own, exceptions, ..
        } = &self
            && exceptions.len() > own.len() / MOST_EXCEPTIONS
        {
            let mut all = Vec::with_capacity(own.len());
            self.each(|_, extent| all.push(extent));
            return Self::Stated(all);
        }
        self
    }

    /// Calls `each` with every extent, in the list's order, and the key that names it to
    /// [`Self::of`].
    fn each(&self, mut each: impl FnMut(usize, Option<Rect>)) {
        match self {
            Self::Stated(all) => {
                for (position, extent) in all.iter().enumerate() {
                    each(position, *extent);
                }
            }
            Self::Borrowed {
                own,
                exceptions,
                stated,
            } => {
                let mut ahead = exceptions.iter().enumerate().peekable();
                for (command, bound) in own.iter().enumerate() {
                    match ahead.next_if(|(_, (excepted, _))| *excepted == command) {
                        Some((at, _)) => {
                            for extent in excepted(exceptions, stated, at) {
                                each(command, *extent);
                            }
                        }
                        None => each(command, *bound),
                    }
                }
            }
        }
    }

    /// Calls `each` with the extents `key` names, in order.
    fn of(&self, key: usize, mut each: impl FnMut(Option<Rect>)) {
        match self {
            Self::Stated(all) => {
                if let Some(extent) = all.get(key) {
                    each(*extent);
                }
            }
            Self::Borrowed {
                own,
                exceptions,
                stated,
            } => match exceptions.binary_search_by_key(&key, |(command, _)| *command) {
                Ok(at) => {
                    for extent in excepted(exceptions, stated, at) {
                        each(*extent);
                    }
                }
                Err(_) => {
                    if let Some(bound) = own.get(key) {
                        each(*bound);
                    }
                }
            },
        }
    }
}

/// The extents of exception `at`, from where its own begin to where the next one's do.
fn excepted<'s>(
    exceptions: &[(usize, usize)],
    stated: &'s [Option<Rect>],
    at: usize,
) -> &'s [Option<Rect>] {
    let begins = |at: usize| exceptions.get(at).map_or(stated.len(), |(_, start)| *start);
    stated
        .get(begins(at)..begins(at.saturating_add(1)))
        .unwrap_or_default()
}

/// Whether two extents are the same, every bit of every coordinate: an exception is any extent that
/// is not its bound exactly, `-0.0` against `0.0` and a NaN against itself included.
fn same_bits(left: Option<Rect>, right: Option<Rect>) -> bool {
    let bits = |rect: Rect| [rect.min.x, rect.min.y, rect.max.x, rect.max.y].map(f32::to_bits);
    left.map(bits) == right.map(bits)
}

/// Reads `list` for a strip planner on the pool: [`StripInputs`].
///
/// `bounds` is the list's [`MarkBounds`] at the target's magnification; a command's bound is read
/// from it only where it was measured under `target.transform` itself, every one of the six numbers
/// the same, and measured again here otherwise. `row_local` is [`unsplittable_rows_where`]'s
/// question to the backend, asked from every thread of the pool.
#[must_use]
pub fn strip_inputs<'a>(
    list: &DisplayList,
    target: TargetSpec,
    bounds: &'a MarkBounds,
    row_local: &(dyn Fn(PlannedMark<'_>) -> bool + Sync),
) -> StripInputs<'a> {
    let commands = list.commands();
    let rows = target.height as usize;
    let shares = rayon::current_num_threads().saturating_mul(SHARES_PER_THREAD);
    let band = rows.div_ceil(shares.max(1)).max(1);
    let measured = bounds
        .exactly_at(target.transform)
        .filter(|measured| measured.len() == commands.len());
    let pieces = commands
        .len()
        .div_ceil(LEAST_PIECE)
        .min(shares)
        .min(
            PIECE_ROWS_BUDGET
                .checked_div(rows.saturating_mul(2))
                .unwrap_or(usize::MAX),
        )
        .max(1);
    let size = commands.len().div_ceil(pieces).max(1);
    let pieces = commands
        .par_chunks(size)
        .enumerate()
        .map(|(index, run)| {
            let first = index.saturating_mul(size);
            let own = measured.and_then(|all| all.get(first..first.saturating_add(run.len())));
            Piece::read(list, target, run, own, row_local, band)
        })
        .collect();
    StripInputs {
        pieces,
        rows,
        band,
        width: f64::from(target.width),
    }
}

impl<'a> Piece<'a> {
    /// Reads one run of top-level commands, `measured` holding their own bounds where known.
    fn read(
        list: &DisplayList,
        target: TargetSpec,
        run: &[Command],
        measured: Option<&'a [Option<Rect>]>,
        row_local: &dyn Fn(PlannedMark<'_>) -> bool,
        band: usize,
    ) -> Self {
        let rows = target.height as usize;
        let to_device = target.transform;
        let mut extents = Extents::new(measured, run.len());
        let mut gathered = Vec::new();
        let mut memo = HashMap::new();
        let mut forbidden = vec![false; rows];
        let mut seen = HashSet::new();
        let walk = Walk {
            list,
            to_device,
            row_local,
            freed: Some(RefCell::new(vec![false; rows])),
        };
        for (index, command) in run.iter().enumerate() {
            let own = match measured {
                Some(all) => all
                    .get(index)
                    .map_or(OwnBound::Unknown, |bound| OwnBound::Known(*bound)),
                // Measured once for the two readers below rather than by each of them; a group's
                // own bound is read by neither, which reads its elements'.
                None if !matches!(command, Command::Group { .. }) => {
                    OwnBound::Known(command.device_bounds(to_device))
                }
                None => OwnBound::Unknown,
            };
            gathered.clear();
            gather_one(
                list,
                command,
                to_device,
                None,
                own,
                &mut gathered,
                &mut memo,
            );
            extents.push(index, &gathered);
            walk.element(command, false, own, &mut forbidden, &mut seen);
        }
        let extents = extents.settled();
        let freed = walk.freed.map(RefCell::into_inner).unwrap_or_default();
        let width = f64::from(target.width);
        let mut reaching = vec![Vec::new(); rows.checked_div(band).unwrap_or(0).saturating_add(1)];
        let mut past_the_keys = false;
        extents.each(|key, extent| {
            let Ok(key) = u32::try_from(key) else {
                past_the_keys = true;
                return;
            };
            let (from, to, _) = rows_and_span(extent, rows, width);
            let to = to.min(rows);
            if from >= to {
                return;
            }
            let top_band = from.checked_div(band).unwrap_or(0);
            let bottom_band = to.saturating_sub(1).checked_div(band).unwrap_or(0);
            for list in reaching
                .get_mut(top_band..=bottom_band)
                .into_iter()
                .flatten()
            {
                // A group's elements share their command's key, and a band names it once.
                if list.last() != Some(&key) {
                    list.push(key);
                }
            }
        });
        if past_the_keys {
            reaching = Vec::new();
        }
        Self {
            extents,
            forbidden,
            freed,
            reaching,
        }
    }
}

impl<'a> StripInputs<'a> {
    /// [`command_extents`]' extents, in the list's order.
    #[cfg(test)]
    fn extents(&self) -> Vec<Option<Rect>> {
        let mut all = Vec::new();
        for piece in &self.pieces {
            piece.extents.each(|_, extent| all.push(extent));
        }
        all
    }

    /// [`row_costs`] of the list's extents, row for row the same `f64`.
    ///
    /// Each band of rows on a thread of its own, every band reading the extents that reach it in
    /// the list's order: the order a row's widths are summed in is the serial one, so each sum
    /// rounds as it did there. Every band reading every extent was itself 43 to 87 ms of the
    /// witness's planning on four to twenty-four threads, and more the more threads there were, so
    /// each piece sorts its extents into the bands they reach while it reads them (ADR 1882).
    #[must_use]
    pub fn costs(&self) -> Vec<f64> {
        let mut costs = vec![0.0_f64; self.rows];
        let band = self.band;
        costs
            .par_chunks_mut(band)
            .enumerate()
            .for_each(|(index, slice)| {
                let first = index.saturating_mul(band);
                let end = first.saturating_add(slice.len());
                let mut add = |extent: Option<Rect>| {
                    let (from, to, span) = rows_and_span(extent, self.rows, self.width);
                    let (from, to) = (from.max(first), to.min(self.rows).min(end));
                    if from >= to {
                        return;
                    }
                    let local = from.saturating_sub(first)..to.saturating_sub(first);
                    for cost in slice.get_mut(local).into_iter().flatten() {
                        *cost += span;
                    }
                };
                for piece in &self.pieces {
                    if piece.reaching.is_empty() {
                        piece.extents.each(|_, extent| add(extent));
                    }
                    for key in piece.reaching.get(index).into_iter().flatten() {
                        piece.extents.of(*key as usize, &mut add);
                    }
                }
            });
        costs
    }

    /// [`unsplittable_rows_where`]'s rows: the union of every piece's.
    #[must_use]
    pub fn unsplittable(&self) -> Vec<bool> {
        self.union(|_| None)
    }

    /// The forbidden rows and the rows the freed fills would have forbidden, together: the union
    /// of [`unsplittable_rows_and_freed_fills`]' two vectors, which is the same in any order where
    /// the second vector alone is not (the type's comment).
    #[must_use]
    pub fn between_fills(&self) -> Vec<bool> {
        self.union(|piece| Some(&piece.freed))
    }

    /// Every piece's forbidden rows, and the rows `also` names of it, joined.
    fn union(&self, also: impl for<'p> Fn(&'p Piece<'a>) -> Option<&'p Vec<bool>>) -> Vec<bool> {
        let mut rows = vec![false; self.rows];
        for piece in &self.pieces {
            for marked in std::iter::once(&piece.forbidden).chain(also(piece)) {
                for (row, set) in rows.iter_mut().zip(marked) {
                    *row |= *set;
                }
            }
        }
        rows
    }

    /// [`replay_ratio`] of the list's extents under `boundaries`, counted on the pool.
    #[must_use]
    pub fn replay_ratio(&self, boundaries: &[u32]) -> f64 {
        let (touches, commands) = self
            .pieces
            .par_iter()
            .map(|piece| {
                let mut counted = Replays::under(boundaries);
                piece.extents.each(|_, extent| counted.add(extent));
                (counted.touches, counted.commands)
            })
            .reduce(
                || (0, 0),
                |(touches, commands), (more, others)| {
                    (
                        touches.saturating_add(more),
                        commands.saturating_add(others),
                    )
                },
            );
        ratio_of(touches, commands)
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
    let (touches, commands) = replays(extents, boundaries);
    ratio_of(touches, commands)
}

/// How many strips of `boundaries` the bounded extents reach, summed, and how many bounded
/// extents there are: [`replay_ratio`]'s two counts, which [`StripInputs::replay_ratio`] adds up
/// over its pieces. Whole numbers, so the sum is the same in any order.
fn replays(extents: &[Option<Rect>], boundaries: &[u32]) -> (u64, u64) {
    let mut counted = Replays::under(boundaries);
    for extent in extents {
        counted.add(*extent);
    }
    (counted.touches, counted.commands)
}

/// [`replays`]' two counts, taken an extent at a time.
struct Replays<'b> {
    /// The division the extents are counted against.
    boundaries: &'b [u32],
    /// How many strips the bounded extents so far reach, summed.
    touches: u64,
    /// How many bounded extents there have been.
    commands: u64,
}

impl<'b> Replays<'b> {
    /// Nothing counted yet, against `boundaries`.
    fn under(boundaries: &'b [u32]) -> Self {
        Self {
            boundaries,
            touches: 0,
            commands: 0,
        }
    }

    /// Counts one extent; one that cannot be bounded is no command a strip replays.
    fn add(&mut self, extent: Option<Rect>) {
        let Some(extent) = extent else { return };
        let boundaries = self.boundaries;
        self.commands = self.commands.saturating_add(1);
        for strip in 0..boundaries.len().saturating_sub(1) {
            let top = boundaries.get(strip).copied().unwrap_or(0);
            let bottom = boundaries
                .get(strip.saturating_add(1))
                .copied()
                .unwrap_or(0);
            if extent.min.y < row_edge(bottom) && extent.max.y > row_edge(top) {
                self.touches = self.touches.saturating_add(1);
            }
        }
    }
}

/// [`replay_ratio`]'s answer from its two counts: one where nothing is drawn.
fn ratio_of(touches: u64, commands: u64) -> f64 {
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
    use std::sync::Arc;

    use std::cell::Cell;

    use super::{
        Extents, Piece, PlannedMark, command_extents, replay_ratio, row_costs, strip_boundaries,
        strip_boundaries_avoiding, strip_inputs, unsplittable_rows,
        unsplittable_rows_and_freed_fills, unsplittable_rows_where,
    };
    use crate::{
        BlendMode, Clip, Color, Command, DisplayList, FillRule, MarkBounds, Paint, Path,
        PathCommand, Point, Rect, Size, Stroke, TargetSpec, Transform,
    };

    fn target(width: u32, height: u32) -> TargetSpec {
        TargetSpec {
            width,
            height,
            transform: Transform::IDENTITY,
        }
    }

    /// A right triangle with its hypotenuse running from `(right, top)` down to `(0, bottom)`.
    ///
    /// Its two other sides are axis-aligned, so under an axis-preserving transform the
    /// hypotenuse is the only segment [`crate::Path::oblique_spans`] reports — which makes the
    /// rows it marks exactly `floor(top) + 1 .. ceil(bottom)` and nothing else.
    fn wedge(top: f32, bottom: f32, right: f32) -> Command {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, top)));
        path.push(PathCommand::LineTo(Point::new(right, top)));
        path.push(PathCommand::LineTo(Point::new(0.0, bottom)));
        path.push(PathCommand::Close);
        Command::Fill {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }
    }

    /// The early-out's discriminating case: a shape that *starts* inside rows already forbidden
    /// and reaches past them is still walked.
    ///
    /// [`super::settled`] skips a path only where every row its whole device extent can reach is
    /// already marked, and the reason to assert that rather than to trust it is that no raster
    /// can: the strips are exact by construction, so a planner that forbade too few cuts would
    /// still draw the identical picture and only ADR 0138's edge coverage would move. Written
    /// against a planted `any`-for-`all`, which leaves rows 20 to 40 legal here (ADR 0687).
    #[test]
    fn a_shape_reaching_past_the_rows_already_marked_is_still_walked() {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        list.push(wedge(10.0, 20.0, 50.0));
        list.push(wedge(15.0, 40.0, 50.0));

        let rows = unsplittable_rows(&list, target(100, 100));

        for (row, forbidden) in rows.iter().enumerate() {
            let expected = (11..40).contains(&row);
            assert_eq!(
                *forbidden, expected,
                "row {row} is {forbidden} where the two wedges' segments make it {expected}"
            );
        }
    }

    /// And the property that licenses the skip: a shape wholly inside rows already forbidden
    /// changes nothing, so leaving it unwalked is exact rather than merely cheap.
    #[test]
    fn a_shape_inside_the_rows_already_marked_changes_nothing() {
        let mut wide = DisplayList::new(Size::new(100.0, 100.0));
        wide.push(wedge(10.0, 40.0, 50.0));
        let alone = unsplittable_rows(&wide, target(100, 100));

        wide.push(wedge(20.0, 30.0, 50.0));
        assert_eq!(unsplittable_rows(&wide, target(100, 100)), alone);
    }

    /// A frame round a 100 × 100 page, `re S`: the stroke whose whole extent is every row.
    fn frame() -> Command {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(5.0, 5.0)));
        path.push(PathCommand::LineTo(Point::new(95.0, 5.0)));
        path.push(PathCommand::LineTo(Point::new(95.0, 95.0)));
        path.push(PathCommand::LineTo(Point::new(5.0, 95.0)));
        path.push(PathCommand::Close);
        Command::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: Stroke {
                width: 2.0,
                ..Stroke::default()
            },
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }
    }

    /// The default answer is ADR 0139's: a stroke forbids its whole extent, so a frame round the
    /// page forbids every row inside it, and a backend that draws it a row at a time frees them all.
    #[test]
    fn a_stroke_forbids_its_extent_unless_its_backend_draws_it_by_rows() {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        list.push(frame());
        let target = target(100, 100);

        let forbidden = unsplittable_rows(&list, target);
        assert!(forbidden.iter().skip(6).take(88).all(|row| *row));
        assert_eq!(
            unsplittable_rows_where(&list, target, &|_| false),
            forbidden
        );
        assert!(
            unsplittable_rows_where(&list, target, &|_| true)
                .iter()
                .all(|row| !*row)
        );
    }

    /// A fill beside it keeps its own rows whatever the backend says of the stroke: each mark is
    /// answered for itself.
    #[test]
    fn a_fill_keeps_its_rows_whatever_a_stroke_is_answered() {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        list.push(frame());
        list.push(wedge(10.0, 40.0, 50.0));

        let rows = unsplittable_rows_where(&list, target(100, 100), &|mark| {
            matches!(mark, PlannedMark::Stroke(_))
        });
        for (row, forbidden) in rows.iter().enumerate() {
            assert_eq!(*forbidden, (11..40).contains(&row), "row {row}");
        }
    }

    /// And a fill the backend draws a row at a time forbids nothing, while one it does not keeps
    /// ADR 0139's rows exactly: the answer frees a fill and never moves the rule (ADR 1837).
    #[test]
    fn a_fill_forbids_its_oblique_rows_unless_its_backend_draws_it_by_rows() {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        list.push(wedge(10.0, 40.0, 50.0));
        let target = target(100, 100);

        assert_eq!(
            unsplittable_rows_where(&list, target, &|_| false),
            unsplittable_rows(&list, target)
        );
        assert!(
            unsplittable_rows_where(&list, target, &|mark| {
                matches!(mark, PlannedMark::Fill(_))
            })
            .iter()
            .all(|row| !*row)
        );
    }

    /// The rows a freed fill would have forbidden are kept beside the rows, so that the two together
    /// are ADR 0139's rows for fills exactly (ADR 1837).
    #[test]
    fn the_freed_fills_rows_and_the_rows_are_the_rule_for_fills_together() {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        list.push(frame());
        list.push(wedge(10.0, 40.0, 50.0));
        list.push(wedge(60.0, 70.0, 50.0));
        let target = target(100, 100);
        let strokes_only = |mark: PlannedMark<'_>| matches!(mark, PlannedMark::Stroke(_));
        let (rows, freed) = unsplittable_rows_and_freed_fills(&list, target, &|_| true);
        assert!(rows.iter().all(|row| !*row));
        let union: Vec<bool> = rows.iter().zip(&freed).map(|(a, b)| *a || *b).collect();
        assert_eq!(union, unsplittable_rows_where(&list, target, &strokes_only));
        assert_eq!(
            unsplittable_rows_and_freed_fills(&list, target, &|_| false),
            (unsplittable_rows(&list, target), vec![false; 100])
        );
    }

    /// A fill whose rows are already forbidden is not asked, and the one asked is stated in the
    /// target's pixels with the extent its command bounds.
    #[test]
    fn a_fill_inside_forbidden_rows_is_not_put_to_the_backend() {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        list.push(wedge(10.0, 40.0, 50.0));
        list.push(wedge(20.0, 30.0, 50.0));
        let asked = Cell::new(0);
        let extent = Cell::new(None);
        let _ = unsplittable_rows_where(&list, target(100, 100), &|mark| {
            if let PlannedMark::Fill(fill) = mark {
                asked.set(asked.get() + 1);
                extent.set(fill.extent);
            }
            false
        });
        assert_eq!(asked.get(), 1);
        assert_eq!(extent.get(), Some(rect(10.0, 40.0, 50.0)));
    }

    /// An element of a knockout group is asked about as one that replaces what is below it,
    /// and an element of an ordinary group as one that does not (§11.4.6).
    #[test]
    fn a_knockout_element_is_asked_about_as_one_that_replaces() {
        for knockout in [false, true] {
            let mut list = DisplayList::new(Size::new(100.0, 100.0));
            list.push(Command::Group {
                commands: vec![frame()],
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
                isolated: true,
                knockout,
                alpha_is_shape: false,
                blending: None,
            });
            let asked = Cell::new(None);
            let _ = unsplittable_rows_where(&list, target(100, 100), &|mark| {
                if let PlannedMark::Stroke(stroke) = mark {
                    asked.set(Some(stroke.replaces));
                }
                true
            });
            assert_eq!(asked.get(), Some(knockout));
        }
    }

    fn rect(top: f32, bottom: f32, width: f32) -> Rect {
        Rect::from_corners(Point::new(0.0, top), Point::new(width, bottom))
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

    /// A page of `count` marks with every construction the planner reads: oblique fills, straight
    /// and curved strokes, a clip chain, a group and a page-wide fill. Its widths run from a
    /// millionth of a unit at the page's left edge, where an `f32` keeps every bit of one, to the
    /// page's, more than an `f64` holds at once, so a row's cost
    /// summed in another order than the list's rounds to another number; and its strokes and its
    /// inner clip keep to the upper half, so the rows a freed fill would forbid are not all
    /// forbidden already. Only every `exceptional`-th mark may be clipped or a group, so that a
    /// page of few exceptions to its own bounds can be asked for as well as one of many.
    fn busy_page(count: usize, exceptional: usize) -> DisplayList {
        let mut list = DisplayList::new(Size::new(613.0, 409.0));
        let outer = list
            .add_clip(Clip {
                path: box_path(3.3, 7.1, 601.7, 400.9),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("a clip");
        let inner = list
            .add_clip(Clip {
                path: wedge_path(20.3, 180.7, 590.1),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: Some(outer),
            })
            .expect("a clip");
        // A linear congruential sequence: the same page on every run, and widths of every kind.
        let mut state = 0x2545_f491_u32;
        let mut next = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f32::from(u16::try_from(state >> 16).unwrap_or(0)) / 65_536.0
        };
        for index in 0..count {
            let (x, y, size) = (next() * 600.0, next() * 400.0, 0.1 + next() * 37.3);
            let high = next() * 190.0;
            let plain = index.checked_rem(exceptional) != Some(0);
            let clip = match index % 5 {
                _ if plain => None,
                0 if index % 4 == 1 => Some(inner),
                1 => Some(outer),
                _ => None,
            };
            let command = match index % 4 {
                3 if plain => wedge(y, y + size, x),
                0 => {
                    let right = if index % 3 == 0 {
                        1e-6 + next() * 1e-6
                    } else {
                        x + size
                    };
                    let mut command = wedge(y, y + size, right);
                    if let Command::Fill { clip: own, .. } = &mut command {
                        *own = clip;
                    }
                    command
                }
                1 | 2 => {
                    let mut path = Path::new();
                    path.push(PathCommand::MoveTo(Point::new(x, high)));
                    if index % 4 == 1 {
                        path.push(PathCommand::LineTo(Point::new(x + size, high + size * 0.3)));
                    } else {
                        path.push(PathCommand::CurveTo(
                            Point::new(x + size, high),
                            Point::new(x, high + size * 0.5),
                            Point::new(x + size, high + size * 0.5),
                        ));
                    }
                    Command::Stroke {
                        path: Arc::new(path),
                        transform: Transform::IDENTITY,
                        stroke: Stroke {
                            width: 0.05 + next() * 3.0,
                            ..Stroke::default()
                        },
                        paint: Paint::Solid(Color::BLACK),
                        clip,
                        mask: None,
                        blend: BlendMode::Normal,
                    }
                }
                _ => Command::Group {
                    commands: vec![wedge(y, y + size, x), wedge(y + 1.5, y + size * 2.0, x)],
                    alpha: 1.0,
                    clip,
                    mask: None,
                    blend: BlendMode::Normal,
                    isolated: true,
                    knockout: index % 8 == 3,
                    alpha_is_shape: false,
                    blending: None,
                },
            };
            list.push(command);
        }
        list.push(Command::Fill {
            path: Arc::new(box_path(0.0, 0.0, 613.0, 409.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        list
    }

    /// A rectangle from `(left, top)` to `(right, bottom)`, as a path.
    fn box_path(left: f32, top: f32, right: f32, bottom: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(left, top)));
        path.push(PathCommand::LineTo(Point::new(right, top)));
        path.push(PathCommand::LineTo(Point::new(right, bottom)));
        path.push(PathCommand::LineTo(Point::new(left, bottom)));
        path.push(PathCommand::Close);
        path
    }

    /// [`wedge`]'s path alone.
    fn wedge_path(top: f32, bottom: f32, right: f32) -> Path {
        match wedge(top, bottom, right) {
            Command::Fill { path, .. } => (*path).clone(),
            _ => Path::new(),
        }
    }

    /// A backend's answer that frees some marks and not others, so both row vectors are asked.
    fn frees_some(mark: PlannedMark<'_>) -> bool {
        match mark {
            PlannedMark::Fill(fill) => fill.extent.is_some_and(|extent| extent.min.y > 400.0),
            PlannedMark::Stroke(stroke) => stroke.stroke.width < 1.5,
        }
    }

    /// Every number [`strip_inputs`] states is the serial functions' own, bit for bit, read with
    /// the bounds measured where the target is and measured at another translation, on pools of
    /// one, four and seven threads — the property the planner's cuts rest on (ADR 1882). The page
    /// is long enough to be read in several pieces, and its widths are not dyadic, so a cost summed
    /// in another order than the list's would round to another number. Asked of a page whose clips
    /// and groups are so many that its runs state their extents in full, and of one whose few are
    /// held beside the bounds the runs borrow (ADR 1940).
    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn the_inputs_read_on_the_pool_are_the_serial_inputs() {
        for (exceptional, borrows) in [(1, false), (31, true)] {
            the_inputs_are_the_serial_inputs(&busy_page(40_000, exceptional), borrows);
        }
    }

    /// [`the_inputs_read_on_the_pool_are_the_serial_inputs`] of one page, whose runs read from
    /// bounds measured where the target is `borrow` them or state their extents in full.
    fn the_inputs_are_the_serial_inputs(list: &DisplayList, borrow: bool) {
        let list = list.clone();
        let target = TargetSpec {
            width: 1_226,
            height: 818,
            transform: Transform::scale(2.0, 2.0).then(Transform::translate(0.0, -0.81)),
        };
        let extents = command_extents(&list, target);
        let costs = row_costs(&extents, target);
        let forbidden = unsplittable_rows_where(&list, target, &frees_some);
        let (alone, freed) = unsplittable_rows_and_freed_fills(&list, target, &frees_some);
        assert_eq!(alone, forbidden);
        let between: Vec<bool> = alone.iter().zip(&freed).map(|(a, b)| *a || *b).collect();
        let divisions = [
            vec![0, 818],
            vec![0, 200, 401, 818],
            vec![0, 64, 128, 700, 818],
        ];

        let elsewhere = target.transform.then(Transform::translate(17.0, -3.0));
        for at in [target.transform, elsewhere] {
            let bounds = MarkBounds::measure(&list, at);
            for threads in [1, 4, 7] {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .expect("a pool");
                let inputs = pool.install(|| strip_inputs(&list, target, &bounds, &frees_some));
                assert!(threads == 1 || inputs.pieces.len() > 1, "read in pieces");
                let borrowed = |piece: &Piece<'_>| {
                    matches!(&piece.extents, Extents::Borrowed { exceptions, .. }
                        if !exceptions.is_empty())
                };
                assert_eq!(
                    inputs.pieces.iter().all(borrowed),
                    borrow && at == target.transform,
                    "borrowed, {threads} threads"
                );
                assert_eq!(inputs.extents(), extents, "extents, {threads} threads");
                let summed = pool.install(|| inputs.costs());
                let bits =
                    |costs: &[f64]| costs.iter().map(|cost| cost.to_bits()).collect::<Vec<_>>();
                assert_eq!(bits(&summed), bits(&costs), "costs, {threads} threads");
                assert_eq!(
                    inputs.unsplittable(),
                    forbidden,
                    "forbidden, {threads} threads"
                );
                assert_eq!(
                    inputs.between_fills(),
                    between,
                    "between, {threads} threads"
                );
                for division in &divisions {
                    let ratio = pool.install(|| inputs.replay_ratio(division));
                    assert_eq!(
                        ratio.to_bits(),
                        replay_ratio(&extents, division).to_bits(),
                        "replay of {division:?}, {threads} threads"
                    );
                }
            }
        }
    }
}
