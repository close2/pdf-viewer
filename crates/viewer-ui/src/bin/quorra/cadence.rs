//! The presenter's clock: how often this window presents, and where that number comes from.
//!
//! Until this module existed the window presented **when something happened** — a key, a wheel
//! notch, a display list arriving — and the interval between two frames was whatever the renderer
//! and the input device between them happened to produce. `doc/todo/36` asks for the other shape:
//! the window presents on the *surface's own* cadence, 60 Hz as the floor and 120 Hz as the
//! target, and what it presents is a correct frame where one is ready and a reprojection
//! ([`crate::stale`]) where one is not.
//!
//! # Three things this is, and one it is not
//!
//! - **A rate limit, not a timer.** [`Cadence::due`] answers "may the window present now", and a
//!   window that has been still is due immediately — the cadence is a *ceiling on how often* a
//!   frame is drawn and never a floor under how soon one is. That is what keeps input latency
//!   what it was: the first present after a key is not held back by a tick.
//! - **A ceiling on latency, not a duty to burn power** (`doc/todo/36`'s second unsettled
//!   question). Nothing here wakes anything: the clock is *armed* only while a frame is owed,
//!   and `about_to_wait` leaves the loop in `ControlFlow::Wait` when none is — so a window
//!   nobody is touching presents nothing at all and spends no tick.
//! - **The surface's number rather than a constant** (the third). `winit` reports the monitor's
//!   refresh rate and [`Cadence::of`] asks for it; a target that is not the display's is the
//!   stutter machine `doc/todo/36` warns about. [`FLOOR`] stands in only where nothing states one,
//!   and the trace says which of the three [`Source`]s answered. **It has to be asked more than
//!   once**, which is ADR 0384's correction: a Wayland surface belongs to no output until it has
//!   been drawn to, so the first answer on that platform is always "nobody knows".
//!
//! What it is **not** is a scheduler. A frame still runs to completion on the event thread, so a
//! correct frame that takes longer than a tick takes the ticks it takes; this clock decides *when
//! a frame may start*, never how long one lasts. ADR 0383's section on what a clock cannot do here
//! prices the difference and names what would remove it.

use std::time::{Duration, Instant};

/// The cadence used where the surface states no refresh rate.
///
/// **`doc/todo/36`'s floor, and it is a floor rather than a guess.** The project owner asked for
/// "at least 60 Hz", so a surface that will not say how often it refreshes is presented to at the
/// slowest rate that still answers the request. Presenting *faster* than an unknown display is
/// the failure the same item names — a rate that is not a multiple of the display's is a stutter
/// machine — so the unknown case takes the floor and not the target.
const FLOOR: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// One millihertz per hertz per second, for turning `winit`'s unit into a period.
const MILLIHERTZ: u64 = 1_000;

/// Where the period came from, because the three are three different claims.
///
/// "We present at 60 Hz", "the display this window is on refreshes at 60 Hz" and "the slowest
/// display attached to this machine refreshes at 60 Hz" are not the same sentence, and a round
/// reporting a rate has to say which one it measured (`doc/todo/36` rule 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// The output this window's surface is on stated it. The answer, and the only one that ends
    /// the asking.
    Window,
    /// No output claims this window yet, so the **slowest** display attached stated it.
    ///
    /// Slowest rather than fastest, and `doc/todo/36`'s own argument decides the direction:
    /// presenting faster than the display is the stutter machine it warns about, so a period that
    /// is not known to be this window's is the longest of the candidates rather than the shortest.
    Slowest,
    /// Nothing stated one, so [`FLOOR`] stands in.
    Floor,
}

/// When this window may next present, and how often it may.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cadence {
    /// One refresh of the surface.
    period: Duration,
    /// Where [`Self::period`] came from. See [`Source`].
    source: Source,
    /// The earliest moment the next frame may be presented.
    ///
    /// In the past for a window that has been still, which is what makes the first present after
    /// an idle period immediate.
    due: Instant,
    /// Whether a frame was asked for before the surface had refreshed, and so is still owed.
    ///
    /// **Without this the clock would only pace the frames that follow a reprojection**, which is
    /// what the first measurement of this feature found: a held key delivered redraw requests
    /// faster than the display refreshes and every one of them became a present, so the window
    /// ran at the *input's* rate. `doc/todo/36` calls that a stutter machine and it is right.
    owing: bool,
}

/// A monitor's refresh rate as a period, or `None` where it states none.
///
/// `winit` reports millihertz and `None` where the platform does not know one — which is the
/// ordinary answer on a virtual display, and the reason [`FLOOR`] exists rather than a panic. A
/// rate of zero is the same answer written differently and is read the same way; Wayland's own
/// toolkit documents zero for an output with no correct rate.
fn period_of(monitor: &winit::monitor::MonitorHandle) -> Option<Duration> {
    monitor
        .refresh_rate_millihertz()
        .filter(|rate| *rate > 0)
        .map(|rate| {
            // Nanoseconds per refresh from millihertz, in integer arithmetic: the workspace
            // forbids unchecked arithmetic and a period computed in floating point would have to
            // justify its rounding as well as its value.
            Duration::from_nanos(
                1_000_000_000_u64
                    .saturating_mul(MILLIHERTZ)
                    .checked_div(u64::from(rate))
                    .unwrap_or(0),
            )
        })
        .filter(|period| !period.is_zero())
}

impl Default for Cadence {
    /// The floor, due now — the cadence of a program that has no window yet.
    fn default() -> Self {
        Self {
            period: FLOOR,
            source: Source::Floor,
            due: Instant::now(),
            owing: false,
        }
    }
}

impl Cadence {
    /// The cadence of the surface this window is on, as far as it can be known yet.
    pub(crate) fn of(window: &winit::window::Window) -> Self {
        let mut cadence = Self::default();
        cadence.ask(window);
        cadence
    }

    /// Asks the platform again what this window's surface refreshes at, and says whether the
    /// answer moved.
    ///
    /// **This exists because of Wayland, and the defect it repairs was invisible on X11.** winit's
    /// `Window::current_monitor` on that backend is the first output in the surface's own
    /// `wl_surface::enter` list, and *a Wayland surface enters no output until it has been drawn
    /// to* — winit's own platform note says windows do not appear until you present to them. The
    /// cadence was read once, in `resumed`, which is strictly before the first present: so on
    /// every Wayland session `current_monitor` answered `None`, [`FLOOR`] stood in, and **120 Hz
    /// could not be reached even in principle**. The project owner's trace says
    /// `the surface states no refresh rate` on a machine with a real display. ADR 0384.
    ///
    /// Two routes, in the order that answers best:
    ///
    /// 1. `current_monitor` — the output this window is actually on. Once it answers, the question
    ///    is settled and this stops asking.
    /// 2. `available_monitors`, which Wayland *does* populate from the start (every `wl_output`
    ///    global, refresh rate included). It does not say which one this window is on, so the
    ///    slowest is taken; see [`Source::Slowest`].
    ///
    /// **What it deliberately does not do is follow a window between displays.** It stops at the
    /// first surface answer rather than re-reading for ever, because a per-frame monitor query is
    /// a cost on every frame to answer a question that changes when somebody drags a window, and
    /// the honest fix for that is `winit` reporting the output change rather than this polling
    /// for it — its Wayland backend receives `surface_enter` and its handler is empty. Written
    /// down as a limit rather than papered over.
    pub(crate) fn ask(&mut self, window: &winit::window::Window) -> bool {
        if self.source == Source::Window {
            return false;
        }
        let (period, source) =
            if let Some(period) = window.current_monitor().as_ref().and_then(period_of) {
                (period, Source::Window)
            } else if let Some(period) = window
                .available_monitors()
                .filter_map(|monitor| period_of(&monitor))
                .max()
            {
                (period, Source::Slowest)
            } else {
                (FLOOR, Source::Floor)
            };
        if (period, source) == (self.period, self.source) {
            return false;
        }
        self.period = period;
        self.source = source;
        true
    }

    /// Whether the window's own output has stated its rate, so there is nothing left to ask.
    pub(crate) fn settled(&self) -> bool {
        self.source == Source::Window
    }

    /// One refresh of the surface, which is the interval every present is spaced by.
    pub(crate) fn period(&self) -> Duration {
        self.period
    }

    /// How this cadence is to be described in a trace: the rate, and where it came from.
    ///
    /// Rule 6 of `doc/todo/36` in one sentence — a round claiming a rate says whether the number
    /// is this window's display's, some display's, or this program's floor, because the three are
    /// different claims.
    pub(crate) fn described(&self) -> String {
        let hertz = 1.0 / self.period.as_secs_f64();
        let milliseconds = self.period.as_secs_f64() * 1e3;
        match self.source {
            Source::Window => {
                format!("{hertz:.1} Hz ({milliseconds:.3} ms), stated by the surface")
            }
            Source::Slowest => format!(
                "{hertz:.1} Hz ({milliseconds:.3} ms) — no output claims this window yet, so the \
                 slowest display attached states it"
            ),
            Source::Floor => format!(
                "{hertz:.1} Hz ({milliseconds:.3} ms) — the surface states no refresh rate, so \
                 doc/todo/36's floor stands in"
            ),
        }
    }

    /// Whether a frame may be presented now.
    ///
    /// True for a window that has been still longer than one refresh, which is every window
    /// answering its first input: the cadence spaces frames out and never holds one back.
    pub(crate) fn due(&self, now: Instant) -> bool {
        now >= self.due
    }

    /// When the next frame may be presented, for `ControlFlow::WaitUntil`.
    pub(crate) fn next(&self) -> Instant {
        self.due
    }

    /// Records that a frame has just been presented, and moves the clock on.
    ///
    /// **Phase-keeping rather than delay-adding**: the next tick is the previous one plus whole
    /// periods, so a frame that overran by three refreshes does not push every later tick three
    /// refreshes late. A clock that had set `now + period` would drift by exactly the cost of
    /// every frame it timed, which on this program's own witness is most of a second.
    pub(crate) fn presented(&mut self, now: Instant) {
        // A present that arrived early keeps the phase; one that arrived late resynchronises to
        // the first tick that has not happened yet, which is `now` rounded up by whole periods.
        while self.due <= now {
            let Some(next) = self.due.checked_add(self.period) else {
                self.due = now;
                return;
            };
            self.due = next;
        }
    }

    /// Arms the clock for a frame that is owed but has not been drawn.
    ///
    /// **Two callers and one meaning.** [`crate::stale::MustFollow`] discharges its obligation
    /// here — the frame replacing a reprojection is asked for *at the next tick* rather than at
    /// once, so a view that keeps moving is answered again instead of waiting behind a render
    /// that takes fifty times the tick — and `redraw_requested` calls it for a redraw that
    /// arrived before the surface had refreshed, which is the same statement about a different
    /// frame.
    ///
    /// An idle window never reaches this, which is `doc/todo/36`'s fourth rule: the clock is
    /// armed by an obligation and by nothing else, so a window nobody is touching owes nothing,
    /// waits for an event and presents nothing.
    pub(crate) fn owed(&mut self, now: Instant) {
        self.owing = true;
        if self.due <= now {
            self.presented(now);
        }
    }

    /// Whether a frame asked for earlier has still not been drawn.
    ///
    /// What `about_to_wait` reads to decide between waiting for an event and waiting for a tick.
    pub(crate) fn owes(&self) -> bool {
        self.owing
    }

    /// Records that the redraw a tick asked for has been run, whatever it decided to draw.
    ///
    /// **Cleared by the redraw rather than by the present**, and the difference is a loop that
    /// terminates: a frame that found nothing to show is still an obligation discharged, and a
    /// clock that waited for pixels before forgetting one would ask for that frame again on
    /// every tick for ever.
    pub(crate) fn serviced(&mut self) {
        self.owing = false;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Cadence, FLOOR, Source};

    /// A cadence with a period this test states, so that the arithmetic is not the machine's.
    fn at(period: Duration, now: Instant) -> Cadence {
        Cadence {
            period,
            source: Source::Window,
            due: now,
            owing: false,
        }
    }

    /// The clock spaces presents out and never holds one back: a window that has been still is
    /// due at once, and a second present within the period is not.
    #[test]
    fn a_still_window_is_due_at_once_and_a_busy_one_is_spaced() {
        let now = Instant::now();
        let period = Duration::from_millis(8);
        let mut cadence = at(period, now);
        assert!(
            cadence.due(now),
            "a window that has been still presents now"
        );
        cadence.presented(now);
        assert!(
            !cadence.due(now),
            "two presents in one refresh is a stutter"
        );
        assert!(!cadence.due(now + period / 2));
        assert!(cadence.due(now + period));
    }

    /// A frame that overran does not push every later tick by what it cost: the clock keeps its
    /// phase, so the ticks stay on the surface's own grid.
    #[test]
    fn an_overrun_frame_does_not_drift_the_clock() {
        let now = Instant::now();
        let period = Duration::from_millis(10);
        let mut cadence = at(period, now);
        cadence.presented(now);
        assert_eq!(cadence.next(), now + period);
        // A frame that took three and a half periods: the next tick is the fourth, not
        // three and a half periods after the fourth.
        let late = now + period * 3 + period / 2;
        cadence.presented(late);
        assert_eq!(cadence.next(), now + period * 4);
    }

    /// The floor is what stands in where the surface states nothing, and it is the rate
    /// `doc/todo/36` calls a floor rather than the one it calls a target.
    #[test]
    fn the_floor_is_sixty_hertz_and_says_so() {
        let cadence = Cadence::default();
        assert_eq!(cadence.period(), FLOOR);
        assert!(
            cadence.described().contains("floor"),
            "{}",
            cadence.described()
        );
        assert!(
            (1.0 / FLOOR.as_secs_f64() - 60.0).abs() < 0.01,
            "the floor is 60 Hz"
        );
        // 120 Hz is the target, and a surface that states it is presented to at it.
        let now = Instant::now();
        let target = at(Duration::from_nanos(1_000_000_000 / 120), now);
        assert!(target.described().contains("stated by the surface"));
        assert!((1.0 / target.period().as_secs_f64() - 120.0).abs() < 0.01);
    }

    /// The three sources are three different sentences and only one of them ends the asking.
    ///
    /// **This is the shape ADR 0384 needed and the old two-state flag could not hold.** A Wayland
    /// surface states no rate until it has been presented to, so the answer at `resumed` is either
    /// "some display says this" or "nothing does" — and a cadence that could not tell those two
    /// apart from the real answer had no way to know it should ask again.
    #[test]
    fn a_cadence_says_which_of_three_things_stated_its_rate() {
        let now = Instant::now();
        let period = Duration::from_nanos(1_000_000_000 / 120);
        for (source, phrase, settled) in [
            (Source::Window, "stated by the surface", true),
            (Source::Slowest, "slowest display attached", false),
            (Source::Floor, "floor stands in", false),
        ] {
            let cadence = Cadence {
                source,
                ..at(period, now)
            };
            assert!(
                cadence.described().contains(phrase),
                "{source:?}: {}",
                cadence.described()
            );
            assert_eq!(cadence.settled(), settled, "{source:?}");
        }
    }

    /// An obligation arms the clock, and arming it never brings a tick forward.
    #[test]
    fn an_obligation_arms_the_clock_without_bringing_a_tick_forward() {
        let now = Instant::now();
        let period = Duration::from_millis(8);
        let mut cadence = at(period, now);
        cadence.presented(now);
        let armed = cadence.next();
        cadence.owed(now);
        assert_eq!(cadence.next(), armed, "a tick already ahead is left alone");
        // A clock whose tick has passed is moved to the next one, so the loop has something to
        // wait until rather than a moment in the past.
        cadence.owed(now + period * 2);
        assert!(cadence.next() > now + period * 2);
    }
}
