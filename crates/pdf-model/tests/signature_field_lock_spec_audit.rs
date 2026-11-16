//! §12.7.5.5 lock only applies after a real signature was placed in the field.
//!
//! Exact-xref PDF 2.0 fixtures with coherent checkbox appearance states.
//! The generator and README beside the fixtures document independent CMS verification.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixed fixture cardinality and complete PDF inputs"
)]

use pdf_model::restriction::{Operation, Restriction, asserted};
use pdf_signature::cms::Digest;
use pdf_signature::signature::{FieldSelection, Integrity, signatures};
use pdf_syntax::{Document, FileBytes};

fn open(bytes: &[u8]) -> Document {
    let document = Document::open(FileBytes::from(bytes.to_vec())).expect("valid fixture PDF");
    assert!(
        !document.was_recovered(),
        "the fixture's exact xref is used"
    );
    document
}

#[test]
fn only_the_validly_signed_field_lock_restricts_its_named_field() {
    let signed_bytes = include_bytes!("fixtures/signed-field-lock/signed-lock.pdf");
    let signed = open(signed_bytes);
    let found = signatures(&signed);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].integrity(signed.bytes()),
        Integrity::Unchanged {
            digest: Digest::Sha256
        },
        "the public API recomputes the ByteRange digest; OpenSSL independently verifies CMS over the extracted bytes"
    );
    assert_eq!(
        pdf_signature::signature::field_locks(&signed),
        [FieldSelection::Include(vec!["name".to_owned()])]
    );
    assert_eq!(
        asserted(&signed, Operation::FillInForm, Some("name"), None),
        [Restriction::FieldLocked]
    );
    assert!(
        asserted(&signed, Operation::FillInForm, Some("other"), None).is_empty(),
        "the second text field is outside the lock's Include list"
    );

    let unsigned = open(include_bytes!(
        "fixtures/signed-field-lock/unsigned-lock.pdf"
    ));
    assert!(
        signatures(&unsigned).is_empty(),
        "no field value means unsigned"
    );
    assert!(pdf_signature::signature::field_locks(&unsigned).is_empty());
    assert!(
        asserted(&unsigned, Operation::FillInForm, Some("name"), None).is_empty(),
        "the same /Lock on an unsigned field is only a future signing instruction"
    );
}
