//! ISO 32000-2 §13.3's own sample formats, decoded to one: thirty-two-bit two's complement, the
//! amplitude in the most significant bits.
//!
//! This is the code that reads the document's bytes, so it runs in `pdf-media-worker` and nowhere
//! else (ADR 3297); the host's half of this crate never calls it. It is also the fuzz target's
//! entry (`fuzz/fuzz_targets/media_sound.rs`).
//!
//! §13.3 says how a sound object holds its samples:
//!
//! > Sample values shall be stored in the stream with the most significant bits first (big-endian
//! > order for samples larger than 8 bits). Samples that are not a multiple of 8 bits shall be
//! > packed into consecutive bytes, starting at the most significant end. If a sample extends
//! > across a byte boundary, the most significant bits shall be placed in the first byte, followed
//! > by less significant bits in subsequent bytes. For dual-channel stereophonic sounds, the samples
//! > shall be stored in an interleaved format, with each sample value for the left (channel 1)
//! > preceding the corresponding sample for the right (channel 2).
//!
//! and what a player does with them:
//!
//! > Sound players shall convert between formats, downsample rates, and combine channels as
//! > necessary to render sound on the target platform.
//!
//! **The conversion between formats is this module's half of that sentence**: every encoding
//! Table 305's `/E` names becomes one sample format, so that the window's sink receives one kind of
//! number whatever the document wrote. The rate and the channels cross as the object states them,
//! because what the target platform takes is the sink's to know — downsampling and combining
//! channels belong to the sink, beside the device they are for.
//!
//! **One output format, chosen so that no input loses a bit.** Table 305's `/B` admits samples up
//! to thirty-two bits in this tree's reading (`pdf_model::media::wave`), so a sample is set in the
//! most significant bits of an `i32` and the rest left zero — the layout a RIFF WAVE file gives a
//! sample, and one in which full scale is the same number for every `/B`. The companded codes are
//! decoded by ITU-T Recommendation G.711 (1988), whose tables give each code a value of its uniform
//! code: fourteen bits for the µ-law (Table 2), thirteen for the A-law (Table 1), placed the same
//! way.

/// Table 305's `/E`: how one sample value is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// `Raw`, the default: "Unspecified or unsigned values in the range 0 to 2B - 1". Read as
    /// unsigned, the reading the table's range states.
    Raw,
    /// `Signed`: "Twos-complement values".
    Signed,
    /// `muLaw`: G.711 µ-law codes, eight bits each.
    MuLaw,
    /// `ALaw`: G.711 A-law codes, eight bits each. ISO 32000-1's Table 159 names it; ISO
    /// 32000-2's printed Table 305 lists it beside `muLaw`, and the text extraction in `doc/md/`
    /// drops the line (`pdf_model::media::wave::Encoding::ALaw`).
    ALaw,
    /// A RIFF WAVE file's PCM sample of sixteen bits or more: whole bytes, least significant
    /// first, two's complement (`crate::wave`, ADR 3349). Eight-bit WAVE samples are unsigned and
    /// cross as [`Self::Raw`].
    Little,
}

impl Encoding {
    /// The byte this encoding crosses the wire as.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Signed => 1,
            Self::MuLaw => 2,
            Self::ALaw => 3,
            Self::Little => 4,
        }
    }

    /// The encoding a wire byte names, or `None` for a byte no encoding has.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Raw),
            1 => Some(Self::Signed),
            2 => Some(Self::MuLaw),
            3 => Some(Self::ALaw),
            4 => Some(Self::Little),
            _ => None,
        }
    }

    /// Whether the samples are G.711 codes, which are eight bits each.
    #[must_use]
    pub fn is_companded(self) -> bool {
        matches!(self, Self::MuLaw | Self::ALaw)
    }
}

/// Table 305's entries for a sound whose samples the object holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundFormat {
    /// `/R`, in samples per second per channel, as the whole number the host rounded it to.
    pub rate: u32,
    /// `/C`, default 1.
    pub channels: u16,
    /// `/B`, default 8.
    pub bits: u8,
    /// `/E`, default `Raw`.
    pub encoding: Encoding,
}

impl SoundFormat {
    /// The sentence saying why these entries describe no samples this decoder reads, or `None`.
    ///
    /// The worker asks it of every request rather than trusting the host's reading, because a
    /// format is a claim about bytes the worker is about to walk.
    #[must_use]
    pub fn refusal(&self) -> Option<String> {
        if self.rate == 0 {
            return Some("a sound whose /R is zero samples per second has no speed".to_owned());
        }
        if self.channels == 0 {
            return Some("a sound of no channels holds no samples".to_owned());
        }
        if !(1..=32).contains(&self.bits) {
            return Some(format!(
                "a {}-bit sample is outside the 1 to 32 bits this decoder reads",
                self.bits
            ));
        }
        if self.encoding.is_companded() && self.bits != 8 {
            // §13.3: "If the encoding is muLaw, R shall be 8000 samples per channel, C shall be 1
            // channel, and B shall be 8 bits per channel." G.711's codes are eight bits (its
            // section 3.1); the rate and the channels do not change how a code is read.
            return Some(format!(
                "a G.711 code is eight bits, and the sound states {}-bit codes",
                self.bits
            ));
        }
        if self.encoding == Encoding::Little && !matches!(self.bits, 16 | 24 | 32) {
            // A WAVE sample of nine bits or more sits in two, three or four whole bytes (`crate::wave`).
            return Some(format!(
                "a little-endian WAVE sample is two, three or four whole bytes, and the sound \
                 states {} bits",
                self.bits
            ));
        }
        None
    }

    /// Bits one frame — a sample of every channel — takes in the stream.
    #[must_use]
    pub fn frame_bits(&self) -> u64 {
        u64::from(self.channels).saturating_mul(u64::from(self.bits))
    }

    /// The fewest bytes that end on a frame's boundary: a stream cut at a multiple of this cuts no
    /// sample and no frame in two, which is how a host splits a sound into requests.
    #[must_use]
    pub fn whole_frames_unit(&self) -> u64 {
        let frame = self.frame_bits();
        // lcm(frame, 8) / 8: the eight-bit byte's factors of two that the frame lacks.
        frame.checked_div(gcd(frame, 8)).unwrap_or(0)
    }
}

/// The greatest common divisor, by Euclid's algorithm.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a.checked_rem(b).unwrap_or(0));
    }
    a
}

/// One request's samples, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The samples of every whole frame, interleaved as the stream holds them, each in the most
    /// significant bits of an `i32`.
    pub samples: Vec<i32>,
    /// Whole samples after the last whole frame, which complete no frame and are not decoded.
    pub unframed: u64,
}

impl Decoded {
    /// The number of frames, given the channels they were decoded for.
    #[must_use]
    pub fn frames(&self, channels: u16) -> usize {
        self.samples
            .len()
            .checked_div(usize::from(channels))
            .unwrap_or(0)
    }
}

/// How many samples, and how many after the last whole frame, `len` bytes of `format` hold.
///
/// Bits after the last whole sample are §13.3's packing padding and hold no sample.
#[must_use]
pub fn sample_counts(format: &SoundFormat, len: usize) -> (u64, u64) {
    let bits = u64::from(format.bits.max(1));
    let whole = u64::try_from(len)
        .unwrap_or(u64::MAX)
        .saturating_mul(8)
        .checked_div(bits)
        .unwrap_or(0);
    let channels = u64::from(format.channels.max(1));
    let framed = whole
        .checked_div(channels)
        .unwrap_or(0)
        .saturating_mul(channels);
    (framed, whole.saturating_sub(framed))
}

/// Decodes `bytes`, a run of a sound object's data, under `format`.
///
/// # Errors
///
/// [`SoundFormat::refusal`]'s sentence, and a run whose samples would pass `max_samples`.
pub fn decode(format: &SoundFormat, bytes: &[u8], max_samples: usize) -> Result<Decoded, String> {
    if let Some(refusal) = format.refusal() {
        return Err(refusal);
    }
    let (framed, unframed) = sample_counts(format, bytes.len());
    let count = usize::try_from(framed)
        .ok()
        .filter(|count| *count <= max_samples)
        .ok_or_else(|| {
            format!("{framed} samples are past the {max_samples} one request may decode")
        })?;
    let mut samples = Vec::with_capacity(count);
    let mut reader = BitReader::new(bytes, format.bits);
    while samples.len() < count {
        let Some(code) = reader.next() else {
            break;
        };
        samples.push(convert(*format, code));
    }
    Ok(Decoded { samples, unframed })
}

/// One sample's code, as the stream holds it, in the output format.
fn convert(format: SoundFormat, code: u32) -> i32 {
    let shift = 32u32.saturating_sub(u32::from(format.bits));
    match format.encoding {
        // The code in the top bits, read as two's complement.
        Encoding::Signed => (code << shift).cast_signed(),
        // The reader took the bytes most significant first; a WAVE sample's first byte is its
        // least significant, so the bytes are reversed into the top of the word, which is where
        // the sample's amplitude sits, and read as two's complement.
        Encoding::Little => ((code << shift).swap_bytes() << shift).cast_signed(),
        // Offset binary: an unsigned code less half its range is the same top-aligned word with
        // its most significant bit flipped.
        Encoding::Raw => ((code << shift) ^ 0x8000_0000).cast_signed(),
        // G.711 Table 2's fourteen-bit uniform code, set in the top fourteen bits.
        Encoding::MuLaw => mu_law(code) << 18,
        // G.711 Table 1's thirteen-bit uniform code, set in the top thirteen bits.
        Encoding::ALaw => a_law(code) << 19,
    }
}

/// A µ-law character signal's quantized value at the decoder output, in G.711 Table 2's uniform
/// units, where 8159 is the law's full scale (Table 2's Note 1).
///
/// G.711 Table 2a's rows: bit 1 is the polarity, 1 for a positive value; bits 2 to 4 name the segment
/// counted down from 8 and bits 5 to 8 the interval within it, likewise inverted — `1 0000000` is
/// 8031, `1 1111111` is 0. Inverting all eight bits makes the segment *s* and the interval *q*
/// count up, and the column of quantized values is then ((2*q* + 33) << *s*) − 33: 0, 2, 4, … 30 in
/// the first segment, 33 to 93 in the second, up to 4191 to 8031 in the eighth. Section 3.2's own
/// example: `00000010` decodes to −7519.
fn mu_law(code: u32) -> i32 {
    let inverted = !code & 0xFF;
    let segment = (inverted >> 4) & 0x7;
    let interval = inverted & 0xF;
    let magnitude = ((interval << 1).saturating_add(33) << segment).saturating_sub(33);
    let value = i32::try_from(magnitude).unwrap_or(0);
    if inverted & 0x80 == 0 {
        value
    } else {
        value.saturating_neg()
    }
}

/// An A-law character signal's quantized value at the decoder output, in G.711 Table 1's uniform
/// units, where 4096 is the law's full scale (Table 1's Note 1).
///
/// Table 1's character signals have their even bits inverted (Note 2), so the code is first
/// combined by exclusive or with `01010101`. Then bit 1 is the polarity, 1 for a positive value (G.711 Table 1a), bits 2 to
/// 4 the segment *s* and bits 5 to 8 the interval *q*; the quantized value is 2*q* + 1 in the first
/// segment, whose intervals are two units wide from zero, and (2*q* + 33) << (*s* − 1) in each
/// later one — `1 0000000` before inversion is 1, `1 1111111` is 4032, `0 0000000` is −1.
fn a_law(code: u32) -> i32 {
    let restored = (code ^ 0x55) & 0xFF;
    let segment = (restored >> 4) & 0x7;
    let interval = restored & 0xF;
    let magnitude = if segment == 0 {
        (interval << 1).saturating_add(1)
    } else {
        (interval << 1).saturating_add(33) << segment.saturating_sub(1)
    };
    let value = i32::try_from(magnitude).unwrap_or(0);
    if restored & 0x80 == 0 {
        value.saturating_neg()
    } else {
        value
    }
}

/// A word whose low `count` bits are set, for `count` from 0 to 64.
fn low_bits(count: u32) -> u64 {
    u64::MAX
        .checked_shr(64u32.saturating_sub(count))
        .unwrap_or(0)
}

/// Reads codes of one width from a byte run, most significant bit first.
struct BitReader<'a> {
    /// The bytes not yet taken into `held`.
    bytes: std::slice::Iter<'a, u8>,
    /// Bits taken and not yet read, in the low `held_bits` bits.
    held: u64,
    /// How many bits `held` holds; always below `width` between reads, so below 32.
    held_bits: u32,
    /// The width of one code, 1 to 32.
    width: u32,
}

impl<'a> BitReader<'a> {
    /// A reader of `width`-bit codes from the start of `bytes`.
    fn new(bytes: &'a [u8], width: u8) -> Self {
        Self {
            bytes: bytes.iter(),
            held: 0,
            held_bits: 0,
            width: u32::from(width),
        }
    }
}

impl Iterator for BitReader<'_> {
    type Item = u32;

    /// The next code, or `None` where fewer bits than a code remain.
    fn next(&mut self) -> Option<u32> {
        while self.held_bits < self.width {
            let byte = self.bytes.next()?;
            // At most 31 bits held and eight more: under the 64 the word holds.
            self.held = (self.held << 8) | u64::from(*byte);
            self.held_bits = self.held_bits.saturating_add(8);
        }
        self.held_bits = self.held_bits.saturating_sub(self.width);
        let code = (self.held >> self.held_bits) & low_bits(self.width);
        self.held &= low_bits(self.held_bits);
        u32::try_from(code).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A format of `bits` and `encoding`, one channel at 8000 samples a second.
    fn mono(bits: u8, encoding: Encoding) -> SoundFormat {
        SoundFormat {
            rate: 8000,
            channels: 1,
            bits,
            encoding,
        }
    }

    /// Decodes and returns the samples, failing the test on a refusal.
    fn samples(format: SoundFormat, bytes: &[u8]) -> Vec<i32> {
        match decode(&format, bytes, usize::MAX) {
            Ok(decoded) => decoded.samples,
            Err(refusal) => panic!("refused: {refusal}"),
        }
    }

    /// G.711 Table 2a's positive rows, the quantized value of each segment's end code (column 7),
    /// and section 3.2's one negative example.
    #[test]
    fn mu_law_codes_decode_to_g711_table_2_values() {
        let table: [(u8, i32); 11] = [
            (0b1000_0000, 8031),
            (0b1000_1111, 4191),
            (0b1001_1111, 2079),
            (0b1010_1111, 1023),
            (0b1011_1111, 495),
            (0b1100_1111, 231),
            (0b1101_1111, 99),
            (0b1110_1111, 33),
            (0b1111_1110, 2),
            (0b1111_1111, 0),
            // §3.2: "the character signal ... should be 00000010 and the value at the decoder
            // output is -7519".
            (0b0000_0010, -7519),
        ];
        let codes: Vec<u8> = table.iter().map(|(code, _)| *code).collect();
        let decoded = samples(mono(8, Encoding::MuLaw), &codes);
        let expected: Vec<i32> = table.iter().map(|(_, value)| value << 18).collect();
        assert_eq!(decoded, expected);
    }

    /// G.711 Tables 1a and 1b, given before the even bits are inverted (column 6), so each is
    /// combined by exclusive or with `01010101` to be the code a stream carries.
    #[test]
    fn a_law_codes_decode_to_g711_table_1_values() {
        let table: [(u8, i32); 12] = [
            (0b1111_1111, 4032),
            (0b1111_0000, 2112),
            (0b1110_0000, 1056),
            (0b1101_0000, 528),
            (0b1100_0000, 264),
            (0b1011_0000, 132),
            (0b1010_0000, 66),
            (0b1000_0000, 1),
            (0b0000_0000, -1),
            (0b0010_0000, -66),
            (0b0110_0000, -1056),
            (0b0111_1111, -4032),
        ];
        let codes: Vec<u8> = table.iter().map(|(before, _)| before ^ 0x55).collect();
        let decoded = samples(mono(8, Encoding::ALaw), &codes);
        let expected: Vec<i32> = table.iter().map(|(_, value)| value << 19).collect();
        assert_eq!(decoded, expected);
    }

    /// Table 305's `Raw`: "unsigned values in the range 0 to 2B - 1", so the middle of the range
    /// is silence and its two ends are the extremes.
    #[test]
    fn raw_samples_are_unsigned_about_the_middle_of_their_range() {
        let decoded = samples(mono(8, Encoding::Raw), &[0x00, 0x80, 0xFF]);
        assert_eq!(decoded, vec![i32::MIN, 0, 0x7F << 24]);
    }

    /// Table 305's `Signed`, sixteen bits: §13.3's "big-endian order for samples larger than 8
    /// bits".
    #[test]
    fn signed_sixteen_bit_samples_are_big_endian_twos_complement() {
        let decoded = samples(
            mono(16, Encoding::Signed),
            &[0x80, 0x00, 0x00, 0x01, 0x7F, 0xFF],
        );
        assert_eq!(decoded, vec![i32::MIN, 1 << 16, 0x7FFF << 16]);
    }

    /// §13.3: "Samples that are not a multiple of 8 bits shall be packed into consecutive bytes,
    /// starting at the most significant end", and a sample that crosses a byte boundary keeps its
    /// most significant bits in the first byte. Twelve-bit signed samples `0x800`, `0x001` and
    /// `0x7FF` are the bytes `80 00 01 7F F0`, the last four bits padding.
    #[test]
    fn samples_across_byte_boundaries_are_read_most_significant_bits_first() {
        let decoded = samples(mono(12, Encoding::Signed), &[0x80, 0x00, 0x01, 0x7F, 0xF0]);
        assert_eq!(decoded, vec![i32::MIN, 1 << 20, 0x7FF << 20]);
        // Four-bit raw samples, two to a byte, the first in the high half.
        let decoded = samples(mono(4, Encoding::Raw), &[0x0F, 0x8F]);
        assert_eq!(decoded, vec![i32::MIN, 0x7 << 28, 0, 0x7 << 28]);
    }

    /// §13.3: "each sample value for the left (channel 1) preceding the corresponding sample for
    /// the right (channel 2)" — kept interleaved, and a last sample with no partner is not a frame.
    #[test]
    fn stereo_samples_stay_interleaved_and_a_lone_last_sample_is_counted_not_decoded() {
        let format = SoundFormat {
            rate: 22_050,
            channels: 2,
            bits: 8,
            encoding: Encoding::Signed,
        };
        let decoded = decode(&format, &[0x01, 0xFF, 0x02, 0xFE, 0x03], usize::MAX)
            .unwrap_or_else(|refusal| panic!("refused: {refusal}"));
        assert_eq!(
            decoded.samples,
            vec![1 << 24, -(1 << 24), 2 << 24, -(2 << 24)]
        );
        assert_eq!(decoded.unframed, 1);
        assert_eq!(decoded.frames(2), 2);
    }

    /// Thirty-two-bit samples, the widest this decoder reads, lose nothing.
    #[test]
    fn thirty_two_bit_samples_cross_unchanged() {
        let decoded = samples(
            mono(32, Encoding::Signed),
            &[0x80, 0, 0, 0, 0x12, 0x34, 0x56, 0x78],
        );
        assert_eq!(decoded, vec![i32::MIN, 0x1234_5678]);
        let decoded = samples(mono(32, Encoding::Raw), &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(decoded, vec![i32::MAX]);
    }

    /// A one-bit sample is either extreme.
    #[test]
    fn one_bit_samples_are_the_two_extremes() {
        let decoded = samples(mono(1, Encoding::Raw), &[0b1010_0000]);
        assert_eq!(decoded.len(), 8);
        assert_eq!(decoded.first(), Some(&0));
        assert_eq!(decoded.get(1), Some(&i32::MIN));
    }

    /// The formats a request may not state, each refused by its sentence before a byte is read.
    #[test]
    fn formats_that_describe_no_samples_are_refused() {
        for format in [
            SoundFormat {
                rate: 0,
                ..mono(8, Encoding::Raw)
            },
            SoundFormat {
                channels: 0,
                ..mono(8, Encoding::Raw)
            },
            mono(0, Encoding::Raw),
            mono(33, Encoding::Signed),
            mono(16, Encoding::MuLaw),
            mono(4, Encoding::ALaw),
        ] {
            assert!(decode(&format, &[0; 4], usize::MAX).is_err(), "{format:?}");
        }
    }

    /// A run whose samples would pass the bound is refused before anything is allocated for them.
    #[test]
    fn a_run_past_the_sample_bound_is_refused() {
        assert!(decode(&mono(1, Encoding::Raw), &[0; 2], 15).is_err());
        assert!(decode(&mono(1, Encoding::Raw), &[0; 2], 16).is_ok());
    }

    /// The unit a host cuts a sound at holds whole frames: three channels of twelve bits make a
    /// thirty-six-bit frame, and nine bytes are two of them.
    /// A RIFF WAVE sample is least significant byte first: `0x01 0x80` is `0x8001`, the most
    /// negative sixteen-bit value but one, and three bytes `0x56 0x34 0x12` are `0x123456` — each
    /// set in the top of the word.
    #[test]
    fn little_endian_wave_samples_are_read_least_significant_byte_first() {
        let sixteen = samples(mono(16, Encoding::Little), &[0x01, 0x80, 0xFF, 0x7F]);
        assert_eq!(sixteen, vec![0x8001_0000_u32.cast_signed(), 0x7FFF_0000]);
        let twenty_four = samples(mono(24, Encoding::Little), &[0x56, 0x34, 0x12]);
        assert_eq!(twenty_four, vec![0x1234_5600]);
        let thirty_two = samples(mono(32, Encoding::Little), &[4, 3, 2, 1]);
        assert_eq!(thirty_two, vec![0x0102_0304]);
        assert!(mono(12, Encoding::Little).refusal().is_some());
        assert!(mono(8, Encoding::Little).refusal().is_some());
    }

    #[test]
    fn the_cutting_unit_ends_on_a_frame() {
        let format = SoundFormat {
            rate: 8000,
            channels: 3,
            bits: 12,
            encoding: Encoding::Signed,
        };
        assert_eq!(format.whole_frames_unit(), 9);
        assert_eq!(mono(16, Encoding::Signed).whole_frames_unit(), 2);
        assert_eq!(mono(1, Encoding::Raw).whole_frames_unit(), 1);
    }
}
