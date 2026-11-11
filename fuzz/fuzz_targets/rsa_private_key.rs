//! Fuzzes RFC 8017's private-key reader and RSA decryption, as `pdf_pki::rsa` runs them.
//!
//! The private key is a host's input rather than a document's (ADR 1134), but a host reads it
//! from a file its user chose, and the ciphertext it is applied to is always a document's. So the
//! reader of both encodings — RFC 5958's `OneAsymmetricKey` and RFC 8017's `RSAPrivateKey` — and
//! section 7.2.2's decryption are fuzzed from their first commit, as the owner's answers A66 and
//! A800 asked (ADR 2801).
//!
//! **Neither panics nor runs unbounded**, over any byte sequence: the input is read as a key, and
//! where it reads, the same bytes are decrypted under it as a ciphertext of the key's length.
//! **A decryption is never longer than the modulus less eleven octets**, which section 7.2.2's
//! output "an octet string of length at most k - 11" states.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_pki::rsa::PrivateKey;

fuzz_target!(|data: &[u8]| {
    let Ok(key) = PrivateKey::from_der(data) else {
        return;
    };
    let length = key.length();
    let mut ciphertext = data
        .iter()
        .copied()
        .cycle()
        .take(length)
        .collect::<Vec<u8>>();
    if let Some(first) = ciphertext.first_mut() {
        // Below the modulus more often than not, so that the decoding is reached.
        *first = 0;
    }
    if let Ok(message) = key.decrypt(&ciphertext) {
        assert!(
            message.len().saturating_add(11) <= length,
            "at most k - 11 octets"
        );
    }
});
