//! Fuzzes ISO 32000-2 §12.8.3.3's signature value — the tree's only ASN.1 over untrusted bytes.
//!
//! A signature's `/Contents` is a byte string a stranger wrote, and from the
//! three-hundred-and-seventy-seventh session this program *parses* it: `pdf_model::der` reads
//! X.690's tag-length-value encoding and `pdf_model::cms` reads RFC 5652's `SignedData` out of
//! it. ADR 0215 committed to this target with the code, on the rule this project already applies
//! to every parser it writes — a reader of hostile input that is not fuzzed is a reader whose
//! bounds are a claim.
//!
//! Four properties are under test.
//!
//! **Parsing terminates and never panics**, over any byte sequence. The fuzz profile keeps
//! overflow checks on, so the workspace's `arithmetic_side_effects` rule is checked here too.
//!
//! **Every bound holds and is observable.** `der::MAX_DEPTH` and `der::MAX_VALUE` bound the
//! encoding; `cms` bounds a signer's attribute lists. The one that needs a target rather than a
//! test is the depth: an indefinite-length value's end is found by scanning its children, so a
//! file that nests those is asking this reader to recurse, and the only thing between it and the
//! stack is a counter.
//!
//! **Nothing that comes back outlives or outgrows its input.** Every accessor hands out a
//! sub-slice of the caller's buffer — that is the whole design of `der` — so a value longer than
//! the input would mean an index had been built out of the file's own arithmetic.
//!
//! **A successful parse is idempotent.** The same bytes must give the same answer twice; the
//! reader carries a depth across nested calls, and state leaking between them is what a single
//! pass cannot see.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_model::cms::signed_data;
use pdf_model::der::{MAX_VALUE, Reader};

/// `cms`'s own ceiling on a signer's attribute lists, which is private to that module and is
/// restated here so this target checks the bound rather than trusting it — and so that
/// `attributes_truncated` must say so whenever either list is at it.
const MAX_ATTRIBUTES: usize = 64;

fuzz_target!(|data: &[u8]| {
    // The reader beneath, walked whole: `signed_data` stops at the first thing it does not need,
    // so most of the encoding would never be visited through it alone.
    walk(data);

    let Ok(cms) = signed_data(data) else {
        // A refusal is a result. Every one is a named `CmsError` — including the `DerError` it
        // carries — rather than a panic, which is the property this arm exercises.
        return;
    };

    let inside = |slice: &[u8]| {
        assert!(
            slice.len() <= data.len(),
            "{} bytes came out of an input of {}",
            slice.len(),
            data.len()
        );
    };
    inside(cms.content_type);
    inside(cms.digest_algorithm);
    if let Some(content) = cms.encapsulated {
        inside(content);
    }
    if let Some(digest) = cms.message_digest {
        inside(digest);
    }
    for oid in cms
        .signed_attribute_types
        .iter()
        .chain(&cms.unsigned_attribute_types)
    {
        inside(oid);
    }
    inside(cms.signature_algorithm);
    inside(cms.signature);
    if let Some(attributes) = cms.signed_attributes {
        inside(attributes);
    }
    if let Some((issuer, serial)) = cms.signer_issuer_and_serial {
        inside(issuer);
        inside(serial);
    }
    if let Some(identifier) = cms.signer_key_identifier {
        inside(identifier);
    }
    // RFC 5652 section 5.4's re-encoding, which is the one thing this module *builds* rather than
    // borrows: a `SET OF` header in front of the file's own attribute bytes. It may be longer
    // than its input by that header and by nothing else.
    if let Some(encoded) = cms.signed_attributes_encoding() {
        assert!(
            encoded.len() <= data.len().saturating_add(6),
            "the signed-attributes re-encoding grew by more than a header"
        );
    }
    let _ = cms.algorithm();
    assert!(cms.signed_attribute_types.len() <= MAX_ATTRIBUTES);
    assert!(cms.unsigned_attribute_types.len() <= MAX_ATTRIBUTES);
    assert!(
        cms.attributes_truncated
            || (cms.signed_attribute_types.len() < MAX_ATTRIBUTES
                && cms.unsigned_attribute_types.len() < MAX_ATTRIBUTES)
    );
    // `MAX_CERTIFICATES` likewise, restated so the bound is checked rather than trusted.
    assert!(cms.certificates.len() <= 64);

    // The two answers a caller asks for, both total over anything that parses.
    if let Some((_, imprint)) = cms.timestamp_imprint() {
        inside(imprint);
    }
    let _ = cms.has_signed_attribute(&[0x2A]);

    let again = signed_data(data).expect("the same bytes parsed once already");
    assert!(
        again == cms,
        "reading the same signature value twice gave two different answers"
    );
});

/// Walks every value in the encoding, depth first, to the reader's own bound.
///
/// The recursion here is bounded by `der::MAX_DEPTH`, which `children` enforces by returning
/// `TooDeep` — so if this function overflows the stack, that bound is the defect and not this
/// target.
fn walk(bytes: &[u8]) {
    let Ok(mut reader) = Reader::new(bytes) else {
        assert!(bytes.len() > MAX_VALUE, "only size refuses a reader");
        return;
    };
    descend(&mut reader, bytes.len());
}

/// One level, and every level under it.
fn descend(reader: &mut Reader<'_>, input: usize) {
    loop {
        match reader.next_value() {
            Err(_) | Ok(None) => return,
            Ok(Some(value)) => {
                assert!(
                    value.contents.len() <= input,
                    "a value of {} bytes inside an input of {input}",
                    value.contents.len()
                );
                let _ = (value.class(), value.tag_number(), value.object_identifier());
                if let Ok(mut children) = value.children() {
                    descend(&mut children, input);
                }
            }
        }
    }
}
