//! Focused §7.4.4.2 coverage: a clear marker after all 4096 table slots are occupied.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "bounded test fixture arithmetic and explicitly masked bit packing"
)]

use pdf_syntax::{Dictionary, Limits, Name, Object};

/// The code indices follow the hand-derived §7.4.4.2 thresholds, not a production encoder.
/// Index 0 is initial clear; the decoder infers entry 256+n at literal n once n >= 2.
/// Therefore entry 511 is created at index 255, 1023 at 767, and 2047 at 1791. The
/// encoder creates entry 4095 while processing literal 3,838; the clear follows at 3839.
#[expect(
    clippy::match_same_arms,
    reason = "spell out the initial nine-bit interval separately from the reset interval"
)]
fn code_width(index: usize, early_change: u8) -> u32 {
    match early_change {
        // First wider code follows entries 511, 1023, and 2047.
        0 => match index {
            0..=255 => 9,
            256..=767 => 10,
            768..=1791 => 11,
            1792..=3839 => 12,
            _ => 9,
        },
        // Table 8 EarlyChange=1 moves each threshold one emitted code earlier.
        1 => match index {
            0..=254 => 9,
            255..=766 => 10,
            767..=1790 => 11,
            1791..=3839 => 12,
            _ => 9,
        },
        _ => unreachable!("the fixture covers the two defined EarlyChange values"),
    }
}

fn pack_codes(codes: &[u16], early_change: u8) -> Vec<u8> {
    let mut packed = Vec::new();
    let mut held = 0u32;
    let mut bits = 0u32;
    for (index, &code) in codes.iter().enumerate() {
        let width = code_width(index, early_change);
        held = (held << width) | u32::from(code);
        bits += width;
        while bits >= 8 {
            packed.push(
                u8::try_from((held >> (bits - 8)) & 0xff)
                    .expect("the output byte is masked to eight bits"),
            );
            bits -= 8;
        }
    }
    if bits != 0 {
        packed.push(
            u8::try_from((held << (8 - bits)) & 0xff)
                .expect("the final output byte is masked to eight bits"),
        );
    }
    packed
}

#[test]
fn full_lzw_table_clear_resets_width_and_dictionary_for_both_earlychange_values() {
    use pdf_syntax::filter::decode_with_parms_reported;

    // A prefix of a byte de Bruijn sequence has no repeated adjacent pair, so the
    // encoder's longest-match rule legitimately emits literals throughout it.
    let mut prefix = Vec::new();
    for low in 0_u8..=255 {
        prefix.push(low);
        for high in (u16::from(low) + 1)..=255 {
            prefix.extend([low, u8::try_from(high).expect("high is at most 255")]);
        }
    }
    prefix.truncate(3838);
    let pairs: std::collections::HashSet<_> = prefix.windows(2).collect();
    assert_eq!(pairs.len(), prefix.len() - 1, "every adjacent pair is new");
    let last_prefix_byte = prefix[prefix.len() - 1];
    assert!(
        !prefix
            .windows(2)
            .any(|pair| pair[0] == last_prefix_byte && pair[1] == b'D'),
        "the final literal plus pending D creates a new pair at entry 4095",
    );
    let mut expected = prefix.clone();
    expected.extend_from_slice(b"DEDE");

    // The encoder adds entry 258 after its first emitted literal, so 3,838 literals
    // create entries 258 through 4095. Its next code is clear at index 3839 (12 bits).
    // The decoder is one addition behind when it reads clear, then restarts at 9 bits.
    // D is the pending next literal; E creates entry 258=DE, which code 258 then reuses.
    let mut codes = Vec::with_capacity(3844);
    codes.push(256);
    codes.extend(prefix.iter().copied().map(u16::from));
    codes.extend([256, u16::from(b'D'), u16::from(b'E'), 258, 257]);
    assert_eq!(codes.len(), 3844);
    assert_eq!(code_width(3839, 0), 12);
    assert_eq!(code_width(3839, 1), 12);
    assert_eq!(code_width(3840, 0), 9);
    assert_eq!(code_width(3840, 1), 9);

    for early_change in [0, 1] {
        let encoded = pack_codes(&codes, early_change);
        let mut parms = Dictionary::new();
        parms.insert(
            Name::new(b"EarlyChange".as_slice()),
            Object::Integer(i64::from(early_change)),
        );
        let decoded =
            decode_with_parms_reported(b"LZWDecode", &encoded, Some(&parms), Limits::DEFAULT)
                .expect("the full-table stream with a clear marker decodes");
        assert!(
            *decoded.data == expected,
            "EarlyChange={early_change}: decoded {} bytes, expected {}; first mismatch {:?}; damage {:?}",
            decoded.data.len(),
            expected.len(),
            decoded
                .data
                .iter()
                .zip(&expected)
                .enumerate()
                .find(|(_, (a, b))| a != b),
            decoded.damage,
        );
        assert_eq!(
            decoded.damage, None,
            "the stream reaches EOD after the reset"
        );
    }
}
