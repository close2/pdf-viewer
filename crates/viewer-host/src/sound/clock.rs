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

/// How far past the device's last word on what it had reached a heard time runs on the monotonic
/// clock: two of the helper's report steps (a twentieth of a second each), so a device that stops
/// taking frames — an underrun, a stall — stops a picture clock slaved to it within that (ADR 3555).
pub const HEARD_REACH: Duration = Duration::from_millis(100);

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
    /// The device's last word on the frames of the pass now sounding that it had reached, and when
    /// it arrived (ADR 3555).
    heard: Option<(u64, Instant)>,
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
            heard: None,
        }
    }

    /// The device's count for the pass now sounding. A count below one already read is a report
    /// overtaken on its way and moves nothing: the clock never runs backwards.
    pub fn played(&mut self, frames: u64) {
        self.sounding = self.sounding.max(frames);
    }

    /// The device's own count of the frames of the pass it had reached when it was paused, which
    /// replaces the reports: a report counts frames written, which run ahead of those heard by the
    /// frames the device still held, so this one may be lower and is taken as it is (ADR 3458).
    pub fn reached(&mut self, frames: u64) {
        self.sounding = frames;
        self.heard = None;
    }

    /// The device had reached `frames` of the pass now sounding as of `now`: written, less what it
    /// still held unplayed (`quorra_sound::Progress::Heard`).
    pub fn heard(&mut self, frames: u64, now: Instant) {
        self.heard = Some((frames, now));
    }

    /// The pass now sounding is over after `frames` — its whole length, which the device's last
    /// report may have fallen short of by a period.
    pub fn sounded(&mut self, frames: u64) {
        self.over = self.over.saturating_add(frames);
        self.sounding = 0;
        self.heard = None;
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

    /// The sound is paused at `now` (ISO 32000-2 §12.6.4.14, Table 218's `/OP` 2): the device's
    /// count for the pass and the part of a freeze that has lasted are folded into the frames
    /// over, the freeze ends, and the two are answered — what the pass had sounded, which is where
    /// a resumption starts in its samples, and what the freeze had lasted, which comes off the
    /// freeze's remainder. The clock then stands still until a pass or a freeze starts again.
    pub fn pause(&mut self, now: Instant) -> (u64, u64) {
        let sounded = std::mem::take(&mut self.sounding);
        self.heard = None;
        let frozen = self.frozen.take().map_or(0, |began| self.since(began, now));
        self.over = self.over.saturating_add(sounded).saturating_add(frozen);
        (sounded, frozen)
    }

    /// The frames since the sound started, at `now`.
    #[must_use]
    pub fn frames(&self, now: Instant) -> u64 {
        let frozen = self.frozen.map_or(0, |began| self.since(began, now));
        self.over
            .saturating_add(self.sounding)
            .saturating_add(frozen)
    }

    /// The time since the sound started, at `now`.
    #[must_use]
    pub fn time(&self, now: Instant) -> Duration {
        self.span(self.frames(now))
    }

    /// The time of the sound a listener hears at `now`: in a pass the device has said what it
    /// had reached, that count run on at the device's rate for at most [`HEARD_REACH`] since it
    /// was said; otherwise [`Self::time`]. This is what a clip's
    /// pictures are slaved to (ADR 3555): the written count runs ahead of the sound by the frames
    /// the device holds, a few tens of milliseconds, and the reports are a twentieth of a second
    /// apart.
    #[must_use]
    pub fn heard_time(&self, now: Instant) -> Duration {
        match (self.heard, self.frozen) {
            (Some((heard, at)), None) => {
                let since = now.saturating_duration_since(at).min(HEARD_REACH);
                let reached =
                    heard.saturating_add(self.since(at, at.checked_add(since).unwrap_or(at)));
                self.span(self.over.saturating_add(reached))
            }
            _ => self.time(now),
        }
    }

    /// The frames of this device from `began` to `now`, on the monotonic clock.
    fn since(&self, began: Instant, now: Instant) -> u64 {
        let nanos = now.saturating_duration_since(began).as_nanos();
        let frames = nanos.saturating_mul(u128::from(self.rate)) / 1_000_000_000;
        u64::try_from(frames).unwrap_or(u64::MAX)
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

    /// A pause folds the pass's count, or the part of a freeze that lasted, into the time, and the
    /// clock stands still until something starts again: a resumed pass counts from zero on top.
    #[test]
    fn a_pause_holds_the_time_where_it_was() {
        let mut clock = MediaClock::new(1000);
        let began = Instant::now();
        clock.played(300);
        assert_eq!(clock.pause(began), (300, 0));
        let later = began
            .checked_add(Duration::from_secs(5))
            .expect("five seconds later is representable");
        assert_eq!(clock.frames(later), 300, "a paused clock stands still");
        clock.played(200);
        assert_eq!(clock.frames(later), 500, "the resumed pass counts on top");
        clock.sounded(700);
        clock.freeze(began);
        let quarter = began
            .checked_add(Duration::from_millis(250))
            .expect("a quarter second later is representable");
        assert_eq!(clock.pause(quarter), (0, 250));
        assert_eq!(clock.frames(later), 1250);
        assert_eq!(clock.thaws_at(1), None, "no freeze lasts while paused");
    }

    /// The heard time is the device's word on what it had reached, run on at its rate for no more
    /// than `HEARD_REACH`, and the written count's time where no such word was said (ADR 3555).
    #[test]
    fn the_heard_time_runs_on_from_the_device_s_word_and_stops_past_its_reach() {
        let start = Instant::now();
        let mut clock = MediaClock::new(1000);
        clock.played(500);
        assert_eq!(clock.heard_time(start), Duration::from_millis(500));
        clock.heard(460, start);
        assert_eq!(clock.heard_time(start), Duration::from_millis(460));
        assert_eq!(
            clock.heard_time(start + Duration::from_millis(30)),
            Duration::from_millis(490)
        );
        assert_eq!(
            clock.heard_time(start + Duration::from_secs(5)),
            Duration::from_millis(460) + HEARD_REACH,
            "a device that said nothing more holds the clock past its reach"
        );
        clock.sounded(1000);
        assert_eq!(clock.heard_time(start), Duration::from_millis(1000));
    }
}
