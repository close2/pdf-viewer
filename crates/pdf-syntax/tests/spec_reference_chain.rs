//! ISO 32000-2 §7.3.10 permits an ordinary indirect object to consist only of a reference.

use std::fmt::Write as _;

use pdf_syntax::{Document, Object, ObjectId};

#[test]
fn ordinary_indirect_objects_may_form_a_reference_chain() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in [
        "<< /Type /Catalog /Pages 2 0 R >>".as_bytes(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".as_bytes(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".as_bytes(),
        b"5 0 R".as_slice(),
        b"(the terminal value)".as_slice(),
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

    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ");
    }
    let _ = write!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );

    let document = Document::open(bytes).expect("the indirect objects and xref are valid");
    assert!(!document.was_recovered());
    assert_eq!(
        document.get(ObjectId::new(4, 0)),
        Object::Reference(ObjectId::new(5, 0)),
        "object 4 is defined solely by its reference"
    );
    assert_eq!(
        document.resolve(&Object::Reference(ObjectId::new(4, 0))),
        Object::String(b"the terminal value".to_vec().into()),
        "resolving object 4 follows its ordinary indirect-reference chain"
    );
}

/// Appends fixture text to a byte vector.
struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
