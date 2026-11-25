//! Two repairs a reader makes without a document in hand, each brought to the one channel
//! (ADR 3477, ADR 3621): a file with no header, read with its offsets counted from its first byte
//! (`syntax/header-missing`, ISO 32000-2 §7.5.2), and a text string whose bytes contradict their
//! encoding, read with U+FFFD in their place (`syntax/text-string-undecodable`, §7.9.2.2).
//!
//! §7.5.2: "The first line of a PDF file shall be a header consisting of the 5 characters %PDF–
//! followed by a version number", and the offsets of the cross-reference table are counted from
//! its percent sign; a file without one has no stated origin.

use pdf_syntax::{Document, Site, text_string_noting};

/// The header class's stable name.
const HEADER: &str = "syntax/header-missing";

/// The text string class's stable name.
const TEXT: &str = "syntax/text-string-undecodable";

/// A file of catalog, page tree and page with a classic cross-reference table, opening with
/// `header` (which may be empty), every offset computed.
fn a_file(header: &str) -> Vec<u8> {
    let mut out = header.as_bytes().to_vec();
    let mut offsets = Vec::new();
    for body in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
    ] {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", offsets.len()).as_bytes());
    }
    let xref_at = out.len();
    out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
    );
    out
}

/// Opens a fixture; one that does not open fails the test.
#[expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot be opened must fail loudly"
)]
fn open(bytes: Vec<u8>) -> Document {
    Document::open(bytes).expect("the fixture opens")
}

/// The names of the classes the document has logged so far.
fn classes(document: &Document) -> Vec<&'static str> {
    document
        .repairs()
        .iter()
        .map(|repair| repair.class.name)
        .collect()
}

#[test]
fn a_file_with_no_header_logs_the_origin_it_was_read_from() {
    let stated = open(a_file("%PDF-1.7\n"));
    assert!(
        !classes(&stated).contains(&HEADER),
        "a file with its header logs nothing: {:?}",
        classes(&stated)
    );
    let headless = open(a_file(""));
    assert!(
        headless
            .get(pdf_syntax::ObjectId::new(3, 0))
            .as_dict()
            .is_some(),
        "its table read from byte zero still reaches the page"
    );
    assert!(
        classes(&headless).contains(&HEADER),
        "{:?}",
        classes(&headless)
    );
}

#[test]
fn a_text_string_says_when_it_stands_in_for_bytes() {
    // A conforming string in each of the three encodings reads with no class.
    for bytes in [
        &b"plain"[..],
        &[254, 255, 0, b'A', 0xD8, 0x3D, 0xDE, 0x00][..],
        &[239, 187, 191, b'o', b'k'][..],
    ] {
        assert!(text_string_noting(bytes).1.is_none(), "{bytes:?}");
    }
    // Each of the four contradictions reads with U+FFFD and the class.
    for bytes in [
        // An unpaired high surrogate after the UTF-16BE prefix.
        &[254, 255, 0xD8, 0x3D, 0, b'A'][..],
        // An odd trailing byte after it.
        &[254, 255, 0, b'A', 0][..],
        // A malformed sequence after the UTF-8 prefix.
        &[239, 187, 191, 0xC3][..],
        // A byte Table D.3 leaves undefined, its `U`: "Undefined code point".
        &[b'a', 0x01][..],
    ] {
        let (text, class) = text_string_noting(bytes);
        assert!(text.contains('\u{FFFD}'), "{bytes:?} read as {text:?}");
        assert_eq!(class.map(|class| class.name), Some(TEXT), "{bytes:?}");
    }
}

#[test]
fn a_reader_holding_the_document_logs_the_string_it_decoded() {
    let document = open(a_file("%PDF-1.7\n"));
    assert_eq!(document.text_string(b"Title", Site::WholeFile), "Title");
    assert!(!classes(&document).contains(&TEXT));
    let read = document.text_string(&[254, 255, 0xDC, 0x00], Site::WholeFile);
    assert_eq!(read, "\u{FFFD}");
    assert!(
        classes(&document).contains(&TEXT),
        "{:?}",
        classes(&document)
    );
}
