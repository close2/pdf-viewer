//! Fuzzes ISO 32000-2 §7.6's standard security handler.
//!
//! The `document` target reaches this code too, but only when random bytes happen to form
//! a trailer with an `/Encrypt` naming a dictionary — which is close to never. This one
//! puts the fuzzer's bytes *inside* a document that is otherwise well formed, so every
//! input explores the clause's own parameter space: revisions, versions, key lengths, crypt
//! filter methods, and the `/O`, `/U`, `/OE`, `/UE` and `/Perms` byte strings the key is
//! derived from.
//!
//! Three properties are at stake, and none of them is "the right key comes out" — a fuzzer
//! cannot know that:
//!
//! - **No panic.** Every one of §7.6's algorithms indexes into a byte string whose length
//!   the file states and does not have to honour: Table 21 requires `/O` and `/U` to be 48
//!   bytes at revision 6 and 32 below it, and a hostile file writes three.
//! - **No unbounded loop.** §7.6.4.3.4's Algorithm 2.B decides when to stop by reading a
//!   byte of ciphertext computed from the document's own bytes, which is a loop whose
//!   trip count the input controls. `MAX_2B_ROUNDS` is what bounds it, and this is what
//!   would notice if it ever stopped doing so.
//! - **No unbounded allocation.** The same algorithm builds a string of 64 repetitions of
//!   the password, the running hash and the 48-byte user key, once per round.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::{Document, Limits};

/// Wraps the fuzzer's bytes as the body of an encryption dictionary.
///
/// The rest of the file is fixed and minimal: a catalogue, a page tree with one page and a
/// short content stream, so that reaching the page exercises decryption of a stream and of
/// the strings around it as well as of the dictionary itself.
fn document_around(encryption_dictionary: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(encryption_dictionary.len().saturating_add(512));
    out.extend_from_slice(b"%PDF-1.7\n");
    out.extend_from_slice(b"1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj\n");
    out.extend_from_slice(b"2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj\n");
    out.extend_from_slice(
        b"3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 99 99] \
          /Contents 4 0 R /Title (encrypted)>> endobj\n",
    );
    out.extend_from_slice(b"4 0 obj <</Length 40>> stream\n");
    out.extend_from_slice(b"0123456789012345678901234567890123456789");
    out.extend_from_slice(b"\nendstream endobj\n");

    out.extend_from_slice(b"5 0 obj <<");
    out.extend_from_slice(encryption_dictionary);
    out.extend_from_slice(b">> endobj\n");

    // No cross-reference table: recovery by scanning finds all five objects, which keeps
    // the fuzzer's bytes from having to encode byte offsets as well as a dictionary.
    out.extend_from_slice(
        b"trailer <</Root 1 0 R /Encrypt 5 0 R \
          /ID [<0102030405060708090A0B0C0D0E0F10> <00>]>>\n%%EOF\n",
    );
    out
}

fuzz_target!(|data: &[u8]| {
    let limits = Limits {
        max_depth: 32,
        max_array_len: 4096,
        max_dict_len: 256,
        max_string_len: 1 << 16,
        max_stream_len: 1 << 22,
    };

    let bytes = document_around(data);

    // Both of §7.6.4.1's password attempts: the default user password every reader tries
    // first, and a supplied one, which is the only way the `SASLprep` and PDFDocEncoding
    // paths are reached.
    for password in ["", "fuzz"] {
        let Ok(document) = Document::open_with_password(bytes.clone(), limits, password) else {
            continue;
        };
        let _ = document.permissions();

        // Decrypting is where the object key and the cipher run. Reaching the page also
        // walks the page dictionary's strings, which §7.6.2 says are encrypted too.
        let Ok(catalog) = document.catalog() else {
            continue;
        };
        let pages = document.get_key(&catalog, "Pages");
        let Some(pages) = pages.as_dict() else {
            continue;
        };
        let kids = document.get_key(pages, "Kids");
        let Some(first) = kids.as_array().and_then(<[_]>::first) else {
            continue;
        };
        let page = document.resolve(first);
        let Some(page) = page.as_dict() else { continue };
        if let Some(stream) = document.get_key(page, "Contents").as_stream() {
            let _ = document.decoded_stream_data(stream);
        }
    }
});
