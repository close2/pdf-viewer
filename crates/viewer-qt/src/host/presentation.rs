//! §12.4.4's presentation: the full-screen window, the clock that advances it, and Table 164's
//! transitions drawn between two settled pages (ADR 1853; filed here by ADR 1908).

use std::collections::VecDeque;

use pdf_render::Rasterizer;
use viewer_core::{Answer, Command, Event, PresentationMode, Query};
use viewer_host::trace::Topic;

use super::Host;

impl Host {
    /// Enters or leaves §12.4.4's presentation, which for this program is the full-screen window.
    ///
    /// The same letter and the same act as the other two hosts; what differs is that the *widgets*
    /// are C++'s, so this side flags [`QtUpdate::window`] and the window asks
    /// [`Host::chrome`] what it should look like now.
    ///
    /// [`QtUpdate::window`]: crate::bridge::ffi::QtUpdate::window
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
            match self.presenting.on_exit() {
                Some(mode) => {
                    self.say(&format!("§12.2's /NonFullScreenPageMode asks for {mode:?}"));
                }
                None => self.say("presentation stopped"),
            }
        }
        self.update.window = true;
    }

    /// Starts §12.4.4.1's clock, which is what makes this a presentation rather than a big page.
    pub(super) fn start_the_clock(&mut self) {
        if self.clock.is_none() {
            self.clock = Some(viewer_host::Clock::started(std::time::Instant::now()));
        }
    }

    /// Stops it, and takes the frame in flight with it.
    ///
    /// The `QTimer` on the C++ side stops the next time it asks [`Host::presentation_wait`], which
    /// is immediately: leaving full screen goes through `applyUpdates`. A window that is not
    /// presenting has nothing to advance, and `CLAUDE.md`'s principle 2 makes a wakeup with
    /// nothing behind it a defect rather than a rounding error.
    fn stop_the_clock(&mut self) {
        self.clock = None;
        self.arming = None;
        self.shown = None;
        if self.playing.take().is_some() {
            self.update.frame = true;
        }
    }

    /// How long the window should wait before the next turn of the clock, in milliseconds.
    ///
    /// `-1` where nothing is presenting, which is what stops the timer. Asked after every pump
    /// rather than set once, because the interval a transition wants and the interval a still
    /// page wants are not the same number.
    ///
    /// **And a document's script timers, while nothing presents** (ADR 1702): one clock per
    /// window, because two clocks each counting from their own last tick would tell the core twice
    /// the time that passed. `-1` while no document holds a timer, so a still window has no timer.
    pub(crate) fn presentation_wait(&mut self) -> i32 {
        if let Some(clock) = self.clock.as_ref() {
            self.script_ticker.stand_down();
            return i32::try_from(clock.interval().as_millis()).unwrap_or(i32::MAX);
        }
        self.script_ticker
            .wake(&self.viewer, std::time::Instant::now())
            .map_or(-1, |wait| {
                i32::try_from(wait.as_millis()).unwrap_or(i32::MAX)
            })
    }

    /// One turn of §12.4.4.1's clock: tell the core how much time passed, and draw what is due.
    ///
    /// **A tick that produces no events repaints nothing**, which is this host's answer to a
    /// viewer that must idle. A page stating no `/Dur` — "the page shall not advance
    /// automatically" — swallows every tick, so the window wakes ten times a second, adds a
    /// number, and flags no update at all.
    pub(crate) fn presentation_tick(&mut self) {
        let now = std::time::Instant::now();
        // No presentation: the timer woke for a document's script timers (ADR 1702).
        if self.clock.is_none() {
            if let Some(millis) = self.script_ticker.tick(now) {
                self.pump(vec![Command::Tick { millis }]);
            }
            return;
        }
        // §12.6.4.15's clock exists for one effect; `Clock::spent` is the turn after it ended, and
        // dropping it here is what leaves a window that is not presenting with no timer at all —
        // `presentation_wait` answers -1 the moment this is `None` (ADR 1216).
        // Armed and not yet begun is not spent: the effect is drawn when the page it moves *to*
        // arrives, which is one render request away.
        if self.arming.is_none() && self.clock.as_ref().is_some_and(viewer_host::Clock::spent) {
            self.clock = None;
            self.shown = None;
            self.update.window = true;
            return;
        }
        let animating = self
            .clock
            .as_ref()
            .is_some_and(viewer_host::Clock::animating);
        let Some(millis) = self.clock.as_mut().and_then(|clock| clock.tick(now)) else {
            // Held: a transition is being drawn, and §12.4.4.1's EXAMPLE puts that before the
            // page is displayed. What is due is a frame.
            if animating {
                self.draw_a_frame(now);
            }
            return;
        };
        let events: Vec<Event> = self.viewer.handle(Command::Tick { millis }).collect();
        if events.is_empty() {
            return;
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue.into_iter().collect());
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

    /// Rasterises the frame of the transition in flight, or puts the page back when it has ended.
    fn draw_a_frame(&mut self, now: std::time::Instant) {
        let viewport = self.viewport_rect();
        let shaped = self.clock.as_mut().map(|clock| clock.frame(viewport, now));
        let list = match shaped {
            Some(Ok(Some(list))) => list,
            Some(Err(problem)) => {
                // Not reachable from a frame — the largest one adds four clips — and said rather
                // than swallowed for the reason every refusal in this tree is.
                self.say(&format!("transition: this frame would not draw: {problem}"));
                self.playing = None;
                self.update.frame = true;
                return;
            }
            None | Some(Ok(None)) => {
                self.playing = None;
                self.update.frame = true;
                return;
            }
        };
        let target = pdf_render::TargetSpec {
            width: self.viewport.0,
            height: self.viewport.1,
            transform: pdf_render::Transform::IDENTITY,
        };
        match self.rasterizer.rasterize(&list, target) {
            Ok(raster) => self.playing = Some(raster),
            Err(error) => {
                self.say(&format!(
                    "transition: this frame would not rasterise: {error}"
                ));
                self.playing = None;
            }
        }
        self.update.frame = true;
    }

    /// Takes `transition` to be drawn when the page it moves *to* arrives, or says why it is not.
    ///
    /// Armed rather than begun: `Viewer::handle` settles after the command that turned the page,
    /// so the events arrive as page change, transition, render request, and the arriving page's
    /// list is in the last of the three. §12.4.4.1's transition is one *to* a page.
    pub(super) fn arm_transition(&mut self, transition: pdf_model::navigation::Transition) {
        if !viewer_host::Clock::shapes(&transition, self.viewport_rect()) {
            // The core has already said *why* through `Event::Reported`.
            return;
        }
        // §12.6.4.15 states no mode: a processor "shall render the state of the page viewing area
        // as it exists after completion of the previous action and display it using a transition
        // specified in the action dictionary". A window that is not presenting therefore gets a
        // clock of its own for the one effect, dropped when it ends (ADR 1216).
        if self.clock.is_none() {
            self.clock = Some(viewer_host::Clock::for_one_transition(
                std::time::Instant::now(),
            ));
        }
        self.arming = Some(transition);
    }

    /// Keeps the page just rendered as a transition's face, and begins one that was armed.
    ///
    /// Two page rasterisations happen here and none per frame: the page being left and
    /// the page arriving, each at its own size and put where the window shows it.
    ///
    /// Only the page of the document in front is a face: a document opening behind it draws its
    /// first page too, and that page is not the one a presented page turn leaves (ADR 1303).
    pub(super) fn face_arrived(&mut self, request: &viewer_core::RenderRequest) {
        if request.document != self.documents.focused() {
            return;
        }
        // At the whole pixel `frame` shows the page at, so that a face is the settled page pixel
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
}
