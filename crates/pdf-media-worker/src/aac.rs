//! An MP4 clip's AAC decoded in the worker, one run of access units at a time (ADR 3382).
//!
//! **ISO/IEC 14496-3**, MPEG-4 audio, which is not held in `doc/`: an `AudioSpecificConfig` — the
//! bytes an MP4 file's `DecoderSpecificInfo` carries ([`crate::mp4`]) — states the audio object type,
//! the sampling rate and the channel configuration, and each access unit of the track is one raw
//! data block of 1 024 samples a channel (960 in the short-frame form). Where the configuration or the
//! stream signals spectral band replication (HE-AAC) the output is at twice the core rate, and where it
//! signals parametric stereo a mono core comes out as two channels.
//!
//! The decoder is `rusty_aac`'s, built without its default `simd` feature (ADR 3382, RFC 0012
//! section 3.3). It is fed the raw access units the sample table names, never ADTS, so nothing in
//! the stream can reconfigure it: the configuration is the one the file's sample entry states.
//!
//! A decoder carries state from one access unit to the next — the transform's overlap, and SBR's
//! envelopes — so a [`Session`] lives for one clip in the worker, from the movie's request to the
//! last run (ADR 3381). An access unit the decoder refuses is played as a frame of silence, counted
//! in [`Pcm::concealed`] for the host to say, and the next is decoded: one damaged block does not
//! end a sound.

use rusty_aac::decode::Decoder;

use crate::mp4::OBJECT_TYPE_MPEG4_AUDIO;

/// The most samples a channel one access unit decodes to: 1 024, doubled by SBR.
pub const MAX_FRAMES_PER_UNIT: usize = 2048;

/// The most channels a decoded clip may have.
///
/// A program configuration element names at most fifteen front, fifteen side and fifteen back
/// elements, each a pair at most, three LFE channels and the rest: under a hundred. The bound is the
/// host's check on what a worker answers, not a choice of which streams play.
pub const MAX_CHANNELS: u16 = 128;

/// One run's samples, as the worker answers them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    /// The output's samples per second per channel.
    pub rate: u32,
    /// The output's channels.
    pub channels: u16,
    /// How many of the run's access units the decoder refused and were played as silence.
    pub concealed: u32,
    /// The samples, interleaved, each in the most significant bits of an `i32` as `crate::sound`'s.
    pub samples: Vec<i32>,
}

/// One clip's decoder, from its configuration to its last access unit.
pub struct Session {
    decoder: Decoder,
    /// The rate and channels of the first access unit decoded, which every later one must keep.
    output: Option<(u32, u16)>,
    /// Samples a channel of the last access unit decoded: a concealed unit's length.
    last_frames: usize,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("output", &self.output)
            .field("last_frames", &self.last_frames)
            .finish_non_exhaustive()
    }
}

impl Session {
    /// A decoder for a track of `object_type` whose `DecoderSpecificInfo` is `config`.
    ///
    /// # Errors
    ///
    /// The sentence for an object type other than ISO/IEC 14496-3 audio, a missing configuration,
    /// or one the decoder does not run.
    pub fn new(object_type: u8, config: &[u8]) -> Result<Self, String> {
        if object_type != OBJECT_TYPE_MPEG4_AUDIO {
            return Err(format!(
                "a sound track of object type 0x{object_type:02x}, where this player decodes \
                 ISO/IEC 14496-3 audio (0x{OBJECT_TYPE_MPEG4_AUDIO:02x})"
            ));
        }
        if config.is_empty() {
            return Err(
                "a sound track with no AudioSpecificConfig, which an AAC decoder starts from"
                    .to_owned(),
            );
        }
        let stream = rusty_aac::config::parse(config).map_err(|error| {
            format!("an AudioSpecificConfig the decoder does not read: {error}")
        })?;
        let frames = stream
            .frame_length
            .saturating_mul(if stream.sbr { 2 } else { 1 });
        let decoder = Decoder::with_stream_config(stream)
            .map_err(|error| format!("an AudioSpecificConfig the decoder does not run: {error}"))?;
        Ok(Self {
            decoder,
            output: None,
            last_frames: frames.min(MAX_FRAMES_PER_UNIT),
        })
    }

    /// Decodes `units`, the next access units of the clip in order, into at most `max_samples`
    /// samples.
    ///
    /// # Errors
    ///
    /// The sentence for output past `max_samples`, an access unit that decodes past
    /// [`MAX_FRAMES_PER_UNIT`] or [`MAX_CHANNELS`], or a stream whose rate or channels change after
    /// its first access unit — the host holds one format a clip.
    pub fn decode(&mut self, units: &[&[u8]], max_samples: usize) -> Result<Pcm, String> {
        let mut samples = Vec::new();
        let mut concealed = 0u32;
        for unit in units {
            let Ok(audio) = self.decoder.decode(unit, None) else {
                concealed = concealed.saturating_add(1);
                // Before the first decoded unit there is no format to be silent in, and the host
                // says the count either way.
                if let Some((_, channels)) = self.output {
                    let silence = self.last_frames.saturating_mul(usize::from(channels));
                    bounded(samples.len(), silence, max_samples)?;
                    samples.resize(samples.len().saturating_add(silence), 0);
                }
                continue;
            };
            let format = (audio.sample_rate, audio.channels);
            if format.1 == 0 || format.1 > MAX_CHANNELS || format.0 == 0 {
                return Err(format!(
                    "an access unit that decodes to {} channel(s) at {} Hz, where this player \
                     holds 1 to {MAX_CHANNELS} channels",
                    format.1, format.0
                ));
            }
            match self.output {
                Some(first) if first != format => {
                    return Err(format!(
                        "a stream whose output changes from {} Hz and {} channel(s) to {} Hz and \
                         {} channel(s) after its first access unit",
                        first.0, first.1, format.0, format.1
                    ));
                }
                _ => self.output = Some(format),
            }
            let frames = audio.frames();
            if frames > MAX_FRAMES_PER_UNIT {
                return Err(format!(
                    "an access unit that decodes to {frames} samples a channel, past the \
                     {MAX_FRAMES_PER_UNIT} one holds"
                ));
            }
            self.last_frames = frames;
            bounded(samples.len(), audio.samples.len(), max_samples)?;
            samples.extend(audio.samples.iter().map(|&sample| top_aligned(sample)));
        }
        let (rate, channels) = self.output.unwrap_or((0, 0));
        Ok(Pcm {
            rate,
            channels,
            concealed,
            samples,
        })
    }
}

/// Refuses `adding` samples to `held` past `max`.
fn bounded(held: usize, adding: usize, max: usize) -> Result<(), String> {
    if held.saturating_add(adding) > max {
        return Err(format!(
            "a run of access units that decodes past the {max} samples one reply carries"
        ));
    }
    Ok(())
}

/// A decoded sample — `rusty_aac`'s `f32`, full scale at ±1 — in the most significant bits of an
/// `i32`, as every sample the worker answers is (`crate::sound`): `x · 2³¹`, held to the type's
/// range, so that a sample at or past full scale is the largest the type holds.
#[must_use]
pub fn top_aligned(sample: f32) -> i32 {
    let scaled = f64::from(sample) * 2_147_483_648.0;
    // `as` saturates at the type's bounds and maps NaN to 0, which is the whole of what this
    // conversion asks of a value outside the range.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a float to integer `as` saturates; that saturation is the clamp"
    )]
    let word = scaled as i32;
    word
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_scale_and_past_it_are_the_largest_words() {
        assert_eq!(top_aligned(0.0), 0);
        assert_eq!(top_aligned(0.5), 1 << 30);
        assert_eq!(top_aligned(-1.0), i32::MIN);
        assert_eq!(top_aligned(1.0), i32::MAX);
        assert_eq!(top_aligned(7.0), i32::MAX);
        assert_eq!(top_aligned(f32::NAN), 0);
    }

    #[test]
    fn a_track_that_is_not_mpeg4_audio_or_has_no_configuration_is_refused() {
        assert!(Session::new(0x6b, &[0x12, 0x10]).is_err());
        assert!(Session::new(OBJECT_TYPE_MPEG4_AUDIO, &[]).is_err());
    }

    #[test]
    fn a_unit_the_decoder_refuses_is_counted_and_the_session_goes_on() {
        // AAC LC, 44.1 kHz, two channels (ISO/IEC 14496-3's AudioSpecificConfig: object type 2,
        // frequency index 4, channel configuration 2).
        let mut session = Session::new(OBJECT_TYPE_MPEG4_AUDIO, &[0x12, 0x10]).expect("LC");
        // An empty unit, and a single channel element cut short after its element tag.
        let pcm = session.decode(&[&[], &[0x00]], 1 << 20).expect("a run");
        assert_eq!(pcm.concealed, 2);
        assert_eq!(pcm.samples, Vec::<i32>::new());
    }
}
