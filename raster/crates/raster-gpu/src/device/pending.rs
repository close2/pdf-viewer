//! A frame between its submission and its pixels: phase 4 of [`super::render`]'s four, and the
//! handle a caller holds while the device draws (ADR 1595).
//!
//! **Why a frame can be held at all.** A frame read back to the processor waits twice after it
//! is submitted — for its passes, then for the copy of its target — and the host does nothing
//! meanwhile. A caller with a second frame to walk whose walk needs nothing of the first frame's
//! pixels can walk it in that time: [`Device::submit`] hands back a [`PendingFrame`] once the
//! passes and the copy are on the device, and [`Device::collect`] waits for exactly that frame's
//! submissions and makes its [`Frame`]. [`Device::render`] is the two back to back.
//!
//! **What makes the pixels the same.** Everything the second frame's walk writes reaches the
//! device through the queue — an upload, an atlas tile, a fresh target — and the queue runs in
//! submission order, so the first frame's passes have read what they read before anything
//! submitted after them writes over it. What the second walk reads of the first is host state
//! made while the first was *walked* — the coverage tiles and regions its meets kept (ADRs 1517,
//! 1529) and the resources it uploaded — which is whole when `submit` returns.
//!
//! **Two kinds complete at once**, so a pending frame never holds either: a frame whose compute
//! chain stamped its queries, whose stamps sit in the device's one set of them and would be
//! written over by the next frame's chain before this one read them; and a zero-size frame, which
//! has nothing to wait for. A compute chain's frame has in any case waited for its passes before
//! it is submitted, since the chain's overflow is read before the frame can be called drawn.

use std::sync::Arc;
use std::time::{Duration, Instant};

use raster_scene::Scene;

use super::Device;
use super::bound::Bound;
use super::record::FramePhases;
use crate::error::RenderError;
use crate::frame::{Counters, EncodeSource, Frame, Payload, TimingProvenance, Timings};
use crate::readback::{self, PendingCopy};
use crate::report::Report;
use crate::target::Target;
use crate::timing::{self, PassQuery};
use crate::viewport::Viewport;

/// A frame submitted to the device and not yet read back: what [`Device::submit`] hands back and
/// [`Device::collect`] turns into a [`Frame`] (ADR 1595).
///
/// Collect it on the device that submitted it. Dropping one uncollected is allowed and costs the
/// frame: its pixels are never read, and the device it was drawn on is unharmed.
#[must_use = "a pending frame's pixels are read by `Device::collect`"]
pub struct PendingFrame {
    state: State,
}

impl std::fmt::Debug for PendingFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self.state {
            State::Drawn(_) => "drawn",
            State::Submitted(_) => "submitted",
        };
        f.debug_struct("PendingFrame")
            .field("state", &state)
            .finish()
    }
}

/// Whether the frame still waits on the device.
enum State {
    /// Complete when submitted: a compute chain's frame, or a zero-size one.
    Drawn(Box<Frame>),
    /// On the device.
    Submitted(Box<Submitted<'static>>),
}

/// What phases 2 and 3 of a frame hand phase 4: a frame drawn already, or one on the device.
pub(super) enum Submission<'t> {
    /// Nothing left to wait for.
    Drawn(Box<Frame>),
    /// The passes, and a readback target's copy, submitted.
    Submitted(Box<Submitted<'t>>),
}

impl Submission<'_> {
    /// Say on the frame whether the frame's atlas was reset after its encode, once the caller
    /// has decided it (ADR 0024).
    pub(super) fn set_repacked(&mut self, repacked: bool) {
        match self {
            Submission::Drawn(frame) => frame.counters.atlas_repacked = repacked,
            Submission::Submitted(submitted) => submitted.counters.atlas_repacked = repacked,
        }
    }
}

/// One submitted frame's state: every figure phases 2 and 3 took, and what phase 4 waits on.
pub(super) struct Submitted<'t> {
    /// The target, which phase 4 presents if it is the surface's.
    pub(super) bound: Bound<'t>,
    /// The frame's pass timestamps, if it took the device's query.
    pub(super) query: Option<PassQuery>,
    /// The content submission, which phase 4 waits on and no later one.
    pub(super) submission: wgpu::SubmissionIndex,
    /// What recording and submitting it cost the host.
    pub(super) submit_host: Duration,
    /// A readback target's copy, and what submitting it cost the host.
    pub(super) copy: Option<(PendingCopy, Duration)>,
    /// Whether the compute chain stamped its queries this frame.
    pub(super) compute_stamped: bool,
    /// The executor's own phases.
    pub(super) phases: FramePhases,
    /// Binding the target.
    pub(super) acquire: Duration,
    /// Phase 1's subdivision.
    pub(super) encode_phases: FramePhases,
    /// Phase 2's spans.
    pub(super) upload_spans: FramePhases,
    /// Phase 1's and phase 2's clocks.
    pub(super) timings: (Duration, Duration),
    /// What the frame counted, read when it was submitted.
    pub(super) counters: Counters,
    /// What the frame reports.
    pub(super) reports: Vec<Report>,
    /// Where its encode came from.
    pub(super) source: EncodeSource,
    /// Held while the frame is pending, so the device knows its query is lent rather than lost.
    pub(super) token: Arc<()>,
}

impl Device {
    /// Submit one frame of `scene` at `viewport`, read back to the processor, and return as soon
    /// as its passes and its copy are on the device (ADR 1595).
    ///
    /// The same frame as [`Device::render`] into [`Target::Readback`] — the same encode, passes,
    /// refusals and pixels — divided where the device starts drawing: [`Device::collect`] waits
    /// for it and reads it. Between the two the caller may walk, encode and submit another frame
    /// on this device; that frame's passes run behind this one's, and nothing it uploads reaches
    /// the device before this frame has drawn.
    ///
    /// A frame whose compute chain stamped the device's compute queries is drawn and read before
    /// this returns, since the next frame's chain writes those queries; its collect costs
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`Device::render`]'s, for every refusal and failure up to the submission.
    pub fn submit(
        &mut self,
        scene: &Scene,
        viewport: &Viewport<'_>,
    ) -> Result<PendingFrame, RenderError> {
        let submission = self.submit_scene(scene, viewport, &Target::Readback)?;
        let state = match submission {
            Submission::Drawn(frame) => State::Drawn(frame),
            Submission::Submitted(submitted) if submitted.compute_stamped => {
                State::Drawn(Box::new(self.complete(*submitted)?))
            }
            Submission::Submitted(submitted) => match into_owned(*submitted) {
                Some(owned) => State::Submitted(Box::new(owned)),
                // A readback target is the frame's own, so this arm is a frame bound to a target
                // `submit` never binds; it is refused by name rather than held.
                None => {
                    return Err(RenderError::ReadbackFailed {
                        detail: "a pending frame was bound to a target it does not own".into(),
                    });
                }
            },
        };
        Ok(PendingFrame { state })
    }

    /// Wait for a frame [`Device::submit`] put on the device, read its target back, and make its
    /// [`Frame`] — [`Device::render`]'s phase 4 (ADR 1595).
    ///
    /// Waits for this frame's own submissions and no later one, so a frame submitted after it
    /// is still drawing when this returns.
    ///
    /// # Errors
    ///
    /// [`Device::render`]'s phase-4 failures: a lost device, or a readback that did not map.
    pub fn collect(&mut self, pending: PendingFrame) -> Result<Frame, RenderError> {
        match pending.state {
            State::Drawn(frame) => Ok(*frame),
            State::Submitted(submitted) => self.complete(*submitted),
        }
    }

    /// Phase 4: wait for the frame's passes, present or read back, and read its timestamps.
    ///
    /// The order is the person's before the instrument's: a surface frame is presented the
    /// moment its passes are done, and the numbers arrive a map later.
    pub(super) fn complete(&mut self, submitted: Submitted<'_>) -> Result<Frame, RenderError> {
        let Submitted {
            bound,
            query,
            submission,
            submit_host,
            copy,
            compute_stamped,
            mut phases,
            acquire,
            encode_phases,
            upload_spans,
            timings: (encode, upload),
            counters,
            reports,
            source,
            token,
        } = submitted;
        // The frame is no longer pending once it is being completed: the query it holds goes
        // back below, or is dropped with a failed read, and either is this call's to decide.
        drop(token);
        let waited = Instant::now();
        if let Err(error) = readback::wait_for(&self.gpu, &submission) {
            return Err(self.abandon_frame(bound, error));
        }
        // What the host spent on the submission: recording and submitting it, then waiting
        // for it — the wall a frame without timestamps reports as its `execute`.
        let execute_wall = submit_host.saturating_add(waited.elapsed());

        let present_started = Instant::now();
        if let Bound::Acquired(surface_texture) = bound {
            self.queue.present(surface_texture);
        }
        phases.push(("target acquire", acquire));
        phases.push(("present", present_started.elapsed()));
        phases.extend(encode_phases);
        phases.extend(upload_spans);
        let (execute, provenance) = timing::read_pass(
            &self.gpu,
            self.timestamps,
            query.as_ref(),
            execute_wall,
            ("content pass", &submission),
            &mut phases,
        )?;
        // The compute lane's own device time, invisible to the frame's one query
        // because its dispatches run in submissions of their own before the content
        // pass — the bulk of what the caller's ADR 0084 could only call
        // "unattributed". Read only on a frame the lane stamped: an unstamped frame's
        // buffers hold an older frame's ticks.
        if compute_stamped && let Some(q) = self.compute_queries.as_ref() {
            timing::read_pass(
                &self.gpu,
                self.timestamps,
                Some(&q.count),
                Duration::ZERO,
                ("compute count pass", &submission),
                &mut phases,
            )?;
            timing::read_pass(
                &self.gpu,
                self.timestamps,
                Some(&q.coverage),
                Duration::ZERO,
                ("compute emit+deposit", &submission),
                &mut phases,
            )?;
        }
        if provenance == TimingProvenance::TimestampQueries {
            // What the content submission cost beyond its own pass: recording, submit,
            // and the wait — host-side, and until now folded silently into the wall.
            phases.push(("content beyond pass", execute_wall.saturating_sub(execute)));
        }
        // Read, so the buffers are unmapped and the set is the next frame's to use.
        // Reached only on the `?` above succeeding, which is the whole condition: a
        // query whose read failed is dropped here instead, and the frame after it makes
        // a fresh one. A frame drawn without one, beside a pending frame that held it,
        // gives nothing back.
        if let Some(query) = query {
            self.pass_query = Some(query);
        }

        // Phase 4's price, which only a readback target pays (brief section 6.1, ADR 0022):
        // submitting the copy, then waiting for it, mapping it and converting it.
        let (payload, readback) = match copy {
            Some((copy, copy_host)) => {
                let started = Instant::now();
                let raster = copy.collect(&self.gpu)?;
                (
                    Payload::Raster(raster),
                    copy_host.saturating_add(started.elapsed()),
                )
            }
            None => (Payload::None, Duration::ZERO),
        };

        Ok(Frame {
            timings: Timings {
                encode,
                upload,
                execute,
                readback,
                execute_provenance: provenance,
                phases,
            },
            counters,
            reports,
            payload,
            encode_source: source,
        })
    }

    /// This frame's timestamp query, taken out of the device for the duration, and `None`
    /// where the adapter has no timestamps to take — or where a pending frame holds the
    /// device's one, and then this frame reports its `execute` by the wall (ADR 1595).
    ///
    /// **Taken rather than borrowed**, because the frame it belongs to needs `&mut self`
    /// for everything else it does; and taken rather than made, because making one costs
    /// **2.43 ms on a device's first frame** — a `QuerySet` and two sixteen-byte buffers,
    /// which the driver charges for once and then hands back from a pool. That was a
    /// fifth of the eleven milliseconds a first frame pays over its successors
    /// (`QUORRA_FEEDBACK.md` section 9), spent on an instrument rather than on the page.
    ///
    /// It goes back at the end of a frame that read it, and does not after one that
    /// could not: a map that failed may leave the buffer mapped, and the next frame's
    /// `map_async` on it would be a validation error rather than a number.
    pub(super) fn take_pass_query(&mut self) -> Option<PassQuery> {
        self.timestamps?;
        if let Some(query) = self.pass_query.take() {
            return Some(query);
        }
        // Absent: lent to a pending frame, which gives it back when collected, or lost to a
        // failed read, when a fresh one is made. Only the device's own handle on the token
        // means nobody holds it.
        (Arc::strong_count(&self.pending) == 1).then(|| PassQuery::new(&self.gpu))
    }
}

/// A submitted frame whose target it owns, as one free of the caller's borrow — which is every
/// readback frame; `None` for a frame bound to a borrowed texture or the surface.
fn into_owned(submitted: Submitted<'_>) -> Option<Submitted<'static>> {
    let Submitted {
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
        timings,
        counters,
        reports,
        source,
        token,
    } = submitted;
    let Bound::Owned(texture) = bound else {
        return None;
    };
    Some(Submitted {
        bound: Bound::Owned(texture),
        query,
        submission,
        submit_host,
        copy,
        compute_stamped,
        phases,
        acquire,
        encode_phases,
        upload_spans,
        timings,
        counters,
        reports,
        source,
        token,
    })
}
