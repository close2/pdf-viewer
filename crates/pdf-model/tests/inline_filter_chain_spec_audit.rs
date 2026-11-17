//! Inline image scan/decode regression for an inline image whose decode filter chain has two stages.
//!
//! The 16 sample bytes are independently fixed; zlib then ASCII85 are the file-side encoding
//! operations, so the PDF decoder list is [/A85 /Fl]. The ASCII85 text contains a whitespace-
//! delimited `EI` token, and whitespace insertion around those two existing ASCII85 characters
//! leaves the encoded value unchanged. The marker must locate the real end through that token.
//! This does not assert that an implementation must use Note 2's optional skip optimization.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "exact small fixtures and literal decoder assertions fail loudly"
)]

use std::fmt::Write as _;
use std::io::Write as _;

use pdf_syntax::{Dictionary, Document};

const SAMPLES: &[u8] = &[
    0x3e, 0xaf, 0xcc, 0x59, 0xb3, 0xfc, 0x6c, 0xdb, 0xf0, 0x07, 0x8f, 0x3c, 0xa9, 0x18, 0x75, 0x18,
];

const A85_PLAIN: &[u8] = b"Gau*EIjT4-T:FXXf_]Md8sDpJ!)4dG)#~>";
const A85_WITH_FAKE_EI: &[u8] = b"Gau* EI jT4-T:FXXf_]Md8sDpJ!)4dG)#~>";

fn pdf_with_inline_image(encoded: &[u8]) -> (Document, Vec<u8>) {
    let mut content = b"BI /W 16 /H 1 /BPC 8 /CS /G /F [/A85 /Fl] ID ".to_vec();
    content.extend_from_slice(encoded);
    content.extend_from_slice(b" EI q Q");
    let objects: [&[u8]; 3] = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 10] /Resources << >> /Contents 4 0 R >>\nendobj\n",
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }
    offsets.push(bytes.len());
    let _ = write!(bytes, "4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    bytes.extend_from_slice(&content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(tail.as_bytes());
    let document = Document::open(bytes).expect("the exact-xref control document is valid");
    assert!(!document.was_recovered(), "control xref offsets are exact");
    assert!(!document.cross_reference_chain_was_cut());
    (document, content)
}

fn scan(document: &Document, content: &[u8]) -> pdf_model::inline_image::Scan {
    pdf_model::inline_image::scan(document, content, 2, &Dictionary::new(), true)
}

fn decoded(encoded: &[u8]) -> Vec<u8> {
    let (document, content) = pdf_with_inline_image(encoded);
    let scanned = scan(&document, &content);
    let stream = scanned.image.expect("a valid chained-filter inline image");
    assert_eq!(
        stream.data.as_ref(),
        encoded,
        "the real ASCII85 marker sets the extent"
    );
    assert_eq!(
        scanned.resume,
        content.len() - b" q Q".len(),
        "the scan resumes at the real EI, past the false encoded token"
    );
    assert!(!document.was_recovered());
    assert!(!document.cross_reference_chain_was_cut());
    document
        .decoded_stream_data(&stream)
        .expect("ASCII85 then Flate decode the sample bytes")
        .to_vec()
}

#[test]
fn inline_a85_outer_filter_chain_ignores_a_valid_encoded_ei_token() {
    // Independent positive control: Python's zlib + ASCII85 encoding of SAMPLES, without the
    // inserted whitespace token, is literal A85_PLAIN. (The payload and encoded bytes are not
    // generated with pdf_syntax's filter code.)
    assert_eq!(decoded(A85_PLAIN), SAMPLES);

    // ASCII85 ignores whitespace, so placing a whitespace-delimited EI around two existing
    // alphabet characters preserves the encoded bytes while defeating a forward EI search.
    assert_eq!(decoded(A85_WITH_FAKE_EI), SAMPLES);
}
