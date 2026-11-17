//! Genuine offline `PAdES` positive and same-range tamper controls for public Report.
//! Fixed trust time avoids wall-clock dependency; unknown revocation is explicitly accepted.
#![expect(clippy::expect_used, reason = "exact PDF fixture invariants")]
use viewer_core::{Command, DocumentId, Event, Viewer};
const DOCUMENT: DocumentId = DocumentId(1);

fn said_about_the_document(viewer: &mut Viewer) -> Vec<String> {
    viewer
        .handle(Command::Report)
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect()
}

const PADES_VALID: &[u8] = include_bytes!("fixtures/pades_audit/pades-valid.pdf");
const PADES_TAMPERED: &[u8] = include_bytes!("fixtures/pades_audit/pades-tampered.pdf");
const PADES_ANCHOR: &[u8] = include_bytes!("fixtures/pades_audit/pades-test-cert.der");

fn pades_report(bytes: &[u8]) -> Vec<String> {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("exact signed PDF");
    assert!(!document.was_recovered());
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.to_vec().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    // This host policy makes the test exercise actual RFC 5280 path validation. The fixture has
    // no revocation material, so acceptance remains explicit and must not be read as "Good".
    viewer
        .handle(Command::Trust(viewer_core::TrustPolicy {
            anchors: pdf_signature::trust::Supply::of(
                vec![("pades-test-cert.der".to_owned(), PADES_ANCHOR.to_vec())],
                "the PAdES test fixture".to_owned(),
                pdf_signature::x509::Instant::from_unix_seconds(1_800_000_000),
            ),
            acceptance: pdf_signature::verdict::Acceptance::UnknownRevocationAccepted,
            lists: pdf_signature::trusted_list::Lists::none(),
        }))
        .for_each(drop);
    said_about_the_document(&mut viewer)
}

#[test]
fn pades_report_checks_certificate_binding_signature_and_document_integrity() {
    let notes = pades_report(PADES_VALID).join("\n");
    assert!(
        notes.contains("this document is signed by Quorra PAdES Fixture Signer"),
        "{notes}"
    );
    assert!(
        notes.contains(
            "signing-certificate-v2 attribute names the certificate it was checked under"
        ),
        "{notes}"
    );
    assert!(
        notes.contains("signature verifies under the 2048-bit RSA (PKCS #1 v1.5) key"),
        "{notes}"
    );
    assert!(notes.contains("signature is locally valid"), "{notes}");
    assert!(
        notes.contains("accept a signature whose revocation status it could not determine"),
        "{notes}"
    );
    assert!(
        notes.contains("the document has not changed since it was signed"),
        "{notes}"
    );
}

#[test]
fn pades_same_range_tamper_withholds_validity_and_reports_changed_signed_bytes() {
    let notes = pades_report(PADES_TAMPERED).join("\n");
    assert!(
        notes.contains(
            "the bytes that signature covers no longer hash to the SHA256 digest it records"
        ),
        "{notes}"
    );
    assert!(
        notes.contains("modified after it was signed (§12.8.1)"),
        "{notes}"
    );
    assert!(
        notes.contains("document under it is not the document it was made over"),
        "{notes}"
    );
    assert!(
        !notes.contains("that signature is locally valid:"),
        "{notes}"
    );
}
