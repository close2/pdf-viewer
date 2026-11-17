//! Fuzzes the SAML 2.0 assertion reader a signature's `signer-attributes-v2` is read with
//! (`pdf_signature::saml`, ADR 2922), and SAML core section 5.4's signature profile over it.
//!
//! An assertion arrives inside a signature's signed attributes, so it is a stranger's XML: the
//! octet, entry, attribute and value bounds (`MAX_OCTETS`, `MAX_ENTRIES`, `MAX_ATTRIBUTES`,
//! `MAX_VALUES`) are what stand between a hostile assertion and the heap, and a document type
//! declaration is refused before any entity could expand. Beyond never panicking — overflow checks
//! stay on in this profile — two properties are under test:
//!
//! - **The reading and the verification are functions of the bytes**: read twice, the same
//!   assertion and the same signature outcome.
//! - **Nothing invalid is relied on, and the bounds hold**: an assertion whose signature is
//!   invalid hands on no attribute (SAML core section 2.3.3), and a read assertion keeps at most
//!   `MAX_ATTRIBUTES` attributes of at most `MAX_VALUES` values; its conditions answer at any
//!   instant.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_signature::saml::{MAX_ATTRIBUTES, MAX_VALUES, Saml};
use pdf_signature::x509::Instant;

fuzz_target!(|data: &[u8]| {
    // The first eight bytes are the instant the conditions are asked about, and the rest is the
    // assertion.
    let (instant, octets) = data.split_at(data.len().min(8));
    let mut seconds = [0_u8; 8];
    seconds[..instant.len()].copy_from_slice(instant);
    let at = Instant::from_unix_seconds(i64::from_le_bytes(seconds));

    let first = Saml::of(octets);
    assert_eq!(first, Saml::of(octets));
    if first.signature.is_invalid() {
        assert!(first.attributes_relied_on().is_empty());
    }
    if let Ok(assertion) = &first.assertion {
        assert!(assertion.attributes.len() <= MAX_ATTRIBUTES);
        assert!(
            assertion
                .attributes
                .iter()
                .all(|attribute| attribute.values.len() <= MAX_VALUES)
        );
        let _ = assertion.conditions_at(at);
        for attribute in &assertion.attributes {
            let _ = attribute.object_identifier();
        }
    }
});
