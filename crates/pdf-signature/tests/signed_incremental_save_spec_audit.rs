//! Composition control for ISO 32000-2 §7.5.6 and §12.8.1 NOTE 1.
//! It checks the original signature's byte-range evidence only: no trust, revocation, or
//! `DocMDP` authorization verdict is asserted.
#![expect(clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::path::Path;

use pdf_signature::signature::{Authenticity, Coverage, Integrity, signatures};
use pdf_syntax::Document;
use pdf_syntax::object::{Name, Object};
use pdf_syntax::write::incremental_update;

fn base_signed() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/docmdp_dts/base-signed.pdf"),
    )
    .expect("checked-in test-signed input is readable")
}

#[test]
fn a_real_incremental_save_preserves_the_signed_revision_and_reports_its_tail() {
    let original_bytes = base_signed();
    let original = Document::open(original_bytes.clone()).expect("signed fixture opens");
    assert!(!original.was_recovered(), "fixture xref is authoritative");

    let before = signatures(&original);
    assert_eq!(
        before.len(),
        1,
        "the base fixture has one certification signature"
    );
    assert!(matches!(
        before[0].coverage(original_bytes.len() as u64),
        Coverage::WholeFile
    ));
    assert!(matches!(
        before[0].integrity(original.bytes()),
        Integrity::Unchanged { .. }
    ));
    assert!(matches!(
        before[0].authenticity(original.bytes()),
        Authenticity::Verified { .. }
    ));

    let Some(Object::Reference(root_id)) = original.trailer().get("Root").cloned() else {
        panic!("the signed fixture has an indirect catalog");
    };
    let mut catalog = original.catalog().expect("catalog dictionary");
    assert_ne!(
        catalog.get("Lang"),
        Some(&Object::String(b"en-US".to_vec().into())),
        "the update changes the catalog language"
    );
    catalog.insert(
        Name::new(&b"Lang"[..]),
        Object::String(b"en-US".to_vec().into()),
    );
    let replacements = BTreeMap::from([(root_id, Object::Dictionary(catalog))]);
    let updated_bytes = incremental_update(&original, &replacements)
        .expect("a supported append-only update is writable");

    assert!(
        updated_bytes.starts_with(&original_bytes),
        "§7.5.6 leaves the signed revision byte-for-byte at the front"
    );
    assert!(
        updated_bytes.len() > original_bytes.len(),
        "the update appends bytes"
    );

    let updated = Document::open(updated_bytes.clone()).expect("updated file opens from scratch");
    assert!(!updated.was_recovered(), "the appended xref chain is exact");
    assert_eq!(
        updated.catalog().expect("updated catalog").get("Lang"),
        Some(&Object::String(b"en-US".to_vec().into())),
        "the new revision is also visible"
    );

    let after = signatures(&updated);
    assert_eq!(after.len(), 1, "the update does not invent a new signature");
    assert!(matches!(
        after[0].integrity(updated.bytes()),
        Integrity::Unchanged { .. }
    ));
    assert!(matches!(
        after[0].authenticity(updated.bytes()),
        Authenticity::Verified { .. }
    ));
    assert!(matches!(
        after[0].coverage(updated_bytes.len() as u64),
        Coverage::Unsigned { tail } if tail > 0
    ));
}
