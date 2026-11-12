//! ISO 32000-2 Annex K.2: saved XFA values agree with the corresponding PDF fields.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly when a checked-in conformance fixture is unavailable"
)]

use std::path::Path;

use pdf_model::view::{Entered, ViewState};
use pdf_model::xfa::Resource;
use pdf_syntax::{Document, Object};

/// A required checked-in fixture from pdf.js's test corpus.
fn open_fixture(name: &str) -> (Vec<u8>, Document) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("required fixture {} is available: {error}", path.display())
    });
    let document = Document::open(bytes.clone()).expect("the checked-in PDF opens");
    (bytes, document)
}

/// A field's inherited `/V` (§12.7.4.1), following the first widget's `/Parent` chain.
fn stated_value(document: &Document, name: &str) -> Option<Object> {
    let table = pdf_model::view::widgets_by_field_name(document);
    let mut id = *table.get(name)?.first()?;
    for _ in 0..32 {
        let object = document.get(id);
        let dict = object.as_dict()?;
        let value = document.get_key(dict, "V");
        if !value.is_null() {
            return Some(value);
        }
        id = dict.get("Parent").and_then(Object::as_reference)?;
    }
    None
}

/// Reads the PDF field value as text, keeping absent/null values distinct from empty strings.
fn pdf_text_value(document: &Document, name: &str) -> Option<String> {
    match stated_value(document, name)? {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    }
}

/// Annex K.2 requires the XFA field value to agree with its corresponding PDF field `/V`.
/// The existing round-trip tests check the XFA side against the user's edit, but do not assert
/// what the saved PDF field itself says. An independently changed text field is checked as a
/// positive control before the target field so a regression cannot make the control unobserved.
#[test]
fn saving_xfa_text_fields_keeps_each_dataset_value_consistent_with_pdf_v() {
    let (original, document) = open_fixture("annotation-text-widget.pdf");
    assert!(
        !document.was_recovered(),
        "the conformance fixture must parse through its own cross-reference data"
    );
    let positive_control = "formulier1[0].#subform[0].Tekstveld2[0]";
    let target = "formulier1[0].#subform[0].Tekstveld7[0]";
    let edits = [
        (
            positive_control,
            Entered::Text("Fish & <chips>".to_owned()),
            // Table 232: Tekstveld2 states `/MaxLen 10`, so the model keeps ten characters
            // (doc/questions/A834).
            "Fish & <ch",
        ),
        (target, Entered::Text("one\rtwo".to_owned()), "one\rtwo"),
    ];
    let mut view = ViewState::of(&document);
    for (name, value, _) in &edits {
        assert!(
            view.set_field(&document, name, value) > 0,
            "{name} accepts its edit"
        );
    }
    let written = view.save(&document).expect("the hybrid form saves");
    assert!(written.xfa_unkept.is_empty(), "{:?}", written.xfa_unkept);
    assert!(
        written.bytes.starts_with(&original),
        "§7.5.6 appends the update"
    );

    let saved = Document::open(written.bytes).expect("the saved PDF opens");
    assert!(!saved.was_recovered(), "the saved xref remains usable");
    let after = Resource::read(&saved)
        .expect("the saved form still states XFA")
        .expect("the saved XFA resource reads");
    // Positive control first: both representations of an independently edited text field must
    // agree before the target field's requirement can fail.
    let (control_name, _, control_value) = &edits[0];
    assert_eq!(
        pdf_text_value(&saved, control_name).as_deref(),
        Some(*control_value),
        "positive control PDF /V"
    );
    assert_eq!(
        after
            .datasets
            .value(&after.template, control_name)
            .as_deref(),
        Some(*control_value),
        "positive control XFA value"
    );

    let (name, _, expected) = &edits[1];
    assert_eq!(
        pdf_text_value(&saved, name).as_deref(),
        Some(*expected),
        "PDF /V"
    );
    assert_eq!(
        after.datasets.value(&after.template, name).as_deref(),
        Some(*expected),
        "Annex K.2 requires the XFA value to agree with its corresponding /V"
    );
}
