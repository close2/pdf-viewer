//! The media clock: where in its time a sound is, read from the audio device's own count of the
//! frames it has taken (RFC 0015 section 5, step 6; ADR 3342).
//!
//! **The device's sample clock is the master.** A sound's time is the frames the device has
//! written divided by its rate — what the sound helper reports as `PLAYED` — and not the processor's
//! clock, which drifts from the device's by the two crystals' difference and has no idea of an
//! underrun. So a duration, a repeat and a later video are timed against what was heard rather
//! than against when it was asked for.
//!
//! **The one span the device cannot count is a freeze.** §13.2.5: "If the simple duration is
//! longer than the intrinsic duration, the player shall freeze the media in its final state until
//! the simple duration has elapsed. … For aural media types, the media is logically frozen but
//! shall not continue to produce sound." Nothing is written to the device then, so nothing is
//! counted, and the clock runs on the monotonic clock from the instant the device's count stopped,
//! for exactly the frames the freeze lasts — the frames a device would have taken, at its rate.

use std::time::{Duration, Instant};

/// One sound's clock, in the device's frames.
#[derive(Debug, Clone)]
pub struct MediaClock {
    /// The device's rate, frames per second.
    rate: u32,
    /// The frames of every pass already over, sounded and frozen.
    over: u64,
    /// The device's count for the pass now sounding.
    sounding: u64,
    /// Where a freeze began, while one lasts.
    frozen: Option<Instant>,
}

impl MediaClock {
    /// A clock at zero for a device of `rate` frames a second.
    #[must_use]
    pub fn new(rate: u32) -> Self {
        Self {
            rate: rate.max(1),
            over: 0,
            sounding: 0,
            frozen: None,
        }
    }

    /// The device's count for the pass now sounding. A count below one already read is a report
    /// overtaken on its way and moves nothing: the clock never runs backwards.
    pub fn played(&mut self, frames: u64) {
        self.sounding = self.sounding.max(frames);
    }

    /// The pass now sounding is over after `frames` — its whole length, which the device's last
    /// report may have fallen short of by a period.
    pub fn sounded(&mut self, frames: u64) {
        self.over = self.over.saturating_add(frames);
        self.sounding = 0;
    }

    /// A freeze begins at `now`.
    pub fn freeze(&mut self, now: Instant) {
        self.frozen = Some(now);
    }

    /// The freeze lasted `frames` and is over: the clock moves by exactly that, whatever the
    /// monotonic clock read when it was noticed.
    pub fn thawed(&mut self, frames: u64) {
        self.frozen = None;
        self.over = self.over.saturating_add(frames);
    }

    /// When a freeze of `frames` that began at [`Self::freeze`]'s instant ends; `None` while no
    /// freeze lasts, or where the end is past what an `Instant` can hold, which is never.
    #[must_use]
    pub fn thaws_at(&self, frames: u64) -> Option<Instant> {
        self.frozen?.checked_add(self.span(frames))
    }

    /// The frames since the sound started, at `now`.
    #[must_use]
    pub fn frames(&self, now: Instant) -> u64 {
        let frozen = self.frozen.map_or(0, |began| {
            let nanos = now.saturating_duration_since(began).as_nanos();
            let frames = nanos.saturating_mul(u128::from(self.rate)) / 1_000_000_000;
            u64::try_from(frames).unwrap_or(u64::MAX)
        });
        self.over
            .saturating_add(self.sounding)
            .saturating_add(frozen)
    }

    /// The time since the sound started, at `now`.
    #[must_use]
    pub fn time(&self, now: Instant) -> Duration {
        self.span(self.frames(now))
    }

    /// `frames` of this device as a span of time.
    fn span(&self, frames: u64) -> Duration {
        let nanos = u128::from(frames)
            .saturating_mul(1_000_000_000)
            .checked_div(u128::from(self.rate))
            .unwrap_or_default();
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The device's counts are the clock: 4000 frames of an 8000 Hz device are half a second, a
    /// late report below the last moves nothing, and a pass's end counts its whole length.
    #[test]
    fn the_device_s_count_is_the_time() {
        let mut clock = MediaClock::new(8000);
        let now = Instant::now();
        clock.played(4000);
        assert_eq!(clock.time(now), Duration::from_millis(500));
        clock.played(3000);
        assert_eq!(clock.frames(now), 4000);
        clock.sounded(8000);
        assert_eq!(clock.time(now), Duration::from_secs(1));
        clock.played(800);
        assert_eq!(clock.time(now), Duration::from_millis(1100));
    }

    /// A freeze runs on the monotonic clock while it lasts, and moves the clock by exactly its own
    /// length when it ends.
    #[test]
    fn a_freeze_is_counted_at_the_device_s_rate() {
        let mut clock = MediaClock::new(1000);
        let began = Instant::now();
        clock.sounded(500);
        clock.freeze(began);
        let later = began
            .checked_add(Duration::from_millis(250))
            .expect("a quarter second later is representable");
        assert_eq!(clock.frames(later), 750);
        assert_eq!(
            clock.thaws_at(1500),
            began.checked_add(Duration::from_millis(1500))
        );
        clock.thawed(1500);
        assert_eq!(clock.thaws_at(1), None);
        assert_eq!(clock.time(later), Duration::from_secs(2));
    }
}
