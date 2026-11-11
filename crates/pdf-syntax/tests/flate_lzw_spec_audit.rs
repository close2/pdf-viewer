//! Clause-specific discriminating checks for ISO 32000-2 §7.4.4.
//!
//! The module tests the individual decoder algorithms. This file pins their shared Table 8
//! defaults and confirms the §7.4.4.4 predictor is applied as part of both filter entry points.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "small hand-built fixtures; masked code packing intentionally narrows to one byte"
)]

use pdf_syntax::{Dictionary, Limits, Name, Object};

fn zlib(data: &[u8]) -> Vec<u8> {
    use std::io::Write as _;

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(data)
        .expect("the in-memory writer accepts the fixture");
    encoder.finish().expect("the zlib stream finishes")
}

/// Packs a short LZW fixture whose entries all fit the initial nine-bit code width.
fn lzw(codes: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut held = 0u32;
    let mut bits = 0u32;
    for &code in codes {
        held = (held << 9) | u32::from(code);
        bits += 9;
        while bits >= 8 {
            out.push(((held >> (bits - 8)) & 0xff) as u8);
            bits -= 8;
        }
    }
    if bits != 0 {
        out.push(((held << (8 - bits)) & 0xff) as u8);
    }
    out
}

fn parms(predictor: Option<i64>) -> Dictionary {
    let mut parms = Dictionary::new();
    if let Some(predictor) = predictor {
        parms.insert(
            Name::new(b"Predictor".as_slice()),
            Object::Integer(predictor),
        );
    }
    parms.insert(Name::new(b"Colors".as_slice()), Object::Integer(2));
    parms.insert(
        Name::new(b"BitsPerComponent".as_slice()),
        Object::Integer(8),
    );
    parms.insert(Name::new(b"Columns".as_slice()), Object::Integer(2));
    parms
}

#[test]
fn table8_defaults_and_png_row_tags_reach_both_decoders() {
    use pdf_syntax::filter::decode_with_parms_reported;

    // Table 8 defaults Predictor to 1. With no predictor entry, a two-byte payload must pass
    // through unchanged; an invented default predictor would change or reject these bytes.
    let plain_lzw = lzw(&[256, 65, 66, 257]);
    let empty_parms = Dictionary::new();
    for empty in [None, Some(&empty_parms)] {
        let decoded = decode_with_parms_reported(b"LZWDecode", &plain_lzw, empty, Limits::DEFAULT)
            .expect("the LZW default parameters decode");
        assert_eq!(&*decoded.data, b"AB");
        assert_eq!(decoded.damage, None, "the valid stream reaches LZW EOD");
    }

    // With Predictor > 1 and the other Table 8 entries omitted, Colors=1, BPC=8 and Columns=1
    // determine a one-byte PNG row. The second row's Up tag makes the default row width and
    // previous-row behavior observable. Exercise both codec dispatches.
    let predictor_only = {
        let mut parms = Dictionary::new();
        parms.insert(Name::new(b"Predictor".as_slice()), Object::Integer(12));
        parms
    };
    let one_color_rows = [0, 5, 2, 1];
    let one_color_expected = [5, 6];
    let one_color_lzw = lzw(&std::iter::once(256)
        .chain(one_color_rows.iter().map(|&byte| u16::from(byte)))
        .chain(std::iter::once(257))
        .collect::<Vec<_>>());
    for (filter, encoded) in [
        (b"FlateDecode".as_slice(), zlib(&one_color_rows)),
        (b"LZWDecode".as_slice(), one_color_lzw),
    ] {
        let decoded =
            decode_with_parms_reported(filter, &encoded, Some(&predictor_only), Limits::DEFAULT)
                .expect("the omitted Table 8 entries use their defaults");
        assert_eq!(&*decoded.data, &one_color_expected);
        assert_eq!(
            decoded.damage, None,
            "the encoded filter reaches its end marker"
        );
    }

    // Three two-pixel, two-component rows carry their own PNG type bytes: None, Sub, then Up.
    // The declared Predictor 10 says PNG; it does not force one row algorithm. Sub uses a
    // two-byte pixel stride, and Up uses the already reconstructed row above.
    let predicted = [
        0, 10, 20, 30, 40, // None -> [10, 20, 30, 40]
        1, 11, 22, 22, 22, // Sub  -> [11, 22, 33, 44]
        2, 1, 1, 2, 2, // Up   -> [12, 23, 35, 46]
    ];
    let expected = [10, 20, 30, 40, 11, 22, 33, 44, 12, 23, 35, 46];
    let predictor = parms(Some(10));

    let flate = decode_with_parms_reported(
        b"FlateDecode",
        &zlib(&predicted),
        Some(&predictor),
        Limits::DEFAULT,
    )
    .expect("the Flate PNG predictor decodes");
    assert_eq!(&*flate.data, &expected);
    assert_eq!(flate.damage, None);

    let codes = std::iter::once(256)
        .chain(predicted.iter().map(|&byte| u16::from(byte)))
        .chain(std::iter::once(257));
    let lzw = decode_with_parms_reported(
        b"LZWDecode",
        &lzw(&codes.collect::<Vec<_>>()),
        Some(&predictor),
        Limits::DEFAULT,
    )
    .expect("the LZW PNG predictor decodes");
    assert_eq!(&*lzw.data, &expected);
    assert_eq!(lzw.damage, None);
}

#[test]
fn an_lzw_clear_code_restarts_the_table_for_the_following_codes() {
    use pdf_syntax::filter::decode_with_parms_reported;

    // Code 258 first means `AB`, then after clear means `DE`; a decoder that carries the old
    // table across clear produces `AB` at the second use instead of `DE`.
    let encoded = lzw(&[256, 65, 66, 258, 256, 68, 69, 258, 257]);
    let decoded = decode_with_parms_reported(b"LZWDecode", &encoded, None, Limits::DEFAULT)
        .expect("the stream's clear-table markers decode");
    assert_eq!(&*decoded.data, b"ABABDEDE");
    assert_eq!(decoded.damage, None, "the valid stream reaches LZW EOD");
}
