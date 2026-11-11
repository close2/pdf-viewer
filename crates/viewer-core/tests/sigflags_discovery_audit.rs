//! Public-consumer probe for the §12.7.3 `/SigFlags` one-way statement.
//!
//! This fixture has a complete classic cross-reference table and an actual signature field/value.
//! Its CMS payload is deliberately invalid; the observable under test is discovery/reporting of a
//! signature candidate, so a caller can tell the user that the value is invalid. It does not test
//! cryptographic validity or treat the absence of bit 1 as either conforming or nonconforming.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::format_push_string,
    reason = "bounded complete fixtures and explicit candidate-discovery assertions"
)]

use pdf_signature::signature::signatures;
use pdf_syntax::Document;

const GAP_BYTES: usize = 64;
const SLOT: &str = "0000000000";

fn candidate(sig_flags: Option<u8>, timestamp: bool) -> Vec<u8> {
    let acroform = match sig_flags {
        Some(flags) => format!("/SigFlags {flags} "),
        None => String::new(),
    };
    let (version, signature_type, sub_filter) = if timestamp {
        ("2.0", "/Type /DocTimeStamp", "/SubFilter /ETSI.RFC3161")
    } else {
        ("1.7", "/Type /Sig", "/SubFilter /adbe.pkcs7.detached")
    };
    let mut body = format!(
        "%PDF-{version}\n\
         1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] {acroform}>> >> endobj\n\
         2 0 obj << /Type /Pages /Count 1 /Kids [3 0 R] >> endobj\n\
         3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Annots [4 0 R] >> endobj\n\
         4 0 obj << /Type /Annot /Subtype /Widget /Rect [0 0 1 1] /F 4 /P 3 0 R /AP << /N 6 0 R >> /FT /Sig /T (Approval) /V 5 0 R >> endobj\n\
         5 0 obj << {signature_type} /Filter /Adobe.PPKLite {sub_filter}\n\
         /ByteRange [{SLOT} {SLOT} {SLOT} {SLOT}] /Contents <{}> >> endobj\n\
         6 0 obj << /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Resources << >> /Length 3 >> stream\nq Q\nendstream endobj\n",
        "00".repeat(GAP_BYTES)
    );
    let gap_start = body.find("/Contents <").expect("contents opener") + "/Contents ".len();
    let gap_end = gap_start + 2 + GAP_BYTES * 2;
    let xref_at = body.len();
    body.push_str("xref\n0 7\n0000000000 65535 f \n");
    for object in 1..=6 {
        let marker = format!("{object} 0 obj");
        let offset = body.find(&marker).expect("object offset");
        body.push_str(&format!("{offset:010} 00000 n \n"));
    }
    body.push_str(&format!(
        "trailer\n<< /Size 7 /Root 1 0 R /ID [<00000000000000000000000000000000> \
         <00000000000000000000000000000000>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    ));
    let file_len = body.len();
    let ranges = [0, gap_start, gap_end, file_len - gap_end];
    let range_text = ranges
        .iter()
        .map(|value| format!("{value:010}"))
        .collect::<Vec<_>>()
        .join(" ");
    let range_start = body.find("/ByteRange [").expect("range key") + "/ByteRange [".len();
    let range_end = range_start + SLOT.len() * 4 + 3;
    body.replace_range(range_start..range_end, &range_text);
    body.into_bytes()
}

fn assert_signature_candidate(sig_flags: Option<u8>) {
    let document =
        Document::open(candidate(sig_flags, false)).expect("complete signature candidate PDF");
    assert!(!document.was_recovered(), "exact xref");
    let found = signatures(&document);
    let [signature] = found.as_slice() else {
        panic!(
            "public signatures API promises every field candidate; got {} with {sig_flags:?}",
            found.len()
        );
    };
    assert_eq!(signature.sub_filter.as_deref(), Some("adbe.pkcs7.detached"));
    assert_eq!(signature.contents.len(), GAP_BYTES);
    assert_range(&document, signature);
}

#[test]
fn absent_sigflags_does_not_hide_a_candidate_from_the_public_signature_api() {
    assert_signature_candidate(Some(1));
    assert_signature_candidate(None);
}

#[test]
fn zero_sigflags_does_not_hide_a_candidate_from_the_public_signature_api() {
    assert_signature_candidate(Some(1));
    assert_signature_candidate(Some(0));
}

#[test]
fn document_timestamp_existence_scans_signature_fields_even_without_sigflags_bit_one() {
    // §12.8.5.2 directly says timestamp existence shall be determined by examining signature
    // fields. The bit-set case is a positive control; absent/zero bit cases must still be examined.
    let declared = Document::open(candidate(Some(1), true)).expect("complete timestamp control");
    assert!(!declared.was_recovered());
    let declared_signatures = signatures(&declared);
    let [timestamp] = declared_signatures.as_slice() else {
        panic!("the bit-set control exposes one signature-field candidate");
    };
    assert!(
        timestamp.timestamp,
        "/Type /DocTimeStamp identifies the candidate"
    );
    assert_eq!(timestamp.sub_filter.as_deref(), Some("ETSI.RFC3161"));
    assert_range(&declared, timestamp);

    assert_timestamp_is_found(&declared);

    let absent = Document::open(candidate(None, true)).expect("complete timestamp PDF");
    assert!(!absent.was_recovered());
    assert_timestamp_is_found(&absent);
}

#[test]
fn document_timestamp_existence_is_examined_when_sigflags_is_explicitly_zero() {
    let declared = Document::open(candidate(Some(1), true)).expect("complete timestamp control");
    assert!(!declared.was_recovered());
    assert_timestamp_is_found(&declared);

    let zero = Document::open(candidate(Some(0), true)).expect("complete timestamp PDF");
    assert!(!zero.was_recovered());
    assert_timestamp_is_found(&zero);
}

fn assert_timestamp_is_found(document: &Document) {
    let candidates = signatures(document);
    if let [candidate] = candidates.as_slice() {
        assert_range(document, candidate);
    }
    let chain = pdf_signature::timestamp::chain(
        document,
        &pdf_signature::trust::TrustAnchors::none(),
        &pdf_signature::revocation::Material::none(),
        pdf_signature::x509::Instant::from_unix_seconds(0),
    );
    assert_eq!(
        chain.links.len(),
        1,
        "the public timestamp chain finds the candidate"
    );
    let [link] = chain.links.as_slice() else {
        panic!("one timestamp link")
    };
    assert!(link.claim.is_err(), "the fake token is named as unreadable");

    let found = signatures(document);
    assert_eq!(
        found.len(),
        1,
        "§12.8.5.2 requires examining signature fields"
    );
    let [timestamp] = found.as_slice() else {
        panic!("one timestamp candidate")
    };
    assert!(
        timestamp.timestamp,
        "the detected candidate is a document timestamp"
    );
    assert_eq!(timestamp.sub_filter.as_deref(), Some("ETSI.RFC3161"));
}

fn assert_range(document: &Document, signature: &pdf_signature::signature::Signature) {
    use pdf_signature::signature::{Coverage, Excluded, SignedEnd};
    assert_eq!(
        signature.coverage(u64::try_from(document.bytes().len()).expect("small PDF")),
        Coverage::WholeFile
    );
    assert_eq!(
        signature.excluded(document.bytes()),
        Excluded::TheSignatureValue
    );
    assert_eq!(
        signature.signed_end(document.bytes()),
        SignedEnd::AtAnEndOfFileMarker
    );
}
