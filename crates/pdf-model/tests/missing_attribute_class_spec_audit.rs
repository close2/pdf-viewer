//! EC3 #254 and §14.7.6.2: unknown classes contribute no attributes.
//! This checks the public attribute-resolution API, not page reflow.

#![expect(clippy::expect_used, reason = "exact-xref fixtures must parse")]

use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Document, Object};

fn document(objects: &[&str]) -> Document {
    use std::fmt::Write as _;
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(out.into_bytes()).expect("a valid file")
}

#[test]
fn missing_classes_contribute_no_attributes_and_preserve_other_sources() {
    for (class_map, class_is_present) in [
        ("", false),
        ("/ClassMap << /Other 7 0 R >>", false),
        ("/ClassMap << /Known 7 0 R >>", true),
    ] {
        let root = format!("<< /Type /StructTreeRoot /K 5 0 R {class_map} >>");
        let document = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>",
            &root,
            "<< /Type /StructElem /P 4 0 R /S /P /C [/Absent /Known] /A 6 0 R >>",
            "<< /O /Layout /BackgroundColor [1 0 0] /Placement /Block >>",
            "<< /O /Layout /BackgroundColor [0 0 1] /TextAlign /Start >>",
        ]);
        assert!(!document.was_recovered(), "exact xref offsets");
        let tree = Tree::of(&document).expect("structure tree");
        let children = tree.children(&document, None);
        let [Child::Element(element)] = children.as_slice() else {
            panic!("one structure element");
        };
        assert_eq!(
            tree.attributes(&document, element).len(),
            if class_is_present { 2 } else { 1 },
            "absent classes add no objects and do not hide later known classes: {class_map}"
        );
        assert_eq!(
            tree.attribute(&document, element, "BackgroundColor")
                .and_then(|value| value.as_array().map(<[Object]>::to_vec)),
            Some(vec![
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(0)
            ]),
            "local red survives missing classes and outranks mapped blue: {class_map}"
        );
        let alignment = tree
            .attribute(&document, element, "TextAlign")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()));
        assert_eq!(
            alignment,
            class_is_present.then(|| b"Start".to_vec()),
            "only the named known class supplies this attribute: {class_map}"
        );
    }
}
