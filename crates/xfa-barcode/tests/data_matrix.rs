//! Data Matrix ECC 200 as XFA 3.3 types it, derived under the owner's answer A857 from `rxing`'s
//! encoder and read back by `rxing`'s decoder.
//!
//! The expected values: the size table's own arithmetic (every module of the mapping matrix holds
//! a codeword's bit, or belongs to the four-module corner pattern); the fixed vectors `rxing`'s
//! `error_correction.rs` test carries from ISO/IEC 16022:2000 Annex R; and, for everything else,
//! the bytes themselves, read back by a decoder that shares no code with the encoder the tables
//! were derived from (ADR 2877) — evidence that the symbol is readable as its data, never the
//! definition of the symbology.

#![expect(
    clippy::expect_used,
    reason = "test code: a symbol that does not encode has failed the test, and the sizes here are \
              a few thousand at most"
)]

use rxing::datamatrix::decoder::Decoder;
use xfa_barcode::data_matrix::{self, ENCODATIONS, Encodation, SIZES, Size};
use xfa_barcode::pdf417::Options;
use xfa_barcode::{Matrix, Refusal, Symbology};

fn data_matrix() -> Symbology {
    Symbology::from_type("dataMatrix").expect("XFA 3.3 page 617 defines dataMatrix")
}

/// The symbol read back by `rxing`'s decoder, its text taken as ISO 8859-1 bytes (the decoder's
/// reading of a byte above 127 without an ECI).
fn read_back(symbol: &Matrix) -> Option<Vec<u8>> {
    let rows: Vec<Vec<bool>> = (0..symbol.height)
        .map(|row| {
            (0..symbol.width)
                .map(|column| symbol.is_dark(row, column))
                .collect()
        })
        .collect();
    let decoded = Decoder::new().decode_bools(&rows).ok()?;
    decoded
        .getText()
        .chars()
        .map(|character| u8::try_from(u32::from(character)).ok())
        .collect()
}

/// Each size's mapping matrix holds its codewords' bits exactly, or with the four modules of the
/// lower right corner's fixed pattern left over; the symbol is its regions each framed by two
/// modules; and the error codewords divide evenly among the blocks. XFA 3.3 page 442: square only.
#[test]
fn every_size_is_consistent_with_its_own_geometry() {
    assert_eq!(SIZES.len(), 24);
    for size in SIZES {
        assert_eq!(size.side, size.regions * (size.region() + 2), "{size:?}");
        let modules = size.mapping() * size.mapping();
        let bits = (size.data + size.error) * 8;
        assert!(modules == bits || modules == bits + 4, "{size:?}");
        assert_eq!(size.error % size.blocks, 0, "{size:?}");
    }
    assert!(SIZES.windows(2).all(|pair| pair[0].data < pair[1].data));
}

/// ISO/IEC 16022:2000 Annex R's example, as `rxing`'s `testRS` carries it: "123456" is the ASCII
/// codewords 142 164 186 (three digit pairs, `130 + value`), and the 10 by 10 symbol's five error
/// codewords are 114 25 5 88 102.
#[test]
fn annex_r_s_example_has_its_error_codewords() {
    let stream = data_matrix::codewords(b"123456", Encodation::Ascii).expect("digits");
    assert_eq!(stream.codewords, [142, 164, 186]);
    assert_eq!(
        data_matrix::error_codewords(&[142, 164, 186], 5),
        [114, 25, 5, 88, 102]
    );
    // "A": 66, the pad 129, and a second pad randomised at position 3 to 70; then 138 234 82 82 95.
    assert_eq!(
        data_matrix::error_codewords(&[66, 129, 70], 5),
        [138, 234, 82, 82, 95]
    );
}

/// The finder pattern of the smallest symbol: a solid left column and bottom row, a top row and a
/// right column alternating, dark at the top left and light at the top right.
#[test]
fn the_finder_pattern_frames_the_symbol() {
    let symbol = data_matrix::encode(b"A").expect("one letter");
    assert_eq!((symbol.width, symbol.height), (10, 10));
    for index in 0..10 {
        assert!(symbol.is_dark(index, 0), "left column, row {index}");
        assert!(symbol.is_dark(9, index), "bottom row, column {index}");
        assert_eq!(symbol.is_dark(0, index), index % 2 == 0, "top row {index}");
    }
    for row in 1..9 {
        assert_eq!(
            symbol.is_dark(row, 9),
            row % 2 == 1,
            "right column, row {row}"
        );
    }
    assert_eq!(read_back(&symbol).as_deref(), Some(&b"A"[..]));
}

/// Data an encodation holds, `length` bytes long, cycling through a set that exercises the
/// encodation's shifts.
fn sample(encodation: Encodation, length: usize) -> Vec<u8> {
    let set: &[u8] = match encodation {
        Encodation::Ascii => b"12ab34CD\x01\xe9 ~5",
        Encodation::C40 => b"HELLO WORLD 2042 ab!\x7f\xc4",
        Encodation::Text => b"hello world 2042 AB?\x1f\xe4",
        Encodation::X12 => b"X12*DATA>0123 \r",
        Encodation::Edifact => b"EDIFACT:+?'^ 0123@",
        Encodation::Base256 => b"\x00\xff\x80binary\x7f\x01",
    };
    set.iter().copied().cycle().take(length).collect()
}

/// Every encodation, at lengths that end its messages at every remainder its packing has, in sizes
/// from the smallest up past the first interleaved one, reads back as its bytes.
#[test]
fn every_encodation_reads_back_at_every_end_case() {
    for encodation in ENCODATIONS {
        for length in (0..=40).chain([61, 62, 100, 150, 200, 260]) {
            let data = sample(encodation, length);
            let (symbol, size) = data_matrix::encode_in(&data, encodation)
                .unwrap_or_else(|refusal| panic!("{encodation:?} {length}: {refusal}"));
            assert_eq!(
                read_back(&symbol).as_deref(),
                Some(&data[..]),
                "{encodation:?}, {length} bytes, {} by {}",
                size.side,
                size.side
            );
        }
    }
}

/// Every one of the 24 sizes, each filled to its last data codeword by ASCII letters (one
/// codeword each), reads back: the placement's four corner shapes, the interleaving of up to ten
/// blocks and the finder patterns of up to 36 regions are all reached.
#[test]
fn every_size_reads_back_full() {
    for size in SIZES {
        let data: Vec<u8> = b"DATAMATRIX"
            .iter()
            .copied()
            .cycle()
            .take(size.data)
            .collect();
        let (symbol, chosen) =
            data_matrix::encode_in(&data, Encodation::Ascii).expect("a size holds its capacity");
        assert_eq!(chosen, size);
        assert_eq!(symbol.width, size.side);
        assert_eq!(
            read_back(&symbol).as_deref(),
            Some(&data[..]),
            "{} by {}",
            size.side,
            size.side
        );
    }
}

/// The automatic choice takes the encodation with the fewest codewords: sixteen capitals are two
/// latches' worth shorter in C40 than in ASCII, and a run of digits is shortest in ASCII's pairs.
#[test]
fn the_automatic_choice_is_the_shortest() {
    let capitals = b"ABCDEFGHIJKLMNOPQRSTUVWX";
    let ascii = data_matrix::codewords(capitals, Encodation::Ascii).expect("held");
    let c40 = data_matrix::codewords(capitals, Encodation::C40).expect("held");
    assert!(c40.needs < ascii.needs);
    let chosen = data_matrix::encode(capitals).expect("held");
    let smallest = data_matrix::smallest(c40.needs).expect("a size");
    assert_eq!(chosen.width, smallest.side);
    assert_eq!(read_back(&chosen).as_deref(), Some(&capitals[..]));
}

/// X12 holds forty characters and EDIFACT the bytes 32 to 94; a byte outside is refused by name.
/// A message past the largest symbol's 1 558 data codewords is refused, and so is a linear type
/// asked for as a matrix.
#[test]
fn what_cannot_be_held_is_refused_by_name() {
    assert_eq!(
        data_matrix::codewords(b"AB+", Encodation::X12),
        Err(Refusal::Unencodable {
            symbology: data_matrix(),
            character: '+'
        })
    );
    assert_eq!(
        data_matrix::codewords(b"AB_", Encodation::Edifact),
        Err(Refusal::Unencodable {
            symbology: data_matrix(),
            character: '_'
        })
    );
    let long = vec![b'A'; 1559];
    assert!(matches!(
        data_matrix::encode_in(&long, Encodation::Ascii),
        Err(Refusal::Capacity { .. })
    ));
    let size: Size = SIZES[23];
    assert!(data_matrix::encode_in(&vec![b'A'; size.data], Encodation::Ascii).is_ok());
    let code3of9 = Symbology::from_type("code3Of9").expect("defined");
    assert_eq!(
        xfa_barcode::encode_matrix(code3of9, b"A", Options::default()),
        Err(Refusal::Dimension(code3of9))
    );
    assert_eq!(
        xfa_barcode::encode(data_matrix(), xfa_barcode::Checksum::None, "A"),
        Err(Refusal::Dimension(data_matrix()))
    );
    assert!(xfa_barcode::encode_matrix(data_matrix(), b"A", Options::default()).is_ok());
}
