//! §12.4.4's presentation: the full-screen window, the `QTimer` `viewer_host::presentation_pump` is
//! turned on, and Table 164's transitions drawn between two settled pages (ADRs 1853, 1908, 2300).

use viewer_core::{Command, PresentationMode};
use viewer_host::native_window::NativeWindow;
use viewer_host::{PresentationPump, Turned};

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
        self.drop_the_clock();
        if self.playing.take().is_some() {
            self.update.frame = true;
        }
    }

    /// How long the window should wait before the next turn of `viewer_host::presentation_pump`,
    /// in milliseconds: §12.4.4's clock while a presentation or a transition runs, a document's
    /// script timers while none does (ADR 1702).
    ///
    /// `-1` where the pump arms nothing — a still window, or a hidden one whose documents hold no
    /// timer — which is what stops the timer. Asked after every pump rather than set once, because
    /// the interval a transition wants and the interval a still page wants are not the same number.
    pub(crate) fn presentation_wait(&mut self) -> i32 {
        PresentationPump::presentation_wait(self, std::time::Instant::now()).map_or(-1, |wait| {
            i32::try_from(wait.as_millis()).unwrap_or(i32::MAX)
        })
    }

    /// One turn of the pump, and the frame it is due flagged for the window.
    pub(crate) fn presentation_tick(&mut self) {
        if let Turned::Frame(frame) = self.turn_the_presentation(std::time::Instant::now()) {
            self.playing = frame;
            self.update.frame = true;
        }
    }

    /// The window's `QWindow` was exposed or stopped being: the pump parks the presentation while
    /// nobody can see it, and the frame of a transition in flight goes with the effect (ADR 2300).
    pub(crate) fn exposure(&mut self, exposed: bool) {
        if self.window_shown(exposed, std::time::Instant::now()) && self.playing.take().is_some() {
            self.update.frame = true;
        }
    }
}
