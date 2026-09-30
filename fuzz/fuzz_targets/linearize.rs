//! Fuzzes Annex F's linearised file, both ways: `pdf_syntax::linearize::state` reading an input's
//! first object as Table F.1's parameter dictionary, and `serialize_linearized` writing a whole
//! document out as a linearised file that this reader must open and recognise (ADRs 1293, 1309).
//!
//! The round trip is the `serialize` target's with Annex F's layout on top: every object up to a
//! bound is copied into an assembly, the page tree's pages become the plan in §7.7.3.2's order,
//! page 0 is §F.3.7's first page, and the file is written in both of §7.5's cross-reference forms.
//! Beyond never panicking, three properties:
//!
//! - **The writer's tally is its output.** The byte count it reports is the length it wrote.
//! - **A file it wrote opens.** A linearised file is an ordinary file first; §F.1: "A PDF processor
//!   that does not support this optional feature can still successfully process linearized files".
//! - **A file it wrote is linearised.** `state` finds Table F.1's dictionary with `/L` equal to the
//!   file's length and `/N` equal to the plan's page count — "A mismatch indicates that the file
//!   is not linearized", so a writer whose own output reads as anything else wrote a lie.

#![no_main]
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::linearize::{Plan, State, serialize_linearized, state};
use pdf_syntax::serialize::{Assembly, Form, Options};
use pdf_syntax::{Document, Limits, Object, ObjectId, Version};

/// How many objects one input may carry into the assembly, for `serialize`'s reason: the file
/// states its own count and a fuzzer will state four billion.
const MAX_OBJECTS: u32 = 512;

/// How many pages the plan names, which bounds the hint tables' rows.
const MAX_PAGES: usize = 64;

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
    let _ = state(&document);

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

    let pages = pdf_model::Pages::new(&document);
    let mut planned = Vec::new();
    for index in 0..pages.len().min(MAX_PAGES) {
        let Some(id) = pages.get(index).and_then(|page| page.id) else {
            return;
        };
        let Some(output) = assembly.copied(0, id) else {
            return;
        };
        planned.push(output);
    }
    if planned.is_empty() {
        return;
    }
    let plan = Plan {
        pages: planned,
        first_page: 0,
    };

    for form in [Form::Table, Form::Stream] {
        let mut bytes = Vec::new();
        let Ok(written) = serialize_linearized(
            &assembly,
            Version { major: 1, minor: 7 },
            Options::new(form),
            &plan,
            &mut bytes,
        ) else {
            continue;
        };
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        assert_eq!(
            length, written.written.bytes,
            "the linearising serializer miscounted its own output"
        );
        let read = Document::open_with_limits(bytes, limits)
            .expect("a linearised file this serializer wrote must open");
        match state(&read) {
            State::Linearized(parameters) => {
                assert_eq!(parameters.length, length, "/L is the file's length");
                assert_eq!(
                    usize::try_from(parameters.pages).ok(),
                    Some(plan.pages.len()),
                    "/N is the plan's page count"
                );
            }
            other => panic!("a file this serializer linearised reads as {other:?}"),
        }
    }
});
