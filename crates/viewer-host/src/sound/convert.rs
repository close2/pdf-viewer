//! ISO 32000-2 §13.3's last sentence, done for the device a window plays on:
//!
//! > Sound players shall convert between formats, downsample rates, and combine channels as
//! > necessary to render sound on the target platform.
//!
//! The media worker has already done the first verb as far as the document's encodings go: every
//! sample of Table 305's four encodings arrives as one `i32` with the sample in its most significant
//! bits (ADR 3297). What is left depends on the device, which is why it is done here, in the host
//! beside it, and not in the worker: the device's sample format, its rate, and its channel count.
//!
//! Each step is the plainest construction that the clause's words describe (ADR 3301):
//!
//! - **Format**: a sample becomes a fraction of full scale, `sample / 2³¹`, in `[-1, 1)`. A sink
//!   writes that in whatever form its device takes.
//! - **Channels**: the clause names no channel positions — Table 305's `/C` is a count — so the
//!   arrangement is a choice, stated: channel `c` of a frame lands on the device's channel
//!   `c mod m`, where `m` is the device's count, and where several land on one they are averaged
//!   ("combine"). Where the device has more channels than the sound, device channel `d` repeats the
//!   sound's channel `d mod n`. A mono sound therefore sounds on every speaker, stereo keeps left on
//!   the device's first channel and right on its second — the first two in WAVE's and ALSA's order
//!   alike — and a four-channel sound on a stereo device sums front and rear of each side.
//! - **Rate**: where the device is slower than the sound ("downsample"), each output frame is the
//!   mean of the input frames its interval covers, a box filter, so that the samples dropped are
//!   averaged in rather than skipped; where it is faster, each output frame is the straight line
//!   between the two input frames either side of it. Positions are exact rationals of the two
//!   rates, so no drift accumulates over a long sound.
//!
//! None of this is a resampler a studio would choose; a box filter lets through more of what lies
//! above the new rate's limit than a windowed sinc does. It is what the sentence asks, its result
//! can be written down by hand, which the tests do, and §13.3's formats ("8000, 11,025, or 22,050
//! samples per second") are all *raised* to a modern device's rate, which is the straight-line
//! half.

/// `2³¹`: the worker's top-aligned `i32` divided by this is a fraction of full scale.
const FULL_SCALE: f64 = 2_147_483_648.0;

/// A device's rate and channel count: what a sound is converted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceFormat {
    /// Frames per second.
    pub rate: u32,
    /// Samples per frame.
    pub channels: u16,
}

/// The most samples a sound converted for a device may hold.
///
/// Sixteen mebisamples, 64 MiB of `f32`: six minutes of 22,050 Hz stereo, §13.3's largest
/// recommended format, at 48,000 Hz. A sound longer than that is a document asking this window
/// to hold more than a sound object was ever meant to be, and is said rather than played.
pub const MAX_PLAYED_SAMPLES: usize = 1 << 24;

/// Why a sound was not converted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConversionError {
    /// The sound states no rate or no channels; the worker never answers one, so this is a caller's
    /// mistake, said rather than divided by.
    #[error("the sound states a rate of {rate} and {channels} channel(s), which hold no frame")]
    NoSound {
        /// Its rate.
        rate: u32,
        /// Its channels.
        channels: u16,
    },
    /// The device states no rate or no channels.
    #[error("the audio device states a rate of {rate} and {channels} channel(s)")]
    NoDevice {
        /// Its rate.
        rate: u32,
        /// Its channels.
        channels: u16,
    },
    /// The converted sound would pass the bound.
    #[error("the sound converts to {samples} samples for this device, past the {limit} it holds")]
    TooLarge {
        /// The samples it would take.
        samples: u64,
        /// The bound.
        limit: usize,
    },
}

/// `samples`, interleaved at `rate` and `channels` and each in the most significant bits of an
/// `i32`, converted to `device` as fractions of full scale, under `limit` samples.
///
/// # Errors
///
/// [`ConversionError`]: a sound or device holding no frame, or a result past `limit`.
pub fn converted(
    rate: u32,
    channels: u16,
    samples: &[i32],
    device: DeviceFormat,
    limit: usize,
) -> Result<Vec<f32>, ConversionError> {
    if rate == 0 || channels == 0 {
        return Err(ConversionError::NoSound { rate, channels });
    }
    if device.rate == 0 || device.channels == 0 {
        return Err(ConversionError::NoDevice {
            rate: device.rate,
            channels: device.channels,
        });
    }
    let frames_in = samples
        .len()
        .checked_div(usize::from(channels))
        .unwrap_or(0);
    let frames_out = frames_after(frames_in, rate, device.rate);
    let wanted = frames_out.saturating_mul(u64::from(device.channels));
    if usize::try_from(wanted).map_or(true, |wanted| wanted > limit) {
        return Err(ConversionError::TooLarge {
            samples: wanted,
            limit,
        });
    }
    let frames: Vec<Vec<f64>> = samples
        .chunks_exact(usize::from(channels))
        .map(|frame| arranged(frame, device.channels))
        .collect();
    let mut out = Vec::with_capacity(usize::try_from(wanted).unwrap_or(0));
    for k in 0..frames_out {
        for channel in 0..usize::from(device.channels) {
            let value = if device.rate < rate {
                averaged(&frames, channel, k, rate, device.rate)
            } else {
                interpolated(&frames, channel, k, rate, device.rate)
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the conversion's one narrowing: an `f32` holds every value a 24-bit \
                          device sample can tell apart, which is the most any device takes"
            )]
            out.push(value as f32);
        }
    }
    Ok(out)
}

/// The frames `frames` input frames at `from` become at `to`: the duration kept, a partial last
/// frame counted whole so that the sound's end is not cut.
fn frames_after(frames: usize, from: u32, to: u32) -> u64 {
    u64::try_from(frames)
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::from(to))
        .div_ceil(u64::from(from))
}

/// One frame's samples as fractions of full scale, arranged onto `device` channels.
fn arranged(frame: &[i32], device: u16) -> Vec<f64> {
    let device = usize::from(device);
    (0..device)
        .map(|target| {
            if frame.len() >= device {
                // Combined: every channel that lands here, averaged.
                let landing: Vec<f64> = frame
                    .iter()
                    .skip(target)
                    .step_by(device)
                    .map(|sample| f64::from(*sample) / FULL_SCALE)
                    .collect();
                // `landing` holds at least channel `target` itself, because `target < device <=
                // frame.len()`.
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a count of channels, at most 65,535, is exact in an `f64`"
                )]
                let count = landing.len() as f64;
                landing.iter().sum::<f64>() / count
            } else {
                // Repeated: the device has channels the sound does not.
                target
                    .checked_rem(frame.len())
                    .and_then(|source| frame.get(source))
                    .map_or(0.0, |sample| f64::from(*sample) / FULL_SCALE)
            }
        })
        .collect()
}

/// Output frame `k`'s `channel` where the device is slower: the mean of the input frames from
/// `⌊k·from/to⌋` up to, not including, `⌊(k+1)·from/to⌋`, as many of them as the sound holds.
fn averaged(frames: &[Vec<f64>], channel: usize, k: u64, from: u32, to: u32) -> f64 {
    let at = |k: u64| {
        let position = k
            .saturating_mul(u64::from(from))
            .checked_div(u64::from(to))
            .unwrap_or(0);
        usize::try_from(position).unwrap_or(usize::MAX)
    };
    let start = at(k).min(frames.len());
    let end = at(k.saturating_add(1)).min(frames.len()).max(start);
    let covered = frames.get(start..end).unwrap_or(&[]);
    if covered.is_empty() {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a run of frames an output frame covers is a rate ratio, at most 2³² and exact \
                  in an `f64`"
    )]
    let count = covered.len() as f64;
    covered
        .iter()
        .map(|frame| frame.get(channel).copied().unwrap_or(0.0))
        .sum::<f64>()
        / count
}

/// Output frame `k`'s `channel` where the device is as fast or faster: on the straight line from
/// input frame `⌊k·from/to⌋` to the next, the last frame held past the sound's end.
fn interpolated(frames: &[Vec<f64>], channel: usize, k: u64, from: u32, to: u32) -> f64 {
    let position = k.saturating_mul(u64::from(from));
    let index =
        usize::try_from(position.checked_div(u64::from(to)).unwrap_or(0)).unwrap_or(usize::MAX);
    let sample = |index: usize| {
        frames
            .get(index.min(frames.len().saturating_sub(1)))
            .and_then(|frame| frame.get(channel))
            .copied()
            .unwrap_or(0.0)
    };
    let here = sample(index);
    let next = sample(index.saturating_add(1));
    #[expect(
        clippy::cast_precision_loss,
        reason = "a remainder below the device's rate, a `u32`, is exact in an `f64`"
    )]
    let fraction = position.checked_rem(u64::from(to)).unwrap_or(0) as f64 / f64::from(to);
    here + (next - here) * fraction
}

#[cfg(test)]
mod tests {
    use super::{ConversionError, DeviceFormat, MAX_PLAYED_SAMPLES, converted};

    /// Full scale's fractions as the worker's top-aligned `i32`: `x · 2³¹`.
    fn top(fraction: f64) -> i32 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "every fraction the tests use is a multiple of 2⁻³¹ within [-1, 1)"
        )]
        let sample = (fraction * 2_147_483_648.0) as i32;
        sample
    }

    fn device(rate: u32, channels: u16) -> DeviceFormat {
        DeviceFormat { rate, channels }
    }

    /// "convert between formats": `sample / 2³¹`, so the most negative sample is −1, half scale
    /// one half, and zero zero. The wrong answer excluded: dividing by `i32::MAX`, which puts the
    /// most negative sample just past −1.
    #[test]
    fn a_sample_becomes_its_fraction_of_full_scale() {
        let out = converted(
            8000,
            1,
            &[i32::MIN, top(0.5), 0, top(-0.25)],
            device(8000, 1),
            64,
        );
        assert_eq!(out, Ok(vec![-1.0, 0.5, 0.0, -0.25]));
    }

    /// "combine channels": a stereo frame on a mono device is the mean of its two samples,
    /// `(0.5 + −0.25) / 2 = 0.125`, and the wrong answers excluded are the left channel alone and
    /// the sum.
    #[test]
    fn stereo_on_a_mono_device_is_the_mean_of_left_and_right() {
        let out = converted(8000, 2, &[top(0.5), top(-0.25)], device(8000, 1), 64);
        assert_eq!(out, Ok(vec![0.125]));
    }

    /// Four channels on a stereo device: channel `c` lands on `c mod 2`, so the device's left is
    /// `(c0 + c2) / 2 = (0.5 + 0.25) / 2` and its right `(c1 + c3) / 2 = (−0.5 + 0) / 2`.
    #[test]
    fn four_channels_on_two_combine_by_their_place_modulo_two() {
        let frame = [top(0.5), top(-0.5), top(0.25), 0];
        let out = converted(8000, 4, &frame, device(8000, 2), 64);
        assert_eq!(out, Ok(vec![0.375, -0.25]));
    }

    /// A mono sound sounds on every channel of the device: device channel `d` repeats the sound's
    /// `d mod 1`; the wrong answer excluded is a silent right channel.
    #[test]
    fn mono_on_a_stereo_device_sounds_on_both() {
        let out = converted(8000, 1, &[top(0.5), top(-0.5)], device(8000, 2), 64);
        assert_eq!(out, Ok(vec![0.5, 0.5, -0.5, -0.5]));
    }

    /// Raised from 8000 Hz to 16,000: twice the frames, each odd one halfway between its
    /// neighbours, and the last held past the end — `[0, 0.25, 0.5, 0.5]`.
    #[test]
    fn a_faster_device_draws_a_straight_line_between_frames() {
        let out = converted(8000, 1, &[0, top(0.5)], device(16_000, 1), 64);
        assert_eq!(out, Ok(vec![0.0, 0.25, 0.5, 0.5]));
    }

    /// 8000 Hz on a 48,000 Hz device, §13.3's µ-law rate on the commonest device: six output frames
    /// per input frame, at sixths of the way — the first frame's run is `0, 1/6 … 5/6` of the step
    /// to the second, here exact as multiples of `6/8 = 0.75` over six.
    #[test]
    fn eight_thousand_on_forty_eight_thousand_is_six_frames_per_frame() {
        let out = converted(8000, 1, &[0, top(0.75)], device(48_000, 1), 64).expect("converted");
        assert_eq!(out.len(), 12);
        let expected = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75];
        for (k, value) in expected.iter().enumerate() {
            let got = out.get(k).copied().unwrap_or(f32::NAN);
            assert!((f64::from(got) - value).abs() < 1e-9, "frame {k}: {got}");
        }
        assert!(out.iter().skip(6).all(|value| (*value - 0.75).abs() < 1e-9));
    }

    /// "downsample rates": halved, each output frame is the mean of the two it covers —
    /// `(0.5 + 0.25) / 2` and `(−0.5 + 0) / 2`; the wrong answer excluded is every other frame
    /// kept, `[0.5, −0.5]`.
    #[test]
    fn a_slower_device_averages_the_frames_each_output_frame_covers() {
        let input = [top(0.5), top(0.25), top(-0.5), 0];
        let out = converted(48_000, 1, &input, device(24_000, 1), 64);
        assert_eq!(out, Ok(vec![0.375, -0.25]));
    }

    /// Three frames to two (a rate ratio of 3 : 2): output frame 0 covers input `[0, 1)`, frame
    /// 1 covers `[1, 3)` — `⌊1·3/2⌋ = 1`, `⌊2·3/2⌋ = 3` — so `[0.5, (0.25 + −0.25) / 2]`.
    #[test]
    fn an_uneven_ratio_covers_whole_frames_by_exact_positions() {
        let input = [top(0.5), top(0.25), top(-0.25)];
        let out = converted(48_000, 1, &input, device(32_000, 1), 64);
        assert_eq!(out, Ok(vec![0.5, 0.0]));
    }

    /// A sound whose conversion would pass the bound is refused before anything is allocated, and
    /// one that states no channels is said rather than divided by.
    #[test]
    fn a_conversion_past_its_bound_or_of_no_frame_is_refused() {
        assert_eq!(
            converted(1, 1, &[0; 2], device(48_000, 2), MAX_PLAYED_SAMPLES),
            Ok(vec![0.0; 192_000])
        );
        assert_eq!(
            converted(1, 1, &[0; 4], device(48_000, 2), 100),
            Err(ConversionError::TooLarge {
                samples: 384_000,
                limit: 100
            })
        );
        assert_eq!(
            converted(8000, 0, &[0; 4], device(48_000, 2), 100),
            Err(ConversionError::NoSound {
                rate: 8000,
                channels: 0
            })
        );
        assert_eq!(
            converted(8000, 1, &[0; 4], device(0, 2), 100),
            Err(ConversionError::NoDevice {
                rate: 0,
                channels: 2
            })
        );
    }
}
