//! ISO 32000-2 §12.7.5.5, Table 236: a signature field's lock dictionary, read from genuinely
//! signed witnesses for each of the shapes the include-list witness beside it does not carry —
//! `/Action /All`, `/Action /Exclude`, and `/P`.
//!
//! Each witness is the document `fixtures/signed-field-lock/README.md` describes, signed with a
//! detached `CAdES` signature over its `/ByteRange`; the signature's integrity is asserted first,
//! so a lock read here is one a signature field holding a real signature states. Table 236 is
//! where each expectation comes from: `All` locks every field of the form, `Exclude` every field
//! but those its `/Fields` names, and `/P` is "\[t]he access permissions granted for this
//! document", whose value 1 permits no changes to the document.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixed fixture cardinality and complete PDF inputs"
)]

use pdf_model::restriction::{Operation, Restriction, asserted};
use pdf_signature::cms::Digest;
use pdf_signature::signature::{
    FieldSelection, Integrity, Modification, field_lock_permissions, field_locks, signatures,
};
use pdf_syntax::{Document, FileBytes};

fn signed(bytes: &[u8]) -> Document {
    let document = Document::open(FileBytes::from(bytes.to_vec())).expect("valid fixture PDF");
    assert!(
        !document.was_recovered(),
        "the fixture's exact cross-reference table is used"
    );
    let found = signatures(&document);
    assert_eq!(found.len(), 1, "one signature field holds a signature");
    assert_eq!(
        found[0].integrity(document.bytes()),
        Integrity::Unchanged {
            digest: Digest::Sha256
        },
        "the signed byte ranges are what the signature covers"
    );
    document
}

fn locked(document: &Document, field: &str) -> bool {
    asserted(document, Operation::FillInForm, Some(field), None).contains(&Restriction::FieldLocked)
}

#[test]
fn a_signed_all_lock_locks_every_field() {
    let document = signed(include_bytes!(
        "fixtures/signed-field-lock/signed-lock-all.pdf"
    ));
    assert_eq!(field_locks(&document), [FieldSelection::All]);
    assert!(locked(&document, "name"));
    assert!(locked(&document, "other"));
    assert_eq!(field_lock_permissions(&document), None, "no /P is stated");
}

#[test]
fn a_signed_exclude_lock_locks_every_field_but_the_named_one() {
    let document = signed(include_bytes!(
        "fixtures/signed-field-lock/signed-lock-exclude.pdf"
    ));
    assert_eq!(
        field_locks(&document),
        [FieldSelection::Exclude(vec!["name".to_owned()])]
    );
    assert!(!locked(&document, "name"), "the excluded field stays open");
    assert!(locked(&document, "other"), "every other field is locked");
}

#[test]
fn a_signed_lock_permission_of_one_permits_no_fill_anywhere() {
    let document = signed(include_bytes!(
        "fixtures/signed-field-lock/signed-lock-p.pdf"
    ));
    assert_eq!(
        field_lock_permissions(&document),
        Some(Modification::None),
        "Table 236 /P 1"
    );
    // The include list still locks its field, and /P reaches the field outside it too.
    assert!(locked(&document, "name"));
    let outside = asserted(&document, Operation::FillInForm, Some("other"), None);
    assert!(
        outside.contains(&Restriction::LockPermission {
            level: Modification::None
        }),
        "the field outside the list is still under /P 1: {outside:?}"
    );
    assert!(!outside.contains(&Restriction::FieldLocked));
}
