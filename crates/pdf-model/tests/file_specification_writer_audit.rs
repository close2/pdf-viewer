//! Regression for ISO 32000-2 §7.11.3 Table 43's `/Desc` condition.
//!
//! The source is a complete PDF with a catalog, page tree, page and exact xref table.
//! This exercises the actual `ViewState` incremental writer. Document-home attachments must
//! carry a description when written into the embedded-files name tree; refusal is also allowed.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a malformed fixture or an omitted writer field must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::attachment::filing::{Payload, Tree};
use pdf_model::view::{Filing, FilingHome, ViewState};
use pdf_syntax::{Document, Object};

fn valid_source() -> Document {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Resources << >> >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = writeln!(out, "{number} 0 obj\n{body}\nendobj");
    }
    let xref_at = out.len();
    let _ = writeln!(
        out,
        "xref\n0 {}\n0000000000 65535 f ",
        objects.len().saturating_add(1)
    );
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    let document = Document::open(out.into_bytes()).expect("complete valid PDF fixture");
    assert!(!document.was_recovered());
    document
}

fn file_specification(document: &Document, name: &str) -> Option<Object> {
    let catalog = document.catalog().expect("catalog");
    let tree = Tree::read(document, &catalog, &|id| document.get(id));
    let key = pdf_syntax::text_string::encode_text_string(name);
    let (_, value) = tree.entries.iter().find(|(entry, _)| *entry == key)?;
    assert!(
        matches!(value, Object::Reference(_)),
        "Table 43 file spec is indirect when /EF is present"
    );
    Some(document.resolve(value))
}

fn attach_and_save(description: Option<&str>) -> Option<Document> {
    let source = valid_source();
    let mut view = ViewState::of(&source);
    let filing = Filing {
        bytes: Payload::new(b"fresh".to_vec()),
        name: "new.txt".to_owned(),
        description: description.map(str::to_owned),
        media_type: Some("text/plain".to_owned()),
        home: FilingHome::Document,
    };
    // Refusing an undescribed document-home file is compliant: the writer must not create a
    // name-tree file specification that misses Table 43's conditional `/Desc` requirement.
    if view.attach(&source, filing).is_err() {
        return None;
    }
    let saved = view.save(&source).ok()?;
    let document = Document::open(saved.bytes).expect("the written update opens");
    assert!(!document.was_recovered());
    Some(document)
}

fn assert_required_file_spec_shape(document: &Document, expected_description: Option<&str>) {
    let specification_object = file_specification(document, "new.txt")
        .expect("the EmbeddedFiles tree references the new specification");
    let Object::Dictionary(specification) = specification_object else {
        panic!("the tree value is a file specification dictionary");
    };
    assert!(
        document
            .get_key(&specification, "Desc")
            .as_string()
            .is_some(),
        "Table 43 requires Desc for a file specification referenced by EmbeddedFiles: {specification:?}"
    );
    if let Some(expected) = expected_description {
        assert_eq!(
            document
                .get_key(&specification, "Desc")
                .as_string()
                .map(<[u8]>::to_vec),
            Some(pdf_syntax::text_string::encode_text_string(expected)),
            "the explicit producer description is preserved"
        );
    }
    assert_eq!(
        document
            .get_key(&specification, "Type")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Filespec".to_vec()),
        "Table 43 requires /Type when /EF is present"
    );
    assert_eq!(
        document
            .get_key(&specification, "F")
            .as_string()
            .map(<[u8]>::to_vec),
        Some(b"new.txt".to_vec())
    );
    assert_eq!(
        document
            .get_key(&specification, "UF")
            .as_string()
            .map(<[u8]>::to_vec),
        Some(b"new.txt".to_vec())
    );
    let Object::Dictionary(ef) = document.get_key(&specification, "EF") else {
        panic!("/EF names the embedded streams: {specification:?}");
    };
    let stream_id = ef
        .get("F")
        .and_then(Object::as_reference)
        .expect("F names an indirect embedded stream");
    assert_eq!(
        Some(stream_id),
        ef.get("UF").and_then(Object::as_reference),
        "the specification writes the matching embedded stream under both filename keys"
    );
    let object = document.get(stream_id);
    let stream = object
        .as_stream()
        .expect("the EF reference resolves to a stream");
    assert_eq!(
        document
            .decoded_stream_data(stream)
            .expect("embedded payload decodes")
            .as_ref(),
        b"fresh"
    );
}

#[test]
fn document_home_file_spec_has_table43_desc() {
    // Positive control first: the actual writer must retain a supplied description and the other
    // required file-spec relationships as raw serialized output.
    let with_description = attach_and_save(Some("a plain text note"))
        .expect("a file with a supplied description is writable");
    assert_required_file_spec_shape(&with_description, Some("a plain text note"));

    // Missing description is a reachable input through Filing.description=None. The writer may
    // decline it or provide a description; it must not put a bare spec into EmbeddedFiles.
    if let Some(without_description) = attach_and_save(None) {
        assert_required_file_spec_shape(&without_description, None);
    }
}
