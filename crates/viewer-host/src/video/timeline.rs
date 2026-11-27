//! §13.2.5's play parameters and §13.2.4.3's section as arithmetic on a clip's pictures: where each
//! pass begins on the movie's timeline, where its pictures are cut, and how long it lasts on the
//! clock (ADR 3593).
//!
//! The sound is cut the same way by `crate::sound::passes`, in the device's frames; the pictures are
//! cut here in time, and both run on the one clock (ADR 3555), so a pass of the sound and a pass of
//! the pictures begin and end at the same instant. ISO 32000-2 §13.2.5 is the rule for both:
//!
//! > If the simple duration is longer than the intrinsic duration, the player shall freeze the
//! > media in its final state until the simple duration has elapsed.
//!
//! For pictures the final state is the last picture, held up; Table 291's `/RC` counts passes,
//! "non-integral values shall be permitted", so a count of 2.5 is two whole passes and a third cut
//! to half its length, as the sound's are.

use std::time::Duration;

use pdf_model::media::{Playing, Section, SimpleDuration};

/// A clip's passes, as its pictures are played.
#[derive(Debug, Clone)]
pub(super) struct Timeline {
    /// The section's beginning on the movie's timeline: where every pass's decode starts.
    begin: Duration,
    /// The section's length where it states an end; `None` to the clip's own end.
    section: Option<Duration>,
    /// Table 291's `/D`.
    simple: Simple,
    /// The passes still owed after the one in hand: `None` for ever, otherwise what is left of the
    /// repeat count.
    left: Option<f64>,
    /// The fraction of a whole pass the one in hand lasts.
    fraction: f64,
    /// The section's length on the pictures, learned when the first pass's pictures ended: §13.2.5's
    /// intrinsic duration, which a pass of the intrinsic duration lasts.
    intrinsic: Option<Duration>,
}

/// Table 291's simple duration, as a length of time.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Simple {
    /// The clip's own length.
    Intrinsic,
    /// For ever: the pictures once, then the last held until the clip is stopped.
    Infinite,
    /// A length the document states.
    Stated(Duration),
}

impl Timeline {
    /// The passes `playing` asks of a clip's pictures, the first one in hand.
    pub(super) fn new(playing: &Playing) -> Self {
        let Section { begin, end } = playing.section;
        let begin = seconds(begin);
        let section = end.map(|end| seconds(end).saturating_sub(begin));
        let simple = match playing.duration {
            SimpleDuration::Intrinsic => Simple::Intrinsic,
            SimpleDuration::Infinite => Simple::Infinite,
            SimpleDuration::Seconds(stated) => Simple::Stated(seconds(stated)),
        };
        let mut timeline = Self {
            begin,
            section,
            simple,
            left: (playing.repeat > 0.0).then_some(playing.repeat),
            fraction: 1.0,
            intrinsic: None,
        };
        // A count of zero passes has nothing to show; the first pass is taken as the sound's is.
        if timeline.next_pass().is_none() {
            timeline.fraction = 0.0;
        }
        timeline
    }

    /// Where every pass's decode begins, on the movie's timeline.
    pub(super) fn begin(&self) -> Duration {
        self.begin
    }

    /// The section's end on the movie's timeline, where it states one: where a pass played
    /// backward begins (ADR 3837).
    pub(super) fn end(&self) -> Option<Duration> {
        self.section
            .map(|section| self.begin.saturating_add(section))
    }

    /// A picture's time in its pass, from its time on the movie's timeline.
    pub(super) fn in_pass(&self, at: Duration) -> Duration {
        at.saturating_sub(self.begin)
    }

    /// How long the pass in hand lasts on the clock, where that is known now: a stated duration's
    /// share, or the intrinsic duration's once learned. `None` for an infinite duration, which
    /// ends only when the clip is stopped, and for an intrinsic one not learned yet.
    pub(super) fn length(&self) -> Option<Duration> {
        match self.simple {
            Simple::Infinite => None,
            Simple::Stated(stated) => Some(stated.mul_f64(self.fraction)),
            Simple::Intrinsic => self.intrinsic.map(|whole| whole.mul_f64(self.fraction)),
        }
    }

    /// The time in the pass from which no picture is shown: the section's end, or the pass's
    /// length where it is shorter. A picture at or past it is cut, and the pass's pictures are over.
    pub(super) fn cut(&self) -> Option<Duration> {
        match (self.section, self.length()) {
            (Some(section), Some(length)) => Some(section.min(length)),
            (one, other) => one.or(other),
        }
    }

    /// When the pass in hand ends, in its own time, once its pictures ended at `pictures_end` —
    /// the last picture's time and interval, or the cut. A stated duration longer than the
    /// pictures holds the last one up until it has elapsed; an intrinsic one lasts as long as the
    /// pictures, or its fraction of them; an infinite one never ends.
    pub(super) fn ends(&self, pictures_end: Duration) -> Option<Duration> {
        match self.simple {
            Simple::Infinite => None,
            Simple::Stated(_) => self.length(),
            Simple::Intrinsic => Some(self.length().unwrap_or_else(|| {
                // The first pass of the intrinsic duration: its own pictures are its length.
                let whole = self
                    .section
                    .map_or(pictures_end, |section| section.min(pictures_end));
                whole.mul_f64(self.fraction)
            })),
        }
    }

    /// The pass in hand ended with its pictures ending at `pictures_end`: the first teaches the
    /// intrinsic duration. Answers whether another pass is owed, and takes it.
    pub(super) fn pass_over(&mut self, pictures_end: Duration) -> bool {
        if self.intrinsic.is_none() {
            self.intrinsic = Some(
                self.section
                    .map_or(pictures_end, |section| section.min(pictures_end)),
            );
        }
        self.next_pass().is_some()
    }

    /// Whether the first pass is a fraction of the intrinsic duration, which is not known until the
    /// pictures of that pass have been decoded: its pictures are then shown whole.
    pub(super) fn first_fraction_unknown(&self) -> bool {
        matches!(self.simple, Simple::Intrinsic) && self.fraction < 1.0 && self.section.is_none()
    }

    /// The next pass's fraction taken, or `None` once the repeat count is spent; a pass of no
    /// length ends the clip even under a count of zero, as a sound's does.
    fn next_pass(&mut self) -> Option<f64> {
        let fraction = match &mut self.left {
            None => 1.0,
            Some(left) if *left <= 0.0 => return None,
            Some(left) => {
                let fraction = left.min(1.0);
                *left -= 1.0;
                fraction
            }
        };
        if matches!(self.simple, Simple::Infinite) {
            // An infinite duration: one pass, held for ever.
            self.left = Some(0.0);
        }
        self.fraction = fraction;
        if self.length() == Some(Duration::ZERO) || self.section == Some(Duration::ZERO) {
            self.left = Some(0.0);
            return None;
        }
        Some(fraction)
    }
}

/// A number of seconds as a span, held to zero and to what a span counts: §13.2.6.3's timespans
/// are never negative, and the reading in `pdf_model::media` has already refused one that is.
fn seconds(seconds: f64) -> Duration {
    Duration::try_from_secs_f64(seconds.max(0.0)).unwrap_or(Duration::MAX)
}

#[cfg(test)]
#[expect(
    clippy::cast_precision_loss,
    reason = "a test's count of frames, a few seconds' worth"
)]
mod tests {
    use super::*;

    fn playing(section: Section, duration: SimpleDuration, repeat: f64) -> Playing {
        Playing {
            section,
            duration,
            repeat,
            ..Playing::default()
        }
    }

    const fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// A section from 1 s to 3 s: every pass decodes from 1 s, a picture's time in its pass is its
    /// time less 1 s, and the pictures are cut at 2 s into the pass.
    #[test]
    fn a_section_places_the_pass_on_the_movie_s_timeline() {
        let timeline = Timeline::new(&playing(
            Section {
                begin: 1.0,
                end: Some(3.0),
            },
            SimpleDuration::Intrinsic,
            1.0,
        ));
        assert_eq!(timeline.begin(), ms(1000));
        assert_eq!(timeline.in_pass(ms(1400)), ms(400));
        assert_eq!(timeline.cut(), Some(ms(2000)));
        assert_eq!(timeline.ends(ms(2000)), Some(ms(2000)));
    }

    /// A stated duration shorter than the pictures cuts them; one longer holds the last picture up
    /// until it has elapsed; an infinite one never ends.
    #[test]
    fn a_stated_duration_cuts_or_freezes_the_pictures() {
        let short = Timeline::new(&playing(Section::WHOLE, SimpleDuration::Seconds(0.5), 1.0));
        assert_eq!(short.cut(), Some(ms(500)));
        assert_eq!(short.ends(ms(500)), Some(ms(500)));
        let long = Timeline::new(&playing(Section::WHOLE, SimpleDuration::Seconds(5.0), 1.0));
        assert_eq!(long.ends(ms(1200)), Some(ms(5000)));
        let endless = Timeline::new(&playing(Section::WHOLE, SimpleDuration::Infinite, 3.0));
        assert_eq!(endless.ends(ms(1200)), None);
        assert_eq!(endless.cut(), None);
    }

    /// A repeat count of 2.5 under the intrinsic duration: two whole passes and a third cut to half
    /// of what the first pass's pictures taught.
    #[test]
    fn a_fractional_repeat_cuts_the_last_pass() {
        let mut timeline = Timeline::new(&playing(Section::WHOLE, SimpleDuration::Intrinsic, 2.5));
        assert_eq!(timeline.cut(), None, "the first pass's length is not known");
        assert!(timeline.pass_over(ms(1200)));
        assert_eq!(timeline.cut(), Some(ms(1200)));
        assert!(timeline.pass_over(ms(1200)));
        assert_eq!(timeline.cut(), Some(ms(600)));
        assert_eq!(timeline.ends(ms(600)), Some(ms(600)));
        assert!(!timeline.pass_over(ms(600)));
    }

    /// An empty section — "an empty range", which the clause calls "valid" — has no pass to show,
    /// and nor does a stated duration of nothing repeated for ever.
    #[test]
    fn an_empty_section_or_duration_shows_nothing() {
        let mut empty = Timeline::new(&playing(
            Section {
                begin: 2.0,
                end: Some(2.0),
            },
            SimpleDuration::Intrinsic,
            0.0,
        ));
        assert_eq!(empty.cut(), Some(Duration::ZERO));
        assert!(!empty.pass_over(Duration::ZERO));
        let nothing = Timeline::new(&playing(Section::WHOLE, SimpleDuration::Seconds(0.0), 0.0));
        assert_eq!(nothing.cut(), Some(Duration::ZERO));
    }

    /// The pictures and the sound agree at every cut: for a section of 0.25 s to 1.75 s, a stated
    /// duration of 0.4 s and a repeat count of 2.5, the sound's passes — `crate::sound::passes`,
    /// in the frames of a 48 000 Hz device — and the pictures' passes end at the same times on the
    /// one clock (ADR 3555). The wrong answer excluded: pictures that ran the section whole while
    /// the sound was cut, which this player did before ADR 3593.
    #[test]
    fn the_pictures_and_the_sound_agree_at_every_cut() {
        let rate = 48_000_u32;
        let stated = playing(
            Section {
                begin: 0.25,
                end: Some(1.75),
            },
            SimpleDuration::Seconds(0.4),
            2.5,
        );
        // The sound's section, 1.5 s of frames, as `sound::sectioned` leaves it.
        let intrinsic = 72_000;
        let mut sound = crate::sound::passes::Passes::new(&stated, intrinsic, rate);
        let mut pictures = Timeline::new(&stated);
        let mut sound_ends = Vec::new();
        let mut picture_ends = Vec::new();
        let mut clock = Duration::ZERO;
        let mut frames = 0_u64;
        while let Some(pass) = sound.next_pass() {
            frames += pass.sounding + pass.frozen.unwrap_or(0);
            sound_ends.push(Duration::from_secs_f64(frames as f64 / f64::from(rate)));
        }
        loop {
            let cut = pictures.cut().expect("a stated duration is known");
            let end = pictures.ends(cut).expect("a stated duration ends");
            clock += end;
            picture_ends.push(clock);
            if !pictures.pass_over(cut) {
                break;
            }
        }
        assert_eq!(picture_ends, sound_ends);
        assert_eq!(picture_ends, vec![ms(400), ms(800), ms(1000)]);
    }
}
