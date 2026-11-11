//! Fuzzes RFC 3394's AES key unwrap, as `pdf_pki::key_wrap` runs it.
//!
//! A key-agreement recipient's `encryptedKey` is a document's bytes, unwrapped under a key the
//! reader derived; a fuzzer that had to reach it through a whole envelope and an agreement would
//! almost never get past the agreement. So the unwrap is fuzzed alone from its first commit (ADR
//! 2821), under keys of the three widths cut from the input itself.
//!
//! **It never panics and its work is the input's**: section 2.2.2's six passes over n blocks.
//! **What comes back is one block shorter than what went in**: "(n+1) 64-bit values" in, "n
//! 64-bit values" out, and only past section 2.2.3.1's integrity check.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_pki::key_wrap::unwrap;

fuzz_target!(|data: &[u8]| {
    for width in [16, 24, 32] {
        let Some((kek, ciphertext)) = data.split_at_checked(width) else {
            return;
        };
        if let Ok(key) = unwrap(kek, ciphertext) {
            assert_eq!(key.len() + 8, ciphertext.len(), "one block shorter");
        }
    }
});
