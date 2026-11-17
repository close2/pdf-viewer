//! Public attachment regression for §7.11.4.1 Table 45's uncompressed checksum.
//!
//! The stream bytes below are a fixed zlib encoding of `payload-after-flate`. The MD5 digest
//! was computed independently for the uncompressed 19-byte payload; the compressed stream has a
//! different digest. This distinguishes stream framing/filter decoding from a checksum over the
//! encoded PDF bytes.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test fixture has fixed bytes and exact offsets"
)]

use std::io::Write as _;

use pdf_model::attachment::attachments;
use pdf_syntax::Document;

const COMPRESSED: &[u8] = &[
    120, 156, 43, 72, 172, 204, 201, 79, 76, 209, 77, 76, 43, 73, 45, 210, 77, 203, 73, 44, 73, 5,
    0, 74, 170, 7, 99,
];
const PAYLOAD: &[u8] = b"payload-after-flate";
const PAYLOAD_MD5_HEX: &str = "29f52318023f658f29a80b071b8c2d62";

fn add_object(bytes: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]) {
    offsets.push(bytes.len());
    let _ = writeln!(bytes, "{} 0 obj", offsets.len());
    bytes.extend_from_slice(body);
    bytes.extend_from_slice(b"\nendobj\n");
}

fn pdf() -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n%\x80\x81\x82\x83\n".to_vec();
    let mut offsets = Vec::new();
    add_object(
        &mut bytes,
        &mut offsets,
        b"<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> >>",
    );
    add_object(
        &mut bytes,
        &mut offsets,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    add_object(
        &mut bytes,
        &mut offsets,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << >> >>",
    );
    add_object(
        &mut bytes,
        &mut offsets,
        b"<< /Names [(payload.bin) 5 0 R] >>",
    );
    add_object(
        &mut bytes,
        &mut offsets,
        b"<< /Type /Filespec /F (payload.bin) /UF (payload.bin) /Desc (test payload) \
           /EF << /F 6 0 R /UF 6 0 R >> >>",
    );

    offsets.push(bytes.len());
    let _ = write!(
        bytes,
        "6 0 obj\n<< /Type /EmbeddedFile /Subtype /application#2Foctet-stream \
         /Filter /FlateDecode /Length {} /Params << /Size {} /CheckSum <{}> >> >>\nstream\n",
        COMPRESSED.len(),
        PAYLOAD.len(),
        PAYLOAD_MD5_HEX,
    );
    bytes.extend_from_slice(COMPRESSED);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes
}

#[test]
fn embedded_file_checksum_is_checked_after_filter_decoding() {
    assert_eq!(PAYLOAD.len(), 19);
    assert_eq!(COMPRESSED.len(), 27);

    let document = Document::open(pdf()).expect("exact-xref embedded-file PDF");
    assert!(!document.was_recovered());
    assert!(!document.cross_reference_chain_was_cut());
    let files = attachments(&document);
    let [file] = files.as_slice() else {
        panic!("one name-tree attachment is discoverable");
    };
    assert_eq!(
        file.size,
        Some(19),
        "Table 45 Size is uncompressed byte size"
    );
    let decoded = document
        .decoded_stream_data(&file.stream)
        .expect("the FlateDecode embedded stream decodes");
    assert_eq!(&*decoded, PAYLOAD);
    assert_eq!(file.checksum_matches(&decoded), Some(true));
    assert_eq!(file.checksum_matches(COMPRESSED), Some(false));
}
