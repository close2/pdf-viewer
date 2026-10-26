//! §13.3's sound object that holds its own samples, written as a RIFF WAVE file for the hand-off.
//!
//! §13.3 says what such a sound is:
//!
//! > If no F entry is present, the sound object itself shall contain the sample data and all other
//! > information needed to define the sound.
//!
//! and a player outside this program cannot be handed a PDF stream and Table 305's entries beside
//! it, so the samples cross as a self-describing file instead — the RIFF form the clause's own NOTE
//! names among the self-describing formats, written on this side (`doc/questions/A631`, ADRs 2382,
//! 2398). Nothing plays in-process: what is built is a file, and the hand-off's level decides
//! whether it leaves.
//!
//! **The file's format is the RIFF WAVE form**, Microsoft's and IBM's *Multimedia Programming
//! Interface and Data Specifications 1.0* (1991), with RFC 2361's registry for the format tags and
//! the content type. Three facts of that form decide the writer, each paraphrased here: a PCM
//! sample sits in the smallest whole number of bytes that holds it, least significant byte first,
//! its amplitude in the most significant bits and the rest zero; a sample of one to eight bits is
//! unsigned and one of nine or more is two's complement; and a format other than PCM — RFC 2361's
//! `WAVE_FORMAT_ALAW` (6) and `WAVE_FORMAT_MULAW` (7) — states its format block's extra size and
//! carries a `fact` chunk giving its length in sample frames.
//!
//! So Table 305's `Raw` (unsigned) and `Signed` samples are unpacked and re-biased into that layout,
//! and µ-law and A-law codes, eight bits each, cross unchanged.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, Stream};

/// The name the written file is handed over under: a sound object files itself under no name.
const FILE_NAME: &str = "sound.wav";

/// Table 305's `/E`: how one sample value is to be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// `Raw`, the default: "Unspecified or unsigned values in the range 0 to 2B - 1".
    Raw,
    /// `Signed`: "Twos-complement values".
    Signed,
    /// `muLaw`: µ-law codes, eight bits each.
    MuLaw,
    /// `ALaw`: A-law codes, eight bits each. ISO 32000-2's printed Table 305 lists it beside `muLaw`
    /// with `Raw` as the default; the text extraction in `doc/md/` drops both lines.
    ALaw,
}

impl Encoding {
    /// The name Table 305 spells it with.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Signed => "Signed",
            Self::MuLaw => "muLaw",
            Self::ALaw => "ALaw",
        }
    }

    /// RFC 2361's `wFormatTag` for the file the samples are written into.
    fn format_tag(self) -> u16 {
        match self {
            Self::Raw | Self::Signed => 1,
            Self::ALaw => 6,
            Self::MuLaw => 7,
        }
    }

    /// Whether the samples are companded codes, which cross unchanged.
    fn is_companded(self) -> bool {
        matches!(self, Self::MuLaw | Self::ALaw)
    }
}

/// What Table 305 says of a sound object's samples, checked for the file it becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleFormat {
    /// `/R`, in samples per second, as the whole number a WAVE header states.
    pub rate: u32,
    /// `/C`, default 1.
    pub channels: u16,
    /// `/B`, default 8.
    pub bits: u8,
    /// `/E`, default `Raw`.
    pub encoding: Encoding,
}

/// A sound object's own samples, kept undecoded for the hand-off to decode and wrap.
#[derive(Debug, Clone)]
pub struct Samples {
    /// The name the written file is handed over under.
    pub name: String,
    /// Table 305's entries, read.
    pub format: SampleFormat,
    /// The sound object, whose data after Table 5's filters is the samples.
    pub stream: Arc<Stream>,
}

/// The RIFF WAVE file one sound's samples were written into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    /// The whole file.
    pub bytes: Vec<u8>,
    /// What the file does not carry of the stream's data, each said (trap 5).
    pub notes: Vec<String>,
}

impl SampleFormat {
    /// Reads Table 305 from a sound object's dictionary, with the sentences the hand-off owes: a
    /// sampling rate that is not a whole number, rounded to the one a WAVE header can state.
    ///
    /// # Errors
    ///
    /// The sentence a person is told: a `/CO` — Table 305 itself says "No standard values have
    /// been defined for the CO and CP entries", so compressed samples are in a format nobody named
    /// — an `/R` that is not a positive rate, a `/C` or `/B` the file cannot carry, an `/E` the
    /// table does not name, and µ-law or A-law codes that are not eight bits.
    pub fn read(document: &Document, sound: &Dictionary) -> Result<(Self, Vec<String>), String> {
        let compression = document.get_key(sound, "CO");
        if !matches!(compression, Object::Null) {
            let named = compression.as_name().map_or_else(
                || "a value that is not a name".to_owned(),
                |name| format!("/{}", String::from_utf8_lossy(name.as_bytes())),
            );
            return Err(format!(
                "the sound object's samples are compressed by Table 305's /CO {named}, for which no \
                 standard value is defined, so this program cannot say what they are and wrote no \
                 file for a player"
            ));
        }
        let mut notes = Vec::new();
        let rate = document.get_key(sound, "R").as_number();
        let Some(stated) = rate.filter(|rate| rate.is_finite() && *rate >= 0.5) else {
            return Err(
                "Table 305's required /R is not a positive sampling rate, so the samples have no \
                 speed a file could state"
                    .to_owned(),
            );
        };
        let rounded = stated.round();
        if rounded > f64::from(u32::MAX) {
            return Err(format!(
                "Table 305's /R {stated} is faster than a RIFF WAVE header can state"
            ));
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number from 1 to u32::MAX, by the two checks above"
        )]
        let whole = rounded as u32;
        if (rounded - stated).abs() > f64::EPSILON {
            notes.push(format!(
                "Table 305's /R {stated}, written as {whole} samples per second, the nearest rate \
                 a RIFF WAVE header can state"
            ));
        }
        let channels = match document.get_key(sound, "C") {
            Object::Null => 1,
            stated => stated
                .as_integer()
                .and_then(|channels| u16::try_from(channels).ok())
                .filter(|channels| *channels >= 1)
                .ok_or_else(|| {
                    "Table 305's /C is not a number of channels from 1 to 65535, the most a RIFF \
                     WAVE header can state"
                        .to_owned()
                })?,
        };
        let bits = match document.get_key(sound, "B") {
            Object::Null => 8,
            stated => stated
                .as_integer()
                .and_then(|bits| u8::try_from(bits).ok())
                .filter(|bits| (1..=32).contains(bits))
                .ok_or_else(|| {
                    "Table 305's /B is not a sample size from 1 to 32 bits, the sizes this \
                     program writes into a RIFF WAVE file"
                        .to_owned()
                })?,
        };
        let encoding = match document.get_key(sound, "E") {
            Object::Null => Encoding::Raw,
            stated => match stated.as_name().map(pdf_syntax::Name::as_bytes) {
                Some(b"Raw") => Encoding::Raw,
                Some(b"Signed") => Encoding::Signed,
                Some(b"muLaw") => Encoding::MuLaw,
                Some(b"ALaw") => Encoding::ALaw,
                _ => {
                    return Err(
                        "Table 305's /E is none of the encodings the table names (Raw, Signed, \
                         muLaw, ALaw), so the samples cannot be read"
                            .to_owned(),
                    );
                }
            },
        };
        // §13.3: "If the encoding is muLaw, R shall be 8000 samples per channel, C shall be 1
        // channel, and B shall be 8 bits per channel." A WAVE file carries any rate and channel
        // count, so only the code size decides whether the samples can be written: a µ-law or an
        // A-law code is eight bits.
        if encoding.is_companded() && bits != 8 {
            return Err(format!(
                "Table 305's /E /{} states {bits}-bit codes, and a µ-law or A-law code is eight \
                 bits, so the samples cannot be read",
                encoding.as_str()
            ));
        }
        Ok((
            Self {
                rate: whole,
                channels,
                bits,
                encoding,
            },
            notes,
        ))
    }

    /// The content type of the file the samples become: RFC 2361's `audio/vnd.wave` with the
    /// format tag as its codec.
    #[must_use]
    pub fn content_type(&self) -> String {
        format!("audio/vnd.wave; codec={}", self.encoding.format_tag())
    }

    /// What the samples are, in a phrase.
    #[must_use]
    pub fn describe(&self) -> String {
        let channels = if self.channels == 1 {
            "1 channel".to_owned()
        } else {
            format!("{} channels", self.channels)
        };
        format!(
            "a sound's own samples, {} per second, {channels}, {}-bit {}",
            self.rate,
            self.bits,
            self.encoding.as_str()
        )
    }

    /// Bytes one sample takes in the file: the smallest whole number that holds it.
    fn container(self) -> usize {
        usize::from(self.bits.div_ceil(8))
    }
}

impl Samples {
    /// The samples of a sound object with no `/F`, with the sentences [`SampleFormat::read`] owes.
    ///
    /// # Errors
    ///
    /// [`SampleFormat::read`]'s sentences.
    pub fn read(document: &Document, stream: &Arc<Stream>) -> Result<(Self, Vec<String>), String> {
        let (format, notes) = SampleFormat::read(document, &stream.dict)?;
        Ok((
            Self {
                name: FILE_NAME.to_owned(),
                format,
                stream: Arc::clone(stream),
            },
            notes,
        ))
    }
}

/// Writes a sound object's decoded samples as a RIFF WAVE file, under `limit` bytes.
///
/// ISO 32000-2 §13.3 says how the stream holds them:
///
/// > Sample values shall be stored in the stream with the most significant bits first (big-endian
/// > order for samples larger than 8 bits). Samples that are not a multiple of 8 bits shall be
/// > packed into consecutive bytes, starting at the most significant end.
///
/// > For dual-channel stereophonic sounds, the samples shall be stored in an interleaved format,
/// > with each sample value for the left (channel 1) preceding the corresponding sample for the
/// > right (channel 2).
///
/// The WAVE form interleaves channels the same way, so the order is kept and each sample is moved
/// into the form's layout: unpacked from the bit stream, set in the most significant bits of its
/// bytes, written least significant byte first, and re-biased where the two signedness rules
/// differ — a `Signed` sample of eight bits or fewer becomes unsigned, a `Raw` one of more becomes
/// two's complement. Bits left over after the last whole sample are the packing's padding; whole
/// samples that complete no frame of every channel are not written and are said.
///
/// `limit` is the bound a decoded stream is held to (`pdf_syntax::Limits::max_stream_len`), because
/// the file is a stream this program hands over and unpacking one-bit samples writes eight times
/// the bytes it reads.
///
/// # Errors
///
/// A file that would pass `limit` or the four gigabytes a RIFF chunk's size can state, and samples
/// that fill no frame at all.
pub fn wave(format: &SampleFormat, samples: &[u8], limit: usize) -> Result<Wave, String> {
    let no_frame = || {
        "the sound object's stream holds no whole frame of samples, so there was nothing to write"
            .to_owned()
    };
    let channels = usize::from(format.channels);
    let container = format.container();
    let whole_samples = samples
        .len()
        .saturating_mul(8)
        .checked_div(usize::from(format.bits))
        .ok_or_else(no_frame)?;
    let frames = whole_samples.checked_div(channels).unwrap_or(0);
    if frames == 0 {
        return Err(no_frame());
    }
    let mut notes = Vec::new();
    let unframed = whole_samples.checked_rem(channels).unwrap_or(0);
    if unframed != 0 {
        notes.push(format!(
            "the last {unframed} sample(s) of the sound object complete no frame of {channels} \
             channels (§13.3's interleaved order) and were not written"
        ));
    }
    let written = frames.saturating_mul(channels);
    let block = channels.saturating_mul(container);
    let data_len = frames.saturating_mul(block);
    let companded = format.encoding.is_companded();
    // RIFF's twelve-byte header, the format chunk — eight bytes of header and sixteen of PCM's
    // block, or eighteen with the extra-size field — then the twelve-byte `fact` chunk a non-PCM
    // file carries, and the data chunk's eight-byte header.
    let header: usize = if companded { 58 } else { 44 };
    let padded = data_len.saturating_add(data_len & 1);
    let file_len = padded.saturating_add(header);
    let too_large = || {
        format!(
            "the sound object's samples would be a {file_len}-byte file, more than this program \
             writes for a player"
        )
    };
    if file_len > limit {
        return Err(too_large());
    }
    let riff_len = u32::try_from(file_len.saturating_sub(8)).map_err(|_| too_large())?;
    let data_len_u32 = u32::try_from(data_len).map_err(|_| too_large())?;
    let frames_u32 = u32::try_from(frames).map_err(|_| too_large())?;
    let block_u16 = u16::try_from(block).map_err(|_| too_large())?;
    let bytes_per_second = format
        .rate
        .checked_mul(u32::from(block_u16))
        .ok_or_else(too_large)?;

    let mut out = Vec::with_capacity(file_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_len.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(if companded { 18_u32 } else { 16 }).to_le_bytes());
    out.extend_from_slice(&format.encoding.format_tag().to_le_bytes());
    out.extend_from_slice(&format.channels.to_le_bytes());
    out.extend_from_slice(&format.rate.to_le_bytes());
    out.extend_from_slice(&bytes_per_second.to_le_bytes());
    out.extend_from_slice(&block_u16.to_le_bytes());
    out.extend_from_slice(&u16::from(format.bits).to_le_bytes());
    if companded {
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(b"fact");
        out.extend_from_slice(&4_u32.to_le_bytes());
        out.extend_from_slice(&frames_u32.to_le_bytes());
    }
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len_u32.to_le_bytes());
    if companded {
        // Eight-bit codes, which a WAVE file stores as they are.
        out.extend_from_slice(samples.get(..written).unwrap_or(samples));
    } else {
        let container_bits = u32::from(format.bits.div_ceil(8)).saturating_mul(8);
        let shift = container_bits.saturating_sub(u32::from(format.bits));
        let top = 1_u32
            .checked_shl(container_bits.saturating_sub(1))
            .unwrap_or(0);
        for index in 0..written {
            let Some(value) = sample(samples, index, format.bits) else {
                break;
            };
            // `shift` is under eight, and `value` holds `bits` bits, so nothing is shifted out.
            let justified = value.wrapping_shl(shift);
            let rebiased = match (format.encoding, container) {
                (Encoding::Signed, 1) | (Encoding::Raw, 2..) => justified ^ top,
                _ => justified,
            };
            out.extend_from_slice(rebiased.to_le_bytes().get(..container).unwrap_or_default());
        }
    }
    out.resize(file_len, 0);
    Ok(Wave { bytes: out, notes })
}

/// Sample `index` of a stream packed most significant bit first, as an unsigned value of `bits`
/// bits; `None` past the end.
fn sample(samples: &[u8], index: usize, bits: u8) -> Option<u32> {
    let bits = usize::from(bits);
    if bits.is_multiple_of(8) {
        let width = bits.checked_div(8)?;
        let at = index.checked_mul(width)?;
        let bytes = samples.get(at..at.checked_add(width)?)?;
        // At most four bytes, by `SampleFormat::read`'s bound on `/B`, so none is shifted out.
        return Some(bytes.iter().fold(0_u32, |value, byte| {
            value.wrapping_shl(8) | u32::from(*byte)
        }));
    }
    let start = index.checked_mul(bits)?;
    (start..start.checked_add(bits)?).try_fold(0_u32, |value, bit| {
        let byte = samples.get(bit.checked_div(8)?)?;
        let mask = 0x80_u8.checked_shr(u32::try_from(bit & 7).ok()?)?;
        Some(value.wrapping_shl(1) | u32::from(byte & mask != 0))
    })
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test code: offsets into the header this file's own writer laid out"
)]
mod tests {
    use super::{Encoding, SampleFormat, wave};

    /// A format with every field stated.
    fn format(channels: u16, bits: u8, encoding: Encoding) -> SampleFormat {
        SampleFormat {
            rate: 8000,
            channels,
            bits,
            encoding,
        }
    }

    /// The little-endian field of `width` bytes at `at`.
    fn field(bytes: &[u8], at: usize, width: usize) -> u32 {
        bytes[at..at + width]
            .iter()
            .rev()
            .fold(0, |value, byte| (value << 8) | u32::from(*byte))
    }

    /// The data chunk's payload of a PCM file (its header is 44 bytes).
    fn pcm_data(file: &[u8]) -> &[u8] {
        let len = usize::try_from(field(file, 40, 4)).expect("a test's length");
        &file[44..44 + len]
    }

    /// A PCM file's header, field by field: RIFF's size, the format block, and the data's length.
    #[test]
    fn a_pcm_header_states_the_tables_rate_channels_and_bits() {
        let file = wave(&format(2, 16, Encoding::Signed), &[0; 8], usize::MAX)
            .expect("two frames are written");
        assert_eq!(&file.bytes[0..4], b"RIFF");
        assert_eq!(field(&file.bytes, 4, 4) as usize, file.bytes.len() - 8);
        assert_eq!(&file.bytes[8..16], b"WAVEfmt ");
        assert_eq!(field(&file.bytes, 16, 4), 16, "PCM's format block");
        assert_eq!(field(&file.bytes, 20, 2), 1, "WAVE_FORMAT_PCM");
        assert_eq!(field(&file.bytes, 22, 2), 2, "channels");
        assert_eq!(field(&file.bytes, 24, 4), 8000, "rate");
        assert_eq!(field(&file.bytes, 28, 4), 8000 * 4, "bytes per second");
        assert_eq!(field(&file.bytes, 32, 2), 4, "block");
        assert_eq!(field(&file.bytes, 34, 2), 16, "bits");
        assert_eq!(&file.bytes[36..40], b"data");
        assert_eq!(field(&file.bytes, 40, 4), 8);
        assert!(file.notes.is_empty(), "{:?}", file.notes);
    }

    /// Eight-bit `Raw` is the WAVE form's own unsigned layout and crosses unchanged.
    #[test]
    fn eight_bit_raw_samples_cross_unchanged() {
        let file = wave(&format(1, 8, Encoding::Raw), &[0, 128, 255], usize::MAX).expect("written");
        assert_eq!(pcm_data(&file.bytes), [0, 128, 255]);
        assert_eq!(file.bytes.len() % 2, 0, "an odd chunk is padded");
    }

    /// Eight-bit `Signed` is re-biased: two's complement 0 is the unsigned midpoint 128.
    #[test]
    fn eight_bit_signed_samples_are_rebiased_to_unsigned() {
        let file = wave(
            &format(1, 8, Encoding::Signed),
            &[0x00, 0x7F, 0x80, 0xFF],
            usize::MAX,
        )
        .expect("written");
        assert_eq!(pcm_data(&file.bytes), [0x80, 0xFF, 0x00, 0x7F]);
    }

    /// Sixteen-bit `Signed`, big-endian in the stream, is byte-swapped and kept two's complement.
    #[test]
    fn big_endian_sixteen_bit_signed_samples_are_swapped() {
        let file = wave(
            &format(1, 16, Encoding::Signed),
            &[0x12, 0x34, 0xFF, 0xFE],
            usize::MAX,
        )
        .expect("written");
        assert_eq!(pcm_data(&file.bytes), [0x34, 0x12, 0xFE, 0xFF]);
    }

    /// Sixteen-bit `Raw` is unsigned and the WAVE form's sixteen bits are not: swapped and
    /// re-biased, so the unsigned midpoint 0x8000 becomes zero.
    #[test]
    fn sixteen_bit_raw_samples_are_swapped_and_rebiased() {
        let file = wave(
            &format(1, 16, Encoding::Raw),
            &[0x80, 0x00, 0xFF, 0xFF],
            usize::MAX,
        )
        .expect("written");
        assert_eq!(pcm_data(&file.bytes), [0x00, 0x00, 0xFF, 0x7F]);
    }

    /// Twelve-bit samples packed across byte boundaries, most significant end first, are unpacked
    /// and set in the high bits of two bytes: `0xABC` and `0x123` from `AB C1 23`.
    #[test]
    fn packed_twelve_bit_samples_are_unpacked_into_the_high_bits() {
        let file = wave(
            &format(1, 12, Encoding::Signed),
            &[0xAB, 0xC1, 0x23],
            usize::MAX,
        )
        .expect("written");
        assert_eq!(pcm_data(&file.bytes), [0xC0, 0xAB, 0x30, 0x12]);
        assert_eq!(
            field(&file.bytes, 34, 2),
            12,
            "the sample size, not the container"
        );
    }

    /// Four-bit `Raw` fills the high nibble of each byte, unsigned.
    #[test]
    fn four_bit_raw_samples_fill_the_high_nibble() {
        let file = wave(&format(1, 4, Encoding::Raw), &[0x1F], usize::MAX).expect("written");
        assert_eq!(pcm_data(&file.bytes), [0x10, 0xF0]);
    }

    /// µ-law and A-law codes cross unchanged under RFC 2361's tags 7 and 6, with the extra-size
    /// field and the `fact` chunk a non-PCM file carries.
    #[test]
    fn companded_codes_cross_unchanged_with_a_fact_chunk() {
        for (encoding, tag) in [(Encoding::MuLaw, 7), (Encoding::ALaw, 6)] {
            let file = wave(&format(1, 8, encoding), &[1, 2, 3, 4], usize::MAX).expect("written");
            assert_eq!(field(&file.bytes, 16, 4), 18);
            assert_eq!(field(&file.bytes, 20, 2), tag);
            assert_eq!(field(&file.bytes, 36, 2), 0, "no extra format bytes");
            assert_eq!(&file.bytes[38..42], b"fact");
            assert_eq!(field(&file.bytes, 46, 4), 4, "frames");
            assert_eq!(&file.bytes[50..54], b"data");
            assert_eq!(&file.bytes[58..], [1, 2, 3, 4]);
        }
    }

    /// A stereo stream with a sample left over writes the whole frames and says the rest.
    #[test]
    fn a_sample_that_completes_no_frame_is_said() {
        let file = wave(&format(2, 8, Encoding::Raw), &[1, 2, 3], usize::MAX).expect("written");
        assert_eq!(pcm_data(&file.bytes), [1, 2]);
        assert!(
            file.notes
                .iter()
                .any(|note| note.contains("complete no frame of 2 channels")),
            "{:?}",
            file.notes
        );
    }

    /// The bound a decoded stream is held to holds the file too, and no frame is no file.
    #[test]
    fn a_file_past_the_bound_or_with_no_frame_is_refused() {
        let why = wave(&format(1, 1, Encoding::Raw), &[0; 16], 100).expect_err("128 samples");
        assert!(why.contains("more than this program writes"), "{why}");
        let why = wave(&format(2, 16, Encoding::Raw), &[0; 3], usize::MAX).expect_err("no frame");
        assert!(why.contains("no whole frame"), "{why}");
    }
}
