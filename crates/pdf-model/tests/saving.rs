//! What ISO 32000-2 §7.5.6's update writes for a field a person changed.
//!
//! `viewer-core`'s `a_saved_document_carries_the_edit_and_the_file_under_it` is the end-to-end
//! statement — a keystroke goes in and a second viewer reads the value out of the saved bytes.
//! What is here is the half that end cannot see: **which objects the update contains**, and in
//! particular the two ways §12.7.4.3's appearance stream can be got wrong without changing what
//! this program itself draws.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use pdf_model::view::ViewState;
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Object};

/// One corpus document, or `None` when the submodule is not checked out.
fn corpus(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// The document `160F-2019.pdf` becomes when `field` is set to `value`.
fn saved(name: &str, field: &str, value: &str) -> Option<Document> {
    let bytes = corpus(name)?;
    let document = Document::open(bytes).expect("the fixture opens");
    let mut view = ViewState::of(&document);
    let applied = view.set_field(&document, field, Some(value));
    assert!(applied > 0, "{name}: {field} is a field this document has");
    let out = view.save(&document).expect("the fixture can be written");
    Some(Document::open(out).expect("what was written can be read"))
}

/// The object `key` names in `dict`, resolved.
fn entry(document: &Document, id: ObjectId, key: &str) -> Object {
    let object = document.get(id);
    let dict = object.as_dict().expect("the object is a dictionary");
    document.get_key(dict, key)
}

#[test]
fn a_text_widget_with_no_appearance_stream_is_given_one() {
    // §7.3.8.1 makes every stream an indirect object, so a widget that had no `/AP` needs an
    // object *added* rather than replaced — which is the half of §7.5.6's "changed, replaced, or
    // deleted" this writer did not do until the hundred-and-forty-fifth session.
    let Some(document) = saved("160F-2019.pdf", "X.minus1", "Ada Lovelace") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };

    // Object 417 is a text widget whose original file states no `/AP` at all.
    let widget = ObjectId {
        number: 417,
        generation: 0,
    };
    let appearances = entry(&document, widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| document.get_key(appearances, "N"))
        .expect("the widget now has an /AP");
    let stream = normal.as_stream().expect("its /N is a stream");

    // §8.10.2's required entries, and §12.7.4.3's marked-content region inside them.
    assert_eq!(
        document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Form".to_vec())
    );
    assert!(document.get_key(&stream.dict, "BBox").as_array().is_some());
    let content = document
        .decoded_stream_data(stream)
        .expect("the written stream decodes");
    let content = String::from_utf8_lossy(&content);
    assert!(content.contains("/Tx BMC"), "{content}");
    assert!(content.contains("Ada Lovelace"), "{content}");
}

#[test]
fn a_check_boxs_states_are_not_replaced_by_one_stream() {
    // §12.7.5.2.3 makes a check box's states "defined by an appearance stream in the appearance
    // dictionary of the field's widget annotation", and the value *selects* among them. A writer
    // that regenerated every widget's appearance the way it regenerates a text field's would
    // replace that subdictionary with a single stream and lose the off state — a page that still
    // renders, in this program and in every other, until somebody clicks the box.
    //
    // The check box is the field *being changed*, which is what makes this discriminating: a
    // save only touches the widgets a person edited, so setting some other field would leave
    // this one alone however wrong the writer was.
    // `bug1675139.pdf`'s `C1` is the fixture rather than any check box, because it is one whose
    // `/MK` states a background *and* a border: a check box that states neither draws nothing at
    // all, so a writer without the guard would produce no stream for it and the test would pass
    // for the wrong reason.
    let Some(document) = saved("bug1675139.pdf", "C1", "On") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };

    // Object 50 is that check box, and its `/AP` `/N` is a dictionary of states.
    let widget = ObjectId {
        number: 50,
        generation: 0,
    };
    let appearances = entry(&document, widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| document.get_key(appearances, "N"))
        .expect("the check box still has an /AP");
    // `matches!` rather than `as_dict()`, and the difference is the whole test: `Object::as_dict`
    // answers for a *stream* too, so a `/N` replaced by one passes an `is_some()` check on it.
    // Confirmed by removing the guard in `appearance::for_saving` — the states really are
    // replaced, and the first version of this assertion did not notice.
    assert!(
        matches!(normal, Object::Dictionary(_)),
        "the check box's states survive: {normal:?}"
    );
}

#[test]
fn nothing_is_owed_to_the_next_reader_when_every_stream_was_written() {
    // Table 224's `/NeedAppearances` asks the next reader to construct what this program could
    // not. A document where it *could* must not set it — a reader that honours the flag would
    // redo work that is already correct, and one that does not would be right to ignore it.
    let Some(document) = saved("form_two_pages.pdf", "Text1", "Ada Lovelace") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let catalog = document.catalog().expect("a /Root");
    let form = document.get_key(&catalog, "AcroForm");
    let flag = form
        .as_dict()
        .map(|form| document.get_key(form, "NeedAppearances"));
    assert!(
        !matches!(flag, Some(Object::Boolean(true))),
        "{flag:?} — every changed widget got a stream, so nothing is owed"
    );
}
