//! §12.4.4's presentation: the full-screen window, the clock that advances it, and Table 164's
//! transitions drawn between two settled pages (ADR 1853; filed here by ADR 1908).

use std::collections::VecDeque;

use pdf_render::Rasterizer;
use viewer_core::{Command, Event, PresentationMode};
use viewer_host::native_window::NativeWindow;

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
        if self.state.presenting.toggle() {
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
            match self.state.presenting.on_exit() {
                Some(mode) => {
                    self.say(&format!("§12.2's /NonFullScreenPageMode asks for {mode:?}"));
                }
                None => self.say("presentation stopped"),
            }
        }
        self.update.window = true;
    }

    /// Stops it, and takes the frame in flight with it.
    ///
    /// The `QTimer` on the C++ side stops the next time it asks [`Host::presentation_wait`], which
    /// is immediately: leaving full screen goes through `applyUpdates`. A window that is not
    /// presenting has nothing to advance, and `CLAUDE.md`'s principle 2 makes a wakeup with
    /// nothing behind it a defect rather than a rounding error.
    fn stop_the_clock(&mut self) {
        self.state.clock = None;
        self.state.arming = None;
        self.state.shown = None;
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
        if let Some(clock) = self.state.clock.as_ref() {
            self.script_ticker.stand_down();
            return i32::try_from(clock.interval().as_millis()).unwrap_or(i32::MAX);
        }
        self.script_ticker
            .wake(&self.state.viewer, std::time::Instant::now())
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
        if self.state.clock.is_none() {
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
        if self.state.arming.is_none()
            && self
                .state
                .clock
                .as_ref()
                .is_some_and(viewer_host::Clock::spent)
        {
            self.state.clock = None;
            self.state.shown = None;
            self.update.window = true;
            return;
        }
        let animating = self
            .state
            .clock
            .as_ref()
            .is_some_and(viewer_host::Clock::animating);
        let Some(millis) = self.state.clock.as_mut().and_then(|clock| clock.tick(now)) else {
            // Held: a transition is being drawn, and §12.4.4.1's EXAMPLE puts that before the
            // page is displayed. What is due is a frame.
            if animating {
                self.draw_a_frame(now);
            }
            return;
        };
        let events: Vec<Event> = self.state.viewer.handle(Command::Tick { millis }).collect();
        if events.is_empty() {
            return;
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue.into_iter().collect());
    }

    /// Rasterises the frame of the transition in flight, or puts the page back when it has ended.
    fn draw_a_frame(&mut self, now: std::time::Instant) {
        let viewport = self.viewport_rect();
        let shaped = self
            .state
            .clock
            .as_mut()
            .map(|clock| clock.frame(viewport, now));
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
            width: self.state.viewport.0,
            height: self.state.viewport.1,
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
}
