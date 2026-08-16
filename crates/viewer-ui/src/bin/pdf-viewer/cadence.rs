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
//!   stutter machine `doc/todo/36` warns about. [`FLOOR`] stands in only where the surface
//!   states nothing, and the trace says which of the two happened.
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

/// When this window may next present, and how often it may.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cadence {
    /// One refresh of the surface.
    period: Duration,
    /// Whether [`Self::period`] is the surface's own number or [`FLOOR`] standing in for it.
    ///
    /// Kept so that the trace can say which, because "we present at 60 Hz" and "this display
    /// refreshes at 60 Hz" are two different claims and only one of them is measured.
    stated: bool,
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

impl Default for Cadence {
    /// The floor, due now — the cadence of a program that has no window yet.
    fn default() -> Self {
        Self {
            period: FLOOR,
            stated: false,
            due: Instant::now(),
            owing: false,
        }
    }
}

impl Cadence {
    /// The cadence of the surface this window is on.
    ///
    /// `winit` reports a monitor's refresh rate in millihertz and `None` where the platform does
    /// not know one — which is the ordinary answer on a virtual display, and the reason [`FLOOR`]
    /// exists rather than a panic. A rate of zero is the same answer written differently and is
    /// read the same way.
    pub(crate) fn of(window: &winit::window::Window) -> Self {
        let stated = window
            .current_monitor()
            .and_then(|monitor| monitor.refresh_rate_millihertz())
            .filter(|rate| *rate > 0)
            .map(|rate| {
                // Nanoseconds per refresh from millihertz, in integer arithmetic: the workspace
                // forbids unchecked arithmetic and a period computed in floating point would
                // have to justify its rounding as well as its value.
                Duration::from_nanos(
                    1_000_000_000_u64
                        .saturating_mul(MILLIHERTZ)
                        .checked_div(u64::from(rate))
                        .unwrap_or(0),
                )
            })
            .filter(|period| !period.is_zero());
        Self {
            period: stated.unwrap_or(FLOOR),
            stated: stated.is_some(),
            due: Instant::now(),
            owing: false,
        }
    }

    /// One refresh of the surface, which is the interval every present is spaced by.
    pub(crate) fn period(&self) -> Duration {
        self.period
    }

    /// How this cadence is to be described in a trace: the rate, and where it came from.
    ///
    /// Rule 6 of `doc/todo/36` in one sentence — a round claiming a rate says whether the number
    /// is the display's or this program's floor, because the two are different claims.
    pub(crate) fn described(&self) -> String {
        let hertz = 1.0 / self.period.as_secs_f64();
        if self.stated {
            format!(
                "{hertz:.1} Hz ({:.3} ms), stated by the surface",
                self.period.as_secs_f64() * 1e3
            )
        } else {
            format!(
                "{hertz:.1} Hz ({:.3} ms) — the surface states no refresh rate, so doc/todo/36's \
                 floor stands in",
                self.period.as_secs_f64() * 1e3
            )
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

    use super::{Cadence, FLOOR};

    /// A cadence with a period this test states, so that the arithmetic is not the machine's.
    fn at(period: Duration, now: Instant) -> Cadence {
        Cadence {
            period,
            stated: true,
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
