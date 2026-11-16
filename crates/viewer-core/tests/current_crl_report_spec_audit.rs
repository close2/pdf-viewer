//! Public Report-path controls for §12.8.3.4.7's current-time CRL window.
//!
//! Both PDFs have valid exact-xref syntax, identical `CAdES` signing structure, the same valid
//! signer chain, the same host-supplied root anchor and the same fixed host time. Their only
//! semantic difference is the signed DSS CRL's `thisUpdate`: one minute before versus one minute
//! after the time at which the signature without a trusted timestamp is checked. The second report must not call the
//! signature locally valid while its CRL is not yet current.
#![expect(
    clippy::expect_used,
    reason = "exact signed fixture and fixed-time invariants"
)]

use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);
const CURRENT: &[u8] = include_bytes!("fixtures/current_crl_report/current.pdf");
const FUTURE_THIS_UPDATE: &[u8] = include_bytes!("fixtures/current_crl_report/future.pdf");
const ROOT: &[u8] = include_bytes!("fixtures/current_crl_report/root.der");
const HOST_TIME: &str = include_str!("fixtures/current_crl_report/host-time-unix.txt");

fn report(bytes: &[u8]) -> Vec<String> {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("exact signed PDF");
    assert!(
        !document.was_recovered(),
        "fixture must resolve through its xref"
    );

    let at = HOST_TIME
        .trim()
        .parse::<i64>()
        .expect("generated fixed host time is a Unix instant");
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.to_vec().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Trust(viewer_core::TrustPolicy {
            anchors: pdf_signature::trust::Supply::of(
                vec![("offline-crl-test-root.der".to_owned(), ROOT.to_vec())],
                "offline current-CRL test fixture".to_owned(),
                pdf_signature::x509::Instant::from_unix_seconds(at),
            ),
            acceptance: pdf_signature::verdict::Acceptance::RevocationMustBeGood,
            lists: pdf_signature::trusted_list::Lists::none(),
        }))
        .for_each(drop);

    let notes: Vec<String> = viewer
        .handle(Command::Report)
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        notes.iter().any(|note| note.contains(
            "the bytes that signature covers still hash to the SHA256 digest it records"
        )),
        "the public report must authenticate the actual PDF ByteRange: {notes:?}"
    );
    notes
}

#[test]
fn signed_crl_current_at_host_time_can_support_local_validity() {
    let notes = report(CURRENT).join("\n");
    assert!(
        notes.contains("signature verifies under the 2048-bit RSA (PKCS #1 v1.5) key"),
        "the control must authenticate the CMS signature, not merely take a report branch:\n{notes}"
    );
    assert!(
        notes.contains(
            "signing-certificate-v2 attribute names the certificate it was checked under"
        ),
        "the control must bind the CMS signer to the certificate whose path is checked:\n{notes}"
    );
    assert!(
        notes.contains("signature is locally valid"),
        "the in-window CRL is the independent positive control; report was:\n{notes}"
    );
    let at = HOST_TIME.trim();
    assert!(
        notes.contains(&format!("as of {at} seconds after the epoch")),
        "the report must identify the generated fixed host time used for this signature without a trusted timestamp:\n{notes}"
    );
}

#[test]
fn signed_crl_not_yet_current_at_host_time_cannot_support_local_validity() {
    let notes = report(FUTURE_THIS_UPDATE).join("\n");
    assert!(
        notes.contains("signature verifies under the 2048-bit RSA (PKCS #1 v1.5) key"),
        "the negative control must still have a valid cryptographic signature:\n{notes}"
    );
    assert!(
        notes.contains(
            "signing-certificate-v2 attribute names the certificate it was checked under"
        ),
        "the negative control must bind the signer to the same certificate:\n{notes}"
    );
    assert!(
        !notes.contains("that signature is locally valid:")
            && notes.contains(
                "the revocation material covering that certificate was not current at the instant asked about"
            ),
        "a CRL whose thisUpdate is after the host validation time is outside the current-time window; report was:\n{notes}"
    );
}
