//! ISO 32000-2 §7.3.8.1's end-of-line marker after `stream`, and the recovery for spaces before it.
//!
//! > The keyword stream that follows the stream dictionary shall be followed by an end-ofline
//! > marker consisting of either a CARRIAGE RETURN and a LINE FEED or just a LINE FEED, and not by
//! > a CARRIAGE RETURN alone.
//!
//! A file that writes `stream`, a space and a line feed is malformed: a space is not an
//! end-of-line marker. The clause places the data after the marker, so the space is not data on
//! any reading of the file either, and this reader skips it and says so on the document
//! (`Document::padded_stream_keywords`). ADR 1365 is the argument.
//!
//! **The pairs are the point** (trap 28): the recovery's guard states when it is needed, the
//! comment above it states when it is right, and each pair below is a file where a guard written
//! more loosely would have taken a producer's data bytes for padding.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that does not parse should fail loudly, and every fixture here \
              is a few hundred bytes"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_syntax::{Document, Object, ObjectId};

/// A one-stream file: object 1 is the stream, written as `dictionary` then `keyword_tail` (the
/// bytes after the `stream` keyword, marker included) then `data`, `\nendstream`. Object 2 is a
/// length, for a dictionary that states `/Length 2 0 R`.
fn file(dictionary: &str, keyword_tail: &[u8], data: &[u8], indirect_length: usize) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let first = out.len();
    out.extend_from_slice(b"1 0 obj\n");
    out.extend_from_slice(dictionary.as_bytes());
    out.extend_from_slice(b"\nstream");
    out.extend_from_slice(keyword_tail);
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let second = out.len();
    out.extend_from_slice(format!("2 0 obj\n{indirect_length}\nendobj\n").as_bytes());
    let third = out.len();
    out.extend_from_slice(b"3 0 obj\n<< /Type /Catalog >>\nendobj\n");
    let xref = out.len();
    let mut table = String::from("xref\n0 4\n0000000000 65535 f\r\n");
    for offset in [first, second, third] {
        write!(table, "{offset:010} 00000 n\r\n").expect("a String takes every write");
    }
    write!(
        table,
        "trailer\n<< /Size 4 /Root 3 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("a String takes every write");
    out.extend_from_slice(table.as_bytes());
    out
}

/// Object 1's data, and what the document reported about its keyword.
fn read(bytes: Vec<u8>) -> (Vec<u8>, Vec<(u32, usize)>) {
    let document = Document::open(bytes).expect("the fixture opens");
    assert!(
        !document.was_recovered(),
        "the fixture's table is right, so nothing may be scanned for"
    );
    let Object::Stream(stream) = document.get(ObjectId::new(1, 0)) else {
        panic!("object 1 is a stream");
    };
    (stream.data.to_vec(), document.padded_stream_keywords())
}

const DATA: &[u8] = b"0 0 m 10 10 l S";

/// A conforming LF, and a conforming CR LF: nothing skipped, nothing reported.
#[test]
fn a_conforming_marker_is_not_a_recovery() {
    for tail in [&b"\n"[..], b"\r\n"] {
        let (data, padded) = read(file("<< /Length 15 >>", tail, DATA, 0));
        assert_eq!(data, DATA);
        assert!(padded.is_empty(), "{tail:?}: {padded:?}");
    }
}

/// `stream`, spaces or tabs, then the marker: the padding and the marker are skipped, and the
/// document says how many bytes of padding there were — whether the length is stated directly,
/// indirectly, or not usable at all and found by searching for `endstream`.
#[test]
fn padding_before_the_marker_is_skipped_and_reported() {
    for (tail, bytes) in [
        (&b" \n"[..], 1),
        (b" \r\n", 1),
        (b"\t \t\r\n", 3),
        (b"  \r", 2),
    ] {
        for dictionary in [
            "<< /Length 15 >>",
            "<< /Length 2 0 R >>",
            "<< /Length 999 >>",
            "<< >>",
        ] {
            let (data, padded) = read(file(dictionary, tail, DATA, DATA.len()));
            assert_eq!(data, DATA, "{tail:?} under {dictionary}");
            assert_eq!(padded, vec![(1, bytes)], "{tail:?} under {dictionary}");
        }
    }
}

/// The first half of the guard: spaces that no marker ends are not padding. `stream`, a space
/// and then the data is a file with no marker at all, which the recovery is not for — and
/// taking the space for padding would be a guess about where the producer's data began.
#[test]
fn spaces_no_marker_ends_are_left_where_they_are() {
    let (data, padded) = read(file("<< /Length 16 >>", b" ", DATA, 0));
    assert_eq!(data, [&b" "[..], DATA].concat());
    assert!(padded.is_empty(), "{padded:?}");
}

/// The second half of the guard: a `/Length` that fits only where the padding is data wins,
/// directly or through a reference. The file's own statement of the stream's extent is never
/// overruled by this reader's reading of the bytes before it — and a stream read that way is
/// not reported, because nothing was skipped.
#[test]
fn a_length_that_counts_the_padding_as_data_wins() {
    let counted = DATA.len().saturating_add(2);
    for dictionary in [
        format!("<< /Length {counted} >>"),
        "<< /Length 2 0 R >>".to_owned(),
    ] {
        let (data, padded) = read(file(&dictionary, b" \n", DATA, counted));
        assert_eq!(data, [&b" \n"[..], DATA].concat(), "{dictionary}");
        assert!(padded.is_empty(), "{dictionary}: {padded:?}");
    }
}

/// Counts the scan lines a fax decode delivers.
struct Counted(u32);

impl pdf_ccitt::Rows for Counted {
    fn row(&mut self, _: &[u32]) {
        self.0 = self.0.saturating_add(1);
    }
}

/// The corpus witness: `4113564.pdf` of the `SafeDocs` crawl writes `stream`, a space and a line
/// feed before every stream it has, six of them `CCITTFaxDecode` images. Each now begins at the
/// byte after the line feed, is reported with its one byte of padding, and decodes as a Group 4
/// image to every row its dictionary states.
///
/// Skips, loudly, where the crawl is absent: `corpus-cache` is machine-local.
#[test]
fn the_crawled_witness_decodes_every_fax_image() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus-cache/safedocs/cc-main-2021-31/4113/4113564.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipped: {} is not on this machine", path.display());
        return;
    };
    let document = Document::open(bytes).expect("the witness opens");

    let mut images = 0;
    for number in document.xref().object_numbers().collect::<Vec<_>>() {
        let Object::Stream(stream) = document.get(ObjectId::new(number, 0)) else {
            continue;
        };
        let filter = stream.dict.get("Filter").and_then(Object::as_name);
        if filter.map(pdf_syntax::Name::as_bytes) != Some(&b"CCITTFaxDecode"[..]) {
            continue;
        }
        images += 1;
        let parms = document.resolve(stream.dict.get("DecodeParms").expect("each states them"));
        let entry = |key: &str| {
            parms
                .as_dict()
                .and_then(|dict| dict.get(key))
                .and_then(Object::as_integer)
                .expect("the witness states /K, /Columns and /Rows")
        };
        let rows = u32::try_from(entry("Rows")).expect("a row count");
        let parameters = pdf_ccitt::Parameters {
            coding: pdf_ccitt::Coding::from_k(entry("K")),
            columns: u32::try_from(entry("Columns")).expect("a column count"),
            rows,
            end_of_line: false,
            encoded_byte_align: false,
            end_of_block: true,
            damaged_rows_before_error: 0,
        };
        let mut counted = Counted(0);
        let summary = pdf_ccitt::decode(&stream.data, &parameters, &mut counted)
            .unwrap_or_else(|error| panic!("object {number}: {error}"));
        assert_eq!(summary.rows, rows, "object {number}");
        assert_eq!(counted.0, rows, "object {number}");

        // And the pair (trap 8): the same data with the space and the line feed handed over as
        // its first two bytes, which is what the stream was before the recovery.
        let before = [&b" \n"[..], &stream.data].concat();
        assert!(
            pdf_ccitt::decode(&before, &parameters, &mut Counted(0)).is_err(),
            "object {number}: the padding as data must be what broke the decode"
        );
    }
    assert_eq!(images, 6, "the census counted six fax images in this file");

    let padded = document.padded_stream_keywords();
    assert!(
        padded.len() >= 6 && padded.iter().all(|&(_, bytes)| bytes == 1),
        "every stream the file writes pads its keyword with one space: {padded:?}"
    );
}
