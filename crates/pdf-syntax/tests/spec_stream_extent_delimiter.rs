//! ISO 32000-2 §7.3.8.1: the optional EOL before `endstream` is not included in `/Length`.

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};

fn stream_file(separator: &[u8]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in [
        "<< /Type /Catalog /Pages 2 0 R >>".as_bytes(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".as_bytes(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".as_bytes(),
    ]
    .into_iter()
    .enumerate()
    {
        offsets.push(bytes.len());
        let _ = writeln!(
            StringWriter(&mut bytes),
            "{} 0 obj",
            number.saturating_add(1)
        );
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }

    offsets.push(bytes.len());
    bytes.extend_from_slice(b"4 0 obj\n<< /Length 3 >>\nstream\nabc");
    bytes.extend_from_slice(separator);
    bytes.extend_from_slice(b"endstream\nendobj\n");

    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ");
    }
    let _ = write!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes
}

/// The count ends at `c`; either permitted EOL separator before `endstream` is excluded.
#[test]
fn optional_eol_before_endstream_is_outside_the_declared_extent() {
    for separator in [&b"\n"[..], b"\r\n"] {
        let document = Document::open(stream_file(separator)).expect("valid stream extent");
        assert!(!document.was_recovered(), "the xref table is intact");
        let object = document.get(ObjectId::new(4, 0));
        let stream = object.as_stream().expect("object 4 is a stream");
        assert_eq!(stream.data.as_ref(), b"abc", "separator {separator:?}");
    }
}

/// Appends fixture text to a byte vector.
struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
