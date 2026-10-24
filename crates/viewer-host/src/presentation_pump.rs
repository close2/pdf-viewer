//! §12.4.4's presentation pump: the page's `/Dur` and its transition's frames as one host tick,
//! parked while the window is hidden or another document comes to the front (ADR 2300).
//!
//! # What every window's timer was asked
//!
//! [`crate::Clock`] decides how often a presentation is ticked, what a tick carries and when
//! Table 164's `/D` has run out; [`crate::script_timers::Ticker`] decides the same for a document's
//! script timers. What is left between them is the *turn*: drop §12.6.4.15's clock once it is
//! spent, ask which clock carries the time, tell the core, react to what it said only where it said
//! anything, and rasterise the frame of a transition in flight. None of that is a toolkit's
//! question, and a window that wrote it for itself would be the copy where windows stop agreeing,
//! which is the test [`crate`] states for what belongs here — the C window's caller among them,
//! which has no toolkit's timer of its own to hang the turn on and is handed it through
//! `quorra_window_tick`.
//!
//! So a toolkit supplies a timer and no decision: one shot armed for
//! [`PresentationPump::presentation_wait`] after every command, because the wait changes whenever
//! a transition starts or ends or a script sets a timer, and a call to
//! [`PresentationPump::turn_the_presentation`] when it fires. What the turn answers is
//! [`Turned`]: nothing to show, a frame to show, or a page the core already moved to.
//!
//! # Parked
//!
//! Two states stop the pump without stopping the presentation, and both are a window's.
//!
//! - **The window is hidden** — minimised, unmapped, not exposed. §12.4.4.1's `/Dur` is "the
//!   maximum length of time, in seconds, that the page shall be displayed before the presentation
//!   automatically advances to the next page", and a page in a window nobody can see is not
//!   displayed, so the time it is hidden is not told to the core: [`PresentationPump::window_shown`]
//!   parks the clock and [`crate::Clock::resume`] counts from the moment it comes back. A window
//!   that woke ten times a second to count time for a page no one sees would be spending a
//!   processor on nothing, which `CLAUDE.md`'s principle 2 forbids.
//! - **Another document comes to the front.** A presentation is the front document's: each
//!   document's [`crate::Presenting`] waits beside its tab while it is behind (ADR 2200), and
//!   [`PresentationPump::bring_the_presentation_forward`] is what every window does when its tab
//!   changes — the clock, the face and the armed transition of the document left are dropped, the
//!   arriving document's presentation is restored, and once the queue has settled
//!   [`PresentationPump::synchronize_if_owed`] tells the core its mode and the window its chrome.
//!   A document opened *behind* takes the front for a moment and gives it back, and that round
//!   trip keeps the front document's clock (ADR 1303).
//!
//! # What is chosen rather than derived
//!
//! - **Hidden time is not display time.** A reading of "shall be displayed"; the alternative, a
//!   slide show advancing in a minimised window, is a presentation shown to nobody.
//! - **A transition in flight when the window is hidden ends on its page**, because Table 164
//!   states a visual effect and no frame of it was seen; the window comes back to the page the
//!   effect was moving to, which is the effect's own end state.
//! - **A document's script timers run while the window is hidden.** `app.setInterval` is not
//!   display, and nothing in the reference ties a timer to a window being seen; while the
//!   presentation is parked its clock tells the core nothing, so the script ticker carries the time
//!   and there is still one clock per window.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use pdf_render::{Raster, Rasterizer, TargetSpec, Transform};
use render_cpu::CpuRasterizer;
use viewer_core::{Command, Event};

use crate::native_window::NativeWindow;
use crate::trace::Topic;

/// The pump's own state, one per window, beside the clock it turns
/// ([`crate::native_window::WindowState::clock`]).
#[derive(Debug, Default)]
pub struct Pump {
    /// The clock of a document's script timers, which stands down while a presentation carries
    /// the time (ADR 1702).
    scripts: crate::script_timers::Ticker,
    /// Whether the window is hidden, which parks the presentation's clock.
    hidden: bool,
    /// Whether a change of the document in front owes the core its mode and the window its chrome
    /// once the queue has settled.
    synchronizing: bool,
    /// What a transition's frames are rasterised with, made at the first frame: a window that never
    /// presents never makes one (`CLAUDE.md` principle 2's "nothing eager").
    rasterizer: Option<CpuRasterizer>,
}

impl Pump {
    /// Whether the window is hidden and the presentation's clock parked.
    #[must_use]
    pub const fn hidden(&self) -> bool {
        self.hidden
    }

    /// Whether a document change still owes its synchronisation.
    #[must_use]
    pub const fn synchronizing(&self) -> bool {
        self.synchronizing
    }

    /// Owes the synchronisation, for a window whose document in front has just obeyed its
    /// catalog — which may have asked for full screen — or has just been replaced.
    pub const fn owe_synchronizing(&mut self) {
        self.synchronizing = true;
    }

    /// Takes the owed synchronisation, answering whether one was owed.
    pub const fn take_synchronizing(&mut self) -> bool {
        let owed = self.synchronizing;
        self.synchronizing = false;
        owed
    }
}

/// What one turn of the pump leaves for the toolkit to do.
#[derive(Debug)]
pub enum Turned {
    /// Nothing on the screen changed: a still page, a tick too soon to carry a millisecond, a
    /// core that had nothing to say, or a parked window.
    Still,
    /// A transition's frame is due: `Some` is the frame, rasterised for the whole viewport, and
    /// `None` is the effect ended or refused, after which the window shows the page again.
    Frame(Option<Raster>),
    /// The core was told that time passed, and what it said has been reacted to and drained.
    Advanced,
}

/// The pump, as every native window turns it; implemented for each one and overridden by none.
pub trait PresentationPump: NativeWindow {
    /// How long the window may sleep before [`Self::turn_the_presentation`], or `None` for a
    /// window that arms nothing: no presentation, no transition, no script timer, or a hidden
    /// window whose documents hold no timer.
    ///
    /// Drops §12.6.4.15's clock once its effect is spent, which is what leaves a window that is not
    /// presenting with no timer armed at all (ADR 1216). Armed and not yet begun is not spent: the
    /// effect is drawn when the page it moves *to* arrives, one render request away.
    fn presentation_wait(&mut self, now: Instant) -> Option<Duration> {
        let state = self.state_mut();
        if state.arming.is_none() && state.clock.as_ref().is_some_and(crate::Clock::spent) {
            state.clock = None;
            state.shown = None;
        }
        match state.clock.as_ref() {
            Some(clock) if !state.pump.hidden => {
                // The presentation's clock carries the time, and a second clock counting from its
                // own last tick would tell the core twice the time that passed (ADR 1702).
                state.pump.scripts.stand_down();
                Some(clock.interval())
            }
            _ => state.pump.scripts.wake(&state.viewer, now),
        }
    }

    /// One turn of §12.4.4.1's clock, or of the script timers while nothing presents: tell the core
    /// how much time passed, and say what is due on the screen.
    ///
    /// **A tick the core says nothing about repaints nothing.** A page stating no `/Dur` — "the
    /// page shall not advance automatically" — swallows every tick, so the window wakes ten times a
    /// second, adds a number and goes back to sleep without touching a pixel; repainting a still
    /// page at that rate would copy a page's worth of samples for a picture that has not changed.
    fn turn_the_presentation(&mut self, now: Instant) -> Turned {
        let state = self.state_mut();
        let presenting = !state.pump.hidden && state.clock.is_some();
        let millis = if presenting {
            if state.arming.is_none() && state.clock.as_ref().is_some_and(crate::Clock::spent) {
                state.clock = None;
                state.shown = None;
                return Turned::Still;
            }
            state.pump.scripts.stand_down();
            let Some(clock) = state.clock.as_mut() else {
                return Turned::Still;
            };
            let animating = clock.animating();
            match clock.tick(now) {
                Some(millis) => millis,
                // Held: a transition is being drawn, and §12.4.4.1's EXAMPLE puts that before the
                // page is displayed. What is due is a frame.
                None if animating => return Turned::Frame(transition_frame(self, now)),
                None => return Turned::Still,
            }
        } else {
            match state.pump.scripts.tick(now) {
                Some(millis) => millis,
                None => return Turned::Still,
            }
        };
        let events: Vec<Event> = self
            .state_mut()
            .viewer
            .handle(Command::Tick { millis })
            .collect();
        if events.is_empty() {
            return Turned::Still;
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.drain(queue);
        Turned::Advanced
    }

    /// The window was hidden or shown again: parks the presentation's clock, or counts from `now`
    /// again, answering whether anything changed. A host whose window is hidden drops the
    /// transition frame it holds; the effect ends on its page ([`crate::Clock::resume`]).
    fn window_shown(&mut self, shown: bool, now: Instant) -> bool {
        let state = self.state_mut();
        if state.pump.hidden != shown {
            return false;
        }
        state.pump.hidden = !shown;
        if let Some(clock) = state.clock.as_mut() {
            clock.resume(now);
        }
        let (window, clock) = if shown {
            ("shown", "turns again")
        } else {
            ("hidden", "is parked")
        };
        let held = if state.clock.is_some() {
            "§12.4.4.1's clock"
        } else {
            "no clock, and nothing"
        };
        state.trace.say(
            Topic::Frames,
            format_args!("PRESENTATION the window is {window}: {held} {clock}"),
        );
        true
    }

    /// Stops the presentation's clock and takes the face and the armed transition with it, so a
    /// window that is not presenting has nothing to advance and no timer to arm.
    fn drop_the_clock(&mut self) {
        let state = self.state_mut();
        state.clock = None;
        state.arming = None;
        state.shown = None;
    }

    /// Restores the presentation of the document now in front, after
    /// [`crate::native_window::WindowState::park_presentation`] kept the one left: the clock, the
    /// face and the armed transition of the document left are dropped unless the change is
    /// `temporary` — a document opened behind, taking the front only to give it back — and the
    /// synchronisation is owed for [`Self::synchronize_if_owed`].
    fn bring_the_presentation_forward(&mut self, temporary: bool) {
        if !temporary {
            self.drop_the_clock();
        }
        let state = self.state_mut();
        state.restore_presentation();
        state.pump.synchronizing = true;
    }

    /// Once `queue` has settled after a document change, tells the core the mode of the
    /// presentation now in front and the window the chrome it may show; starts the clock where
    /// that presentation is full screen.
    ///
    /// After the queue rather than at the change, because the core settles the change's own
    /// commands first, and a render event among them may already have handed the arriving page's
    /// face over (ADR 2200).
    fn synchronize_if_owed(&mut self, queue: &mut VecDeque<Command>) {
        if !queue.is_empty() || !self.state_mut().pump.take_synchronizing() {
            return;
        }
        if self.state().presenting.full_screen() {
            self.start_the_clock();
        }
        let mode = self.state().presenting.mode();
        queue.push_back(Command::Present(mode));
        self.apply_the_chrome();
    }
}

impl<W: NativeWindow + ?Sized> PresentationPump for W {}

/// The frame of the transition in flight, rasterised for the whole viewport, or `None` where it
/// has ended — or would not draw, which is said rather than swallowed.
fn transition_frame<W: NativeWindow + ?Sized>(window: &mut W, now: Instant) -> Option<Raster> {
    let viewport = window.viewport_rect();
    let shaped = window
        .state_mut()
        .clock
        .as_mut()
        .map(|clock| clock.frame(viewport, now));
    let list = match shaped {
        Some(Ok(Some(list))) => list,
        Some(Err(problem)) => {
            // Not reachable from a frame — the largest one adds four clips — and said for the
            // reason every refusal in this tree is.
            window.say(&format!("transition: this frame would not draw: {problem}"));
            return None;
        }
        None | Some(Ok(None)) => return None,
    };
    let state = window.state_mut();
    let target = TargetSpec {
        width: state.viewport.0,
        height: state.viewport.1,
        transform: Transform::IDENTITY,
    };
    let rasterized = state
        .pump
        .rasterizer
        .get_or_insert_with(CpuRasterizer::new)
        .rasterize(&list, target);
    match rasterized {
        Ok(raster) => Some(raster),
        Err(error) => {
            window.say(&format!(
                "transition: this frame would not rasterise: {error}"
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pump;

    /// A pump nobody has touched is shown, owes nothing and has made no rasteriser; a taken
    /// synchronisation is taken once.
    #[test]
    fn a_new_pump_is_shown_owes_nothing_and_made_nothing() {
        let mut pump = Pump::default();
        assert!(!pump.hidden());
        assert!(!pump.take_synchronizing());
        assert!(pump.rasterizer.is_none(), "nothing eager");
        pump.synchronizing = true;
        assert!(pump.synchronizing());
        assert!(pump.take_synchronizing());
        assert!(!pump.take_synchronizing(), "taken once");
    }
}
