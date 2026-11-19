//! Review 2128's audit of ISO 32000-2 §12.8 (`doc/reviews/2128-specification-audit-12-8.md`):
//! one failing test per behaviour finding in `pdf-signature`, each expected value read from the
//! clause it names and none from this crate's own answers.
//!
//! The signed fixtures are `tests/fixtures/spec_audit_2128/`, made by that directory's
//! `generate.py` with a fresh test key; the structural ones are written below.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixed test documents whose layout this file writes or generated"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_signature::cms::Digest;
use pdf_signature::signature::{
    Authenticity, Coverage, Integrity, Right, Signature, permissions, signatures,
};
use pdf_syntax::Document;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/spec_audit_2128")
            .join(name),
    )
    .expect("the generated fixture is checked in")
}

fn only_signature(bytes: &[u8]) -> (Document, Signature) {
    let document = Document::open(bytes.to_vec()).expect("the fixture opens");
    assert!(!document.was_recovered(), "the fixture's xref is exact");
    let mut found = signatures(&document);
    assert_eq!(found.len(), 1, "one signature field");
    let signature = found.pop().expect("one signature");
    (document, signature)
}

/// Whether question 1 and question 2 together clear the way to `Verdict::reached`'s word, which
/// is what `verdict.rs` reads them for: an unchanged or under-the-key digest, and a verification.
fn vouches_for_the_document(integrity: Integrity, authenticity: &Authenticity) -> bool {
    matches!(
        integrity,
        Integrity::Unchanged { .. } | Integrity::UnderTheSignersKey
    ) && matches!(authenticity, Authenticity::Verified { .. })
}

/// The signed byte range's two parts, read off the file's own `/ByteRange`.
fn signed_parts(signature: &Signature) -> (usize, usize) {
    let [(_, before), (after, _)] = signature.byte_range.as_slice() else {
        unreachable!("the generator writes two pairs")
    };
    (
        usize::try_from(*before).expect("a small file"),
        usize::try_from(*after).expect("a small file"),
    )
}

/// 2128-B01. §12.8.3.3.1, `adbe.pkcs7.sha1`: "The SHA-1 digest of the document's byte range shall
/// be encapsulated in the CMS `SignedData` field with `ContentInfo` of type Data. The digest of that
/// `SignedData` shall be incorporated as the normal CMS digest." RFC 5652 section 5.4 then puts the
/// signature over the signed attributes, and section 11.2's `message-digest` is what binds the
/// encapsulated content to them. So a value whose encapsulated digest was replaced after signing
/// is not signed by anyone: question 1 and question 2 may not both clear it.
///
/// The forgery edits one signed byte of the document and writes the new SHA-1 into the
/// encapsulated content, leaving the signed attributes - and so the signature - untouched.
#[test]
fn an_encapsulated_document_digest_is_bound_by_the_message_digest_spec_audit() {
    let original = fixture("sha1-with-attributes.pdf");
    let (document, signature) = only_signature(&original);
    assert_eq!(signature.sub_filter.as_deref(), Some("adbe.pkcs7.sha1"));
    // Control: the generated value is the clause's shape and both answers clear it.
    assert!(vouches_for_the_document(
        signature.integrity(document.bytes()),
        &signature.authenticity(document.bytes())
    ));

    let (before, after) = signed_parts(&signature);
    let old = Digest::Sha1.compute(&[&original[..before], &original[after..]]);
    let mut forged = original.clone();
    let at = forged
        .windows(b"(the signed revision)".len())
        .position(|window| window == b"(the signed revision)")
        .expect("the payload the generator writes");
    forged[at..at.saturating_add(b"(the forged revision)".len())]
        .copy_from_slice(b"(the forged revision)");
    let new = Digest::Sha1.compute(&[&forged[..before], &forged[after..]]);
    assert_ne!(old, new, "the edit is inside the signed range");

    let hex = |bytes: &[u8]| {
        bytes.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
    };
    let (old_hex, new_hex) = (hex(&old), hex(&new));
    let value = &mut forged[before.saturating_add(1)..after.saturating_sub(1)];
    let found = value
        .windows(old_hex.len())
        .position(|window| window == old_hex.as_bytes())
        .expect("the encapsulated digest is in the value");
    value[found..found.saturating_add(new_hex.len())].copy_from_slice(new_hex.as_bytes());

    let (document, signature) = only_signature(&forged);
    let integrity = signature.integrity(document.bytes());
    let authenticity = signature.authenticity(document.bytes());
    assert!(
        !vouches_for_the_document(integrity, &authenticity),
        "a SHA-1 written into the encapsulated content after signing is bound by no signature \
         (§12.8.3.3.1; RFC 5652 sections 5.4 and 11.2): {integrity:?} / {authenticity:?}"
    );
}

/// 2128-B02. §12.8.3.3.1, `adbe.pkcs7.detached`: "The original signed message digest over the
/// document's byte range shall be incorporated as the normal CMS `SignedData` field. No data shall
/// be encapsulated in the CMS `SignedData` field." And §12.8.1: "The digest shall be recomputed and
/// compared with the one stored in the document." A value that encapsulates 17 unrelated bytes
/// and signs them, with no signed attributes, states no digest of this document at all, so it
/// may not be reported as vouching for the document's bytes.
#[test]
fn a_detached_value_that_signs_encapsulated_bytes_does_not_vouch_for_the_document_spec_audit() {
    let bytes = fixture("detached-encapsulating.pdf");
    let (document, signature) = only_signature(&bytes);
    assert_eq!(signature.sub_filter.as_deref(), Some("adbe.pkcs7.detached"));
    let integrity = signature.integrity(document.bytes());
    let authenticity = signature.authenticity(document.bytes());
    assert!(
        !vouches_for_the_document(integrity, &authenticity),
        "the signature is over \"not this document\", and nothing compares the document's \
         digest with anything (§12.8.1, §12.8.3.3.1): {integrity:?} / {authenticity:?}"
    );
}

/// 2128-B03. §12.8.3.4.3 (a): "content-type: shall be present and shall always have the value
/// \"id-data\"" - a rule about "\[t]he attributes in the `SignedData` object", so about the
/// content-type *attribute*, which this value's signed attributes omit while everything else the
/// subclause and §12.8.3.4.2 ask is met. A departure list that is empty says the value meets them.
#[test]
fn a_pades_value_without_a_content_type_attribute_is_a_departure_spec_audit() {
    let bytes = fixture("pades-no-content-type.pdf");
    let (document, signature) = only_signature(&bytes);
    assert_eq!(signature.sub_filter.as_deref(), Some("ETSI.CAdES.detached"));
    // Control: a whole-file range, an unchanged digest and a verification.
    assert_eq!(
        signature.coverage(u64::try_from(bytes.len()).expect("a small file")),
        Coverage::WholeFile
    );
    assert!(vouches_for_the_document(
        signature.integrity(document.bytes()),
        &signature.authenticity(document.bytes())
    ));
    let cms = signature.signed_data().expect("the value reads");
    let departures = signature.pades_departures(&cms, document.bytes());
    assert!(
        !departures.is_empty(),
        "§12.8.3.4.3 (a) requires a content-type attribute and the value carries none"
    );
}

/// A one-signature document with the given signature dictionary, its `/ByteRange` placeholder
/// (`0000000000` four times) written to leave out exactly the `/Contents` string.
fn structural(signature: &str, catalog_extra: &str) -> Vec<u8> {
    let objects = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> {catalog_extra} >>"
        ),
        "<< /Type /Pages /Count 0 /Kids [] >>".to_owned(),
        "<< /Payload (the signed revision) >>".to_owned(),
        "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget /Rect [0 0 0 0] >>".to_owned(),
        signature.to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref = out.len();
    out.push_str("xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let mut bytes = out.into_bytes();
    let placeholder = b"0000000000 0000000000 0000000000 0000000000";
    if let Some(at) = bytes
        .windows(placeholder.len())
        .position(|window| window == placeholder)
    {
        let start = bytes
            .windows(b"/Contents <".len())
            .position(|window| window == b"/Contents <")
            .expect("a /Contents string")
            .saturating_add(b"/Contents ".len());
        let end = start.saturating_add(
            bytes[start..]
                .iter()
                .position(|byte| *byte == b'>')
                .expect("the string closes")
                .saturating_add(1),
        );
        let tail = bytes.len().saturating_sub(end);
        let range = format!("{:010} {start:010} {end:010} {tail:010}", 0);
        bytes[at..at.saturating_add(placeholder.len())].copy_from_slice(range.as_bytes());
    }
    bytes
}

/// 2128-B04. Table 255, `ByteRange`: "An array of pairs of integers (starting byte offset, length
/// in bytes) that shall describe the exact byte range for the digest calculation." An array with
/// an odd member, or a pair one of whose members is no integer, describes no exact range; a reader
/// that drops the odd member computes a range the file did not state.
#[test]
fn a_byte_range_that_is_not_pairs_of_integers_describes_no_range_spec_audit() {
    for (shape, range) in [
        (
            "an odd fifth member",
            "[0000000000 0000000000 0000000000 0000000000 99]",
        ),
        (
            "a third pair whose offset is a string",
            "[0000000000 0000000000 0000000000 0000000000 (x) 1]",
        ),
    ] {
        let bytes = structural(
            &format!(
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
                 /ByteRange {range} /Contents <0000> >>"
            ),
            "",
        );
        let (_document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.coverage(u64::try_from(bytes.len()).expect("a small file")),
            Coverage::Malformed,
            "{shape}: Table 255 states pairs of integers, and this is not that"
        );
    }
}

/// 2128-B05. Table 258, `V`: "The UR transform parameters dictionary version. The value shall be
/// 2.2 . If an unknown version is present, no rights shall be enabled. NOTE This value is a name
/// object, not a number." A `/V 3` is a version present and not 2.2, whatever its type, so with
/// `/P true` no right is granted - the same answer `/V /3.0` gets.
#[test]
fn an_unknown_usage_rights_version_enables_no_rights_whatever_its_type_spec_audit() {
    let ur3 = |version: &str| {
        structural(
            &format!(
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
                 /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <0000> \
                 /Reference [<< /Type /SigRef /TransformMethod /UR3 /TransformParams \
                 << /Type /TransformParams /V {version} /P true /Form [/FillIn] >> >>] >>"
            ),
            "/Perms << /UR3 5 0 R >>",
        )
    };
    let grants = |bytes: &[u8]| {
        let document = Document::open(bytes.to_vec()).expect("the document opens");
        permissions(&document)
            .usage_rights
            .expect("a UR3 signature with a UR transform")
            .grants(Right::FillInForm)
    };
    // Controls: the defined version grants what it names, and an unknown name grants nothing.
    assert!(grants(&ur3("/2.2")));
    assert!(!grants(&ur3("/3.0")));
    assert!(
        !grants(&ur3("3")),
        "Table 258: an unknown version present enables no rights"
    );
}

/// 2128-B06. Table 255, `Contents`: "For document timestamp signatures, ... the `ByteRange` shall
/// specify the complete PDF file contents (excepting the Contents value)", and §12.8.5.2: "The
/// `ByteRange` key shall cover the entire PDF file". Both are keyed on the dictionary being a
/// document timestamp (`/Type /DocTimeStamp`), while the `/SubFilter` value `ETSI.RFC3161` is
/// only what such a dictionary "should" state.
#[test]
fn a_document_timestamp_must_cover_the_whole_file_whatever_its_sub_filter_spec_audit() {
    let bytes = structural(
        "<< /Type /DocTimeStamp /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
         /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <0000> >>",
        "",
    );
    let (_document, signature) = only_signature(&bytes);
    assert!(signature.timestamp, "/Type /DocTimeStamp");
    assert!(
        signature.must_cover_whole_file(),
        "a document timestamp's range shall cover the entire file (Table 255, §12.8.5.2)"
    );
}

/// 2128-B07, `pdf-signature`'s half. §12.8.3.4.2: "The `ByteRange` shall cover the entire PDF
/// file", read with §12.8.1's sentence for a file of several revisions — the range ends at "the
/// \"%%EOF\" comment ... terminating the incremental update that adds the digital signature
/// dictionary to the document" — and with §12.8.4.3, which requires a DSS update "later than the
/// signature update". A revision appended after a `PAdES` signature is therefore not a range that
/// fails to cover the file, and `RangeDoesNotCoverTheFile` may not be reported for it.
#[test]
fn a_revision_appended_after_a_pades_signature_is_not_a_short_range_spec_audit() {
    use std::collections::BTreeMap;

    use pdf_signature::signature::PadesDeparture;
    use pdf_syntax::object::Object;
    use pdf_syntax::write::incremental_update;

    let original = fixture("pades-no-content-type.pdf");
    let document = Document::open(original.clone()).expect("the fixture opens");
    let Some(Object::Reference(root)) = document.trailer().get("Root").cloned() else {
        unreachable!("the generator writes an indirect catalog")
    };
    let catalog = document.catalog().expect("a catalog");
    let updated = incremental_update(
        &document,
        &BTreeMap::from([(root, Object::Dictionary(catalog))]),
    )
    .expect("the update writes");
    let (document, signature) = only_signature(&updated);
    // Control: the signature's own revision is whole and the tail is the appended one.
    assert!(matches!(
        signature.coverage(u64::try_from(updated.len()).expect("a small file")),
        Coverage::Unsigned { .. }
    ));
    assert_eq!(
        signature.coverage(u64::try_from(original.len()).expect("a small file")),
        Coverage::WholeFile
    );
    let cms = signature.signed_data().expect("the value reads");
    let departures = signature.pades_departures(&cms, document.bytes());
    assert!(
        !departures.contains(&PadesDeparture::RangeDoesNotCoverTheFile),
        "a later revision is §12.8.1's multiple-signature case, not a short range: {departures:?}"
    );
}
