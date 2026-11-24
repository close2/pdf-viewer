//! What an H.264 stream says of its pictures' colour: the sequence parameter set's VUI (ITU-T
//! H.264 Annex E, E.1.1), read here because the decoder stops before it (RFC 0015 section 6, round
//! 4; ADR 3529).
//!
//! Each access unit's first slice names a picture parameter set, which names a sequence parameter
//! set; that set's VUI is the picture's [`Signal`]. The syntax walked to reach the VUI is H.264's
//! 7.3.2.1.1 (the sequence parameter set, including 7.3.2.1.1.1's scaling lists, which are read
//! only to be passed), 7.3.2.2's first two fields and 7.3.3's first three. Where the VUI or one of
//! its parts is absent, Annex E's inferences stand: `video_full_range_flag` 0,
//! `colour_primaries`, `transfer_characteristics` and `matrix_coefficients` 2 (unspecified),
//! `chroma_sample_loc_type_top_field` 0 (E.2.1). The clauses are those of Rec. ITU-T H.264 (V16)
//! (06/2026), held as `doc/ITU-T-H.264-202606.pdf` (ADR 3537); they are cited, not quoted.
//!
//! A set this reader cannot walk to its VUI is refused out loud rather than read as having none,
//! because a picture drawn by the wrong matrix is plausible and wrong (trap 5).

use crate::video::{Signal, VideoError};

/// The sequence parameter sets' signals by `seq_parameter_set_id` (0 to 31, 7.4.2.1.1), the
/// picture parameter sets' sequence sets by `pic_parameter_set_id` (0 to 255, 7.4.2.2), and the
/// signal of the last slice seen.
#[derive(Debug, Clone)]
pub(crate) struct Parameters {
    sequences: [Option<Signal>; 32],
    pictures: [Option<u8>; 256],
    current: Signal,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            sequences: [None; 32],
            pictures: [None; 256],
            current: Signal::default(),
        }
    }
}

impl Parameters {
    /// Reads `annex_b`'s parameter sets and returns the signal of its last slice's sequence — the
    /// picture the access unit codes — or the last one seen where it carries no slice.
    ///
    /// # Errors
    ///
    /// A sequence parameter set that ends before its VUI is read.
    pub(crate) fn observe(&mut self, annex_b: &[u8]) -> Result<Signal, VideoError> {
        for unit in nal_units(annex_b) {
            let Some((&header, payload)) = unit.split_first() else {
                continue;
            };
            match header & 0x1f {
                7 => {
                    let (id, signal) = sequence(&rbsp(payload, usize::MAX))?;
                    if let Some(slot) = self.sequences.get_mut(id) {
                        *slot = Some(signal);
                    }
                }
                8 => {
                    let mut bits = Bits::new(rbsp(payload, 16));
                    if let (Some(picture), Some(sequence)) = (bits.ue(), bits.ue())
                        && let (Some(slot), Ok(sequence)) = (
                            self.pictures.get_mut(index(picture)),
                            u8::try_from(sequence),
                        )
                    {
                        *slot = Some(sequence);
                    }
                }
                1 | 5 => {
                    let mut bits = Bits::new(rbsp(payload, 16));
                    let _first_mb_in_slice = bits.ue();
                    let _slice_type = bits.ue();
                    if let Some(signal) = bits
                        .ue()
                        .and_then(|picture| self.pictures.get(index(picture)).copied().flatten())
                        .and_then(|sequence| self.sequences.get(usize::from(sequence)).copied())
                        .flatten()
                    {
                        self.current = signal;
                    }
                }
                _ => {}
            }
        }
        Ok(self.current)
    }
}

/// The NAL units of an Annex B byte stream, each without its start code (B.1.1).
fn nal_units(stream: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut rest = stream;
    std::iter::from_fn(move || {
        let start = rest.windows(3).position(|window| window == [0, 0, 1])?;
        let after = rest.get(start.saturating_add(3)..)?;
        let end = after
            .windows(3)
            .position(|window| window == [0, 0, 1] || window == [0, 0, 0])
            .unwrap_or(after.len());
        let (unit, next) = after.split_at(end);
        rest = next;
        Some(unit)
    })
}

/// At most `limit` bytes of a NAL unit's payload with its emulation prevention bytes removed
/// (7.4.1: a `0x03` after two zero bytes).
fn rbsp(payload: &[u8], limit: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len().min(limit));
    let mut zeros = 0u8;
    for &byte in payload {
        if out.len() >= limit {
            break;
        }
        if zeros >= 2 && byte == 3 {
            zeros = 0;
            continue;
        }
        out.push(byte);
        zeros = if byte == 0 {
            zeros.saturating_add(1)
        } else {
            0
        };
    }
    out
}

/// The profiles whose sequence parameter set carries `chroma_format_idc` and what follows it
/// (7.3.2.1.1).
const CHROMA_PROFILES: [u32; 13] = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];

/// A sequence parameter set's `seq_parameter_set_id` and its VUI's signal.
fn sequence(rbsp: &[u8]) -> Result<(usize, Signal), VideoError> {
    let cut = || {
        VideoError::Refused(
            "a sequence parameter set that ends before its VUI is read (H.264 7.3.2.1.1, E.1.1)"
                .to_owned(),
        )
    };
    let mut bits = Bits::new(rbsp.to_vec());
    let profile = bits.u(8).ok_or_else(cut)?;
    bits.u(16).ok_or_else(cut)?;
    let id = bits.ue().ok_or_else(cut)?;
    if CHROMA_PROFILES.contains(&profile) {
        let chroma_format = bits.ue().ok_or_else(cut)?;
        if chroma_format == 3 {
            bits.u(1).ok_or_else(cut)?;
        }
        bits.ue().ok_or_else(cut)?;
        bits.ue().ok_or_else(cut)?;
        bits.u(1).ok_or_else(cut)?;
        if bits.flag().ok_or_else(cut)? {
            let lists = if chroma_format == 3 { 12 } else { 8 };
            for list in 0..lists {
                if bits.flag().ok_or_else(cut)? {
                    scaling_list(&mut bits, if list < 6 { 16 } else { 64 }).ok_or_else(cut)?;
                }
            }
        }
    }
    bits.ue().ok_or_else(cut)?;
    match bits.ue().ok_or_else(cut)? {
        0 => {
            bits.ue().ok_or_else(cut)?;
        }
        1 => {
            bits.u(1).ok_or_else(cut)?;
            bits.se().ok_or_else(cut)?;
            bits.se().ok_or_else(cut)?;
            // 7.4.2.1.1 bounds the cycle at 255; a larger count is not a set whose later fields
            // can be found, so it is refused rather than read short.
            let cycle = bits.ue().ok_or_else(cut)?;
            if cycle > 255 {
                return Err(VideoError::Refused(format!(
                    "a sequence parameter set whose num_ref_frames_in_pic_order_cnt_cycle is {cycle}, \
                     past 7.4.2.1.1's 255"
                )));
            }
            for _ in 0..cycle {
                bits.se().ok_or_else(cut)?;
            }
        }
        _ => {}
    }
    bits.ue().ok_or_else(cut)?;
    bits.u(1).ok_or_else(cut)?;
    bits.ue().ok_or_else(cut)?;
    bits.ue().ok_or_else(cut)?;
    if !bits.flag().ok_or_else(cut)? {
        bits.u(1).ok_or_else(cut)?;
    }
    bits.u(1).ok_or_else(cut)?;
    if bits.flag().ok_or_else(cut)? {
        for _ in 0..4 {
            bits.ue().ok_or_else(cut)?;
        }
    }
    let mut signal = Signal::default();
    if bits.flag().ok_or_else(cut)? {
        vui(&mut bits, &mut signal).ok_or_else(cut)?;
    }
    Ok((index(id), signal))
}

/// A scaling list read only to be passed (7.3.2.1.1.1).
fn scaling_list(bits: &mut Bits, size: usize) -> Option<()> {
    let (mut last, mut next) = (8i64, 8i64);
    for _ in 0..size {
        if next != 0 {
            next = last.saturating_add(bits.se()?).rem_euclid(256);
        }
        if next != 0 {
            last = next;
        }
    }
    Some(())
}

/// The VUI's fields up to its chroma siting (E.1.1), into `signal`.
fn vui(bits: &mut Bits, signal: &mut Signal) -> Option<()> {
    if bits.flag()? && bits.u(8)? == 255 {
        // aspect_ratio_idc 255, Extended_SAR: sar_width and sar_height.
        bits.u(32)?;
    }
    if bits.flag()? {
        bits.u(1)?;
    }
    if bits.flag()? {
        bits.u(3)?;
        signal.full_range = bits.flag()?;
        if bits.flag()? {
            signal.colour_primaries = byte(bits.u(8)?);
            signal.transfer_characteristics = byte(bits.u(8)?);
            signal.matrix_coefficients = byte(bits.u(8)?);
        }
    }
    if bits.flag()? {
        signal.chroma_sample_loc_type = byte(bits.ue()?);
        bits.ue()?;
    }
    Some(())
}

/// An identifier as an index; one past `usize` indexes nothing.
fn index(id: u32) -> usize {
    usize::try_from(id).unwrap_or(usize::MAX)
}

/// A code as a byte, a larger one held at 255, which names nothing either.
fn byte(code: u32) -> u8 {
    u8::try_from(code).unwrap_or(u8::MAX)
}

/// Bits read most significant first, and Exp-Golomb codes (9.1).
struct Bits {
    bytes: Vec<u8>,
    at: usize,
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "a bit's place within its byte is 0 to 7, and a code read in at most 31 bits doubled \
              and halved fits an `i64`"
)]
impl Bits {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, at: 0 }
    }

    fn flag(&mut self) -> Option<bool> {
        let byte = self.bytes.get(self.at / 8)?;
        let bit = byte >> (7 - self.at % 8) & 1 == 1;
        self.at = self.at.checked_add(1)?;
        Some(bit)
    }

    /// `count` bits, at most 32.
    fn u(&mut self, count: u32) -> Option<u32> {
        (0..count.min(32)).try_fold(0u32, |value, _| {
            Some(value.wrapping_shl(1) | u32::from(self.flag()?))
        })
    }

    /// `ue(v)`, at most 31 leading zeros.
    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0u32;
        while !self.flag()? {
            zeros = zeros.checked_add(1).filter(|&zeros| zeros < 32)?;
        }
        let rest = self.u(zeros)?;
        (1u64 << zeros)
            .checked_sub(1)
            .and_then(|base| base.checked_add(u64::from(rest)))
            .and_then(|value| u32::try_from(value).ok())
    }

    /// `se(v)`.
    fn se(&mut self) -> Option<i64> {
        let coded = i64::from(self.ue()?);
        Some(if coded % 2 == 1 {
            (coded + 1) / 2
        } else {
            -(coded / 2)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_emulation_prevention_byte_is_removed_and_a_lone_three_is_kept() {
        assert_eq!(
            rbsp(&[0, 0, 3, 1, 3, 0, 0, 3], usize::MAX),
            [0, 0, 1, 3, 0, 0]
        );
    }

    #[test]
    fn exp_golomb_codes_read_as_9_1_writes_them() {
        // 1, 010, 011, 00100, 00101: ue 0, 1, 2, 3, 4; then se of 00110 (5) is +3.
        let mut bits = Bits::new(vec![0b1010_0110, 0b0100_0010, 0b1001_1000]);
        assert_eq!(
            [bits.ue(), bits.ue(), bits.ue(), bits.ue(), bits.ue()],
            [Some(0), Some(1), Some(2), Some(3), Some(4)]
        );
        assert_eq!(bits.se(), Some(3));
    }

    /// Packs a string of `0` and `1` into bytes, the last one padded with zeros.
    fn packed(bits: &str) -> Vec<u8> {
        bits.as_bytes()
            .chunks(8)
            .map(|chunk| {
                chunk.iter().enumerate().fold(0u8, |byte, (at, bit)| {
                    byte | u8::from(*bit == b'1')
                        .checked_shl(u32::try_from(7usize.saturating_sub(at)).unwrap_or(0))
                        .unwrap_or(0)
                })
            })
            .collect()
    }

    #[test]
    fn a_picture_order_cycle_past_255_is_refused_not_read_short() {
        // Baseline (66), no constraint flags, level 30, then 7.3.2.1.1's fields up to the cycle:
        // seq_parameter_set_id ue 0, log2_max_frame_num_minus4 ue 0, pic_order_cnt_type ue 1,
        // delta_pic_order_always_zero_flag 0, two se 0 offsets, and a cycle of ue 256.
        let fields = "01000010".to_owned()
            + "00000000"
            + "00011110"
            + "1"
            + "1"
            + "010"
            + "0"
            + "1"
            + "1"
            + "00000000"
            + "100000001";
        let refusal = sequence(&packed(&fields)).expect_err("7.4.2.1.1 bounds the cycle at 255");
        assert!(
            matches!(&refusal, VideoError::Refused(reason) if reason.contains("256")),
            "{refusal:?}"
        );
    }
}
