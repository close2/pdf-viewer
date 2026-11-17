//! Public `Command::Report` witness for EC3 §12.8.3.4.8's historical OCSP `producedAt` condition.
//!
//! Both inputs carry the same signed PDF revision, `CAdES` signer, RFC 3161 signature timestamp,
//! certificate paths, and TSA good CRL. Only the OCSP stream in the later DSS revision changes.
//! Each response says GOOD and has `thisUpdate < trusted genTime < nextUpdate`. The response in
//! `EARLY` was produced before that trusted time; the response in `LATE` was produced after it.
//! The document signer has its own supplied CA anchor, so the TSA's root-issued CRL cannot mask
//! whether the document signer's OCSP response is accepted.
#![expect(
    clippy::expect_used,
    reason = "offline valid fixture and fixed-clock invariants"
)]

use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);
const EARLY: &[u8] = include_bytes!("fixtures/historical_ocsp_report/early.pdf");
const LATE: &[u8] = include_bytes!("fixtures/historical_ocsp_report/late.pdf");
const TSA_ROOT: &[u8] = include_bytes!("fixtures/historical_ocsp_report/root.der");
const DOCUMENT_CA: &[u8] = include_bytes!("fixtures/historical_ocsp_report/doc-ca.der");
const HOST_TIME: &str = include_str!("fixtures/historical_ocsp_report/host-time-unix.txt");
const TRUSTED_TIME: &str = include_str!("fixtures/historical_ocsp_report/trusted-time-unix.txt");

fn report(bytes: &[u8]) -> String {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("incremental-xref PDF");
    assert!(
        !document.was_recovered(),
        "each appended DSS revision resolves by xref"
    );

    let at = HOST_TIME
        .trim()
        .parse::<i64>()
        .expect("fixed host Unix time");
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
                vec![
                    ("offline timestamp root.der".to_owned(), TSA_ROOT.to_vec()),
                    ("offline document CA.der".to_owned(), DOCUMENT_CA.to_vec()),
                ],
                "offline historical OCSP fixture".to_owned(),
                pdf_signature::x509::Instant::from_unix_seconds(at),
            ),
            acceptance: pdf_signature::verdict::Acceptance::RevocationMustBeGood,
            lists: pdf_signature::trusted_list::Lists::none(),
        }))
        .for_each(drop);
    let notes = viewer
        .handle(Command::Report)
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");

    // Same cryptographic/control assertions apply to both cases: the signature bytes and their
    // signer binding authenticate, and the RFC 3161 token's imprint covers the SignerInfo bytes.
    assert!(
        notes
            .contains("the bytes that signature covers still hash to the SHA256 digest it records"),
        "the actual signature ByteRange digest must verify:\n{notes}"
    );
    assert!(
        notes.contains("signature verifies under the 2048-bit RSA (PKCS #1 v1.5) key"),
        "the CMS signature must verify independently of its revocation answer:\n{notes}"
    );
    assert!(
        notes.contains(
            "signing-certificate-v2 attribute names the certificate it was checked under"
        ),
        "the signer certificate binding must pass:\n{notes}"
    );
    assert!(
        notes.contains("imprint inside it is the digest of this signature's own bytes"),
        "the timestamp must bind the actual CMS signature value:\n{notes}"
    );
    let trusted = TRUSTED_TIME.trim();
    assert!(
        notes.contains(&format!("as of {trusted} seconds after the epoch")),
        "the report must use the trusted RFC 3161 time for historical validation:\n{notes}"
    );
    notes
}

#[test]
fn an_ocsp_response_produced_after_the_trusted_past_time_can_support_the_path() {
    let notes = report(LATE);
    assert!(
        notes.contains("that signature is locally valid:"),
        "late-produced GOOD OCSP with a window bracketing genTime is the positive control:\n{notes}"
    );
}

#[test]
fn an_ocsp_response_produced_before_the_trusted_past_time_cannot_support_the_path() {
    let notes = report(EARLY);
    assert!(
        !notes.contains("that signature is locally valid:"),
        "§12.8.3.4.8 requires OCSP producedAt after the trusted past time; this response is earlier, while its thisUpdate/nextUpdate window still brackets that time:\n{notes}"
    );
}
