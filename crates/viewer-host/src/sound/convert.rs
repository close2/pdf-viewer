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
//! - **Speakers**: a sound that names the speaker of each channel — a WAVE file's channel mask, an
//!   AAC configuration's order (ADR 3405) — is mixed by speaker instead where the device has one
//!   or two channels, which is every device whose two channels are, in WAVE's and ALSA's order
//!   alike, the front left and right. No standard this program holds defines a mix of speakers to
//!   fewer: §13.3 says "combine channels" and nothing of how, and the weights are therefore a
//!   choice, taken from ITU-R BS.775's downmix of 3/2 sound to two channels — the front left and
//!   right at 1, the centre at `1/√2` on both sides, each surround at `1/√2` on its own side, and
//!   the low-frequency channel left out. Speakers the recommendation does not name follow its
//!   pattern: one at ear height in front, left or right of the centre, at 1 on its side; any other
//!   off the centre line at `1/√2` on its side; one on the centre line at `1/√2` on both; and a
//!   channel the mask names no speaker for, on the centre line. Each side is the weighted mean
//!   — the sum over the sum of its weights — so that, as in the averaging above, no frame of
//!   samples inside full scale is mixed past it. A device of more channels states a count and no
//!   positions (`quorra_sound`'s `OPENED`), so there the channels keep their order, and a mask that
//!   is not the first speakers one per channel is said.
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
/// `i32`, converted to `device` as fractions of full scale, under `limit` samples. `speakers` is
/// the channels' channel mask, `0` where the sound names none.
///
/// # Errors
///
/// [`ConversionError`]: a sound or device holding no frame, or a result past `limit`.
pub fn converted(
    rate: u32,
    channels: u16,
    speakers: u32,
    samples: &[i32],
    device: DeviceFormat,
    limit: usize,
) -> Result<Vec<f32>, ConversionError> {
    let mut converter = Converter::new(rate, channels, speakers, device, limit)?;
    let frames_in = samples
        .len()
        .checked_div(usize::from(channels))
        .unwrap_or(0);
    let wanted =
        frames_after(frames_in, rate, device.rate).saturating_mul(u64::from(device.channels));
    if usize::try_from(wanted).map_or(true, |wanted| wanted > limit) {
        return Err(ConversionError::TooLarge {
            samples: wanted,
            limit,
        });
    }
    let mut out = Vec::with_capacity(usize::try_from(wanted).unwrap_or(0));
    converter.push(samples, &mut out)?;
    converter.finish(&mut out)?;
    Ok(out)
}

/// The most samples `frames` of a sound at `rate` become on `device`: the bound a player checks
/// before the first run of a sound whose length is known is converted (ADR 3457).
#[must_use]
pub(super) fn samples_after(frames: u64, rate: u32, device: DeviceFormat) -> u64 {
    frames
        .saturating_mul(u64::from(device.rate))
        .div_ceil(u64::from(rate.max(1)))
        .saturating_mul(u64::from(device.channels))
}

/// [`converted`] a run at a time, for a sound that arrives as it is decoded (ADR 3457).
///
/// Every output frame is the one [`converted`] writes for the whole sound: a frame is written once
/// the input frames it is made of have all arrived, and the input frames no later output frame
/// reads are let go, so that what is held is the few frames one output frame spans. The sound's
/// last frames, which [`converted`] makes from the input it has, wait for [`Self::finish`].
#[derive(Debug)]
pub(super) struct Converter {
    rate: u32,
    device: DeviceFormat,
    channels: u16,
    weights: Option<Vec<(f64, f64)>>,
    /// The input frames still read by an output frame to come, arranged for the device; the first
    /// is input frame `first`.
    frames: std::collections::VecDeque<Vec<f64>>,
    first: u64,
    /// The input frames that have arrived.
    arrived: u64,
    /// The next output frame.
    next: u64,
    /// The samples written so far, which `limit` bounds.
    written: usize,
    limit: usize,
}

impl Converter {
    /// A converter of a sound at `rate` and `channels`, its speakers `speakers`, for `device`,
    /// under `limit` samples.
    ///
    /// # Errors
    ///
    /// [`ConversionError::NoSound`] or [`ConversionError::NoDevice`] where either holds no frame.
    pub(super) fn new(
        rate: u32,
        channels: u16,
        speakers: u32,
        device: DeviceFormat,
        limit: usize,
    ) -> Result<Self, ConversionError> {
        if rate == 0 || channels == 0 {
            return Err(ConversionError::NoSound { rate, channels });
        }
        if device.rate == 0 || device.channels == 0 {
            return Err(ConversionError::NoDevice {
                rate: device.rate,
                channels: device.channels,
            });
        }
        Ok(Self {
            rate,
            device,
            channels,
            weights: (speakers != 0 && device.channels <= 2).then(|| weights(speakers, channels)),
            frames: std::collections::VecDeque::new(),
            first: 0,
            arrived: 0,
            next: 0,
            written: 0,
            limit,
        })
    }

    /// `samples`, the sound's next whole frames, taken in, and every output frame they complete
    /// written to `out`.
    ///
    /// # Errors
    ///
    /// [`ConversionError::TooLarge`] where the sound converted so far passes the bound.
    pub(super) fn push(
        &mut self,
        samples: &[i32],
        out: &mut Vec<f32>,
    ) -> Result<(), ConversionError> {
        for frame in samples.chunks_exact(usize::from(self.channels)) {
            self.frames.push_back(match &self.weights {
                Some(weights) => mixed(frame, weights, self.device.channels),
                None => arranged(frame, self.device.channels),
            });
            self.arrived = self.arrived.saturating_add(1);
        }
        // An output frame is complete once the input frames it reads have all arrived: those up
        // to the next one's start where the device is slower, and the one after its start where
        // it is as fast or faster.
        let complete = |converter: &Self, k: u64| {
            let start = converter.position(k);
            if converter.device.rate < converter.rate {
                converter.position(k.saturating_add(1)) <= converter.arrived
            } else if k
                .saturating_mul(u64::from(converter.rate))
                .is_multiple_of(u64::from(converter.device.rate))
            {
                // On an input frame exactly, which the line through it and the next is.
                start < converter.arrived
            } else {
                start.saturating_add(1) < converter.arrived
            }
        };
        let mut until = self.next;
        while complete(self, until) {
            until = until.saturating_add(1);
        }
        self.write(until, out)
    }

    /// The sound's last output frames written to `out`, made as [`converted`] makes them from the
    /// input there is.
    ///
    /// # Errors
    ///
    /// As [`Self::push`].
    pub(super) fn finish(&mut self, out: &mut Vec<f32>) -> Result<(), ConversionError> {
        let last = frames_after(
            usize::try_from(self.arrived).unwrap_or(usize::MAX),
            self.rate,
            self.device.rate,
        );
        self.write(last, out)
    }

    /// The input frame output frame `k` starts at, `⌊k·from/to⌋`.
    fn position(&self, k: u64) -> u64 {
        k.saturating_mul(u64::from(self.rate))
            .checked_div(u64::from(self.device.rate))
            .unwrap_or(0)
    }

    /// Output frames from the next up to `until` written, and the input no later frame reads let go.
    fn write(&mut self, until: u64, out: &mut Vec<f32>) -> Result<(), ConversionError> {
        let count = until.saturating_sub(self.next);
        let samples = usize::try_from(count.saturating_mul(u64::from(self.device.channels)))
            .unwrap_or(usize::MAX);
        let total = self.written.saturating_add(samples);
        if total > self.limit {
            return Err(ConversionError::TooLarge {
                samples: u64::try_from(total).unwrap_or(u64::MAX),
                limit: self.limit,
            });
        }
        let offset = usize::try_from(self.first).unwrap_or(usize::MAX);
        let frames = self.frames.make_contiguous();
        out.reserve(samples);
        for k in self.next..until {
            for channel in 0..usize::from(self.device.channels) {
                let value = if self.device.rate < self.rate {
                    averaged(frames, offset, channel, k, self.rate, self.device.rate)
                } else {
                    interpolated(frames, offset, channel, k, self.rate, self.device.rate)
                };
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the conversion's one narrowing: an `f32` holds every value a 24-bit \
                              device sample can tell apart, which is the most any device takes"
                )]
                out.push(value as f32);
            }
        }
        self.written = total;
        self.next = until;
        // No later output frame reads an input frame before the next one's start.
        let keep_from = self.position(self.next).min(self.arrived);
        while self.first < keep_from && self.frames.pop_front().is_some() {
            self.first = self.first.saturating_add(1);
        }
        Ok(())
    }
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

/// ITU-R BS.775's weight for the centre and for a surround channel in its downmix to two
/// channels: `1/√2`, three decibels down.
const DOWNMIXED: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// The WAVE channel mask's bit for the low-frequency channel, which the mix to two leaves out.
const LOW_FREQUENCY: u32 = 1 << 3;

/// Each channel's weights on the left and right of a two-channel device, by the speaker `speakers`
/// assigns it: the channels in frame order take the mask's set bits from the lowest, and a channel
/// past the last set bit has no speaker. The weights are the module's stated choice.
fn weights(speakers: u32, channels: u16) -> Vec<(f64, f64)> {
    let mut bits = (0..32u32).filter(|bit| speakers & (1 << bit) != 0);
    (0..channels)
        .map(|_| match bits.next() {
            // Front left and right, and the front left and right of centre.
            Some(0 | 6) => (1.0, 0.0),
            Some(1 | 7) => (0.0, 1.0),
            // The low-frequency channel, left out as BS.775 leaves it.
            Some(3) => (0.0, 0.0),
            // Back, side, top front and top back, on the left.
            Some(4 | 9 | 12 | 15) => (DOWNMIXED, 0.0),
            // The same on the right.
            Some(5 | 10 | 14 | 17) => (0.0, DOWNMIXED),
            // Front, back, top, top front and top back centre; a reserved bit; and a channel no
            // speaker is named for: the centre line.
            _ => (DOWNMIXED, DOWNMIXED),
        })
        .collect()
}

/// One frame's samples as fractions of full scale, mixed by their speakers' `weights` onto a
/// device of one or two channels: each side the weighted mean of the channels on it, and one
/// channel the mean of the two sides.
fn mixed(frame: &[i32], weights: &[(f64, f64)], device: u16) -> Vec<f64> {
    let side = |weight: fn(&(f64, f64)) -> f64| {
        let total: f64 = weights.iter().map(weight).sum();
        if total == 0.0 {
            return 0.0;
        }
        frame
            .iter()
            .zip(weights)
            .map(|(sample, weights)| f64::from(*sample) / FULL_SCALE * weight(weights))
            .sum::<f64>()
            / total
    };
    let (left, right) = (side(|weights| weights.0), side(|weights| weights.1));
    if device == 1 {
        vec![f64::midpoint(left, right)]
    } else {
        vec![left, right]
    }
}

/// What the conversion of `channels` channels on `speakers` to `device` does that their order
/// alone would not, said beside the sound; `None` where the sound names no speakers, or names the
/// first speakers one per channel and the device has as many or more channels.
#[must_use]
pub fn speakers_said(speakers: u32, channels: u16, device: DeviceFormat) -> Option<String> {
    let leading = 1u64
        .checked_shl(u32::from(channels))
        .map_or(u64::MAX, |bit| bit.saturating_sub(1));
    let in_order = u64::from(speakers) == leading;
    if speakers == 0 || (in_order && channels <= device.channels) {
        return None;
    }
    if device.channels > 2 {
        return (!in_order).then(|| {
            format!(
                "the sound assigns its {channels} channel(s) to the speakers of channel mask \
                 {speakers:#010x}, and the device states {} channels and no speakers, so each \
                 channel is placed by its order",
                device.channels
            )
        });
    }
    let left_out = if speakers & LOW_FREQUENCY != 0 {
        ", its low-frequency channel left out"
    } else {
        ""
    };
    Some(format!(
        "its {channels} channel(s), on the speakers of channel mask {speakers:#010x}, mixed by \
         speaker to the device's {} with ITU-R BS.775's two-channel weights{left_out} (ADR 3405)",
        device.channels
    ))
}

/// Output frame `k`'s `channel` where the device is slower: the mean of the input frames from
/// `⌊k·from/to⌋` up to, not including, `⌊(k+1)·from/to⌋`, as many of them as the sound holds.
///
/// `frames` begins at input frame `offset`, and holds every frame from there the output frame
/// reads.
fn averaged(frames: &[Vec<f64>], offset: usize, channel: usize, k: u64, from: u32, to: u32) -> f64 {
    let at = |k: u64| {
        let position = k
            .saturating_mul(u64::from(from))
            .checked_div(u64::from(to))
            .unwrap_or(0);
        usize::try_from(position)
            .unwrap_or(usize::MAX)
            .saturating_sub(offset)
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
///
/// `frames` begins at input frame `offset`, as in [`averaged`].
fn interpolated(
    frames: &[Vec<f64>],
    offset: usize,
    channel: usize,
    k: u64,
    from: u32,
    to: u32,
) -> f64 {
    let position = k.saturating_mul(u64::from(from));
    let index = usize::try_from(position.checked_div(u64::from(to)).unwrap_or(0))
        .unwrap_or(usize::MAX)
        .saturating_sub(offset);
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
    use super::{
        ConversionError, Converter, DeviceFormat, MAX_PLAYED_SAMPLES, converted, speakers_said,
    };

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
            0,
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
        let out = converted(8000, 2, 0, &[top(0.5), top(-0.25)], device(8000, 1), 64);
        assert_eq!(out, Ok(vec![0.125]));
    }

    /// Four channels on a stereo device: channel `c` lands on `c mod 2`, so the device's left is
    /// `(c0 + c2) / 2 = (0.5 + 0.25) / 2` and its right `(c1 + c3) / 2 = (−0.5 + 0) / 2`.
    #[test]
    fn four_channels_on_two_combine_by_their_place_modulo_two() {
        let frame = [top(0.5), top(-0.5), top(0.25), 0];
        let out = converted(8000, 4, 0, &frame, device(8000, 2), 64);
        assert_eq!(out, Ok(vec![0.375, -0.25]));
    }

    /// A mono sound sounds on every channel of the device: device channel `d` repeats the sound's
    /// `d mod 1`; the wrong answer excluded is a silent right channel.
    #[test]
    fn mono_on_a_stereo_device_sounds_on_both() {
        let out = converted(8000, 1, 0, &[top(0.5), top(-0.5)], device(8000, 2), 64);
        assert_eq!(out, Ok(vec![0.5, 0.5, -0.5, -0.5]));
    }

    /// Raised from 8000 Hz to 16,000: twice the frames, each odd one halfway between its
    /// neighbours, and the last held past the end — `[0, 0.25, 0.5, 0.5]`.
    #[test]
    fn a_faster_device_draws_a_straight_line_between_frames() {
        let out = converted(8000, 1, 0, &[0, top(0.5)], device(16_000, 1), 64);
        assert_eq!(out, Ok(vec![0.0, 0.25, 0.5, 0.5]));
    }

    /// 8000 Hz on a 48,000 Hz device, §13.3's µ-law rate on the commonest device: six output frames
    /// per input frame, at sixths of the way — the first frame's run is `0, 1/6 … 5/6` of the step
    /// to the second, here exact as multiples of `6/8 = 0.75` over six.
    #[test]
    fn eight_thousand_on_forty_eight_thousand_is_six_frames_per_frame() {
        let out = converted(8000, 1, 0, &[0, top(0.75)], device(48_000, 1), 64).expect("converted");
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
        let out = converted(48_000, 1, 0, &input, device(24_000, 1), 64);
        assert_eq!(out, Ok(vec![0.375, -0.25]));
    }

    /// Three frames to two (a rate ratio of 3 : 2): output frame 0 covers input `[0, 1)`, frame
    /// 1 covers `[1, 3)` — `⌊1·3/2⌋ = 1`, `⌊2·3/2⌋ = 3` — so `[0.5, (0.25 + −0.25) / 2]`.
    #[test]
    fn an_uneven_ratio_covers_whole_frames_by_exact_positions() {
        let input = [top(0.5), top(0.25), top(-0.25)];
        let out = converted(48_000, 1, 0, &input, device(32_000, 1), 64);
        assert_eq!(out, Ok(vec![0.5, 0.0]));
    }

    /// A sound whose conversion would pass the bound is refused before anything is allocated, and
    /// one that states no channels is said rather than divided by.
    #[test]
    fn a_conversion_past_its_bound_or_of_no_frame_is_refused() {
        assert_eq!(
            converted(1, 1, 0, &[0; 2], device(48_000, 2), MAX_PLAYED_SAMPLES),
            Ok(vec![0.0; 192_000])
        );
        assert_eq!(
            converted(1, 1, 0, &[0; 4], device(48_000, 2), 100),
            Err(ConversionError::TooLarge {
                samples: 384_000,
                limit: 100
            })
        );
        assert_eq!(
            converted(8000, 0, 0, &[0; 4], device(48_000, 2), 100),
            Err(ConversionError::NoSound {
                rate: 8000,
                channels: 0
            })
        );
        assert_eq!(
            converted(8000, 1, 0, &[0; 4], device(0, 2), 100),
            Err(ConversionError::NoDevice {
                rate: 0,
                channels: 2
            })
        );
    }

    /// 5.1 in the WAVE order — front left, right, centre, low frequency, back left, right — on a
    /// stereo device (ADR 3405): the left is `(L + C/√2 + Ls/√2) / (1 + 2/√2)` and the right
    /// `(R + C/√2 + Rs/√2) / (1 + 2/√2)`, and the low-frequency channel is on neither. The wrong
    /// answers excluded: the placement by order, which puts the centre on the left alone and the
    /// low-frequency channel on the right, and a centre or a surround at full weight.
    #[test]
    fn five_one_on_a_stereo_device_is_mixed_by_speaker_with_the_low_frequency_left_out() {
        let g = std::f64::consts::FRAC_1_SQRT_2;
        let total = 1.0 + 2.0 * g;
        let frame = [top(0.5), top(-0.5), top(0.25), top(0.75), top(0.125), 0];
        let out = converted(8000, 6, 0x3f, &frame, device(8000, 2), 64).expect("converted");
        #[expect(clippy::cast_possible_truncation, reason = "the result is an `f32`")]
        let expected = [
            ((0.5 + 0.25 * g + 0.125 * g) / total) as f32,
            ((-0.5 + 0.25 * g) / total) as f32,
        ];
        assert_eq!(out, expected);
        let mono = converted(8000, 6, 0x3f, &frame, device(8000, 1), 64).expect("converted");
        #[expect(clippy::cast_possible_truncation, reason = "the result is an `f32`")]
        let mean = (f64::from(expected[0]) / 2.0 + f64::from(expected[1]) / 2.0) as f32;
        assert!((mono[0] - mean).abs() < 1e-6, "{mono:?}");
        // A device of six channels takes the six by their order.
        let six = converted(8000, 6, 0x3f, &frame, device(8000, 6), 64).expect("converted");
        assert_eq!(six, [0.5, -0.5, 0.25, 0.75, 0.125, 0.0]);
    }

    /// A stereo file whose mask names the back pair is heard on the device's left and right, each
    /// at its own side's weighted mean, which is the channel itself; a mono file on the front
    /// centre sounds on both; and a channel past the mask's set bits is on the centre line.
    #[test]
    fn a_mask_places_each_channel_on_its_speakers_side() {
        let back = converted(8000, 2, 0x30, &[top(0.5), top(-0.25)], device(8000, 2), 64);
        assert_eq!(back, Ok(vec![0.5, -0.25]));
        let centre = converted(8000, 1, 0x4, &[top(0.5)], device(8000, 2), 64);
        assert_eq!(centre, Ok(vec![0.5, 0.5]));
        let unnamed = converted(8000, 2, 0x1, &[top(0.5), top(0.25)], device(8000, 2), 64)
            .expect("converted");
        let g = std::f64::consts::FRAC_1_SQRT_2;
        #[expect(clippy::cast_possible_truncation, reason = "the result is an `f32`")]
        let left = ((0.5 + 0.25 * g) / (1.0 + g)) as f32;
        assert_eq!(unnamed, [left, 0.25]);
    }

    /// The sentence is said where the speakers change what is heard and only there: not for no
    /// mask, nor for the first speakers on a device of as many channels; for 5.1 on two, naming the
    /// low-frequency channel; and for a mask past the first speakers on a device of six.
    #[test]
    fn the_speakers_are_said_where_they_change_the_placement() {
        assert_eq!(speakers_said(0, 6, device(48_000, 2)), None);
        assert_eq!(speakers_said(0x3, 2, device(48_000, 2)), None);
        assert_eq!(speakers_said(0x3f, 6, device(48_000, 6)), None);
        let mixed = speakers_said(0x3f, 6, device(48_000, 2)).expect("said");
        assert!(
            mixed.contains("BS.775") && mixed.contains("low-frequency"),
            "{mixed}"
        );
        let ordered = speakers_said(0x30, 2, device(48_000, 6)).expect("said");
        assert!(ordered.contains("by its order"), "{ordered}");
    }

    /// A sound converted a run at a time is the sound converted whole, sample for sample, at a
    /// slower device, a faster one, the same rate and a mix by speakers, whatever the runs' lengths
    /// (ADR 3457).
    #[test]
    fn a_sound_converted_in_runs_is_the_sound_converted_whole() {
        let samples: Vec<i32> = (0..1_234_i32)
            .map(|n| n.wrapping_mul(2_654_435_761_u32.cast_signed()))
            .collect();
        for (rate, channels, speakers, to) in [
            (44_100, 2, 0, device(8_000, 1)),
            (8_000, 2, 0, device(44_100, 2)),
            (22_050, 2, 0, device(48_000, 2)),
            (48_000, 2, 0, device(48_000, 2)),
            (48_000, 6, 0x3F, device(44_100, 2)),
        ] {
            let usable = samples.len() - samples.len() % usize::from(channels);
            let sound = &samples[..usable];
            let whole = converted(rate, channels, speakers, sound, to, MAX_PLAYED_SAMPLES)
                .expect("converted whole");
            for frames_per_run in [1, 7, 64, 1_000] {
                let mut converter =
                    Converter::new(rate, channels, speakers, to, MAX_PLAYED_SAMPLES)
                        .expect("a converter");
                let mut out = Vec::new();
                for run in sound.chunks(frames_per_run * usize::from(channels)) {
                    converter.push(run, &mut out).expect("a run converted");
                }
                converter.finish(&mut out).expect("the last frames");
                assert_eq!(
                    out, whole,
                    "{rate} Hz x{channels} to {to:?}, runs of {frames_per_run}"
                );
            }
        }
    }

    /// The bound holds a sound arriving in runs as it holds a whole one: at the run that passes it.
    #[test]
    fn a_sound_in_runs_past_the_bound_is_refused_where_it_passes() {
        let mut converter = Converter::new(8_000, 1, 0, device(8_000, 1), 10).expect("a converter");
        let mut out = Vec::new();
        converter.push(&[0; 8], &mut out).expect("under the bound");
        assert!(matches!(
            converter.push(&[0; 8], &mut out),
            Err(ConversionError::TooLarge { limit: 10, .. })
        ));
    }
}
