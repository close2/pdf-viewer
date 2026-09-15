//! Fuzzes §12.8.4's revocation material: RFC 5280 section 5's CRLs and RFC 6960's OCSP responses.
//!
//! **Why this target exists at all.** Until the thousand-and-fifty-third session nothing in this
//! tree read a CRL or an OCSP response, and §12.8.4.3 makes both of them *streams out of a
//! stranger's file* — "[a]n array of indirect references to streams, each containing a DER-encoded
//! Certificate Revocation List (CRL)". A new ASN.1 reader over untrusted bytes is what
//! `CLAUDE.md` principle 3's "fuzzing from the first parser commit" is about, and ADR 1067
//! committed to this target with the code.
//!
//! Four properties, and the last two are the ones a unit test cannot reach:
//!
//! **Parsing terminates and never panics.** Every refusal is a named `MaterialRefusal`, including
//! the `DerError` it carries. The fuzz profile keeps overflow checks on, so the workspace's
//! `arithmetic_side_effects` rule is checked here too — and this module computes instants out of
//! digits the file states, which is arithmetic on a stranger's numbers.
//!
//! **Nothing that comes back outlives or outgrows its input.** Every field of a `CertificateList`
//! and of a `BasicResponse` is a sub-slice of the caller's buffer, which is the design
//! `pdf_signature::x509` already has and the property that keeps the whole crate copy-free.
//!
//! **Every bound holds and is observable.** `MAX_REVOKED_ENTRIES` bounds the search through one
//! list's entries and `MAX_SINGLE_RESPONSES` how many statuses one response may state; a search
//! that hits either must come back as a refusal or as `truncated`, never as "not listed". That
//! distinction is the whole of ADR 1067 section 2: a walk that stopped early may not report a
//! clean answer.
//!
//! **The canonical check refines the length check, and never disagrees with it.** ITU-T X.690
//! clause 10.1's first half — a definite length — is one of the nine rules
//! `pdf_signature::der::is_canonical` states, so a region it calls canonical is one
//! `every_length_is_definite` must call definite. The two walk the same encoding by different
//! routes and a fuzzer is what puts a stranger's bytes through both; a disagreement is a defect in
//! one of them, and nothing shorter than an input that reaches it would say which.
//!
//! **A status is never `Good` for material this input did not make verifiable.** The target asks
//! about a certificate of its own choosing under a key of its own choosing, and no CRL or response
//! a fuzzer produces is signed by that key — so `Revocation::Good` here would be a defect in the
//! comparison rather than a lucky input. This is the same shape `x509.rs` uses for a digest nobody
//! signed, applied to the one answer that must never be invented.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` and `assert!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_signature::pkcs1;
use pdf_signature::revocation::{
    self, Material, Revocation, Subject, certificate_list, ocsp_response,
};
use pdf_signature::x509::{self, Certificate, Extensions, Instant, PublicKey, Validity};

fuzz_target!(|data: &[u8]| {
    let inside = |slice: &[u8]| {
        assert!(
            slice.len() <= data.len(),
            "{} bytes came out of an input of {}",
            slice.len(),
            data.len()
        );
    };

    if let Ok(list) = certificate_list(data) {
        inside(list.issuer);
        inside(list.tbs);
        inside(list.signature);
        inside(list.signature_algorithm);
        if let Some(oid) = list.unrecognised_critical {
            inside(oid);
            // An unrecognised critical extension is reported by *its* number, so decoding one has
            // to be total — the same obligation `x509.rs` states for an unverifiable algorithm.
            let _ = x509::dotted(oid);
        }
        // Three serial numbers: one out of the input, which is what a self-referential list would
        // name, and two the list cannot hold. The walk is bounded by `MAX_REVOKED_ENTRIES` and a
        // search that hits the bound must refuse rather than answer "not listed".
        for serial in [data, &[][..], &[0x01][..]] {
            if let Ok(Some(entry)) = list.entry_for(serial) {
                inside(entry.serial_number);
                assert!(
                    entry.serial_number == serial,
                    "an entry answered for a serial number it does not carry"
                );
            }
        }
        // Reading the same bytes twice must give the same answer: the reader carries a depth
        // across nested calls, and state leaking between them is what one pass cannot see.
        let again = certificate_list(data).expect("the same list parsed once already");
        assert!(
            again == list,
            "reading the same revocation list twice gave two different answers"
        );
    }

    if let Ok(response) = ocsp_response(data) {
        inside(response.tbs);
        inside(response.signature);
        inside(response.signature_algorithm);
        if let Some(name) = response.responder_name {
            inside(name);
        }
        if let Some(hash) = response.responder_key_hash {
            inside(hash);
        }
        assert!(
            response.responses.len() <= revocation::MAX_SINGLE_RESPONSES,
            "a response stated more statuses than the bound admits"
        );
        assert!(
            response.certificates.len() <= revocation::MAX_RESPONDER_CERTIFICATES,
            "a response offered more certificates than the bound admits"
        );
        for single in &response.responses {
            inside(single.hash_algorithm);
            inside(single.issuer_name_hash);
            inside(single.issuer_key_hash);
            inside(single.serial_number);
        }
    }

    // Table 261 and RFC 6960 section 4.2.1 both require DER of the material above, and
    // `is_canonical` is what those two readers ask before they read a byte. Asked here directly
    // as well, because the property is about the *check* rather than about either reader: it is
    // total over a stranger's bytes, it is a function of them alone, and what it calls canonical
    // is a subset of what `every_length_is_definite` calls definite (ITU-T X.690 clause 10.1).
    let canonical = pdf_signature::der::is_canonical(data);
    assert!(
        canonical == pdf_signature::der::is_canonical(data),
        "the same region gave two different answers about its encoding"
    );
    if canonical == Ok(None) {
        assert!(
            pdf_signature::der::every_length_is_definite(data) == Ok(true),
            "a region called DER holds a length the narrower check calls indefinite"
        );
    }

    // §12.8.3.3.2's attribute reaches the same two readers through a third door, and this one
    // never refuses: `archived` walks a signer's signed attributes and keeps what reads.
    if let Ok(cms) = pdf_signature::cms::signed_data(data) {
        let archived = revocation::archived(&cms);
        assert!(
            archived
                .lists
                .len()
                .saturating_add(archived.responses.len())
                <= revocation::MAX_ARCHIVED,
            "an attribute contributed more material than the bound admits"
        );
    }

    // The property that matters most, asked of whatever the input turned out to be: a certificate
    // this target chose, under a key this target chose, against material a fuzzer wrote. Nothing
    // here is signed by that key, so `Good` is the one answer that may not come back.
    let ours = subject_certificate();
    let material = Material::read(&[data], &[data]);
    let subject = Subject {
        certificate: &ours,
        issuer_name: ours.subject_encoding,
        issuer_key_bits: ours.public_key_bits,
        issuer_key: ours.public_key,
        issuer_key_usage: ours.extensions.key_usage,
        position: 0,
    };
    for at in [
        Instant::from_unix_seconds(0),
        Instant::from_unix_seconds(1_780_272_000),
        Instant::from_unix_seconds(i64::MAX),
    ] {
        // The property, as a predicate rather than a match: `Revocation` is `#[non_exhaustive]`,
        // so a variant added later must not silently become an accepted answer here — and what is
        // forbidden is one variant rather than all-but-one.
        assert!(
            !matches!(
                revocation::status(&subject, &material, at),
                Revocation::Good { .. }
            ),
            "revocation material nobody signed said a certificate was not revoked"
        );
    }
});

/// The certificate this target asks about, built rather than parsed.
///
/// **Built, because what is wanted here is the opposite of a real certificate.** A positive
/// revocation answer would need material signed by the key that issued this one, and the whole
/// property this target asserts is that no such material can come out of a fuzzer — so the key is
/// a number nobody holds the factors of, and the serial and the names are constants the input
/// cannot reach. `pdf_signature::x509::Certificate` is a view over borrowed slices with public
/// fields, which is what makes this possible without a private key or a pasted blob.
fn subject_certificate() -> Certificate<'static> {
    /// A 2048-bit odd number. Not an RSA modulus anybody generated, which is the point: `pkcs1`
    /// will do its modular exponentiation over it and the comparison will fail, for every
    /// signature there is.
    const MODULUS: &[u8] = &[0xC7; 256];
    /// F4, the exponent every real key states.
    const EXPONENT: &[u8] = &[0x01, 0x00, 0x01];
    /// `SEQUENCE { SET { SEQUENCE { OBJECT IDENTIFIER commonName, UTF8String "q" } } }`, which is
    /// a `Name` a fuzzer would have to guess to make an `issuerNameHash` match.
    const NAME: &[u8] = &[
        0x30, 0x0C, 0x31, 0x0A, 0x30, 0x08, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0C, 0x01, 0x71,
    ];
    Certificate {
        serial_number: &[0x51, 0x55, 0x4F, 0x52, 0x52, 0x41],
        issuer: NAME.get(2..).unwrap_or(&[]),
        subject: NAME.get(2..).unwrap_or(&[]),
        subject_encoding: NAME,
        key_identifier: None,
        public_key: PublicKey::Rsa(pkcs1::PublicKey {
            modulus: MODULUS,
            exponent: EXPONENT,
        }),
        public_key_bits: MODULUS,
        version: 3,
        tbs: NAME,
        indefinite_lengths: false,
        signature_algorithm: &[],
        signature_parameters: None,
        signature: &[],
        validity: Some(Validity {
            not_before: Instant::from_unix_seconds(0),
            not_after: Instant::from_unix_seconds(i64::MAX),
        }),
        extensions: Extensions::default(),
    }
}
