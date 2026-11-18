//! §7.3.10 direct/indirect stream-filter parameter equivalence witnesses.
//!
//! These are ordinary data streams, not cross-reference streams: §7.5.8.2 imposes
//! direct-only constraints on the latter. Both PDFs are valid, exact-xref inputs. The literal
//! expected bytes are derived from the uncompressed PNG rows, independently of the decoder.

#![expect(
    clippy::expect_used,
    reason = "small exact-xref in-memory PDF fixtures with checked-by-construction offsets"
)]

use std::io::Write as _;

use pdf_syntax::{Document, ObjectId};

fn deflate(input: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).expect("write to Vec");
    encoder.finish().expect("finish zlib stream")
}

/// Independent ASCII85 fixture encoder, including the required `~>` EOD marker.
fn ascii85(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    for group in input.chunks(4) {
        let mut value = 0u32;
        for index in 0..4 {
            value = value.wrapping_shl(8) | u32::from(group.get(index).copied().unwrap_or(0));
        }
        if value == 0 && group.len() == 4 {
            output.push(b'z');
            continue;
        }
        let mut digits = [0u8; 5];
        for digit in digits.iter_mut().rev() {
            *digit = b'!'.saturating_add(u8::try_from(value % 85).expect("remainder < 85"));
            value /= 85;
        }
        output.extend_from_slice(&digits[..=group.len()]);
    }
    output.extend_from_slice(b"~>");
    output
}

/// An exact-xref one-page PDF with object 4 as its stream and optional later objects.
fn one_stream(entries: &str, data: &[u8], later: &[(u32, Vec<u8>)]) -> Document {
    let stream_dict = format!("<< {entries} /Length {} >>", data.len()).into_bytes();
    let mut objects: Vec<(u32, Vec<u8>)> = vec![
        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>".to_vec(),
        ),
        (4, stream_dict),
    ];
    objects.extend(later.iter().map(|(number, body)| (*number, body.clone())));
    objects.sort_by_key(|(number, _)| *number);
    let size = objects
        .last()
        .map_or(5usize, |(number, _)| *number as usize + 1);
    let mut offsets = vec![None; size];
    let mut bytes = b"%PDF-1.7\n%\x80\x81\x82\x83\n".to_vec();
    for (number, body) in objects {
        offsets[number as usize] = Some(bytes.len());
        bytes.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        if number == 4 {
            bytes.extend_from_slice(&body);
            bytes.extend_from_slice(b"\nstream\n");
            bytes.extend_from_slice(data);
            bytes.extend_from_slice(b"\nendstream");
        } else {
            bytes.extend_from_slice(&body);
        }
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets.iter().skip(1) {
        match offset {
            Some(offset) => bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes()),
            None => bytes.extend_from_slice(b"0000000000 00000 f \n"),
        }
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    Document::open(bytes).expect("well-formed one-page PDF with exact offsets")
}

fn decoded_stream(document: &Document) -> Vec<u8> {
    let stream = document
        .get(ObjectId {
            number: 4,
            generation: 0,
        })
        .as_stream()
        .expect("object 4 is the stream")
        .clone();
    document
        .decoded_stream_data(&stream)
        .expect("the stated standard-filter chain decodes")
        .to_vec()
}

const PREDICTOR: &str = "/Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns 3";

/// §7.3.10 says a dictionary entry has equivalent semantics when its value is indirect.
/// The direct control and indirect PDF carry the same Flate+PNG-Up bytes and parameters.
#[test]
fn indirect_filter_name_and_decodeparms_dictionary_match_direct_values() {
    // PNG predictor rows: first row is literal; second row stores Up differences [1,2,3].
    let predictor_bytes = [0, 10, 20, 30, 2, 1, 2, 3];
    let encoded = deflate(&predictor_bytes);
    let direct = one_stream(
        &format!("/Filter /FlateDecode /DecodeParms << {PREDICTOR} >>"),
        &encoded,
        &[],
    );
    let indirect = one_stream(
        "/Filter 5 0 R /DecodeParms 6 0 R",
        &encoded,
        &[
            (5, b"/FlateDecode".to_vec()),
            (6, format!("<< {PREDICTOR} >>").into_bytes()),
        ],
    );
    assert!(
        !direct.cross_reference_chain_was_cut(),
        "direct control has a complete xref chain"
    );
    assert!(
        !direct.was_recovered(),
        "direct control uses its exact xref"
    );
    assert!(
        !indirect.cross_reference_chain_was_cut(),
        "indirect fixture has a complete xref chain"
    );
    assert!(
        !indirect.was_recovered(),
        "indirect values use their exact xref"
    );
    let expected = [10, 20, 30, 11, 22, 33];
    assert_eq!(decoded_stream(&direct), expected);
    assert_eq!(decoded_stream(&indirect), expected);
}

/// A legal filter/parameter array may itself be indirect, and each array value may be indirect.
/// The null first parameter entry belongs to ASCII85; the second stage's indirect dictionary
/// reverses the Flate predictor. The decoded bytes are derived from the rows above, not by
/// comparing the reader to itself.
#[test]
fn indirect_filter_and_decodeparms_arrays_resolve_each_stage_value() {
    let predictor_bytes = [0, 10, 20, 30, 2, 1, 2, 3];
    let encoded = ascii85(&deflate(&predictor_bytes));
    let direct = one_stream(
        &format!(
            "/Filter [/ASCII85Decode /FlateDecode] /DecodeParms \
             [null << {PREDICTOR} >>]"
        ),
        &encoded,
        &[],
    );
    let indirect = one_stream(
        "/Filter 7 0 R /DecodeParms 5 0 R",
        &encoded,
        &[
            (5, b"[null 6 0 R]".to_vec()),
            (6, format!("<< {PREDICTOR} >>").into_bytes()),
            (7, b"[8 0 R 9 0 R]".to_vec()),
            (8, b"/ASCII85Decode".to_vec()),
            (9, b"/FlateDecode".to_vec()),
        ],
    );
    assert!(
        !direct.cross_reference_chain_was_cut(),
        "direct control has a complete xref chain"
    );
    assert!(
        !direct.was_recovered(),
        "direct control uses its exact xref"
    );
    assert!(
        !indirect.cross_reference_chain_was_cut(),
        "indirect fixture has a complete xref chain"
    );
    assert!(
        !indirect.was_recovered(),
        "indirect array values use their exact xref"
    );
    let expected = [10, 20, 30, 11, 22, 33];
    assert_eq!(decoded_stream(&direct), expected);
    assert_eq!(decoded_stream(&indirect), expected);
}

fn assert_numeric_parameter(entries: &str, value: &[u8]) {
    let predictor_bytes = [0, 10, 20, 30, 2, 1, 2, 3];
    let encoded = deflate(&predictor_bytes);
    let direct = one_stream(
        &format!("/Filter /FlateDecode /DecodeParms << {PREDICTOR} >>"),
        &encoded,
        &[],
    );
    let indirect = one_stream(
        &format!("/Filter /FlateDecode /DecodeParms << {entries} >>"),
        &encoded,
        &[(5, value.to_vec())],
    );
    assert!(!direct.cross_reference_chain_was_cut());
    assert!(!indirect.cross_reference_chain_was_cut());
    assert!(!direct.was_recovered());
    assert!(!indirect.was_recovered());
    let expected = [10, 20, 30, 11, 22, 33];
    assert_eq!(decoded_stream(&direct), expected);
    assert_eq!(decoded_stream(&indirect), expected);
}

/// The generic equivalence rule also reaches scalar values inside the parameter dictionary.
#[test]
fn indirect_predictor_number_keeps_the_same_decoded_samples() {
    assert_numeric_parameter(
        "/Predictor 5 0 R /Colors 1 /BitsPerComponent 8 /Columns 3",
        b"15",
    );
}

/// Columns is three rather than its default one, so ignoring the reference is observable.
#[test]
fn indirect_columns_number_keeps_the_same_decoded_samples() {
    assert_numeric_parameter(
        "/Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns 5 0 R",
        b"3",
    );
}
