//! Positive control for Table 355's /A allowing an attribute object to be a stream.
//!
//! `Object::as_dict` deliberately exposes a stream's dictionary, so this may pass today. The
//! existing attribute precedence tests use only dictionary-valued attribute objects; this test
//! pins the second form independently with a normal dictionary control.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixtures and explicit assertion failures"
)]

use std::fmt::Write as _;

use pdf_model::structure::Tree;
use pdf_syntax::{Document, FileBytes, Object};

fn document() -> Document {
    let mut bytes = String::from("%PDF-2.0\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>".to_owned(),
        "<< /Type /StructTreeRoot /K [5 0 R] >>".to_owned(),
        "<< /Type /StructElem /S /P /P 4 0 R /A [6 0 R 7 0 R] >>".to_owned(),
        "<< /O /Layout /TextAlign /Center /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /O /Layout /Placement /Block >>".to_owned(),
    ];
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1).expect("String writes do not fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1)
        .expect("String writes do not fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("String writes do not fail");
    }
    write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("String writes do not fail");
    let document =
        Document::open(FileBytes::from(bytes.into_bytes())).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    document
}

#[test]
fn an_attribute_object_stream_and_dictionary_are_both_read() {
    let document = document();
    let tree = Tree::of(&document).expect("structure tree root");
    let Some(pdf_model::structure::Child::Element(element)) =
        tree.children(&document, None).into_iter().next()
    else {
        panic!("one element");
    };

    assert_eq!(
        tree.attribute(&document, &element, "TextAlign"),
        Some(Object::Name(pdf_syntax::Name::new(b"Center".as_slice()))),
        "the stream-valued /A object contributes its attribute dictionary"
    );
    assert_eq!(
        tree.attribute(&document, &element, "Placement"),
        Some(Object::Name(pdf_syntax::Name::new(b"Block".as_slice()))),
        "the ordinary dictionary remains a positive control"
    );
}
