//! The coverage a run of marks can make off the walk's thread, and the seam that lets
//! it (the caller's `doc/QUORRA_ENCODE_THREADS.md`).
//!
//! # What is parallel, and why exactly that
//!
//! [`raster::fill_mask`] and [`raster::flatten`] are **pure functions** of one mark's
//! own geometry: no `&mut self`, no shared table, nothing read that another mark wrote.
//! Every other part of the walk is order-dependent and stays on the walk's thread —
//! the frame budget's running total, the scratch sheet's shelf cursors (whose encounter
//! order ADR 0034 made load-bearing and declined to sort), the atlas allocator, and the
//! instance stream. So the phase splits in three:
//!
//! 1. **the walk**, serial: resolve the clip, choose the lane, and record a [`Job`] —
//!    everything needed to rasterise one mark, and nothing about where it will land;
//! 2. **the fan-out**, parallel: [`rasterise`], once per job, into the job's own
//!    [`CoverageMask`];
//! 3. **the commit**, serial and in encounter order: charge the budget, offer the tile
//!    to the atlas or pack it on the sheet, and append the instance.
//!
//! Because step 3 runs in the order step 1 queued, **every order-dependent number is
//! the number a one-threaded frame produces**: the same charges in the same sequence,
//! the same shelves, the same instances, and therefore the same refusals.
//! `tests/encode_threads.rs` holds that to byte equality across thread counts rather
//! than arguing it.
//!
//! # The one place the queue can be observed, and the rule that closes it
//!
//! A queued job has not charged, not packed and not drawn. Anything that reads or
//! advances the frame's order therefore drains the queue first —
//! [`Encoder::drain_queue`] is called from `charge`, `pack_scratch`, `push_op`, the quad
//! appender and `plan_child`, which between them are every route to an order-dependent
//! effect. Draining empties the queue *before* committing, so the commit reaches those
//! same methods and finds nothing to drain: there is one set of call sites rather than a
//! shadow set that must be kept in step.
//!
//! **A mark whose host work needs no geometry queues rather than drains** (ADR 1409): a
//! rectangle instance or a compute-lane tile behind a queued mark is a job of its own,
//! written or seated at its commit. A drain costs the fan-out everything queued before it,
//! and on a page of text and rules, or a zoom step of strokes between filled shapes, no
//! drain reached the floor.
//!
//! The other observation is the atlas: a queued job has not inserted its key, so a
//! repeat of that key would read `entry: None` from an atlas the first has not reached
//! and both would rasterise and insert. Where the walk's answer for the repeat turns on
//! that entry alone, the repeat is queued behind the first and reads the entry at its
//! commit ([`Encoder::follow_queued`], ADR 1409); everywhere else
//! [`Encoder::prospect_for`] drains before the answer is given rather than before the job
//! is queued, because the lane is chosen on that answer and a drain after the choice comes
//! too late. Nothing else about [`AtlasStore::prospect`](crate::atlas::AtlasStore::prospect)
//! depends on what is queued — ADR 0029 kept "has the atlas room?" out of that answer
//! deliberately, so occupancy cannot change a lane.
//!
//! # Why a scope and not a pool
//!
//! ADR 0023 recorded the caller's answer to "should raster build a thread pool?" as
//! **no — take one rather than make one**, for three reasons that are still true: their
//! `rayon` would be oversubscribed, their confined worker's seccomp filter kills the
//! `/sys` read `glibc` sizes its arenas from, and a pool built at construction is on
//! their time-to-first-page. Their `doc/QUORRA_ENCODE_THREADS.md` section 4 asks for the shape
//! that satisfies all three: a frame that enters threads inside `Device::render` and has
//! left them before the call returns. [`std::thread::scope`] is exactly that, and needs
//! no dependency, no `unsafe`, and nothing alive between frames.
//!
//! **The host names the number**
//! ([`Options::encode_threads`](crate::startup::Options::encode_threads)), and one is
//! the default: a host that has not asked for threads runs the walk it ran before, and
//! a host whose process cannot spawn one is not made to.
//!
//! # Why this is one file, having been looked at as two
//!
//! It is past the length CLAUDE.md's file-scale rule calls a smell (`wc -l` counts it),
//! and it has already given up the seam it had: `commit` is step 3, in its own module.
//! The division proposed for what is left — the [`Job`] record in one module and the
//! fan-out (`partition`, `rasterise_all`, `fan_out`) in another — was looked for and is
//! not there:
//!
//! - **[`rasterise`] is the join, not a member of either half.** It is the pure function
//!   of a [`Job`] that the whole design rests on, and it would have to be filed with the
//!   record or with the threads while being the reason both exist.
//! - **The fan-out balances by [`Job::weight`] and is bounded by [`Job::held`]**, so a
//!   module holding `partition` without the record holds arithmetic over fields it
//!   cannot see. The two constants are the same: `IN_FLIGHT_BUDGET_SHARE` bounds what
//!   jobs may sit queued and `PARALLEL_FLOOR_SEGMENTS` decides whether any go off-thread
//!   at all, and neither means anything without the other side.
//! - **The tests do not divide.** They are statements about the partition, the floor, a
//!   job's weight and the queue's bound — none is a statement about a `Job` on its own,
//!   which is the tell that the record is not a subject.
//!
//! What is not code is mostly this comment, which is the design argument the caller's
//! `doc/QUORRA_ENCODE_THREADS.md` asked for and the reason a reader can check the
//! determinism claim at all.

use std::thread;

use raster_scene::{Color, OutlineId, Rect, Segment, Stroke};

use crate::atlas::{AtlasEntry, GlyphKey};
use crate::raster::{self, CoverageMask, DeviceTransform, Rule};

use super::DrawStyle;
use super::clips::ResolvedClip;
use super::fill::SolidFill;

mod commit;

/// The share of the caller's stated `max_frame_bytes` that may sit in the queue as
/// coverage the commit has not reached yet.
///
/// Derived from the host's own number rather than picked, for the reason `residue.rs`
/// gives about its own share: a caller who lowers `max_frame_bytes` is describing a
/// machine. A sixty-fourth of the 268 MiB default is 4 MB, which is about twenty thousand
/// of the caller's three-pixel marks or two full-page tiles — enough work to divide
/// twenty-four ways many times over, and small next to a budget the frame may spend in
/// full.
///
/// It is a **batching granularity and never a capacity**: a single job larger than this
/// is queued alone and drained straight after, so no frame is refused by it and every
/// tile is still charged exactly once, in encounter order, at the commit.
const IN_FLIGHT_BUDGET_SHARE: u64 = 64;

/// What a frame's queue may hold in coverage the commit has not reached, given the
/// frame's own budget.
pub(super) fn in_flight_limit(frame_budget_bytes: u64) -> u64 {
    frame_budget_bytes.saturating_div(IN_FLIGHT_BUDGET_SHARE)
}

/// The queued weight below which a frame rasterises on the walk's thread, in a filled
/// outline's segments ([`weight_of`]).
///
/// A weight rather than a job count, because six 40 000-segment fills are more work
/// than six thousand triangles and the fan-out should take the first. The measurement
/// that set it is in `raster/doc/notes-encode-threads.md`; the median corpus page (twelve
/// marks, ninety-six segments) is two orders of magnitude below it, which is the
/// property the caller's section 4 asks for by name.
const PARALLEL_FLOOR_SEGMENTS: u64 = 4_096;

/// One mark's rasterisation, lifted out of the walk.
///
/// Borrowed, never owned: the segments are the resource store's, which outlives the
/// frame. What a job may **not** hold is anything the frame's order decides — a sheet
/// position, a budget total, an instance index — because that is the whole of what
/// makes [`rasterise`] a pure function.
pub(super) struct Job<'a> {
    /// The outline as the caller uploaded it.
    segments: &'a [Segment],
    /// Into device space; into *tile* space for a glyph, whose tile is rasterised at the
    /// quantised phase and drawn at the integer origin.
    transform: DeviceTransform,
    /// A stroke expands the flattened outline before it is filled (§8.4.3); `None` is a
    /// fill.
    stroke: Option<Stroke>,
    rule: Rule,
    extent: Extent,
    place: Place<'a>,
    draw: Draw,
    /// What this job is expected to cost, in outline segments — the only size known
    /// before the geometry exists, and the one [`partition`] balances by.
    weight: u64,
    /// An **upper bound** on the host memory this job holds between the walk and the
    /// commit: the widest its coverage tile can be, plus the job record itself.
    ///
    /// A bound rather than the truth, because the truth is the flattened geometry and
    /// that is what the job exists to defer. It is a bound because a Bézier lies inside
    /// the convex hull of its control points, so the box `hull.rs` already computed for
    /// culling contains every flattened point — and both lanes then cut that box by the
    /// same clip and the same target the rasteriser will.
    ///
    /// [`Encoder::enqueue`] keeps the sum of these under
    /// [`Encoder::in_flight_limit`], which is what stops a queue from being a way to
    /// hold a thousand full-page tiles at once where the walk held one (principle 3).
    held: u64,
    /// The stored outline a fill's segments are, when the job is one: it answers once,
    /// for every placement, whether the outline winds two neighbouring values, and a fill
    /// that does keeps its integral without asking again (ADR 1389). `None` for a stroke,
    /// whose pieces are not the outline.
    outline: Option<&'a crate::resources::StoredOutline>,
}

/// The transform a job with no geometry carries, which nothing reads.
const IDENTITY: DeviceTransform = DeviceTransform {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    e: 0.0,
    f: 0.0,
};

/// Which rectangle a job's coverage is rasterised over: the two lanes' own arithmetic,
/// moved here so that one reading of it serves the serial and the parallel path alike.
enum Extent {
    /// The glyph lane: the shape's own device bounds, rounded out. The tile is the
    /// cached picture, so nothing about one placement may narrow it.
    Own,
    /// The path lane: shape ∩ clip ∩ target, rounded out.
    ///
    /// The clip and the target are folded together at enqueue time, which is the same
    /// bound to the bit: `max(max(x0, clip), 0)` and `max(x0, max(clip, 0))` agree for
    /// every pair of floats, NaN included, because `f32::max` is associative and returns
    /// its non-NaN operand.
    Visible {
        /// `[left, top, right, bottom]`, already held to the target.
        rect: [f32; 4],
    },
}

/// Where a job's tile goes once the commit reaches it.
enum Place<'a> {
    /// The atlas already holds this placement: nothing to rasterise, one quad to draw.
    Resident {
        key: GlyphKey,
        origin: [f32; 2],
        entry: AtlasEntry,
    },
    /// Offer the tile to the atlas under this key, and fall through to the sheet when
    /// the atlas will not take it.
    Atlas { key: GlyphKey, origin: [f32; 2] },
    /// A repeat of a key a queued [`Place::Atlas`] job will insert: nothing to rasterise,
    /// and the entry is read at the commit, where the one-threaded walk would have read
    /// it (ADR 1409).
    ///
    /// The fill rides along for the one case where the atlas still lacks the key at the
    /// commit — the first placement made no geometry or found the atlas full — and there
    /// the fill is walked again, at the point in the order where the walk asked.
    Follows {
        key: GlyphKey,
        origin: [f32; 2],
        fill: SolidFill<'a>,
        clip: ResolvedClip,
    },
    /// Pack the tile onto the frame's scratch sheet.
    Sheet,
    /// An instance of the analytic rectangle lane (ADR 0007): no geometry, only a place
    /// in the draw order (ADR 1409).
    Rect { rect: Rect },
    /// A tile of the compute lane (ADR 0080): the device flattens and fills it, so the
    /// host's part is a seat and a record, taken at the commit (ADR 1409).
    Compute {
        outline: OutlineId,
        to_device: DeviceTransform,
        bounds: (f32, f32, f32, f32),
        even_odd: bool,
        clip: ResolvedClip,
    },
}

/// The instance a committed job appends.
pub(super) struct Draw {
    color: Color,
    clip: Rect,
    style: DrawStyle,
    mask: Option<u32>,
    /// The chain whose residue the commit multiplies into the tile, where the mark's clip
    /// has a non-rectangular link (ADR 1395).
    ///
    /// Carried to the commit rather than applied in [`rasterise`], because the residue is
    /// read out of a cache the frame decides about in encounter order
    /// (`super::residue`): the fan-out makes the mark's coverage, which is a pure function
    /// of its geometry, and the commit multiplies the clip in, in the order the walk would
    /// have.
    residue: Option<ResolvedClip>,
}

impl Draw {
    pub(super) fn new(color: Color, clip: Rect, style: DrawStyle, mask: Option<u32>) -> Self {
        Self {
            color,
            clip,
            style,
            mask,
            residue: None,
        }
    }

    /// This draw under `resolved`, whose residue the commit multiplies in when the chain
    /// has one; a chain of rectangles alone leaves the draw as it was.
    pub(super) fn under(self, resolved: &ResolvedClip) -> Self {
        Self {
            residue: resolved.residues.is_some().then(|| resolved.clone()),
            ..self
        }
    }
}

/// What one job's geometry came to: `None` when the mark reaches no pixel, which is a
/// command that legitimately draws nothing rather than an error.
type Rasterised = Option<CoverageMask>;

impl<'a> Job<'a> {
    /// A glyph-lane job: the tile is the shape's own bounds at the quantised phase.
    #[expect(clippy::too_many_arguments)] // one placement's parameters, gathered once at
    // its one call site, where the walk already holds each of them for its own reasons
    pub(super) fn glyph(
        segments: &'a [Segment],
        transform: DeviceTransform,
        rule: Rule,
        key: GlyphKey,
        origin: [f32; 2],
        resident: Option<AtlasEntry>,
        tile_bound: u64,
        draw: Draw,
    ) -> Self {
        let resident_already = resident.is_some();
        Self {
            segments,
            transform,
            stroke: None,
            rule,
            extent: Extent::Own,
            place: match resident {
                Some(entry) => Place::Resident { key, origin, entry },
                None => Place::Atlas { key, origin },
            },
            draw,
            weight: weight_of(segments, resident_already, false),
            held: held_by(tile_bound, resident_already),
            outline: None,
        }
    }

    /// A path-lane job: the tile is the mark held to its clip and to the target.
    pub(super) fn sheet(
        segments: &'a [Segment],
        transform: DeviceTransform,
        stroke: Option<Stroke>,
        rule: Rule,
        rect: [f32; 4],
        tile_bound: u64,
        draw: Draw,
    ) -> Self {
        Self {
            segments,
            transform,
            stroke,
            rule,
            extent: Extent::Visible { rect },
            place: Place::Sheet,
            draw,
            weight: weight_of(segments, false, stroke.is_some()),
            held: held_by(tile_bound, false),
            outline: None,
        }
    }

    /// This fill's segments as the stored outline they are, whose answer to the
    /// two-values question is asked once for all its placements (ADR 1389).
    pub(super) fn of_outline(self, outline: &'a crate::resources::StoredOutline) -> Self {
        Self {
            outline: Some(outline),
            ..self
        }
    }

    fn weight(&self) -> u64 {
        self.weight
    }

    fn held(&self) -> u64 {
        self.held
    }

    /// A repeat of `key`, which a queued job will insert: drawn from the entry the commit
    /// finds, and walked again from `fill` where it finds none (ADR 1409).
    pub(super) fn follows(
        key: GlyphKey,
        origin: [f32; 2],
        fill: SolidFill<'a>,
        clip: ResolvedClip,
    ) -> Self {
        Self {
            segments: &[],
            transform: IDENTITY,
            stroke: None,
            rule: fill.rule,
            extent: Extent::Own,
            draw: Draw::new(fill.color, clip.rect, fill.style, fill.mask),
            place: Place::Follows {
                key,
                origin,
                fill,
                clip,
            },
            weight: 0,
            held: held_by(0, true),
            outline: None,
        }
    }

    /// A compute-lane tile, queued behind the marks before it (ADR 1409).
    pub(super) fn compute(
        outline: OutlineId,
        to_device: DeviceTransform,
        bounds: (f32, f32, f32, f32),
        even_odd: bool,
        clip: ResolvedClip,
        draw: Draw,
    ) -> Self {
        Self {
            segments: &[],
            transform: IDENTITY,
            stroke: None,
            rule: Rule::NonZero,
            extent: Extent::Own,
            place: Place::Compute {
                outline,
                to_device,
                bounds,
                even_odd,
                clip,
            },
            draw,
            weight: 0,
            held: held_by(0, true),
            outline: None,
        }
    }

    /// An analytic rectangle instance, queued behind the marks before it (ADR 1409).
    pub(super) fn rect(rect: Rect, draw: Draw) -> Self {
        Self {
            segments: &[],
            transform: IDENTITY,
            stroke: None,
            rule: Rule::NonZero,
            extent: Extent::Own,
            place: Place::Rect { rect },
            draw,
            weight: 0,
            held: held_by(0, true),
            outline: None,
        }
    }

    /// Whether this job has any geometry to make: a placement the atlas already holds, a
    /// repeat of a queued key and a rectangle cost an instance and nothing else, and the
    /// instrument must not time the nothing (a second interpretation of one page is
    /// `tests/two_rasters.rs`'s whole subject).
    fn rasterises(&self) -> bool {
        matches!(self.place, Place::Atlas { .. } | Place::Sheet)
    }

    /// The atlas key this job would write, for the guard in [`Encoder::enqueue`].
    fn key_written(&self) -> Option<GlyphKey> {
        match self.place {
            Place::Atlas { key, .. } => Some(key),
            Place::Resident { .. }
            | Place::Follows { .. }
            | Place::Sheet
            | Place::Rect { .. }
            | Place::Compute { .. } => None,
        }
    }
}

/// What one segment of a stroke weighs, in a filled outline's segments.
///
/// **A stroke's segment is not a fill's.** The stroke is expanded into a piece per segment
/// and a join per vertex, the pieces at a tight bend are tiled (ADR 1375), and the fill of
/// that expansion asks whether it winds more than two values (ADR 1389) — so the same count
/// of outline segments is many times the work. Measured over the pdf.js corpus's first
/// pages at 1×, every job timed on one pinned performance core (ADR 1395, re-taken by ADR
/// 1407 after ADR 1397 made a stroke cheaper): 166 726 glyph fills averaged 0.138 µs per
/// segment and 57 118 strokes 3.012, a ratio of 21.9. Counted
/// as one each, a page of strokes weighed a twenty-second of its work, stayed under
/// [`PARALLEL_FLOOR_SEGMENTS`] in every drain and drew on the walk's thread alone: 35 ms
/// of a 40 ms page turn on `issue14415.pdf`, 14 ms with this weight.
///
/// Rounded down, so the floor is reached no earlier than the measured costs say it should
/// be. The number moves only where a job runs on which thread, never what it makes: the
/// fan-out's bytes are the one-threaded frame's (`tests/encode_threads.rs`).
const STROKE_SEGMENT_WEIGHT: u64 = 21;

/// A job's share of the fan-out, in the units [`partition`] balances: a filled outline's
/// segments, with a stroke's counted at [`STROKE_SEGMENT_WEIGHT`].
///
/// Segments rather than tile area, because the tile does not exist until the job has run
/// and a partition computed from the geometry would need the geometry. A resident tile
/// rasterises nothing and weighs nothing.
fn weight_of(segments: &[Segment], resident: bool, stroked: bool) -> u64 {
    let segments = segments.len() as u64;
    match (resident, stroked) {
        (true, _) => 0,
        (false, false) => segments,
        (false, true) => segments.saturating_mul(STROKE_SEGMENT_WEIGHT),
    }
}

/// The host memory one queued job can hold: its tile's upper bound, or none at all for a
/// placement the atlas already has, plus the record itself either way.
fn held_by(tile_bound: u64, resident: bool) -> u64 {
    let tile = if resident { 0 } else { tile_bound };
    tile.saturating_add(size_of::<Job<'_>>() as u64)
}

/// One job's coverage: flatten, expand it if it is a stroke, and run the scanline pass
/// over the rectangle its lane chose.
///
/// **This is the whole of the parallel phase.** It takes `&Job` and returns an owned
/// mask; it reads no frame state, writes no frame state, and allocates only what it
/// returns. That is what makes a frame on twenty-four threads the same bytes as a frame
/// on one, rather than a frame that merely usually is.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[expect(clippy::arithmetic_side_effects)] // the same bounded corner arithmetic the two
// lanes did in place, moved here unchanged
pub(super) fn rasterise(job: &Job<'_>) -> Rasterised {
    if !job.rasterises() {
        return None;
    }
    let flattened = match job.stroke {
        Some(_) => raster::flatten_stroke(job.segments, job.transform),
        None => raster::flatten(job.segments, job.transform),
    };
    // A stroke whose pieces tile its set by construction winds two values and keeps its
    // integral (ADR 1397); a fill of an outline known to, likewise — that question is asked
    // on the worker, once per outline, not on the walk (ADR 1389).
    let (polylines, tiles) = match job.stroke {
        Some(stroke) => {
            let stroked = raster::stroke_pieces(
                &flattened,
                stroke,
                raster::resolve_width(stroke, job.transform),
            );
            (stroked.pieces, stroked.tiles)
        }
        None => (flattened, false),
    };
    let (x0, y0, x1, y1) = raster::polyline_bounds(&polylines)?;
    let (vx0, vy0, vx1, vy1) = match job.extent {
        Extent::Own => (x0, y0, x1, y1),
        Extent::Visible { rect } => (
            x0.max(rect[0]),
            y0.max(rect[1]),
            x1.min(rect[2]),
            y1.min(rect[3]),
        ),
    };
    if vx0 >= vx1 || vy0 >= vy1 {
        return None;
    }
    let left = vx0.floor() as i32;
    let top = vy0.floor() as i32;
    let width = (vx1.ceil() as i32 - left).max(0) as u32;
    let height = (vy1.ceil() as i32 - top).max(0) as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let settled = tiles
        || (job.stroke.is_none()
            && job
                .outline
                .is_some_and(|o| o.winds_two_values_as(&polylines)));
    Some(raster::fill_mask_settled(
        &polylines,
        job.rule,
        (left, top, width, height),
        settled,
    ))
}

/// How many jobs each worker takes: balanced by [`Job::weight`], and contiguous.
///
/// **Static, and computed from a size the walk already knew**, so there is no queue, no
/// atomic and no work stealing — the partition is a pure function of the job list, which
/// is one fewer thing between a thread count and the bytes it draws. Contiguous because
/// the results are reassembled in job order either way and adjacent jobs on a page are
/// adjacent in memory.
#[expect(clippy::arithmetic_side_effects)] // `taken + len` is bounded by `jobs.len()` by
// the loop condition, and the target is `u64` arithmetic over a sum of `usize` lengths
fn partition(jobs: &[Job<'_>], workers: usize) -> Vec<usize> {
    let total: u64 = jobs.iter().map(Job::weight).sum();
    let divisor = (workers as u64).max(1);
    let mut lens = Vec::with_capacity(workers);
    let (mut taken, mut carried) = (0_usize, 0_u64);
    for worker in 1..=workers {
        // Where this worker's share ends: `worker/workers` of the whole weight, in
        // integer arithmetic that cannot drift, since the last share ends at `total`.
        let target = total.saturating_mul(worker as u64) / divisor;
        let mut len = 0;
        while taken + len < jobs.len() && (worker == workers || carried < target) {
            carried = carried.saturating_add(jobs[taken + len].weight());
            len += 1;
        }
        taken += len;
        lens.push(len);
    }
    lens
}

/// Rasterise every job, on `threads` threads.
///
/// The caller has already decided that the fan-out is worth taking; this decides only
/// how the work is divided. One thread is a plain loop with no scope at all, which is
/// what a page below the floor and a host that never asked both take.
pub(super) fn rasterise_all(jobs: &[Job<'_>], threads: usize) -> Vec<Rasterised> {
    let mut results: Vec<Rasterised> = (0..jobs.len()).map(|_| None).collect();
    if threads <= 1 || jobs.len() < 2 {
        run(jobs, &mut results);
        return results;
    }
    let lens = partition(jobs, threads);
    let mut rest_jobs = jobs;
    let mut rest_out: &mut [Rasterised] = &mut results;
    thread::scope(|scope| {
        // Every share but the last is spawned and the last runs here, so `threads`
        // threads is `threads - 1` spawns and the calling thread is not left waiting.
        let spawned = lens.len().saturating_sub(1);
        for &len in lens.iter().take(spawned) {
            let take = len.min(rest_jobs.len());
            let (mine, jobs_tail) = rest_jobs.split_at(take);
            let (out, out_tail) = std::mem::take(&mut rest_out).split_at_mut(take);
            rest_jobs = jobs_tail;
            rest_out = out_tail;
            if !mine.is_empty() {
                scope.spawn(move || run(mine, out));
            }
        }
        run(rest_jobs, rest_out);
    });
    results
}

/// How many threads a run of this weight takes: what the host allowed, or one when the
/// run is too small to pay for a spawn.
///
/// The caller's section 4 asks for this by name — *"anything on the frame path that a small page
/// pays for"* is the one thing they excluded, and their own ADR 0228 had to put a
/// measured floor under a `rayon` image resampler for the same reason. Ours is
/// [`PARALLEL_FLOOR_SEGMENTS`].
fn fan_out(weight: u64, allowed: usize) -> usize {
    if weight >= PARALLEL_FLOOR_SEGMENTS {
        allowed
    } else {
        1
    }
}

/// One worker's share.
fn run(jobs: &[Job<'_>], out: &mut [Rasterised]) {
    for (job, slot) in jobs.iter().zip(out) {
        *slot = rasterise(job);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Draw, IN_FLIGHT_BUDGET_SHARE, Job, PARALLEL_FLOOR_SEGMENTS, STROKE_SEGMENT_WEIGHT, fan_out,
        in_flight_limit, partition,
    };
    use crate::atlas::{GlyphKey, PhaseKey};
    use crate::encode::DrawStyle;
    use crate::raster::{DeviceTransform, Rule};
    use raster_scene::{Color, Point, Rect, Segment};

    /// A job of `segments` segments, which is the only field [`partition`] reads.
    fn job(segments: &[Segment]) -> Job<'_> {
        Job::glyph(
            segments,
            DeviceTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            },
            Rule::NonZero,
            GlyphKey {
                outline: 0,
                linear: [0; 4],
                phase: PhaseKey::Quantised(0, 0),
                rule: Rule::NonZero,
            },
            [0.0, 0.0],
            None,
            9,
            Draw::new(
                Color::new(0.0, 0.0, 0.0, 1.0),
                Rect::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)),
                DrawStyle::Over,
                None,
            ),
        )
    }

    fn segments(count: usize) -> Vec<Segment> {
        vec![Segment::MoveTo(Point::new(0.0, 0.0)); count]
    }

    /// **A partition is a partition**: every job is in exactly one share, and the shares
    /// are in job order. Everything the fan-out promises about determinism rests on this
    /// one property, so it is asserted rather than read off the loop.
    #[test]
    fn every_job_is_in_exactly_one_share() {
        let sizes = [3_usize, 40, 1, 900, 7, 7, 7, 250, 2];
        let held: Vec<Vec<Segment>> = sizes.iter().map(|n| segments(*n)).collect();
        let jobs: Vec<Job<'_>> = held.iter().map(|s| job(s)).collect();
        for workers in 1..=12 {
            let lens = partition(&jobs, workers);
            assert_eq!(lens.len(), workers, "one share per worker");
            assert_eq!(
                lens.iter().sum::<usize>(),
                jobs.len(),
                "{workers} workers between them take every job"
            );
        }
    }

    /// The shares are balanced by **weight**, not by count: one enormous job does not
    /// drag its neighbours along behind it.
    #[test]
    fn one_heavy_job_does_not_take_its_neighbours_with_it() {
        let held: Vec<Vec<Segment>> = [1_usize, 1, 4_000, 1, 1]
            .iter()
            .map(|n| segments(*n))
            .collect();
        let jobs: Vec<Job<'_>> = held.iter().map(|s| job(s)).collect();
        let lens = partition(&jobs, 2);
        assert_eq!(
            lens,
            vec![3, 2],
            "the first share ends at the heavy job, the second takes what is left"
        );
    }

    /// **A stroke weighs what it costs** (ADR 1395): a stroke's outline counts each of its
    /// segments [`STROKE_SEGMENT_WEIGHT`] times, so a run of strokes reaches the floor at
    /// the work a run of fills reaches it at — and a fill of the same outline still counts
    /// it once.
    #[test]
    fn a_stroke_weighs_its_segments_at_their_measured_cost() {
        let outline = segments(200);
        let draw = || {
            Draw::new(
                Color::new(0.0, 0.0, 0.0, 1.0),
                Rect::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)),
                DrawStyle::Over,
                None,
            )
        };
        let identity = DeviceTransform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };
        let sheet = |stroke| {
            Job::sheet(
                &outline,
                identity,
                stroke,
                Rule::NonZero,
                [0.0, 0.0, 1.0, 1.0],
                1,
                draw(),
            )
        };
        let stroke = raster_scene::Stroke {
            width: 1.0,
            adjust: false,
            cap: raster_scene::LineCap::Butt,
            join: raster_scene::LineJoin::Miter,
            miter_limit: 10.0,
        };
        assert_eq!(sheet(None).weight(), 200);
        assert_eq!(sheet(Some(stroke)).weight(), 200 * STROKE_SEGMENT_WEIGHT);
        assert_eq!(
            fan_out(sheet(Some(stroke)).weight(), 24),
            24,
            "two hundred stroked segments are past the floor that four thousand filled ones reach"
        );
        assert_eq!(fan_out(sheet(None).weight(), 24), 1);
    }

    /// **The queue is a batching granularity and never a capacity.** A frame's in-flight
    /// limit is a share of the budget the host stated, and a single job larger than it is
    /// still queued — alone — so no frame is refused for being made of large marks.
    #[test]
    fn the_in_flight_limit_follows_the_frame_budget_and_refuses_nothing() {
        assert_eq!(
            in_flight_limit(268 * 1024 * 1024),
            268 * 1024 * 1024 / IN_FLIGHT_BUDGET_SHARE,
            "a sixty-fourth of the default frame budget"
        );
        assert_eq!(
            in_flight_limit(0),
            0,
            "and a budget of nothing is not a panic"
        );
    }

    /// **No small page pays for a large one's lane** (the caller's section 4). The median corpus
    /// page's twelve marks carry 120 segments between them — `tests/archetypes.rs` and
    /// `examples/encode_threads.rs` both read that number off it — and the floor is
    /// thirty-four times higher, so a page that size never enters a scope at all.
    #[test]
    fn a_page_below_the_floor_does_not_fan_out() {
        assert_eq!(fan_out(120, 24), 1, "the median corpus page");
        assert_eq!(fan_out(PARALLEL_FLOOR_SEGMENTS - 1, 24), 1);
        assert_eq!(fan_out(PARALLEL_FLOOR_SEGMENTS, 24), 24);
        assert_eq!(
            fan_out(u64::MAX, 1),
            1,
            "and a host that asked for one thread gets one at any size"
        );
    }
}
