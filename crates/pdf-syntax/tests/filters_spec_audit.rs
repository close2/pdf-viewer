//! Specification audit of ISO 32000-2 §7.4 (filters) and §7.3.8's stream entries.
//!
//! Each test is one behaviour finding of `doc/reviews/2142-specification-audit-7-4.md` and fails
//! on the tree the audit read; each carries a control that passes there. The documents are
//! written here with exact cross-reference offsets, and every expected value is derived from the
//! clause the test names, never from what another reader produces.

#![expect(
    clippy::expect_used,
    reason = "small exact-xref in-memory PDF fixtures with checked-by-construction offsets"
)]

use std::io::Write as _;

use pdf_syntax::filter::decode_reported;
use pdf_syntax::{Document, Limits, ObjectId, StreamRefusal};

fn deflate(input: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).expect("write to Vec");
    encoder.finish().expect("finish zlib stream")
}

/// An exact-xref one-page PDF whose object 4 is a stream with `entries` and `data`.
///
/// `/Size` is 5, so any reference to object 5 or above names an undefined object.
fn one_stream(entries: &str, data: &[u8]) -> Document {
    let objects: [(u32, Vec<u8>); 4] = [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>".to_vec(),
        ),
        (
            4,
            format!("<< {entries} /Length {} >>", data.len()).into_bytes(),
        ),
    ];
    let mut offsets = Vec::new();
    let mut bytes = b"%PDF-1.7\n%\x80\x81\x82\x83\n".to_vec();
    for (number, body) in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        bytes.extend_from_slice(&body);
        if number == 4 {
            bytes.extend_from_slice(b"\nstream\n");
            bytes.extend_from_slice(data);
            bytes.extend_from_slice(b"\nendstream");
        }
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    Document::open(bytes).expect("well-formed one-page PDF with exact offsets")
}

/// Object 4's stream run through its stated filter chain.
fn decoded(document: &Document) -> Result<Vec<u8>, StreamRefusal> {
    let stream = document
        .get(ObjectId::new(4, 0))
        .as_stream()
        .expect("object 4 is the stream")
        .clone();
    document
        .decoded_stream_data_reported(&stream)
        .map(|decoded| decoded.data.to_vec())
}

/// 2142-B01. Table 5: `/Filter` is "\[t]he name, or an array of zero, one or several names, of
/// filter(s) that shall be applied", and §7.4.1: "A PDF reader shall invoke the corresponding
/// decoding filter or filters". An array element that is not a name names no filter this reader
/// can invoke, so the stream cannot be decoded as stated; handing back the output of the other
/// stages alone is a decode the file did not state (trap 5).
#[test]
fn a_filter_entry_that_is_not_a_name_is_not_skipped_spec_audit() {
    // Control: the one-stage chain decodes "ABC" (§7.4.2).
    let control = one_stream("/Filter [/ASCIIHexDecode]", b"414243>");
    assert_eq!(decoded(&control), Ok(b"ABC".to_vec()), "control");

    let skipped = one_stream("/Filter [/ASCIIHexDecode 7]", b"414243>");
    assert!(
        decoded(&skipped).is_err(),
        "a two-element chain whose second element is not a filter name decoded as the first \
         stage alone: {:?}",
        decoded(&skipped)
    );

    let not_a_filter = one_stream("/Filter 7", b"414243>");
    assert!(
        decoded(&not_a_filter).is_err(),
        "a /Filter that is neither a name nor an array came back as the raw bytes: {:?}",
        decoded(&not_a_filter)
    );
}

/// 2142-B02. §7.4.2: "A GREATER-THAN SIGN (3Eh) indicates EOD"; §7.4.3: the encoding uses "the
/// 2-character sequence ~> (7Eh)(3Eh) as its EOD marker". §7.3.8.2: the filters' data "shall be
/// self-limiting" through that marker and "\[a]ll of these constraints shall be consistent".
/// `Decoded::damage` is documented as `None` only "where it reached the filter's end-of-data",
/// and `RunLengthDecode`, `LZWDecode` and `FlateDecode` report `Damage::Truncated` when their
/// marker is missing; the two ASCII filters report data without theirs as whole.
#[test]
fn ascii_data_without_its_eod_marker_is_reported_short_spec_audit() {
    let limits = Limits::default();
    // Controls: each with its marker is whole.
    let hex = decode_reported(b"ASCIIHexDecode", b"414243>", None, limits).expect("hex control");
    assert_eq!((&*hex.data, hex.damage), (&b"ABC"[..], None), "hex control");
    // "ABCD" is 0x41424344 = 1 094 861 636 = 20·85⁴ + 82·85³ + 67·85² + 80·85 + 11 by §7.4.3's
    // relation, and those digits plus 33 are the five characters below.
    let base85 = decode_reported(b"ASCII85Decode", b"5sdq,~>", None, limits).expect("85 control");
    assert_eq!(
        (&*base85.data, base85.damage),
        (&b"ABCD"[..], None),
        "85 control"
    );

    let hex_cut = decode_reported(b"ASCIIHexDecode", b"414243", None, limits);
    assert!(
        hex_cut
            .as_ref()
            .ok()
            .is_none_or(|decoded| decoded.damage.is_some()),
        "hexadecimal data with no `>` was reported as reaching its end-of-data: {hex_cut:?}"
    );
    let base85_cut = decode_reported(b"ASCII85Decode", b"5sdq,", None, limits);
    assert!(
        base85_cut
            .as_ref()
            .ok()
            .is_none_or(|decoded| decoded.damage.is_some()),
        "base-85 data with no `~>` was reported as reaching its end-of-data: {base85_cut:?}"
    );
}

/// The PNG rows the predictor tests share: row 0 tagged None, row 1 tagged Up (ISO/IEC 15948),
/// three one-byte samples each, so the unpredicted data is `10 20 30 11 22 33`.
const PNG_ROWS: [u8; 8] = [0, 10, 20, 30, 2, 1, 2, 3];
const UNPREDICTED: [u8; 6] = [10, 20, 30, 11, 22, 33];

/// 2142-B03. Table 8: `/Predictor` is an integer, and "\[i]f the value is greater than 1, the
/// filter shall assume that the data were differenced before being encoded". A `/Predictor` that
/// is present but not an integer is read as absent — Table 8's default 1 — so the PNG rows come
/// back with their algorithm tags in them as the decoded data. Either reading `12.0` as 12 or
/// refusing the stream would be a reading of the clause; returning the predicted rows as data is
/// the one answer that is not (the tree's own `apply_predictor` comment: "unpredicted data read
/// as predicted is silently wrong").
#[test]
fn a_predictor_that_is_not_an_integer_is_not_read_as_no_predictor_spec_audit() {
    let encoded = deflate(&PNG_ROWS);
    let control = one_stream(
        "/Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 3 >>",
        &encoded,
    );
    assert_eq!(decoded(&control), Ok(UNPREDICTED.to_vec()), "control");

    let real = one_stream(
        "/Filter /FlateDecode /DecodeParms << /Predictor 12.0 /Columns 3 >>",
        &encoded,
    );
    let got = decoded(&real);
    assert!(
        got.as_ref()
            .ok()
            .is_none_or(|bytes| bytes.as_slice() == UNPREDICTED),
        "a /Predictor of 12.0 handed back the predicted rows as data: {got:?}"
    );
}

/// 2142-B04. Table 8: `/BitsPerComponent` "Valid values are 1 , 2 , 4 , 8 , and ( PDF 1.5 ) 16".
/// The TIFF predictor refuses any other depth (ADR 2729: "data predicted at a depth nobody can
/// state cannot be un-predicted at a guessed one"); the PNG group, whose bytes-per-pixel and row
/// length the depth decides, un-predicts at the invalid depth and reports the result as whole.
#[test]
fn a_png_predictor_refuses_a_depth_table_8_does_not_list_spec_audit() {
    // Eight three-bit samples are three bytes a row; one row, tagged None.
    let encoded = deflate(&[0, 0x12, 0x34, 0x56]);
    let tiff = one_stream(
        "/Filter /FlateDecode /DecodeParms << /Predictor 2 /BitsPerComponent 3 /Columns 8 >>",
        &deflate(&[0x12, 0x34, 0x56]),
    );
    assert!(
        decoded(&tiff).is_err(),
        "control: TIFF at depth 3 is refused"
    );

    let png = one_stream(
        "/Filter /FlateDecode /DecodeParms << /Predictor 12 /BitsPerComponent 3 /Columns 8 >>",
        &encoded,
    );
    assert!(
        decoded(&png).is_err(),
        "PNG prediction at depth 3 was un-predicted and reported whole: {:?}",
        decoded(&png)
    );
}

/// 2142-B05. Table 6's Parameters column says no for `RunLengthDecode`, and Table 8, whose
/// entries `/Predictor` is one of, lists the optional parameters of `LZWDecode` and `FlateDecode`
/// alone; §7.4.4.4 states the predictors for those two filters. The run-length decode of the
/// bytes below is `1 2 3` (§7.4.5: a length byte of 2 copies the next three bytes, 128 is EOD); a
/// TIFF predictor run over it gives `1 3 6`, which is no filter's output.
#[test]
fn a_predictor_is_not_applied_after_a_filter_that_takes_no_parameters_spec_audit() {
    let encoded = [2u8, 1, 2, 3, 128];
    let control = one_stream("/Filter /RunLengthDecode", &encoded);
    assert_eq!(decoded(&control), Ok(vec![1, 2, 3]), "control");

    let with_parms = one_stream(
        "/Filter /RunLengthDecode /DecodeParms << /Predictor 2 /Columns 3 >>",
        &encoded,
    );
    let got = decoded(&with_parms);
    assert!(
        got.as_ref()
            .ok()
            .is_none_or(|bytes| bytes.as_slice() == [1, 2, 3]),
        "a predictor was applied to RunLengthDecode's output: {got:?}"
    );
}

/// 2142-B06. Table 5's `/F`: "If this entry is present, the bytes between stream and endstream
/// shall be ignored." §7.3.10: "An indirect reference to an undefined object ... shall be treated
/// as a reference to the null object", and §7.3.9: "Specifying the null object as the value of a
/// dictionary entry ... shall be equivalent to omitting the entry entirely." So `/F 9 0 R`, with
/// no object 9, is no `/F`, and the stream's own bytes are its data.
#[test]
fn an_external_file_entry_that_is_null_is_absent_spec_audit() {
    let direct = one_stream("/Filter /ASCIIHexDecode /F (outside.dat)", b"414243>");
    assert_eq!(
        decoded(&direct),
        Err(StreamRefusal::External),
        "control: a stated file specification makes the stream external"
    );

    let undefined = one_stream("/Filter /ASCIIHexDecode /F 9 0 R", b"414243>");
    assert_eq!(
        decoded(&undefined),
        Ok(b"ABC".to_vec()),
        "an /F naming an undefined object refused the stream as external"
    );
}

/// 2142-B07. §7.4.10, Table 14's `/Name`: "The name shall correspond to an entry in the CF entry
/// of the encryption dictionary ... or one of the standard crypt filters" (Table 26 lists one,
/// `Identity`). A document with no encryption dictionary has no `/StdCF`, so a stream that names
/// it cannot be decrypted; passing its bytes through as the decoded data is ciphertext handed
/// over as plaintext (trap 5), where the same name in an encrypted document that lacks it is
/// refused (`named_method` answers `None`).
#[test]
fn a_crypt_filter_naming_no_defined_filter_is_not_a_pass_through_spec_audit() {
    let identity = one_stream("/Filter /Crypt /DecodeParms << /Name /Identity >>", b"ABC");
    assert_eq!(decoded(&identity), Ok(b"ABC".to_vec()), "control: Identity");

    let named = one_stream("/Filter /Crypt /DecodeParms << /Name /StdCF >>", b"ABC");
    assert!(
        decoded(&named).is_err(),
        "a Crypt filter naming an undefined crypt filter passed its bytes through: {:?}",
        decoded(&named)
    );
}
