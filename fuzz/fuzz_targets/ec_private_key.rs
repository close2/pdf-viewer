//! Fuzzes RFC 5915's elliptic-curve private-key reader and SEC 1's Diffie–Hellman primitive, as
//! `pdf_pki::ec` and `pdf_pki::key` run them.
//!
//! The private key is a host's input rather than a document's (ADR 1134), but a host reads it from
//! a file its user chose, and the point it is combined with — a key-agreement recipient's
//! `originatorKey` — is always a document's. So the reader of all three encodings
//! (`ECPrivateKey`, RFC 5958's `OneAsymmetricKey` around one, and RFC 8017's `RSAPrivateKey`, which
//! `pdf_pki::key` hands on) and the primitive are fuzzed from their first commit (ADR 2821).
//!
//! **Neither panics nor runs unbounded**, over any byte sequence: the input is read as a key, and
//! where it reads as an elliptic-curve key, the same bytes are offered to it as a point.
//! **A shared value is one field element**: SEC 1 section 3.3.1's `z` is the x-coordinate, which
//! section 2.3.5 writes in exactly the curve's field width.
//! **The two readers agree**: `pdf_pki::key` reads as an elliptic-curve key exactly what
//! `pdf_pki::ec` reads.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_pki::{ec, key};

fuzz_target!(|data: &[u8]| {
    let direct = ec::PrivateKey::from_der(data);
    let either = key::PrivateKey::from_der(data);
    assert_eq!(
        direct.is_ok(),
        matches!(either, Ok(key::PrivateKey::Ec(_))),
        "the two readers agree on what is an elliptic-curve key"
    );
    let Ok(key) = direct else {
        return;
    };
    for start in [0, data.len() / 2] {
        if let Ok(shared) = key.agree(&data[start..]) {
            assert_eq!(
                shared.len(),
                key.curve().field_octets(),
                "z is one field element"
            );
        }
    }
});
