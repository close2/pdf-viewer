//! Fuzzes ISO 32000-2 §7.5's structure on the way *out*: a document is opened, every object it
//! holds is copied into an assembly, the assembly is written as a whole file, and the result is
//! opened again with this tree's own reader.
//!
//! # Why this target exists
//!
//! RFC 0002 section 11.3 states the cost of the amendment that admitted the serializer: "this project
//! starts *producing* files other parsers read. A malformed output is this project's defect in
//! a way a misrendered page never was." A crash here is one; so is a file this reader wrote and
//! cannot open, which is why the round trip rather than the write alone is the target.
//!
//! # What the input is
//!
//! Whatever bytes libFuzzer supplies, read as a document. The population that matters is the
//! malformed one: a lying `/Length`, an object referring to one the table does not have, a
//! cycle — the serializer meets all three through `Document`, which is exactly how a transform
//! meets them.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::serialize::{Assembly, Form, serialize};
use pdf_syntax::{Document, Limits, Object, ObjectId, Version};

/// How many objects one input may carry into the assembly.
///
/// A bound rather than the file's own count, because the file states it and a fuzzer will state
/// four billion. The interesting structure — a closure, a cycle, a lying length — is reachable
/// well under this.
const MAX_OBJECTS: u32 = 512;

fuzz_target!(|data: &[u8]| {
    let limits = Limits {
        max_depth: 32,
        max_array_len: 4096,
        max_dict_len: 256,
        max_string_len: 1 << 16,
        max_stream_len: 1 << 22,
    };

    let Ok(document) = Document::open_with_limits(data.to_vec(), limits) else {
        return;
    };
    let Some(root) = document
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
    else {
        return;
    };

    let mut assembly = Assembly::new(vec![&document]);
    for number in 1..=MAX_OBJECTS {
        if assembly.copy(0, ObjectId::new(number, 0)).is_err() {
            return;
        }
    }
    let Some(mapped) = assembly.copied(0, root) else {
        return;
    };
    assembly.set_root(mapped);

    for form in [Form::Table, Form::Stream] {
        let mut bytes = Vec::new();
        let Ok(written) = serialize(&assembly, Version { major: 1, minor: 7 }, form, &mut bytes)
        else {
            continue;
        };
        // The count the writer reports is a count of the bytes it wrote, and a caller that
        // budgets on it must be able to.
        assert_eq!(
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            written.bytes,
            "the serializer miscounted its own output"
        );
        // The whole point: a file this reader wrote is a file this reader opens. `/Root` is
        // §7.5.5's one required entry and the assembly named one, so a re-read that cannot
        // reach a catalog is a defect in what was written rather than in the input.
        let read = Document::open_with_limits(bytes, limits)
            .expect("a file this serializer wrote must open");
        read.catalog()
            .expect("a file whose trailer names /Root must reach it");
        // And the object graph must still be safe to walk, which is what a consumer does next.
        let _ = read.get(ObjectId::new(1, 0));
    }
});
