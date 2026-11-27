//! H.264 decoded in the worker: an MP4 video track's samples, or an elementary stream's access
//! units, in; pictures out in the order they are output (RFC 0015 section 6, ADR 3526).
//!
//! The decoder is `rusty_h264-decoder`'s scalar build — `default-features = false, features =
//! ["std"]` — which keeps its global allocator and its SIMD crate out of the worker (ADR 2428, RFC
//! 0012 section 2.2). It reads ITU-T H.264's Annex B byte stream, so an MP4 sample, whose NAL units
//! each stand behind a length of [`crate::mp4::VideoTrack::length_size`] bytes (ISO/IEC 14496-15),
//! is rewritten with a start code before each unit, and the `avcC` record's parameter sets are given
//! to the decoder first.
//!
//! # Output order
//!
//! The decoder answers each access unit in decoding order with the picture it decoded. ITU-T H.264
//! outputs pictures in increasing picture order count between instantaneous decoding refreshes
//! (Annex C's bumping process, C.4.5), and holds at most `max_dec_frame_buffering` of them, which
//! Annex A's level limits (A.3.1, A.3.2) bound at sixteen frames for every level. ITU-T H.264 is not
//! held in `doc/`; these are cited, not quoted. So a session holds pictures until it holds more
//! than [`REORDER_FRAMES`], then outputs the one of lowest order count; at an IDR access unit and
//! at the stream's end it outputs every picture it holds, in order. Holding the bound rather than
//! the stream's own `max_num_reorder_frames` costs latency and memory and never order; a stream
//! whose memory management operation 5 resets the order count mid-sequence is the case it does not
//! see, and the conformance walk is where that would show.
//!
//! # Backward
//!
//! A movie played backward (ISO 32000-2 §13.4, Table 307's negative `/Rate`) is decoded one group
//! of pictures at a time, each from its sync sample in a session of its own, and held as a [`Group`]
//! until the group ends; its pictures are then let out last picture first (ADR 3837). A group whose
//! planes pass [`MAX_GROUP_BYTES`] is refused by its size.
//!
//! # What is refused
//!
//! Interlaced H.264 — field pictures or MBAFF — which the decoder refuses by name and no decoder
//! this program admits reads: [`VideoError::Interlaced`], which the host hands to this machine's
//! own player and says so. Every other refusal of the decoder's is [`VideoError::Refused`] with the
//! decoder's own words, and a picture past [`MAX_PICTURE_PIXELS`] is refused by its size.

use rusty_h264_decoder::{DecodeError, Decoder};

/// The most frames a session holds for reordering: ITU-T H.264's largest decoded picture buffer,
/// sixteen frames at every level (Annex A, A.3.1 and A.3.2).
pub const REORDER_FRAMES: usize = 16;

/// The largest picture a session outputs, in luma samples: 1920 by 1088, the 1080-line frame RFC
/// 0015 section 4.1 sized the video worker for (ADR 3528). A larger one is refused by its size.
pub const MAX_PICTURE_PIXELS: usize = 1920 * 1088;

/// The most bytes of decoded planes one group of pictures played backward holds (ISO 32000-2
/// §13.4, Table 307's negative `/Rate`; ADR 3837): 96 MiB, thirty-two 1920 by 1088 pictures.
///
/// A movie played backward shows each group's last picture first, and a group's pictures can only
/// be decoded forward from its sync sample, so the whole group is decoded and held before its first
/// picture is let out. The bound is what `Profile::Media`'s 256 MiB address space leaves beside a
/// session's sixteen reference frames and the ring (`tests/video.rs` measures the peak); a group
/// past it is refused by its size, never cut short.
pub const MAX_GROUP_BYTES: usize = 96 << 20;

/// What a stream says of its pictures' colour, read from its sequence parameter set's VUI (ITU-T
/// H.264 Annex E, E.1.1), with Annex E's inference for each code the stream leaves out (E.2.1):
/// the codes, not their meaning, which the host's renderer resolves (ADR 3529).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signal {
    /// `video_full_range_flag`; absent, 0.
    pub full_range: bool,
    /// `colour_primaries` (Table E-3); absent, 2.
    pub colour_primaries: u8,
    /// `transfer_characteristics` (Table E-4); absent, 2.
    pub transfer_characteristics: u8,
    /// `matrix_coefficients` (Table E-5); absent, 2.
    pub matrix_coefficients: u8,
    /// `chroma_sample_loc_type_top_field` (Figure E-1), a code past 255 held at 255; absent, 0.
    pub chroma_sample_loc_type: u8,
}

impl Default for Signal {
    /// A stream with no VUI.
    fn default() -> Self {
        Self {
            full_range: false,
            colour_primaries: 2,
            transfer_characteristics: 2,
            matrix_coefficients: 2,
            chroma_sample_loc_type: 0,
        }
    }
}

/// One decoded picture: 8-bit 4:2:0, cropped as its sequence parameter set says, each plane tight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Luma width in samples.
    pub width: usize,
    /// Luma height in samples.
    pub height: usize,
    /// The luma plane, `width × height` bytes.
    pub y: Vec<u8>,
    /// The blue-difference plane, `⌈width / 2⌉ × ⌈height / 2⌉` bytes.
    pub u: Vec<u8>,
    /// The red-difference plane, as long as [`Picture::u`].
    pub v: Vec<u8>,
    /// What its sequence parameter set says of its colour.
    pub signal: Signal,
}

/// Why a session stopped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VideoError {
    /// Field or MBAFF coding, which no admitted decoder reads: handed to this machine's own player.
    #[error(
        "an interlaced H.264 stream ({0}), which no decoder this program admits reads; it is \
         handed to this machine's own player"
    )]
    Interlaced(String),
    /// Anything else the stream or the decoder refuses, in its own words.
    #[error("{0}")]
    Refused(String),
}

/// One stream's decoder and the pictures it holds for output order.
pub struct Session {
    decoder: Decoder,
    /// The bytes of the length before each NAL unit of a sample; none for an elementary stream.
    length_size: Option<u8>,
    /// Decoded pictures not yet output, with their picture order counts.
    held: Vec<(i32, Picture)>,
    /// The Annex B rewriting of the last sample, kept so its allocation is reused.
    rewritten: Vec<u8>,
    /// The parameter sets' signals, read beside the decoder (`crate::vui`).
    parameters: crate::vui::Parameters,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("length_size", &self.length_size)
            .field("held", &self.held.len())
            .finish_non_exhaustive()
    }
}

impl Session {
    /// A session for an MP4 track's samples: `parameter_sets` (the `avcC` record's, each a NAL unit)
    /// are given to the decoder before any sample.
    ///
    /// # Errors
    ///
    /// A length size other than 1, 2 or 4, and the decoder's refusal of a parameter set — an MBAFF
    /// sequence among them, which is [`VideoError::Interlaced`].
    pub fn for_track(length_size: u8, parameter_sets: &[Vec<u8>]) -> Result<Self, VideoError> {
        if !matches!(length_size, 1 | 2 | 4) {
            return Err(VideoError::Refused(format!(
                "NAL lengths of {length_size} bytes, where ISO/IEC 14496-15 allows 1, 2 or 4"
            )));
        }
        let mut session = Self::elementary();
        session.length_size = Some(length_size);
        let mut sets = Vec::new();
        for set in parameter_sets {
            sets.extend_from_slice(&START_CODE);
            sets.extend_from_slice(set);
        }
        if !sets.is_empty() {
            session.decode(&sets)?;
        }
        Ok(session)
    }

    /// A session for an Annex B elementary stream, each call given one access unit.
    #[must_use]
    pub fn elementary() -> Self {
        Self {
            decoder: Decoder::new(),
            length_size: None,
            held: Vec::new(),
            rewritten: Vec::new(),
            parameters: crate::vui::Parameters::default(),
        }
    }

    /// Decodes one MP4 sample — one access unit, its NAL units behind lengths — and returns the
    /// pictures it lets out, in output order.
    ///
    /// # Errors
    ///
    /// A length that runs past the sample, and as [`Session::access_unit`].
    pub fn sample(&mut self, sample: &[u8]) -> Result<Vec<Picture>, VideoError> {
        let size = usize::from(self.length_size.unwrap_or(4));
        let mut rewritten = std::mem::take(&mut self.rewritten);
        rewritten.clear();
        let mut rest = sample;
        while !rest.is_empty() {
            let (length, after) = rest.split_at_checked(size).ok_or_else(|| {
                VideoError::Refused("a sample that ends inside a NAL unit's length".to_owned())
            })?;
            let length = length
                .iter()
                .fold(0usize, |sum, byte| sum.wrapping_shl(8) | usize::from(*byte));
            let (unit, after) = after.split_at_checked(length).ok_or_else(|| {
                VideoError::Refused("a NAL unit that runs past its sample".to_owned())
            })?;
            rewritten.extend_from_slice(&START_CODE);
            rewritten.extend_from_slice(unit);
            rest = after;
        }
        let answered = self.access_unit(&rewritten);
        self.rewritten = rewritten;
        answered
    }

    /// Decodes one access unit of an Annex B byte stream and returns the pictures it lets out.
    ///
    /// # Errors
    ///
    /// The decoder's refusal ([`VideoError::Interlaced`] for field or MBAFF coding), and a picture
    /// past [`MAX_PICTURE_PIXELS`].
    pub fn access_unit(&mut self, annex_b: &[u8]) -> Result<Vec<Picture>, VideoError> {
        let mut out = Vec::new();
        if rusty_h264_decoder::au_is_idr(annex_b) {
            out.append(&mut self.finish());
        }
        if let Some(picture) = self.decode(annex_b)? {
            let order = self.decoder.last_poc();
            self.held.push((order, picture));
            if self.held.len() > REORDER_FRAMES {
                out.extend(self.next_out());
            }
        }
        Ok(out)
    }

    /// Every picture still held, in output order: the end of the stream.
    pub fn finish(&mut self) -> Vec<Picture> {
        let mut held = std::mem::take(&mut self.held);
        held.sort_by_key(|(order, _)| *order);
        held.into_iter().map(|(_, picture)| picture).collect()
    }

    /// The held picture of lowest order count.
    fn next_out(&mut self) -> Option<Picture> {
        let (index, _) = self
            .held
            .iter()
            .enumerate()
            .min_by_key(|(_, (order, _))| *order)?;
        Some(self.held.swap_remove(index).1)
    }

    fn decode(&mut self, annex_b: &[u8]) -> Result<Option<Picture>, VideoError> {
        let signal = self.parameters.observe(annex_b)?;
        let frame = self.decoder.decode(annex_b).map_err(refusal)?;
        let Some(frame) = frame else {
            return Ok(None);
        };
        let pixels = frame.width.saturating_mul(frame.height);
        if pixels > MAX_PICTURE_PIXELS {
            return Err(VideoError::Refused(format!(
                "a picture of {} by {}, past the {MAX_PICTURE_PIXELS} samples this worker outputs",
                frame.width, frame.height
            )));
        }
        Ok(Some(Picture {
            width: frame.width,
            height: frame.height,
            y: frame.y,
            u: frame.u,
            v: frame.v,
            signal,
        }))
    }
}

/// One group of pictures decoded for a backward pass, held in output order until the group ends and
/// then let out last picture first (ISO 32000-2 §13.4, Table 307: "If the value of this entry is
/// negative, the movie shall be played backward with respect to Start and Duration"; ADR 3837).
#[derive(Debug, Default)]
pub struct Group {
    pictures: Vec<Picture>,
    bytes: usize,
}

impl Group {
    /// Holds `pictures`, the next a session let out.
    ///
    /// # Errors
    ///
    /// [`VideoError::Refused`] naming the bound where the group's planes would pass
    /// [`MAX_GROUP_BYTES`]; the group is then not to be played, and nothing of it has been let out.
    pub fn hold(&mut self, pictures: Vec<Picture>) -> Result<(), VideoError> {
        for picture in pictures {
            let planes = picture
                .y
                .len()
                .saturating_add(picture.u.len())
                .saturating_add(picture.v.len());
            let bytes = self.bytes.saturating_add(planes);
            if bytes > MAX_GROUP_BYTES {
                return Err(VideoError::Refused(format!(
                    "a group of pictures past the {MAX_GROUP_BYTES} bytes of decoded planes a \
                     movie played backward holds: its picture {} of {} by {} reaches {bytes}",
                    self.pictures.len().saturating_add(1),
                    picture.width,
                    picture.height
                )));
            }
            self.bytes = bytes;
            self.pictures.push(picture);
        }
        Ok(())
    }

    /// The bytes of decoded planes held.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// The group's pictures, last picture first.
    pub fn reversed(self) -> impl Iterator<Item = Picture> {
        self.pictures.into_iter().rev()
    }
}

/// An Annex B byte stream cut into the units [`Session::access_unit`] takes, each ending after a
/// coded slice with the parameter sets and other units before it attached: what an elementary
/// stream, such as a conformance bitstream, is fed as.
#[must_use]
pub fn access_units(stream: &[u8]) -> Vec<&[u8]> {
    rusty_h264_decoder::split_access_units(stream)
}

/// Annex B's four-byte start code (B.1.1), written before each NAL unit of a rewritten sample.
const START_CODE: [u8; 4] = [0, 0, 0, 1];

/// The decoder's refusal as this module's: its interlace refusals by name, the rest in its words.
fn refusal(error: DecodeError) -> VideoError {
    match error {
        DecodeError::Unsupported(what) if what.contains("interlace") => {
            VideoError::Interlaced(what.to_owned())
        }
        other => VideoError::Refused(format!("the H.264 stream: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Group, MAX_GROUP_BYTES, Picture, Signal, VideoError};

    /// A 1920 by 1088 picture whose every sample is `index`.
    fn picture(index: u8) -> Picture {
        const LUMA: usize = 1920 * 1088;
        const CHROMA: usize = 960 * 544;
        Picture {
            width: 1920,
            height: 1088,
            y: vec![index; LUMA],
            u: vec![index; CHROMA],
            v: vec![index; CHROMA],
            signal: Signal::default(),
        }
    }

    /// A group lets its pictures out last first, and holds thirty-two 1080-line pictures — 3 133 440
    /// bytes of planes each — and refuses the thirty-third by name (ADR 3837).
    #[test]
    fn a_group_reverses_and_refuses_past_its_bound() {
        let mut group = Group::default();
        group
            .hold((0..32).map(picture).collect())
            .expect("thirty-two pictures");
        assert!(group.bytes() <= MAX_GROUP_BYTES);
        let refused = group.hold(vec![picture(32)]).expect_err("the thirty-third");
        assert!(
            matches!(&refused, VideoError::Refused(said) if said.contains("played backward")),
            "{refused:?}"
        );
        let order: Vec<u8> = group.reversed().map(|picture| picture.y[0]).collect();
        assert_eq!(order, (0..32).rev().collect::<Vec<u8>>());
    }
}
