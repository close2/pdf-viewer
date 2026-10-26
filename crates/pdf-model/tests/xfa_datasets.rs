//! §K.2's first writer rule over the corpora's four hybrid forms: filled, saved, read back.
//!
//! > The XFA field values shall be consistent with the corresponding V entries of the PDF field
//! > objects.
//!
//! RFC 0011 section 4's census found four documents in `doc/pdf.js` and `doc/corpora` that carry
//! an XFA resource and do not ask to be regenerated; each is filled here in the fields its
//! template's kinds allow — text, check button, exclusion group, single and multiple choice,
//! a globally bound choice — saved by §7.5.6's update, re-opened, and its `datasets` packet read
//! again. Three things are asserted for each (ADR 2396):
//!
//! 1. the save names no field it could not keep (`Written::xfa_unkept` is empty);
//! 2. every field filled reads back from the data with the value its `/V` was written with,
//!    turned into XFA's terms by the clause the comment beside it names;
//! 3. every field **not** filled reads back from the data as it did before the save — the
//!    producer's values are kept, not re-derived.
//!
//! The documents are pdf.js's test files and are read from its checkout; where it is absent the
//! test says so and passes, as every pdf.js-corpus test in this crate does.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::path::Path;

use pdf_model::view::{Entered, ViewState};
use pdf_model::xfa::Resource;
use pdf_syntax::{Document, Object};

/// One of pdf.js's test documents, or `None` when the checkout is absent.
fn open(name: &str) -> Option<(Vec<u8>, Document)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes.clone()).expect("a corpus document opens");
    Some((bytes, document))
}

/// A field's `/V`, inherited up §12.7.4.1's chain from its first widget, as text.
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

/// Fills `fills`, saves, re-opens, and checks the three assertions; `expected` is each filled
/// field's data value after the save.
fn round_trip(file: &str, fills: &[(&str, Entered, &str)]) {
    let Some((original, document)) = open(file) else {
        eprintln!("{file}: the pdf.js checkout is absent; nothing judged");
        return;
    };
    let before = Resource::read(&document)
        .expect("a hybrid states an XFA resource")
        .expect("its resource reads");
    let names: Vec<String> = pdf_model::view::widgets_by_field_name(&document)
        .into_keys()
        .collect();
    let mut view = ViewState::of(&document);
    for (name, value, _) in fills {
        assert!(
            view.set_field(&document, name, value) > 0,
            "{file}: {name} took no value"
        );
    }
    let written = view.save(&document).expect("the hybrid saves");
    assert!(
        written.xfa_unkept.is_empty(),
        "{file}: {:?}",
        written.xfa_unkept
    );
    assert!(
        written.bytes.starts_with(&original),
        "{file}: §7.5.6 appends"
    );
    if let Ok(out) = std::env::var("XFA_DATASETS_OUT") {
        let _ = std::fs::write(Path::new(&out).join(file), &written.bytes);
    }
    let saved = Document::open(written.bytes).expect("the save opens");
    let after = Resource::read(&saved)
        .expect("the resource is still stated")
        .expect("the written packet reads");
    for (name, _, expected) in fills {
        assert_eq!(
            after.datasets.value(&after.template, name).as_deref(),
            Some(*expected),
            "{file}: {name}'s data value, beside the /V {:?}",
            stated_value(&saved, name)
        );
    }
    for name in names
        .iter()
        .filter(|name| !fills.iter().any(|(f, _, _)| f == name))
    {
        assert_eq!(
            after.datasets.value(&after.template, name),
            before.datasets.value(&before.template, name),
            "{file}: {name} was not filled and its data moved"
        );
    }
}

/// Text fields: the data value is the text (XFA 3.3's `textEdit`), a carriage return — a line
/// break in a `/V` by §12.7.4.3 — and markup characters included.
#[test]
fn annotation_text_widget_keeps_its_text_values() {
    round_trip(
        "annotation-text-widget.pdf",
        &[
            (
                "formulier1[0].#subform[0].Tekstveld2[0]",
                Entered::Text("Fish & <chips>".to_owned()),
                "Fish & <chips>",
            ),
            (
                "formulier1[0].#subform[0].Tekstveld7[0]",
                Entered::Text("one\rtwo".to_owned()),
                "one\rtwo",
            ),
            (
                "formulier1[0].#subform[0].Tekstveld1[0]",
                Entered::Cleared,
                "",
            ),
        ],
    );
}

/// Check buttons take their template `items`' on and off values (`1`, `0`); a radio set — a
/// named `exclGroup` — takes the on value of the button that is on, which Table 230's `/Opt`
/// states as the export value of the widget whose state is on: the second widget of the second
/// group has state `1`, `/Opt [(1) (2)]` makes its export value `2`, and the template's second
/// button's on item is `2`.
#[test]
fn annotation_button_widget_keeps_check_and_radio_values() {
    round_trip(
        "annotation-button-widget.pdf",
        &[
            (
                "formulier1[0].#subform[0].Selectievakje1[0]",
                Entered::Text("1".to_owned()),
                "1",
            ),
            (
                "formulier1[0].#subform[0].Selectievakje3[0]",
                Entered::Text("Off".to_owned()),
                "0",
            ),
            (
                "formulier1[0].#subform[0].LijstKeuzerondje[1]",
                Entered::Text("1".to_owned()),
                "2",
            ),
            (
                "formulier1[0].#subform[0].LijstKeuzerondje[0]",
                Entered::Text("0".to_owned()),
                "1",
            ),
        ],
    );
}

/// A choice's data value is its export value; a multiple-selection list's data is a group of
/// `value` elements, which [`pdf_model::xfa::Datasets::value`] joins by line feeds (XFA 3.3
/// page 198).
#[test]
fn annotation_choice_widget_keeps_single_and_multiple_choices() {
    round_trip(
        "annotation-choice-widget.pdf",
        &[
            (
                "formulier1[0].#subform[0].Vervolgkeuzelijst1[0]",
                Entered::Chosen(vec![4]),
                "Amet",
            ),
            (
                "formulier1[0].#subform[0].Keuzelijst2[0]",
                Entered::Chosen(vec![0, 2, 7]),
                "Lorem\nDolor\nElit",
            ),
            (
                "formulier1[0].#subform[0].Keuzelijst1[0]",
                Entered::Cleared,
                "",
            ),
        ],
    );
}

/// A subform stating `bind match="none"` is passed through, so its fields' data sits under the
/// record — where the producer's packet already put it — and a `match="global"` choice binds the
/// value of its name in the record first (XFA 3.3 chapter 6, pages 246–247).
#[test]
fn bug1815476_keeps_text_and_globally_bound_choices() {
    round_trip(
        "bug1815476.pdf",
        &[
            (
                "topmostSubform[0].Page1[0].EMAIL[0]",
                Entered::Text("someone@example.org".to_owned()),
                "someone@example.org",
            ),
            (
                "topmostSubform[0].Page1[0].Nivel[0]",
                Entered::Chosen(vec![3]),
                "B2",
            ),
            (
                "topmostSubform[0].Page1[0].Seccion[0]",
                Entered::Chosen(vec![2]),
                "Béjar",
            ),
        ],
    );
}

/// A save that changes no field leaves the packet alone: the update holds no replacement for it.
#[test]
fn a_save_that_changes_no_field_writes_no_packet() {
    let Some((_, document)) = open("annotation-text-widget.pdf") else {
        return;
    };
    let resource = Resource::read(&document).expect("stated").expect("reads");
    let written = ViewState::of(&document).save(&document).expect("saves");
    let saved = Document::open(written.bytes).expect("opens");
    assert_eq!(
        saved.get(resource.datasets_stream),
        document.get(resource.datasets_stream)
    );
}
