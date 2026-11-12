//! Fuzzes RFC 3394's AES key unwrap and RFC 3217's Triple-DES one, as `pdf_pki::key_wrap` runs
//! them.
//!
//! A key-agreement recipient's `encryptedKey` is a document's bytes, unwrapped under a key the
//! reader derived; a fuzzer that had to reach it through a whole envelope and an agreement would
//! almost never get past the agreement. So the unwrap is fuzzed alone from its first commit (ADR
//! 2821), under keys of the three widths cut from the input itself.
//!
//! **It never panics and its work is the input's**: section 2.2.2's six passes over n blocks.
//! **What comes back is one block shorter than what went in**: "(n+1) 64-bit values" in, "n
//! 64-bit values" out, and only past section 2.2.3.1's integrity check. The Triple-DES unwrap
//! (ADR 2827) is fuzzed under a 24-octet key cut the same way, and what passes its checksum and
//! parity is a Triple-DES key.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_pki::key_wrap::{unwrap, unwrap_triple_des};

fuzz_target!(|data: &[u8]| {
    // RFC 3217 section 3.2 under a 24-octet key: what it returns is step 9's Triple-DES key.
    if let Some((kek, ciphertext)) = data.split_at_checked(24)
        && let Ok(key) = unwrap_triple_des(kek, ciphertext)
    {
        assert_eq!(key.len(), 24, "a Triple-DES key is 24 octets");
    }
    for width in [16, 24, 32] {
        let Some((kek, ciphertext)) = data.split_at_checked(width) else {
            return;
        };
        if let Ok(key) = unwrap(kek, ciphertext) {
            assert_eq!(
                key.len().checked_add(8),
                Some(ciphertext.len()),
                "one block shorter"
            );
        }
    }
});
