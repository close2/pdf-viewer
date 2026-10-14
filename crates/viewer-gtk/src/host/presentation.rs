//! §12.4.4's presentation: the clock that turns the pages, the script timers it carries, and the transitions' faces.

use std::collections::VecDeque;

use gtk4::glib;
use pdf_render::Rasterizer;
use viewer_core::{Answer, Command, Event, PresentationMode, Query};

use viewer_host::trace::Topic;

use crate::page;

use super::stand_in::Picture;
use super::{Host, with};

impl Host {
    /// Enters or leaves §12.4.4's presentation, which for this program is the full-screen window.
    ///
    /// `p`, the same letter `viewer-ui` and `viewer-qt` bind. That the window and §12.4.4.2's mode
    /// are one act is a documented choice, argued in `viewer_host::presentation`; what crosses the
    /// boundary is `Command::Present`, which has existed since ADR 0316 and needed no change.
    pub(super) fn present_or_stop(&mut self) {
        if self.presenting.toggle() {
            self.start_the_clock();
            self.dispatch(Command::Present(PresentationMode::On));
            self.say(
                "presenting full screen (§7.7.2's FullScreen page mode) — §12.4.4's /Dur advances \
                 the page, its /Trans is drawn, and Escape comes back",
            );
        } else {
            self.stop_the_clock();
            self.dispatch(Command::Present(PresentationMode::Off));
            // §12.2: "[t]he document's page mode, specifying how to display the document on
            // exiting full-screen mode". `None` is Table 147's own condition unmet, and then what
            // goes back is what the reader had — a choice rather than a reading (ADR 0470).
            if let Some(mode) = self.presenting.on_exit() {
                self.say(&format!("§12.2's /NonFullScreenPageMode asks for {mode:?}"));
                self.show_page_mode(mode);
            } else {
                self.say("presentation stopped");
            }
        }
        self.apply_chrome();
    }

    /// Starts §12.4.4.1's clock, which is what makes this a presentation rather than a big page.
    pub(super) fn start_the_clock(&mut self) {
        if self.clock.is_none() {
            self.clock = Some(viewer_host::Clock::started(std::time::Instant::now()));
        }
    }

    /// Takes down the pending timer, if there still is one.
    ///
    /// **`SourceId::remove` panics on a source that is already gone** — it unwraps a `Result` — so
    /// the id is looked up before it is used. A one-shot destroys itself when it fires, and the
    /// callback that fires it clears [`Host::armed`]; this is the case where it could not, because
    /// the host was borrowed by a nested main loop (§7.6.4.1's password dialogue runs one).
    fn disarm(&mut self) {
        let Some(armed) = self.armed.take() else {
            return;
        };
        if let Some(source) = glib::MainContext::default().find_source_by_id(&armed) {
            source.destroy();
        }
    }

    /// Stops it, and takes the timer with it.
    ///
    /// **The source is removed rather than left to fire on a `None` clock.** A window that is not
    /// presenting has nothing to advance, and `CLAUDE.md`'s principle 2 makes a wakeup with
    /// nothing behind it a defect rather than a rounding error.
    fn stop_the_clock(&mut self) {
        self.clock = None;
        self.arming = None;
        self.shown = None;
        self.disarm();
    }

    /// Arms the next turn of the clock on GTK's own main loop.
    ///
    /// A one-shot that is re-armed after every turn, which is the shape [`Host::pump_search`]
    /// already uses and which answers what a repeating source cannot: the interval changes when a
    /// transition starts and stops, and a presentation that ends leaves nothing armed at all.
    pub(super) fn pump_presentation(&mut self) {
        self.disarm();
        // §12.6.4.15's clock exists for one effect. `Clock::spent` is the frame after it ended,
        // and dropping it here is what leaves a window that is not presenting with no timer
        // armed at all (ADR 1216).
        // Armed and not yet begun is not spent: the effect is drawn when the page it moves *to*
        // arrives, which is one render request away.
        if self.arming.is_none() && self.clock.as_ref().is_some_and(viewer_host::Clock::spent) {
            self.clock = None;
            self.shown = None;
        }
        let Some(interval) = self.clock.as_ref().map(viewer_host::Clock::interval) else {
            self.arm_script_timers();
            return;
        };
        // The presentation's clock carries the time, and a second clock would count it twice.
        self.script_ticker.stand_down();
        let me = self.me.clone();
        self.armed = Some(glib::timeout_add_local_once(interval, move || {
            with(&me, |host| {
                host.armed = None;
                host.turn_the_clock();
            });
        }));
    }

    /// Arms the one-shot for the soonest timer a document's script set, or nothing where none is
    /// held, which is every window whose documents set none (ADR 1702).
    fn arm_script_timers(&mut self) {
        let Some(wait) = self
            .script_ticker
            .wake(&self.viewer, std::time::Instant::now())
        else {
            return;
        };
        let me = self.me.clone();
        self.armed = Some(glib::timeout_add_local_once(wait, move || {
            with(&me, |host| {
                host.armed = None;
                host.tick_the_scripts();
            });
        }));
    }

    /// Tells the core how much time passed for the script timers, and reacts to what the runs
    /// that came due did; [`Self::pump`] arms the next one.
    fn tick_the_scripts(&mut self) {
        match self.script_ticker.tick(std::time::Instant::now()) {
            Some(millis) => self.dispatch(Command::Tick { millis }),
            None => self.pump_presentation(),
        }
    }

    /// One turn of §12.4.4.1's clock: tell the core how much time passed, and draw what is due.
    ///
    /// **A tick that produces no events repaints nothing**, and that is the whole of this host's
    /// answer to a viewer that must idle. A page stating no `/Dur` — "the page shall not advance
    /// automatically" — swallows every tick, so the window wakes ten times a second, adds a
    /// number, and goes back to sleep without touching a pixel. Repainting a still page at that
    /// rate would copy a page's worth of samples into a fresh texture for a picture that has not
    /// changed.
    fn turn_the_clock(&mut self) {
        let now = std::time::Instant::now();
        let animating = self
            .clock
            .as_ref()
            .is_some_and(viewer_host::Clock::animating);
        let Some(millis) = self.clock.as_mut().and_then(|clock| clock.tick(now)) else {
            // Held: a transition is being drawn, and §12.4.4.1's EXAMPLE puts that before the
            // page is displayed. What is due is a frame.
            if animating {
                self.refresh();
            }
            self.pump_presentation();
            return;
        };
        let events: Vec<Event> = self.viewer.handle(Command::Tick { millis }).collect();
        if events.is_empty() {
            self.pump_presentation();
            return;
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue);
    }

    /// The viewport a transition's frames are shaped in, in this window's own device pixels.
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    fn viewport_rect(&self) -> pdf_render::Rect {
        pdf_render::Rect::from_corners(
            pdf_render::Point::new(0.0, 0.0),
            pdf_render::Point::new(self.viewport.0 as f32, self.viewport.1 as f32),
        )
    }

    /// Takes `transition` to be drawn when the page it moves *to* arrives, or says why it is not.
    ///
    /// Armed rather than begun: `Viewer::handle` settles after the command that turned the page,
    /// so the events arrive as page change, transition, render request, and the arriving page's
    /// list is in the last of the three. §12.4.4.1's transition is one *to* a page, so waiting for
    /// that page's own request is the clause's order as well as this host's.
    pub(super) fn arm_transition(&mut self, transition: pdf_model::navigation::Transition) {
        if !viewer_host::Clock::shapes(&transition, self.viewport_rect()) {
            // The core has already said *why* through `Event::Reported`; a second sentence here
            // would say it twice.
            return;
        }
        // §12.6.4.15 states no mode: a processor "shall render the state of the page viewing area
        // as it exists after completion of the previous action and display it using a transition
        // specified in the action dictionary". So a window that is not presenting gets a clock of
        // its own for the one effect and drops it when it ends (ADR 1216).
        if self.clock.is_none() {
            self.clock = Some(viewer_host::Clock::for_one_transition(
                std::time::Instant::now(),
            ));
        }
        self.arming = Some(transition);
    }

    /// Keeps the page just rendered as a transition's face, and begins one that was armed.
    ///
    /// Two page rasterisations happen here and none per frame: the page being left and the page
    /// arriving, each at its own size and put where the window shows it. A transition that
    /// re-rasterised per frame would pay a page's interpretation sixty times a second for the
    /// length of it.
    ///
    /// Only the page of the document in front is a face: a document opening behind it draws its
    /// first page too, and that page is not the one a presented page turn leaves (ADR 1303).
    pub(super) fn face_arrived(&mut self, request: &viewer_core::RenderRequest) {
        if request.document != self.documents.focused() {
            return;
        }
        // At the whole pixel `place` shows the page at, so that a face is the settled page pixel
        // for pixel and the transition's last frame meets it without a seam (ADR 1865).
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => viewer_host::stand_in::whole_pixel(geometry.origin),
            _ => (0.0, 0.0),
        };
        let arriving = (std::sync::Arc::clone(&request.list), request.target, origin);
        let Some(transition) = self.arming.take() else {
            self.shown = Some(arriving);
            return;
        };
        let leaving = self.shown.replace(arriving.clone());
        let Some((list, target, at)) = leaving else {
            self.say(&format!(
                "transition: {:?} was named with no page to move from, so the page is shown at \
                 once",
                transition.style
            ));
            return;
        };
        let began = std::time::Instant::now();
        let (Some(outgoing), Some(incoming)) = (
            viewer_host::clock::face_as_shown(&list, target, at, self.viewport),
            viewer_host::clock::face_as_shown(&arriving.0, arriving.1, arriving.2, self.viewport),
        ) else {
            self.say(&format!(
                "transition: {:?} was named but the pages behind it would not rasterise, so the \
                 page is shown at once",
                transition.style
            ));
            return;
        };
        self.trace.say(
            Topic::Frames,
            format_args!(
                "TRANSITION {:?} over {} s: two {}x{} pages rasterised in {:?}",
                transition.style,
                transition.duration,
                self.viewport.0,
                self.viewport.1,
                began.elapsed()
            ),
        );
        if let Some(clock) = self.clock.as_mut() {
            clock.begin(transition, outgoing, incoming, std::time::Instant::now());
        }
    }

    /// The frame of a transition in flight, as one texture filling the viewport.
    ///
    /// `None` where there is nothing being drawn, and then the window shows the page — which is
    /// the transition's own end state, so the two answers meet without a seam.
    pub(super) fn transition_placement(&mut self) -> Option<Picture> {
        let viewport = self.viewport_rect();
        let now = std::time::Instant::now();
        let shaped = self.clock.as_mut().map(|clock| clock.frame(viewport, now));
        let list = match shaped {
            Some(Ok(Some(list))) => list,
            Some(Err(problem)) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                self.say(&format!("transition: this frame would not draw: {problem}"));
                return None;
            }
            None | Some(Ok(None)) => return None,
        };
        let target = pdf_render::TargetSpec {
            width: self.viewport.0,
            height: self.viewport.1,
            transform: pdf_render::Transform::IDENTITY,
        };
        let raster = match self.rasterizer.rasterize(&list, target) {
            Ok(raster) => raster,
            Err(error) => {
                self.say(&format!(
                    "transition: this frame would not rasterise: {error}"
                ));
                return None;
            }
        };
        match page::texture(&raster) {
            Ok(texture) => Some(Picture {
                extent: (raster.width, raster.height),
                texture,
                origin: (0.0, 0.0),
                bytes: raster.data.len(),
            }),
            Err(error) => {
                eprintln!("note: {error}");
                None
            }
        }
    }
}
