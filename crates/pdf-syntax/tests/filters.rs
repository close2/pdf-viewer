//! §7.4's two structural rules: a filter array is a pipeline, and a predictor is part of it.
//!
//! The individual codecs are checked beside themselves in `filter.rs`, and `lzw.rs` checks one
//! of them against ninety-six corpus encodings of a single image. What has no home there is the
//! *chaining*: which order the stages run in, which `/DecodeParms` each one gets, and that a
//! predictor is undone inside the decode rather than left for a caller. The last of those is
//! trap 4's own archetype — the code once called it "the caller's responsibility" and then did
//! not, and every modern PDF failed with a misleading `/Root is not a dictionary`.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not decode should fail loudly, and every fixture \
              here is a few hundred bytes, so no offset or index can overflow"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A one-page document whose object 4 is a stream with the given dictionary entries and data.
fn document_with_stream(entries: &str, data: &[u8]) -> Document {
    let head = format!(
        "%PDF-1.7\n\
         1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n\
         4 0 obj\n<< {entries} /Length {} >>\nstream\n",
        data.len()
    );

    // The stream's data is binary, so the file is assembled as bytes; every object but the
    // last is a chunk of `head`, and object 1's offset is where the header line ends.
    let mut bytes = Vec::new();
    let mut offsets = vec![9usize];
    let chunks: Vec<&str> = head.split_inclusive("endobj\n").collect();
    for chunk in &chunks {
        if !bytes.is_empty() {
            offsets.push(bytes.len());
        }
        bytes.extend_from_slice(chunk.as_bytes());
    }
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let table_at = bytes.len();
    let size = offsets.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{table_at}\n%%EOF\n"
    );
    bytes.extend_from_slice(table.as_bytes());

    Document::open(bytes).expect("the fixture is a valid PDF")
}

/// The decoded bytes of object 4.
fn decoded(document: &Document) -> Vec<u8> {
    let object = document.get(pdf_syntax::ObjectId {
        number: 4,
        generation: 0,
    });
    let stream = object.as_stream().expect("object 4 is a stream").clone();
    document
        .decoded_stream_data(&stream)
        .expect("every filter in the fixture is supported")
        .to_vec()
}

/// ISO 32000-2 §7.4.1: an array of filters is a pipeline, decoded in the order written.
///
/// > Filters may be cascaded to form a pipeline that passes the stream through two or more
/// > decoding transformations in sequence.
///
/// The fixture is `ASCII85` around `Flate`, which is the combination the clause's own examples
/// use and the only one whose stages are not commutative in an obvious way: run the other way
/// round, neither stage can consume what it is handed. So the test is not that the answer is
/// right so much as that there *is* one — and the assertion on the text is what stops a reader
/// that decodes only the first stage from passing.
#[test]
fn a_filter_array_is_a_pipeline_decoded_in_the_order_written() {
    // "the quick brown fox" deflated, then ASCII85-encoded.
    let flated = deflate(b"the quick brown fox");
    let encoded = ascii85(&flated);

    let document = document_with_stream("/Filter [/ASCII85Decode /FlateDecode]", &encoded);
    assert_eq!(
        decoded(&document),
        b"the quick brown fox",
        "both stages must run, in the order the array gives them"
    );
}

/// ISO 32000-2 §7.4.1: each stage takes its own entry of a `/DecodeParms` array.
///
/// > Some filters may take parameters to control how they operate. These optional parameters
/// > shall be specified by the DecodeParms entry in the stream's dictionary
///
/// The array form is what this pins: `/DecodeParms [null <</Predictor 12 …>>]` gives the
/// predictor to the *second* filter, and a reader that takes the array's first entry, or the
/// whole array, applies no predictor and hands back PNG-filtered rows — numbers that are
/// plausible and all wrong.
#[test]
fn each_stage_takes_its_own_entry_of_a_decodeparms_array() {
    // Two rows of four bytes under PNG filter type 2 (Up): the first row is the data itself
    // and the second is its difference from the first.
    let rows: Vec<u8> = vec![2, 10, 20, 30, 40, 2, 1, 1, 1, 1];
    let encoded = ascii85(&deflate(&rows));

    let document = document_with_stream(
        "/Filter [/ASCII85Decode /FlateDecode] \
         /DecodeParms [null << /Predictor 12 /Colors 1 /BitsPerComponent 8 /Columns 4 >>]",
        &encoded,
    );
    assert_eq!(
        decoded(&document),
        vec![10, 20, 30, 40, 11, 21, 31, 41],
        "the predictor belongs to the second filter and is undone inside the decode"
    );
}

/// ISO 32000-2 §7.4.4.4: the PNG group's filter type is per row, not per stream.
///
/// > lists the five basic PNG predictor algorithms (and a sixth that chooses the optimum
/// > predictor function separately for each row)
///
/// `/Predictor` says only *that* the data is PNG-predicted; ISO/IEC 15948 puts a type byte in
/// front of every row, and the five types are chosen independently. A reader that applies the
/// declared value to every row gets the first row right and every row after it wrong, which on
/// a cross-reference stream is a table of fabricated byte offsets.
#[test]
fn each_png_row_carries_its_own_filter_type() {
    // Four rows of three bytes: None, Sub, Up, Average — four of the five types, each
    // encoding the same row `[7, 8, 9]` from a different predecessor.
    let data: Vec<u8> = vec![
        0, 7, 8, 9, // None: the bytes as they are
        1, 7, 1, 1, // Sub: each byte less the one before it
        2, 0, 0, 0, // Up: no change from the row above
        3, 4, 1, 1, // Average: each byte less floor((left + above) / 2)
    ];
    let undone = pdf_syntax::filter::apply_predictor(&data, 15, 1, 8, 3)
        .expect("a PNG predictor over three columns");
    assert_eq!(
        &*undone,
        &[7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9],
        "every row must be undone by the type it carries"
    );
}

/// ISO 32000-2 §7.4.4.4: TIFF Predictor 2 is a horizontal difference and carries no type byte.
///
/// > The first, the TIFF group, shall be the single function that is Predictor 2 as defined in
/// > the Adobe TIFF Revision 6.0 specification.
///
/// The other half of the clause, and the one whose shape differs: there is no per-row byte, so
/// a reader that treats a 2 as though it were a PNG code eats a data byte from every row.
#[test]
fn the_tiff_predictor_is_a_horizontal_difference_with_no_type_byte() {
    let data: Vec<u8> = vec![10, 5, 5, 100, 1, 1];
    let undone = pdf_syntax::filter::apply_predictor(&data, 2, 1, 8, 3)
        .expect("TIFF Predictor 2 over three columns");
    assert_eq!(&*undone, &[10, 15, 20, 100, 101, 102]);
}

/// A predictor code the clause does not define is refused rather than passed through.
///
/// Table 10 lists 1, 2 and 10 to 15 and nothing else. Passing unknown data through reads as
/// success, and for a cross-reference stream that is a table of invented offsets — so the
/// refusal is the safe answer and it is the one under test.
#[test]
fn a_predictor_the_clause_does_not_define_is_refused() {
    assert!(pdf_syntax::filter::apply_predictor(&[0, 1, 2, 3], 7, 1, 8, 3).is_none());
    assert!(
        pdf_syntax::filter::apply_predictor(&[0, 1, 2, 3], 2, 1, 8, 3).is_some(),
        "while a code the table does list decodes, so the refusal is about the code and \
         not about the data"
    );
    // Table 10's 1 is "no prediction", which `decode_with_parms` answers before reaching
    // here — so `apply_predictor` never sees it and says so by refusing that too.
    assert!(pdf_syntax::filter::apply_predictor(&[0, 1, 2, 3], 1, 1, 8, 3).is_none());
}

/// ISO 32000-2 §7.4.4.4: the fifth PNG filter type, Paeth, and each of its three answers.
///
/// The clause defers the algorithms to ISO/IEC 15948, where the Paeth predictor chooses
/// whichever of the left, upper and upper-left neighbours is nearest to `left + up - up_left`.
/// **Which of the three it returns is the whole of the filter**, so a case that happens to
/// pick the same byte every time would also pass against a decoder that implemented Up — the
/// row below is built so that the three positions answer *up*, *up-left* and *left* in turn,
/// and no simpler filter reproduces it.
///
/// It is here because until the seven-hundred-and-fifty-second session type 4 had no test at
/// all: the other four are covered by `each_png_row_carries_its_own_filter_type`, which stops
/// at Average.
#[test]
fn the_paeth_filter_chooses_between_its_three_neighbours() {
    let data: Vec<u8> = vec![
        0, 100, 95, 90, // None, so the row above is exactly these three bytes
        4, 5, 20, 10, // Paeth
    ];
    let undone = pdf_syntax::filter::apply_predictor(&data, 12, 1, 8, 3)
        .expect("a PNG predictor over three columns");
    // Position 0 has no left or upper-left, so the estimate is the upper byte and *up* wins:
    // 5 + 100 = 105. Position 1 estimates 105 + 95 - 100 = 100, which is upper-left exactly,
    // so *up-left* wins: 20 + 100 = 120. Position 2 estimates 120 + 90 - 95 = 115, nearest to
    // the left byte, so *left* wins: 10 + 120 = 130.
    assert_eq!(&*undone, &[100, 95, 90, 105, 120, 130]);
}

/// A trailing type byte with no row after it is accepted, and its type is never examined.
///
/// §7.4.4.4 puts one type byte in front of each row, so a stream whose last byte is a lone
/// tag is malformed — and this crate's rule for malformed input is to keep what decoded
/// rather than to discard it. The row contributes nothing because it has no bytes, which
/// means the tag never has to be one the clause defines: an undefined tag *with* a row is
/// refused by `a_predictor_the_clause_does_not_define_is_refused`'s sibling path, and an
/// undefined tag with no row is this.
///
/// Pinned because it is a behaviour that is easy to lose while tidying: it survives only as
/// long as the tag is examined where the bytes are, and the seven-hundred-and-fifty-second
/// session moved that examination (ADR 0667).
#[test]
fn a_trailing_type_byte_with_no_row_keeps_what_decoded() {
    // Three columns, so a row is a tag plus three bytes; the `9` is a fourth chunk of one.
    let undone = pdf_syntax::filter::apply_predictor(&[0, 1, 2, 3, 9], 12, 1, 8, 3)
        .expect("the whole rows still decode");
    assert_eq!(
        &*undone,
        &[1, 2, 3],
        "the complete row survives and the empty one adds nothing"
    );
}

/// Deflates with the same library the decoder uses, which is what a producer would have done.
fn deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).expect("writing to a Vec");
    encoder.finish().expect("finishing into a Vec")
}

/// ASCII85-encodes, including §7.4.3's `~>` terminator.
///
/// Written here rather than pulled in, because the decoder under test is the only other
/// implementation in the tree and a shared one would compare it with itself.
fn ascii85(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for group in data.chunks(4) {
        let mut word = 0u32;
        for index in 0..=3 {
            word = word.wrapping_shl(8) | u32::from(group.get(index).copied().unwrap_or(0));
        }
        if word == 0 && group.len() == 4 {
            out.push(b'z');
            continue;
        }
        let mut digits = [0u8; 5];
        let mut value = word;
        for digit in digits.iter_mut().rev() {
            *digit = b'!'.saturating_add(u8::try_from(value % 85).expect("a remainder below 85"));
            value /= 85;
        }
        // A partial final group writes one more character than it has bytes.
        out.extend_from_slice(digits.get(..group.len() + 1).unwrap_or(&digits));
    }
    out.extend_from_slice(b"~>");
    out
}
