//! A document's script timers ticked, and its sounds said: what a window sends so that
//! `app.setInterval` and `app.setTimeOut` fire, and what it does with `app.beep` (ADR 1702).
//!
//! # Why the decision is here and not in a host
//!
//! `viewer-core` has no clock (`doc/ui-boundary.md`'s rule 3), so a timer a script sets is counted
//! down in the milliseconds a host's [`viewer_core::Command::Tick`] says passed, and the core
//! answers [`viewer_core::Query::TimerDue`] with how long a host may sleep before the next one is
//! owed. *When to look at a wall clock, what a tick carries and when to stop* are the same three
//! questions for every window — `glib::timeout_add_local_once`, a `QTimer` and winit's
//! `ControlFlow::WaitUntil` differ in every letter and agree about every one of them — so they are
//! answered once, here, for [`crate::Clock`]'s reason.
//!
//! # The rules
//!
//! - **Nothing is armed while no document holds a timer.** [`Ticker::wake`] answers `None` and
//!   forgets when it last counted, so a still window has no source armed and wakes for nothing, and
//!   the launch path is untouched: a document's open scripts run after the first present, so no
//!   timer can exist before it (`CLAUDE.md` principle 2).
//! - **One clock per window.** A presentation's [`crate::Clock`] already ticks the core with the
//!   time that passed, and two clocks each counting from their own last tick would tell the core
//!   that twice the time went by. So while a window presents, the presentation carries the time and
//!   this ticker stands down ([`Ticker::stand_down`]); timers then fire on the presentation's tenth
//!   of a second.
//! - **A tick carries the time that really passed**, measured from when the ticker began counting
//!   or last ticked, never an assumed step: a toolkit's timer fires late under load, and the core's
//!   count stays true.

use std::time::{Duration, Instant};

use pdf_model::view::Sound;
use viewer_core::{Answer, Query, Viewer};

/// When a window last told the core that time passed, for its script timers alone.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ticker {
    /// The moment counting began or the last tick was sent; `None` while no timer is held.
    since: Option<Instant>,
}

impl Ticker {
    /// How long the window may sleep before its next tick, asked after every command: `None` where
    /// no open document holds a timer, which is a window that arms nothing.
    ///
    /// Starts counting the first time a timer is held, so the time a timer waits is measured from
    /// the command that set it.
    pub fn wake(&mut self, viewer: &Viewer, now: Instant) -> Option<Duration> {
        let Answer::TimerDue(Some(due)) = viewer.query(Query::TimerDue) else {
            self.since = None;
            return None;
        };
        let since = *self.since.get_or_insert(now);
        let passed = now.saturating_duration_since(since);
        let wait = Duration::from_millis(u64::from(due)).saturating_sub(passed);
        // Up to the next whole millisecond: a toolkit's timer counts in them, and a wait rounded
        // down would wake before the tick it is for could carry one.
        let micros = u64::try_from(wait.as_micros()).unwrap_or(u64::MAX);
        Some(Duration::from_millis(micros.div_ceil(1000)))
    }

    /// The whole milliseconds the tick a window sends now carries, measured since it last
    /// counted; `None` where nothing was being counted.
    ///
    /// **The fraction of a millisecond stays counted.** The count moves on by exactly what the tick
    /// carries, never to `now`: a tick that dropped its remainder would tell the core 299 of a
    /// timer's 300 and then 0 for ever after, a window spinning on ticks of nothing.
    pub fn tick(&mut self, now: Instant) -> Option<u32> {
        let since = self.since?;
        let passed = now.saturating_duration_since(since);
        let millis = u32::try_from(passed.as_millis()).unwrap_or(u32::MAX);
        self.since = Some(
            since
                .checked_add(Duration::from_millis(u64::from(millis)))
                .unwrap_or(now),
        );
        Some(millis)
    }

    /// Stops counting, because another clock is telling the core the time: a presentation's.
    pub fn stand_down(&mut self) {
        self.since = None;
    }
}

/// The line a window prints when it plays a script's `app.beep` (ADR 1702).
///
/// `toolkit` names what played it. The reference numbers five sounds and lets a system play one
/// for all of them; a toolkit with one sound says which was asked for and that its one was played.
#[must_use]
pub fn played(document: &str, sound: Sound, toolkit: &str) -> String {
    format!(
        "a script in {document} asked for the {} sound; {toolkit} played its system sound \
         (ADR 1702)",
        sound.name()
    )
}

/// The line a window with no sound to play prints in place of a script's `app.beep`: the refusal
/// by name, never silence (ADR 1702).
#[must_use]
pub fn unplayed(document: &str, sound: Sound, why: &str) -> String {
    format!(
        "a script in {document} asked for the {} sound, and none was played: {why} (ADR 1702)",
        sound.name()
    )
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Ticker;

    /// A tick carries the time since counting began, then since the last tick; a ticker that was
    /// never counting carries nothing.
    #[test]
    fn a_tick_carries_the_time_since_the_ticker_last_counted() {
        let start = Instant::now();
        let mut ticker = Ticker::default();
        assert_eq!(ticker.tick(start), None, "nothing was being counted");
        ticker.since = Some(start);
        assert_eq!(ticker.tick(start + Duration::from_millis(250)), Some(250));
        assert_eq!(ticker.tick(start + Duration::from_millis(400)), Some(150));
        ticker.stand_down();
        assert_eq!(ticker.tick(start + Duration::from_millis(900)), None);
    }

    /// A fraction of a millisecond is carried by the next tick rather than lost, so ticks a
    /// toolkit sends at sub-millisecond spacing still add up to the time that passed.
    #[test]
    fn the_fraction_of_a_millisecond_is_carried_to_the_next_tick() {
        let start = Instant::now();
        let mut ticker = Ticker { since: Some(start) };
        assert_eq!(
            ticker.tick(start + Duration::from_micros(299_900)),
            Some(299)
        );
        assert_eq!(ticker.tick(start + Duration::from_micros(300_100)), Some(1));
    }

    /// A window whose viewer holds no document holds no timer, so it arms nothing and counts
    /// nothing.
    #[test]
    fn a_window_with_no_timer_arms_nothing() {
        let viewer = viewer_core::Viewer::new(800, 600, 1.0);
        let mut ticker = Ticker::default();
        assert_eq!(ticker.wake(&viewer, Instant::now()), None);
        assert_eq!(ticker.tick(Instant::now()), None);
    }
}
