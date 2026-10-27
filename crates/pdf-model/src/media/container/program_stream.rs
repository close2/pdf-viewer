//! An MPEG-2 program stream: ITU-T Rec. H.222.0 | ISO/IEC 13818-1 (1995 edition), clause 2.5.3.
//!
//! A program stream is a run of packs, each opening with a `pack_header()` (Table 2-33): the
//! `pack_start_code` 0x000001BA, the bits `01`, a 42-bit system clock reference in three parts
//! split by marker bits, a 22-bit `program_mux_rate` (zero is forbidden) and a
//! stuffing length. This reads that first header and checks the marker bits the table fixes, which
//! is what tells an MPEG-2 program stream from the other things a `video/mpeg` clip can be.
//!
//! **The clause states no length and no named offset**, and that is what this reads. The system
//! clock reference is the intended time of arrival of a byte at the program target decoder, a time
//! of delivery and not of presentation; presentation times are in PES
//! packet headers, and the last picture's duration is in the video bitstream (ISO/IEC 13818-2),
//! which this program does not hold. So the length is not stated, and a time or frame offset over a
//! program stream stays a value nobody here can place; but H.222.0 defines no mechanism that names
//! an offset in the stream, so a marker over one is absent, which §13.2.4.3 ignores (ADR 2489).
//!
//! A header whose bits are not the ones Table 2-33 fixes is refused as not an MPEG-2 program
//! stream. That includes any other system layer that opens with the same start code: H.222.0 does
//! not describe one, and no text for one is held here, so none is read or named.

use super::{Bytes, Timeline};

/// `pack_start_code`, clause 2.5.3.4: the four bytes 0x000001BA.
const PACK_START_CODE: [u8; 4] = [0x00, 0x00, 0x01, 0xBA];

/// Reads the first pack header of the program stream `bytes`.
pub(super) fn timeline(bytes: &[u8]) -> Result<Timeline, String> {
    let mut header = Bytes { rest: bytes };
    let Some(code) = header.array::<4>() else {
        return Err("its program stream is shorter than one pack header".to_owned());
    };
    if code != PACK_START_CODE {
        return Err(
            "its bytes do not open with a pack header, which a program stream does".to_owned(),
        );
    }
    // Fields after the start code, Table 2-33: '01' and the clock's top three bits (byte 0), the
    // marker bit among the next two, and so on to the stuffing length (byte 9).
    let Some(
        [
            first,
            _,
            third,
            _,
            fifth,
            sixth,
            rate_high,
            rate_low,
            rate_end,
            stuffing,
        ],
    ) = header.array::<10>()
    else {
        return Err("its program stream is shorter than one pack header".to_owned());
    };
    // The marker bits: after clock bits 32..30 (bit 2 of byte 0), after bits 29..15 (bit 2 of
    // byte 2), after bits 14..0 (bit 2 of byte 4), after the extension (bit 0 of byte 5), after
    // the mux rate (the last two bits of byte 8).
    let marked = first >> 6 == 0b01
        && first & 0b100 != 0
        && third & 0b100 != 0
        && fifth & 0b100 != 0
        && sixth & 0b1 != 0
        && rate_end & 0b11 == 0b11;
    if !marked {
        return Err("its pack header does not carry the marker bits Table 2-33 fixes".to_owned());
    }
    // program_mux_rate is 22 bits: byte 6, byte 7 and the top six bits of byte 8.
    let rate = (u32::from(rate_high) << 14) | (u32::from(rate_low) << 6) | u32::from(rate_end >> 2);
    if rate == 0 {
        return Err(
            "its pack header states a program_mux_rate of 0, which is forbidden".to_owned(),
        );
    }
    // The three low bits of the last byte count stuffing bytes that follow; one pack header is all
    // this reads, so they need only be present.
    let stuffing = usize::from(stuffing & 0b111);
    if header.take(stuffing).is_none() {
        return Err("its pack header is cut inside its stuffing".to_owned());
    }
    Ok(Timeline {
        seconds: None,
        frames: None,
        markers: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pack header with system clock reference `base`, extension 0, `rate` and `stuffing` bytes.
    fn pack(base: u64, rate: u32, stuffing: u8) -> Vec<u8> {
        let mut out = PACK_START_CODE.to_vec();
        let field = (0b01u64 << 46)
            | (((base >> 30) & 0b111) << 43)
            | (1 << 42)
            | (((base >> 15) & 0x7FFF) << 27)
            | (1 << 26)
            | ((base & 0x7FFF) << 11)
            | (1 << 10)
            // The 9-bit extension, zero, then its marker bit.
            | (1 << 0);
        out.extend_from_slice(&field.to_be_bytes()[2..]);
        let rate_bytes = (rate << 2) | 0b11;
        out.extend_from_slice(&rate_bytes.to_be_bytes()[1..]);
        out.push(0xF8 | stuffing);
        out.extend(std::iter::repeat_n(0xFF, usize::from(stuffing)));
        out
    }

    #[test]
    fn a_pack_header_with_its_marker_bits_is_a_program_stream_that_names_no_length_or_offset() {
        let read = timeline(&pack(90_000, 25_000, 0)).expect("a program stream");
        assert_eq!(read.seconds, None);
        assert_eq!(read.frames, None);
        assert!(read.markers.is_empty());
        assert!(timeline(&pack(1, 1, 7)).is_ok());
    }

    #[test]
    fn a_marker_bit_that_is_zero_a_forbidden_rate_and_other_bytes_are_refused() {
        let good = pack(90_000, 25_000, 0);
        // Each of the five marker positions, cleared in turn.
        for (index, mask) in [(4, 0b100u8), (6, 0b100), (8, 0b100), (9, 0b1), (12, 0b11)] {
            let mut bad = good.clone();
            bad[index] &= !mask;
            assert!(timeline(&bad).is_err(), "marker at byte {index} cleared");
        }
        assert!(timeline(&pack(1, 0, 0)).is_err());
        assert!(timeline(b"\x00\x00\x01\xB3\x00\x00").is_err());
        assert!(timeline(&good[..good.len() - 1]).is_err());
        assert!(timeline(&pack(1, 1, 3)[..15]).is_err());
    }

    #[test]
    fn a_pack_header_in_another_form_is_refused_by_table_2_33_and_names_no_other_standard() {
        let mut other = PACK_START_CODE.to_vec();
        other.extend_from_slice(&[0x21, 0x00, 0x01, 0x00, 0x01, 0x80, 0x00, 0x01, 0xFF, 0xFF]);
        let said = timeline(&other).expect_err("not Table 2-33's form");
        assert!(said.contains("Table 2-33"), "{said}");
        assert!(!said.contains("11172"), "{said}");
    }
}
