//! Fuzzes the three readers of ISO 32000-2 §12.8.3.4.4's stored signature policy document: ETSI
//! TR 102 272 clause 6's ASN.1 syntax (`pdf_signature::policy_rules::PolicyDocument`), ETSI TS 119
//! 172-3's (`policy_rules::ts_119_172_3`) and ETSI TS 119 172-2's XML (`policy_rules::ts_119_172_2`,
//! ADR 2604).
//!
//! A policy document is stored in the file beside the signature (`policy.rs`'s `Store::Document`),
//! so its octets are as untrusted as any other stream's, and each reader allocates per entry it
//! reads: the shared length, nesting and entry budgets (`MAX_POLICY_OCTETS`, `der::MAX_DEPTH`,
//! `MAX_ENTRIES`) are what stand between a hostile document and the heap. Beyond never panicking —
//! overflow checks stay on in this profile — three properties are under test:
//!
//! - **One reader accepts a document, and it is the one `Syntax::of` names**: `enforce` dispatches
//!   on `Syntax::of`, so a document a second reader would also accept, or one the dispatcher sends
//!   to a reader that refuses it, would be judged by rules nobody wrote.
//! - **The enforced result is a function of the bytes**: read and judged twice against the same
//!   signature and the same facts, the same findings come back.
//! - **The XML reader's two steps agree**: a tree `Reading::parse` accepts is read by
//!   `Reading::document` the same way twice, since the second step re-charges the entry budget the
//!   first left.
//! - **Canonical XML 1.0 is a function of the bytes and its own fixed point**
//!   (`policy_rules::canonical_xml`, ADR 2644): the `PolicyComponents` element of any input, with
//!   and without comments, canonicalises the same way twice, and its canonical form read as a
//!   document of its own canonicalises to itself, since the Recommendation's section 2.1 makes the
//!   form a function of the data model alone and the form preserves that model.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_signature::cms::{self, Digest, SignedData};
use pdf_signature::policy_rules::canonical_xml::{self, Algorithm, Apex, Comments, Method};
use pdf_signature::policy_rules::{
    self, Finding, MAX_POLICY_OCTETS, PolicyDocumentError, SignerFacts, SigningTime, Syntax,
    TimeSource, ts_119_172_2, ts_119_172_3,
};
use pdf_signature::revocation::Material;
use pdf_signature::x509::Instant;

/// RFC 5754's `id-sha256`, the signature's digest algorithm.
const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
/// RFC 4055's `sha256WithRSAEncryption`, the signature's algorithm and its certificate's.
const SHA256_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];

/// The signature every document is judged against: a `SignedData` naming the two attributes RFC
/// 5652 section 5.3 requires, so that a rule over signed attributes has something to count.
fn signature() -> SignedData<'static> {
    SignedData {
        content_type: cms::ID_DATA,
        encapsulated: None,
        certificates: Vec::new(),
        signers: 1,
        digest: Some(Digest::Sha256),
        digest_algorithm: SHA256,
        signature_algorithm: SHA256_RSA,
        signature_algorithm_parameters: None,
        signature: &[],
        signed_attributes: None,
        signer_issuer_and_serial: None,
        signer_key_identifier: None,
        message_digest: None,
        signing_certificate: None,
        signing_certificate_v2: None,
        signature_timestamp: None,
        signed_attribute_types: vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST],
        unsigned_attribute_types: Vec::new(),
        attributes: Vec::new(),
        attributes_truncated: false,
    }
}

/// Two sets of facts: no signing time at all, and a time-stamped one, so that a signing period is
/// judged both as unestablished and against an instant.
fn facts() -> [SignerFacts<'static>; 2] {
    let at = |signing_time| SignerFacts {
        signing_time,
        key_bits: Some(2048),
        carries_signer_certificate: true,
        certificate_signature_algorithm: Some(SHA256_RSA.to_vec()),
        path_at: None,
        material: Material::default(),
    };
    [
        at(None),
        at(Some(SigningTime {
            instant: Instant::from_unix_seconds(1_780_000_000),
            spelled: "20260528T201320Z".to_owned(),
            source: TimeSource::SignatureTimestamp,
        })),
    ]
}

/// Each reader's findings under each set of facts, or its refusal.
fn judged(data: &[u8], syntax: Syntax) -> Result<Vec<Vec<Finding>>, PolicyDocumentError> {
    let cms = signature();
    let facts = facts();
    match syntax {
        Syntax::Tr102272 => {
            let document = policy_rules::PolicyDocument::parse(data)?;
            Ok(facts
                .iter()
                .map(|facts| policy_rules::findings(&document, &cms, facts))
                .collect())
        }
        Syntax::Ts1191723 => {
            let document = ts_119_172_3::PolicyDocument::parse(data)?;
            Ok(facts
                .iter()
                .map(|facts| ts_119_172_3::findings(&document, &cms, facts))
                .collect())
        }
        Syntax::Ts1191722 => {
            let reading = ts_119_172_2::Reading::parse(data)?;
            let document = reading.document()?;
            assert_eq!(
                reading.document().as_ref(),
                Ok(&document),
                "the XML tree is read the same way twice"
            );
            Ok(facts
                .iter()
                .map(|facts| ts_119_172_2::findings(&document, &cms, facts))
                .collect())
        }
    }
}

/// The canonical form of the `PolicyComponents` element `depth` elements down, or the refusal.
fn canonical(
    data: &[u8],
    depth: usize,
    method: &Method<'_>,
) -> Result<Vec<u8>, PolicyDocumentError> {
    let apex = Apex {
        depth,
        namespaces: &ts_119_172_2::NAMESPACE,
        local: "PolicyComponents",
    };
    let mut out = Vec::new();
    canonical_xml::canonicalize(data, &apex, method, &mut |piece| {
        out.extend_from_slice(piece);
    })?;
    Ok(out)
}

fuzz_target!(|data: &[u8]| {
    for comments in [Comments::Omitted, Comments::Kept] {
        let method = Method {
            algorithm: Algorithm::Canonical10,
            comments,
            prefix_list: None,
        };
        let first = canonical(data, 1, &method);
        assert_eq!(
            first,
            canonical(data, 1, &method),
            "the canonical form is a function of the bytes"
        );
        if let Ok(form) = first
            && form.len() <= MAX_POLICY_OCTETS
        {
            assert_eq!(
                canonical(&form, 0, &method).as_ref(),
                Ok(&form),
                "a canonical form canonicalises to itself"
            );
        }
    }

    let named = Syntax::of(data);
    for syntax in [Syntax::Tr102272, Syntax::Ts1191723, Syntax::Ts1191722] {
        let first = judged(data, syntax);
        if first.is_ok() {
            assert_eq!(
                named,
                Some(syntax),
                "a document {syntax} accepts is dispatched to its reader"
            );
        }
        assert_eq!(
            first,
            judged(data, syntax),
            "{syntax}'s answer is a function of the bytes"
        );
    }
});
