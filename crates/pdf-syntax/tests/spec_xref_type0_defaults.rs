//! A type 0 Table 18 record needs only its stated type byte.

use std::fmt::Write as _;

use pdf_syntax::{Document, Object, ObjectId};

#[test]
fn type_zero_entry_without_next_free_or_generation_fields_marks_object_deleted() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".as_slice(),
        b"2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n".as_slice(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n".as_slice(),
        b"4 0 obj\n(the old value)\nendobj\n".as_slice(),
    ] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }

    // The earlier classic section says object 4 is in use.
    let previous = bytes.len();
    bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ");
    }
    let _ = write!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{previous}\n%%EOF\n"
    );

    // In the hybrid update, the one-byte type-0 record has no next-free or generation
    // fields: /W [1 0 0]. The stream's own entry is in the companion classic table.
    let stream_at = bytes.len();
    bytes.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XRef /Size 6 /Index [4 1] /W [1 0 0] /Length 1 /Prev {previous} >>\nstream\n"
        )
        .as_bytes(),
    );
    bytes.push(0);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    let table_at = bytes.len();
    let mut table = format!("xref\n5 1\n{stream_at:010} 00000 n \n");
    let _ = writeln!(
        table,
        "trailer\n<< /Size 6 /Root 1 0 R /XRefStm {stream_at} /Prev {previous} >>\nstartxref\n{table_at}\n%%EOF"
    );
    bytes.extend_from_slice(table.as_bytes());

    let document = Document::open(bytes).expect("valid hybrid xref fixture");
    assert!(
        !document.was_recovered(),
        "the declared sections are readable"
    );
    assert_eq!(
        document.get(ObjectId::new(4, 0)),
        Object::Null,
        "type 0's absent fields do not prevent its deletion from overriding the older entry"
    );
}

/// Appends UTF-8 fixture text to a byte buffer through `fmt::Write`.
struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
