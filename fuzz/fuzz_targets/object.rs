//! Fuzzes object parsing.
//!
//! Checks that parsing terminates and respects its resource bounds on arbitrary input.
//! Deeply nested and self-referential structures are the interesting cases: `[[[[...` must
//! hit the depth limit rather than the stack, and a truncated dictionary must be an error
//! rather than a hang.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::{Limits, Parser};

fuzz_target!(|data: &[u8]| {
    // Tighter than the default so the limit paths are reached by short inputs, which is
    // where a fuzzer spends most of its time.
    let limits = Limits {
        max_depth: 32,
        max_array_len: 1024,
        max_dict_len: 256,
        max_string_len: 1 << 16,
        max_stream_len: 1 << 20,
    };

    let mut parser = Parser::with_limits(data, limits);
    let mut last = parser.position();

    // Parse repeatedly: a single object exercises only the first construct in the input.
    for _ in 0..64 {
        match parser.parse_object() {
            Ok(_) => {}
            Err(_) => break,
        }
        let now = parser.position();
        assert!(now > last, "the parser failed to advance at byte {last}");
        last = now;
    }
});
