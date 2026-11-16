//! ISO §12.8.2.2/Table257: a genuine timestamp-only update is not a document change.
//! Fixtures use test-only certificates; crypto checks establish integrity/key authenticity,
//! not revocation status or trust in a public authority.
#![expect(clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use pdf_signature::revision::{Comparison, Judgement};
use pdf_signature::signature::{Authenticity, Integrity, Modification, permissions, signatures};
use pdf_syntax::Document;

fn open_fixture(name: &str) -> Document {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/docmdp_dts")
        .join(name);
    let bytes = std::fs::read(path).expect("checked-in fixture is readable");
    let document = Document::open(bytes).expect("valid exact-xref PDF");
    assert!(
        !document.was_recovered(),
        "the fixture uses its stated xref tables"
    );
    document
}

fn assert_signature_crypto(document: &Document) {
    let found = signatures(document);
    assert_eq!(
        found.len(),
        2,
        "the certification and registered DTS are present"
    );
    assert!(matches!(
        found[0].integrity(document.bytes()),
        Integrity::Unchanged { .. }
    ));
    assert!(matches!(
        found[0].authenticity(document.bytes()),
        Authenticity::Verified { .. }
    ));
    assert!(matches!(
        found[1].integrity(document.bytes()),
        Integrity::Unchanged { .. }
    ));
    assert!(matches!(
        found[1].authenticity(document.bytes()),
        Authenticity::Verified { .. }
    ));
}

#[test]
fn valid_docmdp_p1_and_dts_only_revision_are_within_the_carveout() {
    let document = open_fixture("dts-only.pdf");
    assert_signature_crypto(&document);
    assert_eq!(
        permissions(&document).doc_mdp,
        Some(Modification::None),
        "the certification signature's /P 1 is parsed"
    );

    let found = signatures(&document);
    let comparison = Comparison::of(&found[0], &document)
        .expect("the full, unrecovered signed revision is comparable");
    let judgement = comparison.against(Modification::None);
    assert!(
        matches!(
            judgement,
            Judgement::WithinWhatIsPermitted {
                level: Modification::None,
                objects: 0,
                ..
            }
        ),
        "DTS-only update: {judgement:?}"
    );
}

#[test]
fn valid_dts_does_not_exempt_an_unrelated_page_change_in_the_same_revision() {
    let document = open_fixture("dts-plus-page-change.pdf");
    assert_signature_crypto(&document);
    assert_eq!(
        permissions(&document).doc_mdp,
        Some(Modification::None),
        "the same /P 1 certification is present in the control"
    );

    let found = signatures(&document);
    let comparison = Comparison::of(&found[0], &document)
        .expect("the full, unrecovered signed revision is comparable");
    assert!(
        matches!(
            comparison.against(Modification::None),
            Judgement::NotPermitted { .. } | Judgement::NotClassified { .. }
        ),
        "the timestamp covers a same-length page MediaBox change as well as DTS registration"
    );
}
