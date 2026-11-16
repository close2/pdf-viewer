//! §12.8.2.3 recommends withdrawing a UR signature after an edit exceeds its rights.
//!
//! This exercises the product's adopted withdrawal policy, not a mandatory ISO SHALL.
//! Real signed PDF 1.7 fixtures grant `FullSave` and `FillIn` in both cases; only one
//! grants annotation creation. Test-only certificates establish cryptographic
//! self-consistency, not public trust or an external entitlement to additional rights.

#![expect(
    clippy::expect_used,
    reason = "required fixture and saved-state observations"
)]

use pdf_model::view::{Markup, ViewState};
use pdf_signature::cms::Digest;
use pdf_signature::signature::{Authenticity, Integrity, permissions};
use pdf_syntax::Document;

fn add_and_save(bytes: &[u8]) -> Document {
    let document = Document::open(bytes.to_vec()).expect("signed fixture opens");
    assert!(!document.was_recovered());
    let signature = permissions(&document)
        .usage_rights_signature
        .expect("UR signature identified through catalog permissions");
    assert_eq!(
        signature.integrity(document.bytes()),
        Integrity::Unchanged {
            digest: Digest::Sha256
        }
    );
    assert!(matches!(
        signature.authenticity(document.bytes()),
        Authenticity::Verified { .. }
    ));
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one page")
        .id
        .expect("indirect page");
    let mut view = ViewState::of(&document);
    let annotation = view
        .add_markup(
            &document,
            page,
            Markup::Highlight,
            [1.0, 1.0, 0.0],
            &[[10.0, 30.0, 40.0, 30.0, 10.0, 20.0, 40.0, 20.0]],
        )
        .expect("nonempty annotation geometry");
    let saved = view.save(&document).expect("incremental save").bytes;
    assert!(
        saved.starts_with(bytes),
        "signed original bytes remain untouched"
    );
    let reopened = Document::open(saved).expect("saved PDF opens");
    assert!(!reopened.was_recovered());
    let marks = reopened.get(annotation);
    let marks = marks.as_dict().expect("new annotation is written");
    assert_eq!(
        reopened
            .get_key(marks, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(b"Highlight".as_slice())
    );
    let page = reopened.get(page);
    let page = page.as_dict().expect("page remains");
    let annotations = reopened.get_key(page, "Annots");
    assert!(
        annotations
            .as_array()
            .expect("annotations array")
            .iter()
            .any(|item| item.as_reference() == Some(annotation)),
        "saved annotation is attached, not an unused object"
    );
    reopened
}

#[test]
fn annotation_save_withdraws_usage_rights_when_creation_is_not_granted() {
    let granted = add_and_save(include_bytes!(
        "fixtures/usage-rights-annotations/annotation-granted.pdf"
    ));
    assert!(
        permissions(&granted).usage_rights_signature.is_some(),
        "positive control: this edit is explicitly granted"
    );
    let withheld = add_and_save(include_bytes!(
        "fixtures/usage-rights-annotations/annotation-withheld.pdf"
    ));
    assert!(
        permissions(&withheld).usage_rights_signature.is_none(),
        "annotation creation is not granted; unrelated form-fill and full-save rights must not preserve the UR grant"
    );
}
