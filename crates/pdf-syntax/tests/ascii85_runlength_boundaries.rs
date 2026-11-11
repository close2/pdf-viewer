//! Boundary assertions for the byte-oriented stream filters in ISO 32000-2 §7.4.

use std::sync::Arc;

use pdf_syntax::Limits;
use pdf_syntax::filter::{Damage, FilterRefusal, Pump, Pumped, Pumping, Stage, decode_reported};

/// §7.4.2 maps digit pairs to bytes, admits upper- and lower-case A–F, ignores the six PDF
/// white-space bytes from §7.2, and stops at `>` even if more source bytes follow.
#[test]
fn ascii_hex_decodes_both_letter_cases_and_ignores_pdf_whitespace() {
    let decoded = decode_reported(
        b"ASCIIHexDecode",
        b"0 1\t2 3\n4 5\r6 7\x0c8 9\0A b C d E f>ignored",
        None,
        Limits::DEFAULT,
    )
    .expect("hex digits with PDF white space are valid");

    assert_eq!(
        decoded.data.as_ref(),
        &[0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]
    );
    assert_eq!(decoded.damage, None);
}

/// Errata Collection 3, Issue #293: once ASCII85 sees `~`, the next character must be `>`;
/// another character is an error. The fixture uses one valid `z` zero-run before the malformed
/// marker, so the error cannot come from the alphabet or group checks.
#[test]
fn ascii85_tilde_followed_by_anything_but_greater_than_is_an_error() {
    let input = b"z~x";

    assert_eq!(
        decode_reported(b"ASCII85Decode", input, None, Limits::DEFAULT).err(),
        Some(FilterRefusal::Corrupt),
        "Issue #293 requires an error after `~x`; `~>` is the only EOD marker"
    );
}

/// §7.4.3 writes the encoder's partial-group rule, which a decoder reverses: append 4−n zero
/// bytes, encode the padded value, then keep the first n+1 base-85 characters. The published
/// arithmetic gives the canonical partial encodings `M` → digit 9 plus a grave accent,
/// `Ma` → `9jn`, and `Man` →
/// `9jqo`; these check each permitted tail size.
#[test]
fn ascii85_partial_groups_decode_one_two_and_three_bytes() {
    for (encoded, expected) in [
        (&b"9`~>"[..], &b"M"[..]),
        (&b"9jn~>"[..], &b"Ma"[..]),
        (&b"9jqo~>"[..], &b"Man"[..]),
    ] {
        let decoded = decode_reported(b"ASCII85Decode", encoded, None, Limits::DEFAULT)
            .expect("2, 3, or 4 characters make a valid final group");
        assert_eq!(decoded.data.as_ref(), expected, "{encoded:?}");
    }
}

/// §7.4.3 lists `z` inside a group as impossible. A leading digit puts the parser in a group,
/// so this `z` is not the standalone four-zero-byte shorthand.
#[test]
fn ascii85_z_inside_a_group_is_an_error() {
    assert_eq!(
        decode_reported(b"ASCII85Decode", b"!z!!!~>", None, Limits::DEFAULT).err(),
        Some(FilterRefusal::Corrupt),
        "z is legal only between groups"
    );
}

/// §7.4.3 imports §7.2's whitespace rule for this filter. The separated form uses the same
/// base-85 digits and must therefore decode to the same three bytes as the compact form.
#[test]
fn ascii85_ignores_pdf_whitespace_between_digits() {
    let decoded = decode_reported(b"ASCII85Decode", b"9j \tq\ro\n~>", None, Limits::DEFAULT)
        .expect("PDF white space is ignored between base-85 digits");
    assert_eq!(decoded.data.as_ref(), b"Man");
}

/// The same §7.4.3 EOD marker is used by the resumable reader for content streams. This valid
/// one-byte `z` stream has four decoded zero bytes; its `~>` must finish cleanly in the pump too.
#[test]
fn ascii85_windowed_reader_accepts_the_two_byte_eod_marker() {
    let whole = decode_reported(b"ASCII85Decode", b"z~>", None, Limits::DEFAULT)
        .expect("the fixture is a valid whole-buffer ASCII85 stream");
    assert_eq!(whole.data.as_ref(), &[0; 4]);
    assert_eq!(whole.damage, None);

    let mut pump = Pump::new(Pumping::single(Stage::Ascii85), Arc::from(&b"z~>"[..]));
    let mut output = [0_u8; 8];

    assert_eq!(
        pump.pump(&mut output),
        Pumped::Ended(4),
        "the valid ~> marker must finish without treating > as an illegal digit"
    );
    assert_eq!(&output[..4], &[0; 4]);
}

/// The same Issue #293 rule reaches the windowed content-stream path: a tilde followed by `x`
/// must be reported as damage. `x` is itself in the ASCII85 alphabet, isolating the EOD check.
#[test]
fn ascii85_windowed_reader_reports_a_malformed_eod_marker() {
    let mut pump = Pump::new(Pumping::single(Stage::Ascii85), Arc::from(&b"z~x"[..]));
    let mut output = [0_u8; 8];

    assert_eq!(
        pump.pump(&mut output),
        Pumped::Damaged(4, Damage::Corrupt),
        "the valid zero-run prefix stands, but ~x is not an EOD marker"
    );
    assert_eq!(&output[..4], &[0; 4]);
}

/// §7.4.5's literal and repeat ranges meet at four endpoints. The headers and payloads below
/// are generated from those rules, and the expected bytes are checked independently of the
/// decoder: a literal run copies length+1 bytes, while a repeat copies 257-length instances.
#[test]
fn runlength_literal_and_repeat_run_endpoints_follow_their_lengths() {
    let literal: Vec<u8> = (0_u8..=127).collect();
    let mut encoded = Vec::with_capacity(1 + 1 + 1 + literal.len() + 1 + 1 + 1 + 1);
    encoded.extend_from_slice(&[0, 0x33]);
    encoded.push(127);
    encoded.extend_from_slice(&literal);
    encoded.extend_from_slice(&[129, 0xA5, 255, 0x5A, 128]);

    let mut expected = vec![0x33];
    expected.extend_from_slice(&literal);
    expected.extend_from_slice(&[0xA5; 128]);
    expected.extend_from_slice(&[0x5A, 0x5A]);

    let decoded = decode_reported(b"RunLengthDecode", &encoded, None, Limits::DEFAULT)
        .expect("all four run endpoints and EOD form a valid stream");
    assert_eq!(decoded.data.as_ref(), expected);
    assert_eq!(decoded.damage, None, "the final 128 byte is EOD");
}

/// §7.4.5 says a length value of 128 denotes EOD. Complete run payloads without that marker
/// still produce their declared bytes, but the stream is truncated; this separates the EOD
/// requirement from the decoder's recovery choice to retain the prefix.
#[test]
fn runlength_complete_run_without_eod_is_reported_as_truncated() {
    let decoded = decode_reported(
        b"RunLengthDecode",
        &[2, b'a', b'b', b'c'],
        None,
        Limits::DEFAULT,
    )
    .expect("the complete run prefix is recoverable");

    assert_eq!(decoded.data.as_ref(), b"abc");
    assert_eq!(
        decoded.damage,
        Some(Damage::Truncated),
        "a complete run is not a complete filter stream without the 128 EOD byte"
    );
}
