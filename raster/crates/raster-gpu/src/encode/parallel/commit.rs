//! The encoder's side of the seam: what it queues, when it must stop queueing, and how
//! a rasterised job is placed.
//!
//! Split from its parent because they are two subjects. [`super`] is the *work* — a
//! `Job`, a pure function over one, and the arithmetic that divides a list of them — and
//! nothing in it can see a frame. This is the *frame*: the budget, the atlas, the sheet
//! and the instance stream, all of which are ordered and all of which stay on the
//! calling thread. A child module rather than a sibling, so `Job`'s fields stay private
//! to the pair (ADR 0051).
//!
//! Read [`super`]'s header first: the three phases, the drain rule and the atlas guard
//! are stated there, and this file is where two of the three happen.

use raster_scene::{Point, Rect};

use crate::atlas::{AtlasEntry, CacheProspect, GlyphKey, GlyphPlacement};
use crate::error::RenderError;
use crate::startup::Coverage;

use super::super::fill::SolidFill;
use super::super::instance::CoverageSource;
use super::super::{Encoder, ResolvedClip};
use super::{Draw, Job, Place, Rasterised, fan_out, rasterise, rasterise_all};

impl<'a> Encoder<'a> {
    /// What the atlas would do for this placement, asked of an atlas the queue has
    /// already reached.
    ///
    /// **The guard, and it belongs here rather than at [`Encoder::enqueue`].** A queued
    /// job has not inserted its key yet, so a repeat of that key would read `entry: None`
    /// and be *built* as a second rasterisation of one picture — and by the time the
    /// queue is drained the lane has already been chosen on the stale answer, which is
    /// exactly the "chosen on one reading, drawn on another" hazard
    /// [`AtlasStore::prospect`](crate::atlas::AtlasStore::prospect) is written against.
    /// Committing first is what the serial walk does: the repeat then finds the entry.
    ///
    /// Costs one lookup in an empty set per solid fill when no queue exists, and nothing
    /// at all when the host asked for one thread.
    pub(in crate::encode) fn prospect_for(
        &mut self,
        placement: GlyphPlacement,
        width: u32,
        height: u32,
        placed_once: bool,
    ) -> Result<CacheProspect, RenderError> {
        if self.threads > 1 && self.queued_keys.contains(&placement.key) {
            self.drain_queue()?;
        }
        // The room probe (ADR 0093) is asked only where a refused tile has somewhere
        // better to go — the hybrid's device flattening. **Its answer picks the lane, and
        // the lanes are not the same picture**: an admitted tile is rasterised at the
        // quantised phase (ADR 0009), a refused one at its own transform, and on
        // `issue1905.pdf` the two are 10 levels apart. So it is answered from the atlas a
        // one-threaded walk reads, every insert before it committed: where a queued insert
        // could still change the answer, the queue is drained first, and where none could
        // it is asked at once (ADR 1407).
        let probe_room = self.compute_assist && self.coverage == Coverage::Cpu;
        if self.probe_unsettled(&placement.key, width, height) {
            self.drain_queue()?;
        }
        Ok(self
            .atlas
            .prospect(placement, width, height, placed_once, probe_room))
    }

    /// Whether the room probe's answer for this placement could still be changed by an
    /// atlas insert the queue holds ([`AtlasStore::probe_settled`](crate::atlas::AtlasStore::probe_settled)):
    /// never where the probe is not asked, nor where nothing is queued to insert (ADR 1407).
    ///
    /// **Drained for rather than carried to the commit.** A fill carried there and walked
    /// again (`Job::follows`) was measured: the carried fills add no weight, so the queue
    /// they sit in drains no sooner and every fill behind them stays unsettled — 123 536 on
    /// the corpus at 1× against 551 drains, and `issue12295.pdf` 22× slower. Draining costs
    /// the pages whose atlas is near full 10% over twelve of them (ADR 1407).
    fn probe_unsettled(&self, key: &GlyphKey, width: u32, height: u32) -> bool {
        self.compute_assist
            && self.coverage == Coverage::Cpu
            && !self.queued_inserts.is_empty()
            && !self
                .atlas
                .probe_settled(key, width, height, &self.queued_inserts)
    }

    /// Records a queued atlas insert of a tile the hull's box calls `width × height`: at its
    /// quantised phase the tile is at most one pixel wider and one taller (ADR 1407).
    pub(in crate::encode) fn note_atlas_insert(&mut self, width: u32, height: u32) {
        if self.threads > 1 {
            self.queued_inserts
                .add(width.saturating_add(1), height.saturating_add(1));
        }
    }

    /// The largest coverage tile a mark with these bounds can make: the same arithmetic
    /// [`Encoder::visible_tile`] does, in bytes.
    ///
    /// `bounds` is the shape's control hull, so this is an **upper** bound on the tile
    /// the rasteriser will produce from the flattened geometry — which is what
    /// `Job::held` needs and the only thing about a mark's size that is known before it
    /// is flattened.
    pub(in crate::encode) fn tile_bound(
        &self,
        bounds: (f32, f32, f32, f32),
        resolved: &ResolvedClip,
    ) -> u64 {
        self.visible_tile(bounds, resolved)
            .map_or(0, |(_, _, width, height)| {
                u64::from(width).saturating_mul(u64::from(height))
            })
    }

    /// The rectangle a path-lane job rasterises over — clip ∩ target — or `None` when
    /// this mark may not leave the walk's thread.
    ///
    /// One condition: **[`Coverage::Gpu`]** asks [`Encoder::take_gpu_lane`] a second
    /// question about the *flattened* triangle count, so a job that skipped the flattening
    /// would be choosing its lane on one reading and drawing on another — the hazard ADR
    /// 0029 names. [`Coverage::Compute`] asks it too and is answered no on sight
    /// ([`Encoder::gpu_lane_admissible`]), so a stroke or a fill the compute kernels do not
    /// take is rasterised by the processor on that lane as on this one, and leaves the
    /// thread the same way: a zoom step's strokes are the fan-out's (ADR 1409).
    ///
    /// **A residue clip leaves the thread too** (ADR 1395). Its product is read out of a
    /// cache the walk decides about in encounter order (`super::super::residue`), so the
    /// job makes only the mark's coverage and the commit multiplies the clip in
    /// ([`Draw::under`](super::Draw::under)); [`Encoder::residue_intersection`] drains the
    /// queue before any other caller reads that cache, so every decision in it is taken in
    /// the order a one-threaded walk takes it. Such a tile is sized by the chain's
    /// [`mark_bounds`](ResolvedClip::mark_bounds), which is what
    /// [`Encoder::visible_tile`] sizes it by in place.
    ///
    /// Folding the clip and the target together here is exact: `f32::max` is associative
    /// and returns its non-NaN operand, so `max(max(x0, clip), 0)` is
    /// `max(x0, max(clip, 0))` for every input, which is what makes this the same bound
    /// [`Encoder::coverage_tile`] computes in place.
    pub(in crate::encode) fn deferrable_bounds(&self, resolved: &ResolvedClip) -> Option<[f32; 4]> {
        (self.coverage != Coverage::Gpu).then(|| {
            if resolved.residues.is_some() {
                self.folded(resolved.mark_bounds())
            } else {
                self.visible_rect(resolved)
            }
        })
    }

    /// Clip ∩ target as a job's rectangle — the folded bound `deferrable_bounds`
    /// documents, shared with the compute lane's route (ADR 0080).
    pub(in crate::encode) fn visible_rect(&self, resolved: &ResolvedClip) -> [f32; 4] {
        self.folded(resolved.rect)
    }

    /// `clip` ∩ target, as `[left, top, right, bottom]`.
    #[expect(clippy::cast_precision_loss)] // a viewport extent, far inside f32's exact
    // integer range
    fn folded(&self, clip: Rect) -> [f32; 4] {
        [
            clip.min.x.max(0.0),
            clip.min.y.max(0.0),
            clip.max.x.min(self.viewport.width as f32),
            clip.max.y.min(self.viewport.height as f32),
        ]
    }

    /// Take a job: queue it when the host asked for threads, run it here otherwise.
    pub(in crate::encode) fn enqueue(&mut self, job: Job<'a>) -> Result<(), RenderError> {
        if self.threads <= 1 {
            // The clock is opened here rather than around the whole of `enqueue`,
            // because what ADR 0023 calls geometry is the flattening and the scanline
            // pass and not the commit that follows them — `drain_queue` opens the same
            // span around the fan-out for the same reason.
            let span = if job.rasterises() {
                self.clock.start()
            } else {
                None
            };
            let mask = rasterise(&job);
            self.clock.geometry(span);
            return self.commit(&job, mask);
        }
        // **The queue is bounded in bytes, not only in jobs.** Where the walk held one
        // coverage tile at a time, a queue holds every tile in flight — so a page of
        // large marks could hold a gigabyte between the walk and the commit, un-charged,
        // which is principle 3's "never allocate from an unchecked number" arriving
        // through the back door. Draining first keeps the peak at one limit plus one job,
        // and a drain is always legal: it is the order the walk would have run in anyway.
        if !self.queue.is_empty()
            && self.queued_bytes.saturating_add(job.held()) > self.in_flight_limit
        {
            self.drain_queue()?;
        }
        if let Some(key) = job.key_written() {
            self.queued_keys.insert(key);
        }
        self.queued_weight = self.queued_weight.saturating_add(job.weight());
        self.queued_bytes = self.queued_bytes.saturating_add(job.held());
        self.queue.push(job);
        Ok(())
    }

    /// Rasterise everything queued and commit it, in encounter order.
    ///
    /// The queue is emptied **before** the commit runs, so the commit reaches the same
    /// ordinary methods the walk does and finds nothing to drain. A repeat whose key the
    /// atlas still lacks at its commit is walked again there and may queue a job of its
    /// own (ADR 1409); the next commit drains it before placing anything, and the loop
    /// drains what the last one left, so the queue is empty when this returns.
    pub(in crate::encode) fn drain_queue(&mut self) -> Result<(), RenderError> {
        while !self.queue.is_empty() {
            let jobs = std::mem::take(&mut self.queue);
            self.queued_keys.clear();
            self.queued_inserts.clear();
            self.queued_bytes = 0;
            let weight = std::mem::take(&mut self.queued_weight);
            let threads = fan_out(weight, self.threads);
            // The fan-out's own wall clock is this frame's geometry, whoever ran it: the
            // instrument's subject is what the frame spent, not what one thread did
            // (ADR 0023).
            let span = if weight > 0 { self.clock.start() } else { None };
            let masks = rasterise_all(&jobs, threads);
            self.clock.geometry(span);
            for (job, mask) in jobs.iter().zip(masks) {
                self.commit(job, mask)?;
            }
        }
        Ok(())
    }

    /// Place one rasterised job: the third phase, in encounter order.
    fn commit(&mut self, job: &Job<'a>, mask: Rasterised) -> Result<(), RenderError> {
        match &job.place {
            Place::Resident { key, origin, entry } => {
                self.commit_glyph(*key, *origin, Some(*entry), None, &job.draw)
            }
            Place::Atlas { key, origin } => self.commit_glyph(*key, *origin, None, mask, &job.draw),
            Place::Follows {
                key,
                origin,
                fill,
                clip,
            } => {
                // A walk-again earlier in this drain may have queued a job, which is
                // earlier in the order than this read of the atlas: its insert is committed
                // first, so the entry and the room a walk-again reads are the walk's.
                self.drain_queue()?;
                match self.atlas.get(key) {
                    Some(entry) => self.commit_glyph(*key, *origin, Some(entry), None, &job.draw),
                    None => self.fill_solid(fill, clip),
                }
            }
            Place::Sheet => self.commit_sheet(mask, &job.draw),
            Place::Rect { rect } => {
                self.drain_queue()?;
                self.write_rect_instance(*rect, job.draw.color, job.draw.style, job.draw.mask);
                Ok(())
            }
            Place::Compute {
                outline,
                to_device,
                bounds,
                even_odd,
                clip,
            } => {
                // Emptied first, so the tile is written here rather than queued again.
                self.drain_queue()?;
                self.fill_compute(
                    *outline,
                    to_device,
                    *bounds,
                    *even_odd,
                    job.draw.color,
                    job.draw.style,
                    job.draw.mask,
                    clip,
                )
            }
        }
    }

    /// Queue a repeat of a key the queue will write, where the one-threaded walk's answer
    /// about it is known before the atlas has the key, and say so — or `false`, and the
    /// caller asks [`Encoder::prospect_for`], which drains.
    ///
    /// **Why this is the walk's answer, read later rather than guessed.** Under
    /// [`Coverage::Cpu`] with no residue clip, the walk's lane for a solid fill turns on
    /// [`AtlasStore::prospect`](crate::atlas::AtlasStore::prospect) alone, and of what that
    /// reads only the entry depends on what is queued: the tile's admission is a question
    /// about its size and the atlas's extent, and the room probe is asked only where there
    /// is no entry. So the walk's answer for a tile the atlas admits is the entry the queued
    /// job leaves, and the commit reads it in encounter order, where the walk would have.
    /// Where the commit finds none — the first placement made no geometry, or found the
    /// atlas full — the fill is walked again from there (ADR 1409).
    ///
    /// A drain here is what it saves: on a page of text a key recurs within a few words,
    /// so a repeat that drained would leave no run long enough to reach the fan-out's floor.
    pub(in crate::encode) fn follow_queued(
        &mut self,
        fill: &SolidFill<'a>,
        resolved: &ResolvedClip,
        placement: GlyphPlacement,
        (width, height): (u32, u32),
    ) -> Result<bool, RenderError> {
        let follows = self.threads > 1
            && self.coverage == Coverage::Cpu
            && resolved.residues.is_none()
            && self.queued_keys.contains(&placement.key)
            && self.atlas.admits(width, height);
        if follows {
            let job = Job::follows(placement.key, placement.origin, *fill, resolved.clone());
            self.enqueue(job)?;
        }
        Ok(follows)
    }

    /// The glyph lane's commit: the atlas is offered the tile, and the placement draws
    /// from wherever it ends up.
    ///
    /// `resident` and `tile` are the two ways a placement can have a picture and they
    /// are exclusive by construction — [`Job::glyph`] chooses one at the walk, which is
    /// the reading `AtlasStore::prospect` insists on being made once.
    #[expect(clippy::cast_precision_loss)] // a tile corner and an atlas extent are both
    // integers bounded by the device dimension, far inside f32's exact range
    fn commit_glyph(
        &mut self,
        key: GlyphKey,
        origin: [f32; 2],
        resident: Option<AtlasEntry>,
        tile: Option<crate::raster::CoverageMask>,
        draw: &Draw,
    ) -> Result<(), RenderError> {
        let [ix, iy] = origin;
        let first_use = self.atlas_keys.insert(key);
        let entry = if let Some(entry) = resident {
            if first_use {
                self.atlas_requested_bytes = self
                    .atlas_requested_bytes
                    .saturating_add(u64::from(entry.width).saturating_mul(u64::from(entry.height)));
            }
            Some(entry)
        } else {
            // No geometry: the placement marks nothing, and the key stays counted —
            // which is what the serial walk did, since it took the key before it knew.
            let Some(tile) = tile else { return Ok(()) };
            self.charge_tile(tile.width, tile.height)?;
            if first_use {
                self.atlas_requested_bytes = self
                    .atlas_requested_bytes
                    .saturating_add(u64::from(tile.width).saturating_mul(u64::from(tile.height)));
            }
            let span = self.clock.start();
            let inserted = self.atlas.insert(key, &tile);
            self.clock.staging(span);
            if inserted.is_none() {
                // Atlas full: this tile draws uncached, and the device repacks the atlas
                // after the frame. Same pixels either way — one rasteriser feeds both
                // paths.
                self.atlas_pressure = true;
                // Counted per *placement* and not per key, because what this measures is
                // the work the frame did rather than the keys it wanted: a mark whose key
                // another placement already failed to insert is rasterised, packed and
                // sampled all over again (`Counters::atlas_overflow_tiles`).
                self.atlas_overflow_tiles = self.atlas_overflow_tiles.saturating_add(1);
                let dest = Point::new(ix + tile.left as f32, iy + tile.top as f32);
                return self
                    .push_scratch_quad(&tile, dest, draw.color, draw.clip, draw.style, draw.mask);
            }
            inserted
        };
        // One count per distinct key that reached an entry, however it reached it
        // (ADR 0050).
        if first_use && entry.is_some() {
            self.atlas_entries_used = self.atlas_entries_used.saturating_add(1);
        }
        let Some(entry) = entry else { return Ok(()) };
        let dest = Point::new(ix + entry.tile_left as f32, iy + entry.tile_top as f32);
        let device_rect = Rect::new(
            dest,
            Point::new(dest.x + entry.width as f32, dest.y + entry.height as f32),
        );
        if device_rect.intersection(draw.clip).is_empty() {
            return Ok(());
        }
        self.push_quad_instance(
            dest,
            entry.width as f32,
            entry.height as f32,
            entry.x as f32,
            entry.y as f32,
            CoverageSource::Atlas,
            draw.color,
            draw.clip,
            draw.style,
            draw.mask,
        )
    }

    /// The path lane's commit: the tile is charged, packed and drawn.
    #[expect(clippy::cast_precision_loss)] // a tile's corner is an integer device pixel
    fn commit_sheet(
        &mut self,
        tile: Option<crate::raster::CoverageMask>,
        draw: &Draw,
    ) -> Result<(), RenderError> {
        let Some(mut tile) = tile else { return Ok(()) };
        self.charge_tile(tile.width, tile.height)?;
        // The clip meets the mark here, as it does in `Encoder::coverage_tile`, and after
        // the same charge: the walk would have charged, rasterised and met the residue, and
        // the rasterising is the only step that moved (ADR 1395).
        if let Some(resolved) = &draw.residue {
            self.meet_residue(&mut tile, resolved)?;
        }
        let dest = Point::new(tile.left as f32, tile.top as f32);
        self.push_scratch_quad(&tile, dest, draw.color, draw.clip, draw.style, draw.mask)
    }
}
