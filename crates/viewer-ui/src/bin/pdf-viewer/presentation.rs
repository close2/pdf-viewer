//! §12.4.4's presentation: the window, the clock this host drives, and the transition it draws.
//!
//! `viewer-core` has no clock by rule 3, so what the core supplies is the *shape* of a transition
//! frame; when to draw it, and out of which two pages, is a wall-clock question and therefore a
//! host's. Since ADR 0316 the core does keep the presentation **mode**, because §12.4.4.2
//! conditions a state machine on it.
//!
//! **And since ADR 0470 there is a window.** Table 29's `/PageMode /FullScreen` is "[f]ull-screen
//! mode, with no menu bar, window controls, or any other window visible", §12.2's three hide flags
//! are the same subject in the smaller, and `/NonFullScreenPageMode` is the way back out —
//! `viewer_host::Presenting` is all four sentences, shared with the other two hosts, and what is
//! this host's is `winit`'s `set_fullscreen` and which key asks for it.

use std::sync::Arc;

use pdf_render::{Image, Point, Raster, Rasterizer as _, Rect, TargetSpec, Transform};
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, PresentationMode, Query, RenderRequest};

use crate::app::App;
use crate::trace::Topic;

/// §12.4.4's presentation, while this window is driving one.
///
/// **The existence of this value is what "presentation mode" means here**, which is ADR 0135's
/// answer rather than this round's: `viewer-core` has no clock and no mode, so "is a presentation
/// running" is answered by whether a host is ticking. `p` starts and stops it.
///
/// **What a tick means and how long a transition has left are [`viewer_host::Clock`]'s** since
/// the two native hosts were given a clock of their own: the four questions §12.4.4.1 asks —
/// how often to look at a wall clock, what a tick carries, whether the clock runs during a
/// transition, and when Table 164's `/D` has run out — have the same answer in all three hosts
/// and a different spelling in each event loop. What is left here is winit's.
pub(crate) struct Presentation {
    /// §12.4.4.1's clock, shared with `viewer-gtk` and `viewer-qt`.
    pub(crate) clock: viewer_host::Clock,
    /// When the next tick is due, which is what the event loop waits until.
    pub(crate) wake: std::time::Instant,
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
    /// Enters or leaves §12.4.4's presentation: the window, the clock, and the mode the core keeps.
    ///
    /// **Three things now, where ADR 0135 had one and ADR 0316 had two.** That first session
    /// decided `viewer-core` had no presentation *state* — "is a presentation running" was
    /// answered by whether something was driving the clock — and ADR 0316 amended it on §12.4.4.2,
    /// which conditions a state machine on the mode itself: NOTE 3 respects the navigation nodes
    /// "only when in presentation mode", and a person stepping through a slide show by hand drives
    /// no clock at all. So the key sends `Command::Present` as well, and what the core does with
    /// it is the nodes, the groups NOTE 2 asks to be saved, and the `/Trans` of a page turned to
    /// by hand.
    ///
    /// **And the third is the window** (ADR 0470). This doc comment used to say full screen was
    /// "deliberately still not part of it" because "§12.4.4.1 … says nothing about a window", and
    /// that was true of §12.4.4 and false of the standard: Table 29 names `FullScreen` — "with no
    /// menu bar, window controls, or any other window visible" — and §12.2 states both the
    /// smaller flags and the page mode to come back to. That the two acts are *one* key is this
    /// program's documented choice, argued in [`viewer_host::presentation`].
    pub(crate) fn present_or_stop(&mut self) {
        if self.presenting.toggle() {
            self.begin_presentation();
        } else {
            self.leave_full_screen();
        }
    }

    /// Starts the presentation the window is already in full screen for.
    ///
    /// Separate from [`App::present_or_stop`] because a document may ask for it: Table 29's
    /// `/PageMode /FullScreen` is a statement about how the document opens, so this runs with no
    /// key pressed and with [`viewer_host::Presenting`] already saying full screen.
    pub(crate) fn begin_presentation(&mut self) {
        if self.presentation.is_some() {
            return;
        }
        let now = std::time::Instant::now();
        self.presentation = Some(Presentation {
            clock: viewer_host::Clock::started(now),
            wake: now,
        });
        self.dispatch(Command::Present(PresentationMode::On));
        self.apply_chrome();
        println!(
            "presentation: running full screen (§7.7.2's FullScreen page mode) — §12.4.4's /Dur \
             advances the page, its /Trans is drawn, the arrow keys walk §12.4.4.2's states \
             before they turn the page, and Escape comes back"
        );
    }

    /// Leaves full screen and §12.4.4's mode with it, putting back what §12.2 says to.
    ///
    /// Answers whether there was a presentation to leave, because Escape is bound to several
    /// things in this window and the one that took the key has to say so.
    pub(crate) fn leave_full_screen(&mut self) -> bool {
        if !self.presenting.leave() && self.presentation.is_none() {
            return false;
        }
        self.presentation = None;
        self.dispatch(Command::Present(PresentationMode::Off));
        // §12.2: "[t]he document's page mode, specifying how to display the document on exiting
        // full-screen mode". `None` is Table 147's own condition unmet — the catalog did not ask
        // to open full screen — and then what goes back is what the reader had, which is a choice
        // rather than a reading (ADR 0470).
        if let Some(mode) = self.presenting.on_exit() {
            println!("presentation: stopped — §12.2's /NonFullScreenPageMode asks for {mode:?}");
            self.show_page_mode(mode);
        } else {
            println!("presentation: stopped — no clock is being driven");
        }
        self.apply_chrome();
        true
    }

    /// Puts the window in the state [`viewer_host::Presenting`] says the document asked for.
    ///
    /// Three of [`viewer_host::Chrome`]'s four fields have nothing to hide in this host and one
    /// has three: winit draws no menu bar, no tool bar and no scroll bars, so what Table 29's "any
    /// other window visible" reaches here is the sidebar, the find bar and the About card. The
    /// three it cannot obey are said out loud once by [`App::report_unobeyable_chrome`] rather
    /// than dropped, which is trap 5 in an interface.
    pub(crate) fn apply_chrome(&mut self) {
        let chrome = self.presenting.chrome();
        if let Some(state) = self.state.as_ref() {
            state.window.set_fullscreen(
                self.presenting
                    .full_screen()
                    .then(|| winit::window::Fullscreen::Borderless(None)),
            );
        }
        if !chrome.other_windows {
            self.panel.shown = false;
            self.about.shown = false;
            if self.find.shown {
                self.find.toggle();
                self.pages_left = 0;
                self.dispatch(Command::Find(viewer_core::Find::Stop));
            }
        }
        // The page's viewport is the window less the panel, so a panel that appeared or went is a
        // resize as far as the core is concerned — and this also redraws.
        self.resize_page();
    }

    /// Says which of §12.2's flags this window has no chrome to obey.
    ///
    /// Once, when the document opens, and only for what it actually asked for: a document that
    /// says nothing gets no line. Table 147's three are a tool bar, a menu bar and the window's
    /// own scroll bars and navigation controls, and this host draws none of the three — which is
    /// a fact about winit rather than about the reading, and the other two hosts obey all three.
    pub(crate) fn report_unobeyable_chrome(&self) {
        let Answer::Preferences(preferences) = self.viewer.query(Query::Preferences) else {
            return;
        };
        let asked: Vec<&str> = [
            (preferences.hide_toolbar, "/HideToolbar"),
            (preferences.hide_menubar, "/HideMenubar"),
            (preferences.hide_window_ui, "/HideWindowUI"),
        ]
        .into_iter()
        .filter_map(|(stated, name)| stated.then_some(name))
        .collect();
        if !asked.is_empty() {
            println!(
                "note: this document asks for {} (§12.2) — this window has no menu bar, tool bar \
                 or scroll bars of its own to hide; the GTK and Qt hosts obey all three",
                asked.join(", ")
            );
        }
    }

    /// Tells the core how long the page has been up, where a presentation is running.
    ///
    /// The arithmetic — including the rule that the clock is *held* while a transition is being
    /// drawn, which is §12.4.4.1's EXAMPLE rather than a convenience — is
    /// [`viewer_host::Clock::tick`]'s, shared with the other two hosts. What is this host's is
    /// when the loop comes round to ask.
    pub(crate) fn drive_the_clock(&mut self) {
        let now = std::time::Instant::now();
        let Some(millis) = self
            .presentation
            .as_mut()
            .and_then(|presentation| presentation.clock.tick(now))
        else {
            return;
        };
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
        if !viewer_host::Clock::shapes(&transition, viewport) {
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
        // `viewer_host::face_target` is that composition, shared with the two native hosts so
        // that three windows cannot disagree about where a transition leaves the page.
        let arriving =
            viewer_host::face_target(request.target, (origin.0 + edge, origin.1), (width, height));
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
            presentation
                .clock
                .begin(transition, outgoing, incoming, std::time::Instant::now());
        }
        self.redraw();
    }

    /// The frame of a transition in flight, or `None` when there is nothing being drawn.
    ///
    /// Table 164's `/D` decides when there is nothing left to draw, and so does the clock that
    /// restarts on the arriving page when there is not — both inside [`viewer_host::Clock`],
    /// which is where the other two hosts read the same two rules.
    fn transition_frame(&mut self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let viewport = Rect::from_corners(Point::new(0.0, 0.0), whole(width, height));
        let now = std::time::Instant::now();
        let shaped = self
            .presentation
            .as_mut()
            .map(|presentation| presentation.clock.frame(viewport, now))?;
        match shaped {
            Ok(list) => list,
            Err(problem) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                println!("note: transition: this frame would not draw: {problem}");
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
