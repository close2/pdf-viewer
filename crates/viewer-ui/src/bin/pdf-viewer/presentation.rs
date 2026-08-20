//! §12.4.4's presentation: the clock this host drives, and the transition it draws with it.
//!
//! `viewer-core` has no clock by rule 3 and no presentation *state* by ADR 0135, so "is a
//! presentation running" is answered by whether something is ticking — which makes the whole
//! mode a field and a key in this module. What the core supplies is the *shape* of a transition
//! frame; when to draw it, and out of which two pages, is a wall-clock question and therefore a
//! host's.

use std::sync::Arc;

use pdf_render::{Image, Point, Raster, Rasterizer as _, Rect, TargetSpec, Transform};
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, PresentationMode, Query, RenderRequest};

use crate::app::App;
use crate::trace::Topic;

/// How often the clock is offered to `viewer-core` while a presentation is running and nothing
/// is being animated.
///
/// §12.4.4.1's `/Dur` is "in seconds", and `Command::Tick` carries the milliseconds that actually
/// passed rather than an assumed step, so the only thing this decides is *when the advance is
/// noticed* — a tenth of a second against a duration a document states in whole ones. Ten times a
/// second rather than every frame, because between transitions a slide show has nothing to draw
/// and a window that redrew anyway would spend a processor on a still page.
pub(crate) const PRESENTATION_TICK: std::time::Duration = std::time::Duration::from_millis(100);

/// §12.4.4's presentation, while this window is driving one.
///
/// **The existence of this value is what "presentation mode" means here**, which is ADR 0135's
/// answer rather than this round's: `viewer-core` has no clock and no mode, so "is a presentation
/// running" is answered by whether a host is ticking. `p` starts and stops it.
pub(crate) struct Presentation {
    /// When the clock was last read, so that a tick carries the milliseconds that really passed.
    ticked: std::time::Instant,
    /// When the next tick is due, which is what the event loop waits until.
    pub(crate) wake: std::time::Instant,
    /// The transition being drawn, where one is in flight.
    pub(crate) playing: Option<Playing>,
}

/// One transition, mid-flight: what it is, when it began, and the two pages it is between.
pub(crate) struct Playing {
    /// Table 164's style, duration and direction, as the page arrived at states them.
    transition: pdf_model::navigation::Transition,
    /// When the first frame of it was drawn.
    began: std::time::Instant,
    /// The page being left, as it was last presented.
    outgoing: Image,
    /// The page being moved to, at the same size.
    incoming: Image,
}

/// The far corner of a window, as a point in its own device pixels.
#[expect(
    clippy::cast_precision_loss,
    reason = "window dimensions are far below f32's exact integer range"
)]
fn whole(width: u32, height: u32) -> Point {
    Point::new(width as f32, height as f32)
}

/// One page of a §12.4.4 transition: the display list drawn to pixels, ready to be drawn again.
///
/// [`CpuRasterizer`] and not the graphics device, because this asks for *pixels* and a presenter
/// answers only by presenting. It happens twice per transition rather than per frame.
fn face(list: &pdf_render::DisplayList, target: TargetSpec) -> Option<Image> {
    let raster: Raster = CpuRasterizer::new().rasterize(list, target).ok()?;
    viewer_core::transition::drawable(&raster)
}

impl App {
    /// Enters or leaves §12.4.4's presentation mode: the clock, and the mode the core keeps.
    ///
    /// **Two things now, where ADR 0135 had one.** That session decided `viewer-core` had no
    /// presentation *state* — "is a presentation running" was answered by whether something was
    /// driving the clock — and ADR 0316 amended it on §12.4.4.2, which conditions a state machine
    /// on the mode itself: NOTE 3 respects the navigation nodes "only when in presentation mode",
    /// and a person stepping through a slide show by hand drives no clock at all. So the key sends
    /// `Command::Present` as well, and what the core does with it is the nodes, the groups NOTE 2
    /// asks to be saved, and the `/Trans` of a page turned to by hand.
    ///
    /// Full screen is deliberately still not part of it. §12.4.4.1 says a processor "may allow a
    /// document to be displayed in the form of a presentation or slide show" and says nothing
    /// about a window; what the clause states is the advance timing, the transition and the
    /// states, and those are what this drives.
    pub(crate) fn present_or_stop(&mut self) {
        if self.presentation.take().is_some() {
            self.dispatch(Command::Present(PresentationMode::Off));
            println!("presentation: stopped — no clock is being driven");
            self.redraw();
            return;
        }
        let now = std::time::Instant::now();
        self.presentation = Some(Presentation {
            ticked: now,
            wake: now,
            playing: None,
        });
        self.dispatch(Command::Present(PresentationMode::On));
        println!(
            "presentation: running — §12.4.4's /Dur advances the page, its /Trans is drawn, and \
             the arrow keys walk §12.4.4.2's states before they turn the page"
        );
        self.redraw();
    }

    /// Tells the core how long the page has been up, where a presentation is running.
    ///
    /// **The clock is held while a transition is being drawn**, and that is the clause's own
    /// arithmetic rather than a convenience: §12.4.4.1's EXAMPLE describes "a page to be
    /// displayed for 5 seconds" whose 3.5-second transition happens "[b]efore the page is
    /// displayed", so the transition is not part of the display duration it precedes. A tick
    /// during it would spend the new page's `/Dur` on the effect that introduces it.
    pub(crate) fn drive_the_clock(&mut self) {
        let Some(presentation) = self.presentation.as_mut() else {
            return;
        };
        if presentation.playing.is_some() {
            presentation.ticked = std::time::Instant::now();
            return;
        }
        let now = std::time::Instant::now();
        let elapsed = now.saturating_duration_since(presentation.ticked);
        presentation.ticked = now;
        let millis = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
        if millis == 0 {
            return;
        }
        self.dispatch(Command::Tick { millis });
    }

    /// Takes `transition` to be drawn as soon as there is a page to draw it to, or says why this
    /// program will not draw it at all.
    ///
    /// Two rasters are taken per transition and none per frame: the page being left, as it was
    /// last presented, and the page arriving, at the same size. Both are drawn by
    /// [`CpuRasterizer`] — the one rasteriser this host can ask for *pixels* rather than for a
    /// present — and each crosses to the graphics device once, because [`pdf_render::Image`]
    /// holds its samples behind an `Arc` and quorra's caches are keyed by that pointer.
    ///
    /// The cost is therefore two page renders at the start of a transition and two image draws
    /// per frame after it, which is the trade this host makes deliberately: a transition frame
    /// that re-rasterised both pages would pay a page's interpretation sixty times a second.
    pub(crate) fn arm_transition(&mut self, transition: pdf_model::navigation::Transition) {
        let Some((width, height, _)) = self.window() else {
            return;
        };
        // A window that is not presenting draws the page, which is the transition's end state.
        if self.presentation.is_none() {
            println!(
                "note: transition: {:?} over {} s — nothing is presenting, so the page is shown \
                 at once (press p)",
                transition.style, transition.duration
            );
            return;
        }
        let viewport = Rect::from_corners(Point::new(0.0, 0.0), whole(width, height));
        if viewer_core::transition::frame(&transition, viewport, 0.0).is_none() {
            // The core has already said *why* through `Event::Reported`; a second sentence here
            // would say it twice.
            return;
        }
        // **Armed rather than begun, and the ordering is the reason.** `Viewer::handle` settles
        // *after* the command that turned the page, so the events arrive as page change,
        // transition, render request — and the arriving page's display list is in the last of
        // the three. A transition begun here would have rasterised the page being left twice
        // and animated it against itself, which is what the window showed before this was found.
        // §12.4.4.1's transition is one *to* a page, so waiting for that page's own request is
        // the clause's own order as well as this host's.
        self.arming = Some(transition);
    }

    /// Draws the armed transition between the page last presented and the one just asked for.
    ///
    /// Two rasters are taken here and none per frame: see [`App::arm_transition`] for why this
    /// is not the moment the event arrived.
    pub(crate) fn begin_transition(
        &mut self,
        request: &RenderRequest,
        transition: pdf_model::navigation::Transition,
    ) {
        let began = std::time::Instant::now();
        let Some((width, height, _)) = self.window() else {
            return;
        };
        let Some((list, target)) = self.presented.clone() else {
            return;
        };
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        // The same composition `present` makes, because the page arriving has to be drawn where
        // it is about to be presented — otherwise the last frame of the transition would move.
        let arriving = TargetSpec {
            width,
            height,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0 + edge, origin.1)),
        };
        let (Some(outgoing), Some(incoming)) = (face(&list, target), face(&request.list, arriving))
        else {
            println!(
                "note: transition: {:?} was named but the pages behind it would not rasterise, \
                 so the page is shown at once",
                transition.style
            );
            return;
        };
        // The cost of starting one, which is where all of a transition's page work is: a frame
        // after this draws two images and interprets nothing.
        self.trace.say(
            Topic::Frames,
            format_args!(
                "TRANSITION {:?} over {} s: two {width}x{height} pages rasterised in {:?}",
                transition.style,
                transition.duration,
                began.elapsed()
            ),
        );
        if let Some(presentation) = self.presentation.as_mut() {
            presentation.playing = Some(Playing {
                transition,
                began: std::time::Instant::now(),
                outgoing,
                incoming,
            });
        }
        self.redraw();
    }

    /// The frame of a transition in flight, or `None` when there is nothing being drawn.
    ///
    /// Ends the transition at `/D` seconds, which is the one place this host decides that time
    /// has run out: the fraction handed to `viewer_core::transition::frame` is elapsed over
    /// duration, linear, because Table 164 states a duration and no curve.
    fn transition_frame(&mut self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let presentation = self.presentation.as_mut()?;
        let playing = presentation.playing.as_ref()?;
        let elapsed = playing.began.elapsed().as_secs_f32();
        let progress = if playing.transition.duration > 0.0 {
            elapsed / playing.transition.duration
        } else {
            1.0
        };
        if progress >= 1.0 {
            presentation.playing = None;
            // The clock restarts where the transition ends, so the page gets the whole of its
            // own `/Dur` — see `drive_the_clock`.
            presentation.ticked = std::time::Instant::now();
            return None;
        }
        let viewport = Rect::from_corners(Point::new(0.0, 0.0), whole(width, height));
        let shaped = viewer_core::transition::frame(&playing.transition, viewport, progress)?;
        match shaped.draw(viewport, &playing.outgoing, &playing.incoming) {
            Ok(list) => Some(list),
            Err(problem) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                println!("note: transition: this frame would not draw: {problem}");
                presentation.playing = None;
                None
            }
        }
    }

    /// What this frame draws: a transition's own picture, or the page itself.
    ///
    /// A transition frame is already in the window's pixels — it is two page rasters placed by
    /// [`viewer_core::transition`] — so it draws at the identity transform where a page draws
    /// through its own placement.
    /// **The frame is handed over in an `Arc` because that is the identity the presenter reuses
    /// a scene by** (ADR 0351): `render_quorra::PresentFrame::page` pins what it is given, so a
    /// display list drawn once and dropped cannot have its address recycled under the entry
    /// keyed on it. A transition frame is a fresh list on every frame of the animation, so each
    /// one is a fresh `Arc` and each one rebuilds — which is what a moving picture is.
    pub(crate) fn frame_to_draw(
        &mut self,
        list: &Arc<pdf_render::DisplayList>,
        target: TargetSpec,
        width: u32,
        height: u32,
    ) -> (Option<Arc<pdf_render::DisplayList>>, TargetSpec) {
        if let Some(frame) = self.transition_frame(width, height) {
            return (
                Some(Arc::new(frame)),
                TargetSpec {
                    width,
                    height,
                    transform: Transform::IDENTITY,
                },
            );
        }
        // What the screen is about to show, kept so that the next transition has a page to move
        // *from*. Only the page itself: a transition frame is already a picture of two of them.
        // The **first** page of Table 29's arrangement, because §12.4.4's presentation mode shows
        // one page at a time and a transition is a picture between two of those.
        self.presented = Some((Arc::clone(list), target));
        (None, target)
    }
}
