//! PDF417 as ISO/IEC 15438:2006 defines it and XFA 3.3 types it, read back by an independent
//! decoder.
//!
//! The expected values are the standard's: Annex A's table held to clause 5.3.1, the worked
//! examples of Table 6 (Text Compaction), Annex C (Byte), Annex D (Numeric), Annex Q (error
//! correction) and clause 5.9.2 (the Symbol Length Descriptor), and Annex F's coefficients. `rxing`'s
//! reader is evidence that a symbol is readable as its data, never the definition (ADR 2877).

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a symbol that does not encode has failed the test, and an image of a few \
              hundred pixels a side cannot overflow its arithmetic"
)]

use rxing::common::HybridBinarizer;
use rxing::pdf417::PDF417Reader;
use rxing::{BinaryBitmap, DecodeHints, Luma8LuminanceSource, Reader};
use xfa_barcode::pdf417::{self, ANNEX_A, Compaction, Layout, Options, Shape};
use xfa_barcode::{Matrix, Refusal, Symbology};

/// Clause 5.3.1: eight elements of one to six modules, 17 modules in all, the cluster
/// `(b1 − b2 + b3 − b4 + 9) mod 9` of the four bars, and 929 codewords in three mutually exclusive
/// clusters — so no two of the 2 787 entries repeat.
#[test]
fn annex_a_holds_to_clause_5_3_1() {
    let mut seen = std::collections::BTreeSet::new();
    for (codeword, clusters) in ANNEX_A.iter().enumerate() {
        for (sequence, cluster) in clusters.iter().zip([0, 3, 6]) {
            let digits: Vec<i32> = sequence
                .to_string()
                .bytes()
                .map(|digit| i32::from(digit - b'0'))
                .collect();
            assert_eq!(digits.len(), 8, "{codeword}");
            assert!(
                digits.iter().all(|width| (1..=6).contains(width)),
                "{codeword}"
            );
            assert_eq!(digits.iter().sum::<i32>(), 17, "{codeword}");
            let bars = (digits[0] - digits[2] + digits[4] - digits[6] + 9) % 9;
            assert_eq!(bars, cluster, "{codeword} in cluster {cluster}");
            assert!(seen.insert(*sequence), "{codeword} repeats {sequence}");
        }
    }
    // Figure 3's example, 51111125, is codeword 0 in cluster 3.
    assert_eq!(ANNEX_A[0][1], 51_111_125);
}

/// Table 6: "PDF417" in Text Compaction is P D, F `ml`, 4 1, 7 `ps` — 453, 178, 121, 239.
#[test]
fn table_6_s_example() {
    assert_eq!(
        pdf417::compacted(b"PDF417", Compaction::Text),
        [453, 178, 121, 239]
    );
    // And it is the choice the encoder makes for that data on its own.
    assert_eq!(pdf417::data_codewords(b"PDF417"), [453, 178, 121, 239]);
}

/// Annex C: the bytes 231 101 11 97 205 2 are the codewords 387 700 208 213 302, behind the latch
/// 924 because six is a multiple of six (clause 5.4.3.3).
#[test]
fn annex_c_s_example() {
    assert_eq!(
        pdf417::compacted(&[231, 101, 11, 97, 205, 2], Compaction::Byte),
        [924, 387, 700, 208, 213, 302]
    );
    // Seven bytes: latch 901, one group of five, the seventh byte as itself.
    assert_eq!(
        pdf417::compacted(&[231, 101, 11, 97, 205, 2, 7], Compaction::Byte),
        [901, 387, 700, 208, 213, 302, 7]
    );
}

/// Annex D: "000213298174000" with a 1 before it is 1 624 434 632 282 200 in base 900; clause
/// 5.4.4.2: 44 digits make 15 codewords, and fewer `INT(digits / 3) + 1`.
#[test]
fn annex_d_s_example() {
    assert_eq!(
        pdf417::compacted(b"000213298174000", Compaction::Numeric),
        [902, 1, 624, 434, 632, 282, 200]
    );
    assert_eq!(
        pdf417::compacted(&[b'7'; 44], Compaction::Numeric).len(),
        1 + 15
    );
    assert_eq!(
        pdf417::compacted(&[b'1'; 28], Compaction::Numeric).len(),
        1 + 10
    );
}

/// Clause 5.10's generator `(x − 3)(x − 3²)…(x − 3ᵏ)` gives Annex F's tables F.1 to F.3, and Annex
/// Q's worked example — `5 453 178 121 239` at level 1 — gives 452 327 657 619.
#[test]
fn annex_f_and_annex_q() {
    assert_eq!(pdf417::coefficients(2), [27, 917]);
    assert_eq!(pdf417::coefficients(4), [522, 568, 723, 809]);
    assert_eq!(
        pdf417::coefficients(8),
        [237, 308, 436, 284, 646, 653, 428, 379]
    );
    assert_eq!(
        pdf417::error_codewords(&[5, 453, 178, 121, 239], 4),
        [452, 327, 657, 619]
    );
}

/// Clause 5.9.2's example: 12 columns, 24 rows and level 4 (32 error codewords) hold n = 256 data
/// codewords; Table 9: k = 2^(s+1).
#[test]
fn clause_5_9_2_s_example() {
    let layout = Layout {
        rows: 24,
        columns: 12,
        level: 4,
    };
    assert_eq!(layout.error_codewords(), 32);
    assert_eq!(layout.data_capacity(), 256);
    // The Symbol Length Descriptor is n, and the pads 900 fill to it.
    let (codewords, chosen) = pdf417::codewords(
        b"PDF417",
        Options {
            level: Some(4),
            shape: Shape::Fixed {
                rows: 24,
                columns: 12,
            },
            compact: false,
        },
    )
    .expect("fits");
    assert_eq!(chosen, layout);
    assert_eq!(codewords.len(), 288);
    assert_eq!(&codewords[..6], &[256, 453, 178, 121, 239, 900]);
    assert!(codewords[5..256].iter().all(|pad| *pad == 900));
}

/// ISO/IEC 15438 Table E.1 (Annex E), when the template states no level.
#[test]
fn the_default_level_is_annex_e_s_recommendation() {
    assert_eq!(pdf417::recommended_level(40), 2);
    assert_eq!(pdf417::recommended_level(41), 3);
    assert_eq!(pdf417::recommended_level(320), 4);
    assert_eq!(pdf417::recommended_level(321), 5);
    let (_, layout) = pdf417::codewords(b"PDF417", Options::default()).expect("fits");
    assert_eq!(layout.level, 2);
}

/// The symbol as an image — three pixels a module, a quiet zone of four modules (clause 5.8.3's
/// least is two) — read by `rxing`'s PDF417 reader.
fn read_back(symbol: &Matrix) -> Option<Vec<u8>> {
    let scale = 3;
    let quiet = 4 * scale;
    let width = symbol.width * scale + 2 * quiet;
    let height = symbol.height * scale + 2 * quiet;
    let mut luma = vec![255_u8; width * height];
    for row in 0..symbol.height {
        for column in 0..symbol.width {
            if symbol.is_dark(row, column) {
                for y in 0..scale {
                    for x in 0..scale {
                        luma[(quiet + row * scale + y) * width + quiet + column * scale + x] = 0;
                    }
                }
            }
        }
    }
    let source = Luma8LuminanceSource::new(
        luma,
        u32::try_from(width).expect("small"),
        u32::try_from(height).expect("small"),
    )
    .ok()?;
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
    let hints = DecodeHints {
        PureBarcode: Some(true),
        ..DecodeHints::default()
    };
    let result = PDF417Reader.decode_with_hints(&mut bitmap, &hints).ok()?;
    result
        .getText()
        .chars()
        .map(|character| u8::try_from(u32::from(character)).ok())
        .collect()
}

fn encode(data: &[u8], options: Options) -> (Matrix, Layout) {
    pdf417::encode(data, options)
        .unwrap_or_else(|refusal| panic!("{:?}: {refusal}", String::from_utf8_lossy(data)))
}

/// Data that reaches every sub-mode switch of Table 7, each compaction, and their joins.
/// An empty message is a Symbol Length Descriptor and pads, which this decoder reports as no
/// symbol found; it is pinned by `an_empty_message_is_its_length_and_pads` from clause 5.9.2 alone.
const SAMPLES: [&[u8]; 8] = [
    b"PDF417",
    b"Hello, World! The quick brown fox jumps over the lazy dog.",
    b"lower UPPER lower U l MiXeD 123 #$% <>@[] a;b\"c\t\r\nend",
    b"1234567890123456789012345678901234567890123456789012345678901",
    b"Order 00012345678901234 shipped; ref 98765432109876.",
    b"\x00\x01\x02\xfe\xff binary \x80\x81",
    b"\xe9t\xe9",
    b"a",
];

/// Clause 5.9.2: with no source data, n is the matrix's whole data capacity and every codeword
/// after the Symbol Length Descriptor is the pad 900.
#[test]
fn an_empty_message_is_its_length_and_pads() {
    let (codewords, layout) = pdf417::codewords(b"", Options::default()).expect("fits");
    let n = layout.data_capacity();
    assert_eq!(usize::from(codewords[0]), n);
    assert!(codewords[1..n].iter().all(|pad| *pad == 900));
    assert_eq!(codewords.len(), n + layout.error_codewords());
}

/// Every sample reads back as its bytes at every error correction level, in the default shape.
#[test]
fn every_sample_reads_back_at_every_level() {
    for data in SAMPLES {
        for level in 0..=8 {
            let options = Options {
                level: Some(level),
                ..Options::default()
            };
            let (symbol, layout) = encode(data, options);
            assert_eq!(
                read_back(&symbol).as_deref(),
                Some(data),
                "{:?} at level {level}, {layout:?}",
                String::from_utf8_lossy(data)
            );
        }
    }
}

/// Fixed matrices (padded), ratios, column counts, and Compact PDF417 read back; every row is the
/// width clause 5.2.2 and Annex G give it.
#[test]
fn every_shape_and_compact_reads_back() {
    let data = SAMPLES[1];
    let shapes = [
        Shape::Fixed {
            rows: 10,
            columns: 6,
        },
        Shape::Fixed {
            rows: 30,
            columns: 4,
        },
        Shape::Ratio(1.0),
        Shape::Ratio(10.0),
        Shape::Columns(1),
        Shape::Columns(30),
        Shape::Square,
    ];
    for shape in shapes {
        for compact in [false, true] {
            let (symbol, layout) = encode(
                data,
                Options {
                    level: Some(2),
                    shape,
                    compact,
                },
            );
            let width = if compact { 35 } else { 69 };
            assert_eq!(symbol.width, 17 * layout.columns + width);
            assert_eq!(symbol.height, layout.rows * pdf417::ROW_HEIGHT);
            assert_eq!(
                read_back(&symbol).as_deref(),
                Some(data),
                "{shape:?}, compact {compact}, {layout:?}"
            );
        }
    }
}

/// Every row count from 3 to 90 is reached (each row indicator's three forms over 30 row groups),
/// one column wide.
#[test]
fn every_row_count_reads_back() {
    for rows in 3..=90 {
        let data: Vec<u8> = b"ROWS".iter().copied().cycle().take(rows).collect();
        let (symbol, layout) = encode(
            &data,
            Options {
                level: Some(0),
                shape: Shape::Fixed { rows, columns: 2 },
                compact: false,
            },
        );
        assert_eq!(layout.rows, rows);
        assert_eq!(
            read_back(&symbol).as_deref(),
            Some(&data[..]),
            "{rows} rows"
        );
    }
}

/// A level past 8 (page 614 allows 0 to 8), a fixed matrix too small for the data, and data past
/// clause 5.2.3's 928 symbol characters are refused by name.
#[test]
fn what_cannot_be_held_is_refused_by_name() {
    let pdf417 = Symbology::from_type("pdf417").expect("defined");
    let refused = |data: &[u8], options| matches!(pdf417::encode(data, options), Err(Refusal::Capacity { symbology, .. }) if symbology == pdf417);
    assert!(refused(
        b"A",
        Options {
            level: Some(9),
            ..Options::default()
        }
    ));
    assert!(refused(
        &[b'A'; 40],
        Options {
            level: Some(0),
            shape: Shape::Fixed {
                rows: 3,
                columns: 2
            },
            compact: false,
        }
    ));
    assert!(refused(&[0xFF; 1200], Options::default()));
    assert!(xfa_barcode::encode_matrix(pdf417, b"A", Options::default()).is_ok());
}
