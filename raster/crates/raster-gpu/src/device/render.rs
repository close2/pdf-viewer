//! One frame, from the call to the [`Frame`]: the phase order, the seam a retained
//! encode is replayed across, and the order the refusals are taken in.
//!
//! Four phases, numbered as every other module here cites them: **1** classify,
//! rasterise coverage and count (`crate::encode`); **2** allocate and stage
//! (`super::staging`); **3** record and submit (`super::record`); **4** wait, present or
//! read back, and read the timestamps (`super::pending`), of which only a `Readback` target
//! pays for the read. [`Device::render`] and [`Device::render_retained`] differ in exactly one
//! of the four — phase 1 — which is why phases 2 and 3 are `submit_encoded` and phase 4 is
//! `complete`, reached identically by both (ADR 0048); [`Device::submit`] and
//! [`Device::collect`] are `render`'s first three phases and its fourth, held apart (ADR
//! 1595).
//!
//! **The order the refusals are taken in is a decision, not an accident.** Every
//! refusal a scene can earn is taken before the target is bound, because a `Surface`
//! refusal must cost no swapchain acquire; the frame's internal textures are priced
//! while nothing of the frame exists yet (brief section 5: count, then allocate); and a [`Frame`]
//! is constructed only after every fallible step has succeeded, so a failed frame
//! cannot report itself drawn.

use std::time::{Duration, Instant};

use raster_scene::{MAX_COORDINATE, Scene};

use super::Device;
use super::bound::Bound;
use super::damage::DamagePlan;
use super::pending::{Submission, Submitted};
use crate::encode::{self, Encoded};
use crate::error::RenderError;
use crate::frame::{Counters, EncodeSource, Frame, Payload, Raster, TimingProvenance, Timings};
use crate::layers;
use crate::readback;
use crate::report::Report;
use crate::retained::{EncodeKey, RetainedScene};
use crate::target::Target;
use crate::viewport::Viewport;

impl Device {
    /// Render one frame of `scene` at `viewport` into `target`.
    ///
    /// The scene is not consumed and carries no target knowledge: the same scene
    /// renders at any number of viewports (brief section 2.3).
    ///
    /// # Errors
    ///
    /// A refused frame is an `Err` naming what was refused — see [`RenderError`]'s
    /// variants. On `Err`, nothing was presented and no pixels are claimed drawn.
    // Taking `Target` by value is brief section 2.4's signature: a target is a discriminant plus a
    // borrow, and a caller-side `&Target` would only add a level of indirection.
    #[expect(clippy::needless_pass_by_value)]
    pub fn render(
        &mut self,
        scene: &Scene,
        viewport: &Viewport<'_>,
        into: Target<'_>,
    ) -> Result<Frame, RenderError> {
        match self.submit_scene(scene, viewport, &into)? {
            Submission::Drawn(frame) => Ok(*frame),
            Submission::Submitted(submitted) => self.complete(*submitted),
        }
    }

    /// Phases 1 to 3 of [`Device::render`]: everything up to the device drawing, which
    /// [`Device::complete`] waits for (ADR 1595).
    pub(super) fn submit_scene<'t>(
        &mut self,
        scene: &Scene,
        viewport: &Viewport<'_>,
        into: &Target<'t>,
    ) -> Result<Submission<'t>, RenderError> {
        self.validate_viewport(viewport)?;

        let mut reports = Vec::new();
        let damage = Self::plan_damage(viewport, into, &mut reports)?;

        // Phase 1: classify, rasterise coverage, and count (encode.rs). Runs before
        // any allocation and regardless of target size, so refusals are identical
        // across targets.
        let encode_started = Instant::now();
        let generation = self.atlas.generation;
        let encoded = self.encode_scene(scene, viewport)?;
        let encode_time = encode_started.elapsed();
        // A reset the encode took for a stale atlas is a repack of this frame's (ADR 1467
        // section 4), reported as one.
        let reset_in_encode = self.atlas.generation != generation;

        let mut submission = self.submit_encoded(
            &encoded,
            viewport,
            into,
            &damage,
            reports,
            encode_time,
            EncodeSource::Encoded,
        )?;
        // Reported on the frame that caused it rather than on the one that pays for it:
        // this is the frame whose atlas layout stopped being the layout, and a caller
        // holding a `RetainedScene` learns here that its encode is now stale. Settled once
        // the frame is submitted: a reset moves what later frames upload, and the queue
        // puts every such upload behind this frame's passes.
        submission.set_repacked(self.settle_atlas(&encoded) || reset_in_encode);
        Ok(submission)
    }

    /// Render one frame of a scene the caller retains, replaying the encode of its last
    /// frame when nothing an encode depends on has changed (ADR 0048).
    ///
    /// **The same pixels as [`Device::render`] over the same scene and viewport**, and
    /// the same refusals: a replayed frame skips only phase 1, and every check phase 1
    /// is not — the frame budget for internal textures, the target's own contract, the
    /// passes themselves — runs on every call. A frame that would be refused is refused
    /// identically, whether it encoded or replayed, because an encode is retained only
    /// after it succeeded and is discarded the moment any input it read has moved.
    ///
    /// [`Frame::encode_source`] says which of the two this frame was. What survives
    /// which change, and what no design can make survive, is
    /// [`RetainedScene`]'s own table.
    ///
    /// # Errors
    ///
    /// Exactly [`Device::render`]'s, over the scene the handle holds.
    ///
    /// [`Frame::encode_source`]: crate::frame::Frame::encode_source
    // As `render`: a target is a discriminant plus a borrow.
    #[expect(clippy::needless_pass_by_value)]
    pub fn render_retained(
        &mut self,
        retained: &mut RetainedScene,
        viewport: &Viewport<'_>,
        into: Target<'_>,
    ) -> Result<Frame, RenderError> {
        self.validate_viewport(viewport)?;

        let mut reports = Vec::new();
        let damage = Self::plan_damage(viewport, &into, &mut reports)?;

        // The key is taken before the encode, and that is the safe order: encoding
        // inserts atlas tiles, which never move an existing entry, so the generation it
        // reads is the one the encode is valid under — while a *reset* triggered by this
        // frame bumps it afterwards and invalidates what this frame stored, which is
        // exactly right, since a repack moves every tile the instances name.
        let key = EncodeKey::new(
            self.id,
            viewport,
            self.coverage,
            self.atlas.generation,
            self.resources.generation(),
        );
        let encode_started = Instant::now();
        // Borrowed from `retained`, which is not `self`: the encode and the device it
        // draws through are two objects, so nothing here needs a clone.
        let generation = self.atlas.generation;
        let (source, encoded) = retained.prepare(key, |scene, list| match list {
            Some(list) => self.replay_scene(scene, viewport, list),
            None => self.encode_scene(scene, viewport),
        })?;
        let encode_time = encode_started.elapsed();
        let reset_in_encode = self.atlas.generation != generation;

        let mut frame = self
            .submit_encoded(
                encoded,
                viewport,
                &into,
                &damage,
                reports,
                encode_time,
                source,
            )
            .and_then(|submission| match submission {
                Submission::Drawn(frame) => Ok(*frame),
                Submission::Submitted(submitted) => self.complete(*submitted),
            });
        // A replay inserted nothing, so there is nothing for a repack to settle — and
        // resetting after one would bump the generation the replayed encode is keyed
        // under and cost the next frame an encode for no reason.
        if let Ok(frame) = frame.as_mut()
            && source == EncodeSource::Encoded
        {
            frame.counters.atlas_repacked = self.settle_atlas(encoded) || reset_in_encode;
        }
        frame
    }

    /// Phase 1: classify, rasterise coverage, and count (`encode.rs`).
    ///
    /// **A frame's lanes, and so its admission, are its own** (ADR 1467 section 4). The
    /// atlas is a cache the pages before this one filled, and where the frame was refused
    /// room in it while it holds entries this frame never used, the frame was decided by
    /// those pages: glyphs that a fresh atlas takes went to the scratch sheet, the sheet's
    /// extent moved, and a page that fits alone was refused after another. So such a
    /// frame — drawn or refused — is encoded again on a reset atlas, which is the frame a
    /// fresh device draws. It costs one encode on the frame after a change of working
    /// set, and nothing where the atlas holds only what the frame uses.
    fn encode_scene(
        &mut self,
        scene: &Scene,
        viewport: &Viewport<'_>,
    ) -> Result<Encoded, RenderError> {
        self.atlas.begin_frame();
        // Once a render, before either walk: a frame encoded again on a reset atlas reads
        // the meets its first walk kept, which are the same bytes (ADR 1517).
        self.kept_meets.begin_render(self.limits.max_frame_bytes);
        let encoded = self.encode_once(scene, viewport);
        if !self.atlas.stale_for_frame() {
            return encoded;
        }
        self.atlas.reset();
        self.atlas.begin_frame();
        self.encode_once(scene, viewport)
    }

    /// One walk of the scene against the atlas as it stands.
    fn encode_once(
        &mut self,
        scene: &Scene,
        viewport: &Viewport<'_>,
    ) -> Result<Encoded, RenderError> {
        encode::encode(
            scene,
            viewport,
            self.limits.max_frame_bytes,
            self.limits.max_target_size,
            &self.resources,
            &mut self.atlas,
            &mut self.kept_meets,
            self.glyph_quantum,
            self.coverage,
            self.compute_assist,
            self.coverage_samples,
            self.instrument_encode,
            self.encode_threads,
        )
    }

    /// Phase 1 replayed from records (`encode/replay.rs`, ADR 0087): the per-scene
    /// answers come from the list, only the per-viewport arithmetic runs.
    fn replay_scene(
        &mut self,
        scene: &Scene,
        viewport: &Viewport<'_>,
        list: &encode::ReplayList,
    ) -> Result<Encoded, RenderError> {
        self.kept_meets.begin_render(self.limits.max_frame_bytes);
        encode::replay(
            scene,
            viewport,
            self.limits.max_frame_bytes,
            self.limits.max_target_size,
            &self.resources,
            &mut self.atlas,
            &mut self.kept_meets,
            self.glyph_quantum,
            self.coverage,
            self.compute_assist,
            self.coverage_samples,
            self.instrument_encode,
            self.encode_threads,
            list,
        )
    }

    /// Phases 2 and 3 of a frame: price, allocate, upload, record and submit — everything
    /// before the device has to have drawn anything.
    ///
    /// Everything that is not phase 1, which is the seam a retained encode is replayed
    /// across (ADR 0048): the two callers differ only in where the [`Encoded`] came
    /// from, and every refusal below this line is taken by both. Phase 4 is
    /// [`Device::complete`], which [`Device::render`] calls at once and [`Device::collect`]
    /// when its caller has walked whatever it walks beside the device (ADR 1595).
    #[expect(clippy::too_many_arguments)] // one frame's inputs, named once at two call sites
    pub(super) fn submit_encoded<'t>(
        &mut self,
        encoded: &Encoded,
        viewport: &Viewport<'_>,
        into: &Target<'t>,
        damage: &DamagePlan,
        reports: Vec<Report>,
        encode_time: Duration,
        source: EncodeSource,
    ) -> Result<Submission<'t>, RenderError> {
        // Before the zero-size branch and before anything is drawn: what a frame says
        // about itself has to be true of a frame that draws nothing too, and the claim
        // this report makes is about the scene rather than about the pixels.
        let mut reports = reports;
        encode::empty_stack_reports(&encoded.used_functions, &self.resources, &mut reports);
        if viewport.width == 0 || viewport.height == 0 {
            return Self::zero_size_frame(viewport, into, encoded, encode_time, reports, source)
                .map(|frame| Submission::Drawn(Box::new(frame)));
        }

        self.price_internal_textures(encoded, viewport, damage)?;

        // Phase 2: allocate (sized by phase 1) and schedule uploads — including
        // the device-resident form of any image, ramp or mesh drawn for the first
        // time this frame.
        let paint_started = Instant::now();
        let paint_bytes = self.ensure_paint_textures(encoded)?;
        let paint_time = paint_started.elapsed();

        // Every refusal a scene can earn has been taken; bind the target last, so
        // the acquire happens only for a frame that will run.
        //
        // Timed and reported: the caller's feedback section 13 prints a `device` minus our
        // phases remainder to name the acquire and the present, and says it no longer
        // believes that subtraction is a duration of anything (brief section 11.1's clocks do not
        // mix). Two clock reads a frame is nothing next to being able to name them.
        let acquire_started = Instant::now();
        let bound = self.bind_target(into, viewport)?;
        let acquire = acquire_started.elapsed();

        let query = self.take_pass_query();
        // The frame's body, and the loop it lives in (ADR 0095): the compute chain
        // runs with no mid-frame readback, guarded against a persistent capacity, and
        // the eight bytes saying whether it fit are read here — after the wait the
        // frame pays anyway, before anything is presented. A frame whose emit met the
        // capacity grows to the scanned total and runs again, whole; a steady frame
        // pays this branch nothing but the read.
        let mut attempts = 0_u32;
        let (upload_time, upload_bytes, upload_spans, compute_stamped, ran) = loop {
            let mut upload = self.upload(encoded)?;
            let upload_time = upload.time.saturating_add(paint_time);
            let upload_bytes = upload.bytes.saturating_add(paint_bytes);
            let upload_spans = std::mem::take(&mut upload.spans);
            let compute_stamped = upload.compute_stamped;
            let compute_run = upload.compute_run.take();
            let ran = match self.run_frame(encoded, &bound, upload, query.as_ref(), damage) {
                Ok(ran) => ran,
                Err(error) => return Err(self.abandon_frame(bound, error)),
            };
            let Some(run) = compute_run else {
                break (
                    upload_time,
                    upload_bytes,
                    upload_spans,
                    compute_stamped,
                    ran,
                );
            };
            match run.overflowed(&self.gpu)? {
                None => {
                    break (
                        upload_time,
                        upload_bytes,
                        upload_spans,
                        compute_stamped,
                        ran,
                    );
                }
                Some(total) => {
                    // Growth refuses by name past the budget, exactly as the
                    // per-frame allocation it replaces did.
                    self.compute_persist
                        .grow(&self.gpu, total, self.limits.max_frame_bytes)?;
                    attempts = attempts.saturating_add(1);
                    if attempts > 3 {
                        // Three growths that still overflow mean the scanned total
                        // and the emit disagree, which the shared walk makes
                        // impossible; refusing beats presenting a truncated frame.
                        return Err(RenderError::ReadbackFailed {
                            detail: "the compute capacity kept overflowing its own scan".into(),
                        });
                    }
                }
            }
        };
        // A replayed frame spent nothing on geometry or staging, and the clock inside a
        // retained encode still holds what the frame that *made* it spent — so the
        // subdivision has to come from the source, not from the clock (ADR 0048).
        let encode_phases = match source {
            // A record replay ran its own (much shorter) walk, and its clock holds
            // what that walk spent — the same claim an ordinary encode's makes.
            EncodeSource::Encoded | EncodeSource::RecordReplayed => {
                encoded.encode_phases.phases(encode_time)
            }
            EncodeSource::Replayed => encoded.encode_phases.replayed(),
        };
        let (submit_host, submission, phases, layer_textures) = ran;

        // A target this frame owns is copied out behind its passes now, so that the copy is
        // on the device before anything waits for the frame (ADR 1595).
        let copy = match &bound {
            Bound::Owned(texture) => {
                let started = Instant::now();
                let copy = readback::copy_out(
                    &self.gpu,
                    &self.queue,
                    texture,
                    viewport.width,
                    viewport.height,
                    self.limits.max_target_size,
                )?;
                Some((copy, started.elapsed()))
            }
            Bound::Borrowed(_) | Bound::Acquired(_) => None,
        };

        Ok(Submission::Submitted(Box::new(Submitted {
            bound,
            query,
            submission,
            submit_host,
            copy,
            compute_stamped,
            phases,
            acquire,
            encode_phases,
            upload_spans,
            timings: (encode_time, upload_time),
            counters: self.counters(encoded, upload_bytes, layer_textures),
            reports,
            source,
            token: std::sync::Arc::clone(&self.pending),
        })))
    }

    /// Price the compositor's internal textures while nothing of the frame
    /// exists yet (brief section 5: count then allocate; the refusal names both numbers).
    ///
    /// Before the target is bound on purpose: a `Surface` refusal must cost no
    /// swapchain acquire, because a texture acquired and then dropped unpresented
    /// leaves the swapchain a semaphore no submission will ever wait on — the
    /// viewer measured that as every later acquire timing out, permanently.
    /// A patched frame renders through the root pair even when flat.
    fn price_internal_textures(
        &self,
        encoded: &Encoded,
        viewport: &Viewport<'_>,
        damage: &DamagePlan,
    ) -> Result<(), RenderError> {
        let patches = matches!(damage, DamagePlan::Patch { rects, .. } if !rects.is_empty());
        let internal_bytes =
            layers::internal_texture_bytes(encoded, viewport.width, viewport.height, patches);
        if internal_bytes > self.limits.max_frame_bytes {
            return Err(RenderError::FrameBudgetExceeded {
                needed: internal_bytes,
                budget: self.limits.max_frame_bytes,
            });
        }
        Ok(())
    }

    /// What this frame counted, for the [`Frame`] that reports it.
    ///
    /// Everything here is read from the encode or from what phase 2 and phase 3 spent;
    /// nothing is computed a second way. `atlas_entries` is the one number taken from
    /// the device rather than the encode, because it is the atlas's state after this
    /// frame's tiles went in.
    fn counters(&self, encoded: &Encoded, upload_bytes: u64, layer_textures: u32) -> Counters {
        Counters {
            commands: encoded.commands,
            lanes: encoded.lanes,
            clip_distinct_regions: encoded.clip_distinct_regions,
            clip_residue_regions: encoded.clip_residue_regions,
            clip_residue_tiles: encoded.clip_residue_tiles,
            distinct_outlines: encoded.distinct_outlines,
            atlas_entries: u32::try_from(self.atlas.entry_count()).unwrap_or(u32::MAX),
            atlas_distinct_keys: encoded.atlas_distinct_keys,
            atlas_working_set_bytes: encoded.atlas_requested_bytes,
            atlas_overflow_tiles: encoded.atlas_overflow_tiles,
            // Set by the caller of `submit_encoded`, which is where the repack is
            // decided: a frame that has not settled its atlas yet has not repacked
            // it, and a `Frame` may not carry a number that is not true.
            atlas_repacked: false,
            segments: encoded.segments,
            tiles: encoded.tiles,
            coverage: encoded.coverage,
            commands_culled: encoded.commands_culled,
            layers_culled: encoded.layers_culled,
            bytes_uploaded: upload_bytes,
            layer_textures,
        }
    }

    /// After a drawn frame that encoded: repack the atlas when this frame's tiles no
    /// longer fit it, and only when repacking can change that (ADR 0024, ADR 0050).
    ///
    /// Answers `true` when the atlas was reset, which is the one event that moves every
    /// tile and so invalidates every [`RetainedScene`] encode keyed on the layout
    /// ([`Counters::atlas_repacked`](crate::frame::Counters::atlas_repacked) reports it).
    ///
    /// Three conditions, and the third is ADR 0050's:
    ///
    /// - **a tile fell through to scratch** — otherwise the atlas is serving the frame;
    /// - **the frame's own working set would fit an empty atlas by bytes**. When the
    ///   distinct keys are simply larger than the cache, resetting throws away the part
    ///   that fits and hits and every frame pays the packing again: measured at 100× on
    ///   the zoom ladder, 6.0 ms of encode against 0.6 ms for keeping what fits;
    /// - **the atlas holds entries this frame did not use**. A repack reclaims exactly
    ///   those, and nothing else: the survivors are re-inserted in the frame's own
    ///   encounter order, which is the order they were inserted in to begin with, so a
    ///   repack of an atlas holding nothing else *provably* reproduces the layout it
    ///   replaced — the same tiles in the same shelves, and the same tile overflowing at
    ///   the end. Taking it anyway was an atlas that reset on every frame of a page it
    ///   could never hold, and a retained encode that never survived one.
    fn settle_atlas(&mut self, encoded: &Encoded) -> bool {
        let atlas_bytes = self.atlas.byte_size();
        let resident = u32::try_from(self.atlas.entry_count()).unwrap_or(u32::MAX);
        let repack = encoded.atlas_pressure
            && encoded.atlas_requested_bytes <= atlas_bytes
            && resident > encoded.atlas_entries_used;
        if repack {
            self.atlas.reset();
        }
        repack
    }

    fn validate_viewport(&self, viewport: &Viewport<'_>) -> Result<(), RenderError> {
        if !viewport.transform.is_finite() {
            return Err(RenderError::NonFiniteViewportTransform);
        }
        // The scene boundary holds every rectangle corner, every outline point and every
        // command transform to `MAX_COORDINATE`; this is the third factor of a device
        // coordinate and the only one that used to be checked for finiteness alone. With
        // all three bounded the largest device coordinate is about `4e27`, which leaves
        // `f32` eleven orders of magnitude of headroom — and that headroom is what
        // `raster::accumulate_edge`'s slope test spends. See
        // [`RenderError::ViewportTransformTooLarge`].
        let coefficient = viewport.transform.max_coefficient();
        if coefficient > MAX_COORDINATE {
            return Err(RenderError::ViewportTransformTooLarge {
                coefficient,
                limit: MAX_COORDINATE,
            });
        }
        let limit = self.limits.max_target_size;
        if viewport.width > limit || viewport.height > limit {
            return Err(RenderError::TargetTooLarge {
                width: viewport.width,
                height: viewport.height,
                limit,
            });
        }
        Ok(())
    }

    /// A zero-size readback is a legitimate frame — a zero-size raster follows from a
    /// zero-size window. The other targets cannot exist at zero size.
    fn zero_size_frame(
        viewport: &Viewport<'_>,
        into: &Target<'_>,
        encoded: &Encoded,
        encode_time: Duration,
        reports: Vec<Report>,
        source: EncodeSource,
    ) -> Result<Frame, RenderError> {
        match into {
            Target::Readback => Ok(Frame {
                timings: Timings {
                    encode: encode_time,
                    upload: Duration::ZERO,
                    execute: Duration::ZERO,
                    readback: Duration::ZERO,
                    // Nothing executed; a zero wall clock is the honest source.
                    execute_provenance: TimingProvenance::WallClock,
                    phases: Vec::new(),
                },
                counters: Counters {
                    commands: encoded.commands,
                    ..Counters::default()
                },
                reports,
                payload: Payload::Raster(Raster::new(viewport.width, viewport.height, Vec::new())),
                encode_source: source,
            }),
            Target::Surface => Err(RenderError::ZeroSizeTarget { target: "Surface" }),
            Target::Texture(_) => Err(RenderError::ZeroSizeTarget { target: "Texture" }),
        }
    }
}
