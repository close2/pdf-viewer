//! How this window gets pixels: which surface is behind it, and what one frame does.
//!
//! Two paths and never both — a graphics device or the processor's own surface — and the whole of
//! the difference is in [`Surface`]. Bringing one up is the launch path's last step and is
//! therefore measured; drawing on it is the frame, and the frame is where every other module's
//! work arrives, the page from `viewer-core` and the overlays from this host.
//!
//! **A frame is two things happening at once on both of them**, which is ADR 0391 for the window
//! with a device and ADR 0461 for the one without: [`crate::renderer`] and [`crate::composer`] draw
//! pages on a thread of their own and this module presents, on the clock, whatever that thread has
//! finished — moved to where the view now is where it is not of the view now being asked for. What
//! one call to [`App::present`] does is therefore *adopt, ask, place* rather than *draw and wait*,
//! and the whole of the difference to how long a person waits for a window to answer them is in
//! that.
//!
//! So the two paths differ in their rasteriser and in **one** thing about the policy: what a
//! stand-in costs the thread that presents ([`crate::stale::Standing`]). Everything else —
//! `doc/todo/37`'s five rules, the retained pages, the clock — is one implementation asked twice.

use std::sync::Arc;

use pdf_render::{TargetSpec, Transform};
use viewer_core::{Command, Rendered};
use winit::window::Window;

use crate::app::App;
use crate::timing::Stages;

#[path = "surface/bring_up.rs"]
mod bring_up;
#[path = "surface/device_adoption.rs"]
mod device_adoption;
#[path = "surface/device_frame.rs"]
mod device_frame;
#[path = "surface/processor_frame.rs"]
mod processor_frame;
#[path = "surface/settling.rs"]
mod settling;

/// What the window predicted for a frame, as a person reads it (ADRs 1831, 1886).
fn predicted(chose: &crate::renderer::Prediction) -> String {
    chose.said()
}

/// How this window's pixels reach it: with a graphics device, or without one.
///
/// **Never both, and that is the point.** A process holding [`Surface::Processor`] has created no
/// `wgpu::Instance`, selected no adapter and made no device, so a driver that faults while it loads
/// cannot reach it (ADR 0221).
///
/// **Both variants are now the same arrangement over two rasterisers** (ADR 0461): a thread that
/// draws pages, a store of finished pictures on this thread, and a present on the clock's tick.
/// What they do not share is the price of a stand-in, which is [`crate::stale::Standing`].
pub(crate) enum Surface {
    /// The window's surface, held apart from the device that made it (raster's ADR 0056): this
    /// thread presents finished rasters and [`crate::renderer`]'s thread draws them.
    Device(Box<crate::renderer::Window>),
    /// The processor's raster copied onto the window, with the overlays composited into it
    /// first. `--cpu`, and a device that would not come up.
    ///
    /// **It carries the last frame's pixels and the retained pages beside the surface**, which the
    /// device path's variant keeps in `crate::renderer` for the same reason: the base is the
    /// window's own picture, chrome excluded, held wherever it already exists.
    Processor(Box<crate::composer::Composer>),
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(window) => formatter.debug_tuple("Device").field(window).finish(),
            Self::Processor(composer) => {
                formatter.debug_tuple("Processor").field(composer).finish()
            }
        }
    }
}

/// The window, and whatever puts pixels on it.
pub(crate) struct State {
    pub(crate) window: Arc<Window>,
    /// The graphics device's surface, or the software one. See [`Surface`].
    pub(crate) surface: Surface,
    /// The surface size in device pixels, updated on `WindowEvent::Resized`.
    pub(crate) size: (u32, u32),
}

impl App {
    /// The presenting half of this window, where there is one.
    ///
    /// A method rather than a `let` at each site because every one of them has to end the borrow
    /// before touching another field of `App`: the clock and the stand-in policy are fields
    /// beside `state`, and a window held across a call to either is a borrow the compiler is
    /// right to refuse.
    pub(crate) fn device_window(&mut self) -> Option<&mut crate::renderer::Window> {
        match self.state.as_mut()?.surface {
            Surface::Device(ref mut window) => Some(window),
            Surface::Processor(_) => None,
        }
    }

    /// [`Self::device_window`] to look at rather than to ask of — [`Self::composer_ref`]'s reason:
    /// [`App::mind_a_long_draw`] asks the one frame on this window that can be stopped whether it
    /// is overlong (ADR 1821).
    pub(crate) fn device_window_ref(&self) -> Option<&crate::renderer::Window> {
        match self.state.as_ref()?.surface {
            Surface::Device(ref window) => Some(window),
            Surface::Processor(_) => None,
        }
    }

    /// The presenting half of this window where the processor is what draws on it.
    ///
    /// The same shape as [`Self::device_window`] and for the same borrow reason: the clock and the
    /// stand-in policy are fields beside `state`, and a composer held across a call to either is a
    /// borrow the compiler is right to refuse.
    pub(crate) fn composer(&mut self) -> Option<&mut crate::composer::Composer> {
        match self.state.as_mut()?.surface {
            Surface::Processor(ref mut composer) => Some(composer),
            Surface::Device(_) => None,
        }
    }

    /// The same thing to look at rather than to ask of.
    ///
    /// A second method rather than a borrow of the first, because the two callers outside this
    /// module differ exactly there: [`App::mind_a_long_draw`] reads a clock and
    /// [`App::stop_the_long_draw`] raises an interrupt, and a `&mut` taken to ask a question is a
    /// borrow the rest of `App` then cannot have.
    pub(crate) fn composer_ref(&self) -> Option<&crate::composer::Composer> {
        match self.state.as_ref()?.surface {
            Surface::Processor(ref composer) => Some(composer),
            Surface::Device(_) => None,
        }
    }

    /// Draws the frame the window asked for, and tells the core what became of it.
    ///
    /// **The frame's number is the frame's** (ADR 0227). The timer is read before the launch
    /// timeline closes and the accessibility tree is published, so `present -> presented in T` is
    /// not the frame plus the bridge plus the timeline's own printing: on a page turn the tree is
    /// rebuilt and published, and that work is not the graphics device's.
    pub(crate) fn redraw_requested(&mut self) {
        // **The clock's gate, and it is on every frame rather than only on the ones that follow
        // a reprojection** (`doc/todo/36`). A redraw that arrives before the surface has
        // refreshed would be overdrawn before anybody saw it, so it is deferred to the tick
        // instead — which is what stops the window running at whatever rate the *input* device
        // happens to deliver. Nothing is lost: `about_to_wait` sees the obligation and asks
        // again, and a window that has been still is due at once, so the first frame after an
        // input is not held back by so much as one refresh. The question is whether this refresh
        // has had its present, not whether its tick has come: a tick that put nothing up leaves
        // it open for the frame the render thread wakes this loop with (ADR 1926).
        let now = std::time::Instant::now();
        if !self.cadence.may_present(now) {
            self.cadence.owed(now);
            return;
        }
        self.cadence.serviced();
        // Before the frame rather than after it: a tick can advance the page, and a page that
        // advanced after its frame was drawn would be one frame late for the whole slide show.
        self.drive_the_clock();
        let started = std::time::Instant::now();
        let mut stages = Stages::default();
        let outcome = self.present(&mut stages);
        stages.total = started.elapsed();
        if matches!(outcome, Some(Rendered::Presented | Rendered::Raster(_))) {
            // The command line's later documents, and not one frame sooner (`CLAUDE.md` section
            // 2).
            if !self.launched {
                self.launched = true;
                self.arrival_due = true;
            }
            self.launch.arrived(self.trace, &stages);
            // **After the timeline is closed, never before it.** Everything the accessibility
            // bridge does is off the launch path by construction, and this line is where that is
            // enforced rather than merely intended.
            let attending = std::time::Instant::now();
            self.attend();
            stages.attend = attending.elapsed();
        }
        // Rule 1's other half: *every* frame that is not a reprojection clears the flag, a frame
        // that drew nothing included. A flag that could outlive the redraw answering it would
        // leave `about_to_wait` asking for a frame that never comes, which is a spinning loop.
        if stages.approximated.is_none() {
            self.stale.real();
        }
        // What the window actually put up, which is what `doc/todo/36`'s rule 6 counts: a
        // rendering and a reprojection are both presents and a frame that drew nothing is not
        // one. The clock is moved on by exactly these, so an idle window — which produces none —
        // never advances it and never wakes for it.
        stages.presented = stages.approximated.is_some()
            || matches!(outcome, Some(Rendered::Presented | Rendered::Raster(_)));
        if stages.presented {
            self.cadence.presented(std::time::Instant::now());
            self.settle_cadence();
        }
        let outcome_said = match (&outcome, stages.approximated) {
            (None, Some(from)) => from.word().to_owned(),
            (None, None) => "nothing to show".to_owned(),
            (Some(Rendered::Presented), _) => "presented".to_owned(),
            (Some(Rendered::Failed(why)), _) => format!("failed: {why}"),
            (Some(Rendered::Raster(_)), _) => "a raster".to_owned(),
            // Neither of the last two is this host's, and both are written out rather than
            // swept into a catch-all, which is what nothing in `viewer-core` being
            // `#[non_exhaustive]` is for: this window presents on its own device, so it never
            // hands back pixels and never keeps a page's list for itself.
            (Some(Rendered::Listed), _) => "a list the host kept".to_owned(),
        };
        let trace = self.trace;
        self.frames.frame(trace, &stages, &outcome_said);
        self.pipelines_compiled();
        let Some(rendered) = outcome else {
            return;
        };
        // **One acknowledgement per page of the arrangement**, because the core holds one
        // outstanding request per page and a page never answered for is a page it goes on
        // believing is not yet on the screen. A token the core has since superseded drops itself,
        // which is the whole reason a token exists.
        //
        // `Rendered` is deliberately not `Clone` — tier 1's variant carries a whole page of
        // pixels — so what is repeated is the *answer* rather than the value. This host is tier 2
        // and the only two it can produce are a present and a refusal; `Rendered::Raster` is tier
        // 1's and `Rendered::Listed` belongs to a host that keeps a page's marks rather than
        // drawing them, and nothing here builds either.
        let refused = match &rendered {
            Rendered::Failed(why) => Some(why.clone()),
            Rendered::Presented | Rendered::Raster(_) | Rendered::Listed => None,
        };
        for token in std::mem::take(&mut self.unacknowledged) {
            let rendered = refused
                .clone()
                .map_or(Rendered::Presented, Rendered::Failed);
            self.dispatch(Command::RenderReady { token, rendered });
        }
        // The reader has their page, so the document may now be asked what it says about itself
        // — §12.8's signatures above all, whose answer reads and digests the signed part of the
        // file. Once per opened document, after the acknowledgements rather than among them, and
        // the rule is `viewer_host::report::Due`'s so that four hosts ask alike. ADR 1044.
        if self.report_due.after_a_frame() {
            self.dispatch(Command::Report);
            // And its open sequence, after the first present and never in front of it (ADR 1602).
            self.dispatch(Command::Presented);
        }
        // Table 147's `/FitWindow` and `/CenterWindow` are about the first displayed page's size,
        // which is known now and not before (ADR 1429).
        if self.presenting.owes_placing() {
            self.place_the_window();
        }
    }
}

/// A page's target as the core asks for it, placed in this window: the window's extent, with the
/// page's placement — the core's origin and the panel's edge — composed into its transform.
///
/// One function for the frame's arrangement and for page one drawn while it is read (ADR 1855), so
/// that the target the page is drawn ahead at and the one its frame is asked at are one arithmetic.
pub(crate) fn in_the_window(
    page: TargetSpec,
    origin: (f32, f32),
    edge: f32,
    (width, height): (u32, u32),
) -> TargetSpec {
    TargetSpec {
        width,
        height,
        transform: page
            .transform
            .then(Transform::translate(origin.0 + edge, origin.1)),
    }
}

/// The render thread's [`viewer_device::Wake`] as this window's event loop takes it: a user event,
/// which a loop that has exited does not want and nobody is told about (ADR 1926).
pub(crate) fn wake_on(proxy: winit::event_loop::EventLoopProxy<()>) -> viewer_device::Wake {
    viewer_device::Wake::new(move || {
        let _ = proxy.send_event(());
    })
}
