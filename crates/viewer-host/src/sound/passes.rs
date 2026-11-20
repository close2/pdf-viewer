//! §13.2.5's play parameters as arithmetic on a converted sound: the level each sample is scaled
//! by, and the passes the simple duration and the repeat count cut it into (ADR 3341).
//!
//! A pass is one iteration of Table 291's `/D`: the frames that sound, cut to the duration where
//! it is shorter than the sound, and the frames frozen after them where it is longer — "the media
//! is logically frozen but shall not continue to produce sound". Table 291's `/RC` counts passes,
//! "Negative values shall be invalid; non-integral values shall be permitted … Zero means repeat
//! forever", so a count of 2.5 is two whole passes and a third cut to half the duration.

use pdf_model::media::{Playing, SimpleDuration};

/// One iteration of the simple duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Pass {
    /// The frames from the sound's start that sound.
    pub(super) sounding: u64,
    /// The frames frozen after them, in silence; `None` for ever, where the duration is infinite.
    pub(super) frozen: Option<u64>,
}

/// The passes one sound is played as, in order.
#[derive(Debug, Clone)]
pub(super) struct Passes {
    /// The sound's own length: §13.2.5's intrinsic duration, in frames.
    intrinsic: u64,
    /// One pass's length in frames, `None` where Table 292's `/S` is `F`.
    simple: Option<u64>,
    /// The passes still owed: `None` for ever, otherwise what is left of the repeat count.
    left: Option<f64>,
}

impl Passes {
    /// The passes `playing` asks of a sound `intrinsic` frames long, on a device of `rate`.
    pub(super) fn new(playing: &Playing, intrinsic: u64, rate: u32) -> Self {
        let simple = match playing.duration {
            SimpleDuration::Intrinsic => Some(intrinsic),
            SimpleDuration::Infinite => None,
            SimpleDuration::Seconds(seconds) => Some(frames_of(seconds * f64::from(rate))),
        };
        Self {
            intrinsic,
            simple,
            left: (playing.repeat > 0.0).then_some(playing.repeat),
        }
    }

    /// The next pass, or `None` once the repeat count is spent. A pass of no length ends the sound
    /// even under a count of zero: a duration of nothing repeated for ever is still nothing, and a
    /// player that looped on it would spin without a frame reaching the device.
    pub(super) fn next_pass(&mut self) -> Option<Pass> {
        let fraction = match &mut self.left {
            None => 1.0,
            Some(left) if *left <= 0.0 => return None,
            Some(left) => {
                let fraction = left.min(1.0);
                *left -= 1.0;
                fraction
            }
        };
        let Some(simple) = self.simple else {
            // An infinite duration: the sound once, then frozen until it is stopped.
            self.left = Some(0.0);
            return Some(Pass {
                sounding: self.intrinsic,
                frozen: None,
            });
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a frame count past 2⁵³ is past `MAX_PLAYED_SAMPLES` and every duration a \
                      person waits for"
        )]
        let length = if fraction < 1.0 {
            frames_of(simple as f64 * fraction)
        } else {
            simple
        };
        if length == 0 {
            self.left = Some(0.0);
            return None;
        }
        let sounding = length.min(self.intrinsic);
        Some(Pass {
            sounding,
            frozen: Some(length.saturating_sub(sounding)),
        })
    }
}

/// A number of frames, rounded to the nearest and held to what a `u64` counts.
fn frames_of(frames: f64) -> u64 {
    if frames.is_nan() || frames <= 0.0 {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive and rounded, and `as` saturates at `u64::MAX` past it"
    )]
    let frames = frames.round() as u64;
    frames
}

/// Table 291's `/V` applied: each sample scaled by the level and held to full scale, since a level
/// above the recorded one asks for more than a sample can carry. Nothing is done at the default.
pub(super) fn levelled(samples: &mut [f32], level: f64) {
    if (level - 1.0).abs() < f64::EPSILON {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a level is a small number, and an `f32` sample's precision is the one played"
    )]
    let level = level as f32;
    for sample in samples {
        *sample = (*sample * level).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing(duration: SimpleDuration, repeat: f64) -> Playing {
        Playing {
            duration,
            repeat,
            ..Playing::default()
        }
    }

    fn all(mut passes: Passes) -> Vec<Pass> {
        std::iter::from_fn(|| passes.next_pass()).take(10).collect()
    }

    /// Table 291's defaults: the whole sound, once, nothing frozen.
    #[test]
    fn the_defaults_are_one_whole_pass() {
        let passes = Passes::new(&Playing::default(), 100, 8000);
        assert_eq!(
            all(passes),
            [Pass {
                sounding: 100,
                frozen: Some(0)
            }]
        );
    }

    /// A duration of 5 ms at 8000 Hz is 40 frames: a 100-frame sound is cut to 40, and a 10-frame
    /// one sounds 10 and is frozen for 30 (§13.2.5's freeze).
    #[test]
    fn a_duration_cuts_a_longer_sound_and_freezes_a_shorter_one() {
        let short = playing(SimpleDuration::Seconds(0.005), 1.0);
        assert_eq!(
            all(Passes::new(&short, 100, 8000)),
            [Pass {
                sounding: 40,
                frozen: Some(0)
            }]
        );
        assert_eq!(
            all(Passes::new(&short, 10, 8000)),
            [Pass {
                sounding: 10,
                frozen: Some(30)
            }]
        );
    }

    /// `/RC 2.5` over a 40-frame duration: two whole passes and a third of 20 frames. NOTE 2's
    /// "the entire play-pause sequence is repeated RC times" holds for a frozen tail too.
    #[test]
    fn a_repeat_count_is_whole_passes_and_a_fraction() {
        let repeated = playing(SimpleDuration::Seconds(0.005), 2.5);
        let whole = Pass {
            sounding: 40,
            frozen: Some(0),
        };
        assert_eq!(
            all(Passes::new(&repeated, 100, 8000)),
            [
                whole,
                whole,
                Pass {
                    sounding: 20,
                    frozen: Some(0)
                }
            ]
        );
        let frozen = Pass {
            sounding: 10,
            frozen: Some(30),
        };
        assert_eq!(
            all(Passes::new(
                &playing(SimpleDuration::Seconds(0.005), 2.0),
                10,
                8000
            )),
            [frozen, frozen]
        );
    }

    /// "Zero means repeat forever": the passes do not run out; an infinite duration is one pass
    /// frozen for ever; and a duration of nothing ends even under a count of zero.
    #[test]
    fn zero_is_for_ever_and_nothing_is_nothing() {
        let forever = Passes::new(&playing(SimpleDuration::Intrinsic, 0.0), 8, 8000);
        assert_eq!(all(forever).len(), 10);
        assert_eq!(
            all(Passes::new(
                &playing(SimpleDuration::Infinite, 3.0),
                8,
                8000
            )),
            [Pass {
                sounding: 8,
                frozen: None
            }]
        );
        assert_eq!(
            all(Passes::new(
                &playing(SimpleDuration::Seconds(0.0), 0.0),
                8,
                8000
            )),
            []
        );
    }

    /// `/V 50` halves each sample; `/V 200` doubles it and holds it to full scale; `/V 0` is mute.
    #[test]
    fn the_level_scales_each_sample() {
        let mut samples = [0.5, -0.25, 0.75];
        levelled(&mut samples, 0.5);
        assert_eq!(samples, [0.25, -0.125, 0.375]);
        let mut loud = [0.5, -0.75];
        levelled(&mut loud, 2.0);
        assert_eq!(loud, [1.0, -1.0]);
        let mut mute = [0.5];
        levelled(&mut mute, 0.0);
        assert_eq!(mute, [0.0]);
    }
}
