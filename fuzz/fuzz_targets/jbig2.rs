//! Fuzzes the `JBIG2Decode` filter, ISO 32000-2 §7.4.7, as the confined worker runs it.
//!
//! The codec is `hayro-jbig2` and runs in the sandboxed process (principle 3); what this tree
//! writes around it is `pdf_sandbox`'s filter — the embedded organisation §7.4.7 describes, the
//! `/JBIG2Globals` stream Table 12 places before the page's segments, the retry on a whole prefix
//! of segments, the bilevel sense, and [`pdf_sandbox`]'s pixel bound. The in-process isolation runs
//! the very functions the worker calls, so this reaches all of it without a process per input.
//!
//! The first two bytes are the globals' length (big-endian, clamped to what follows), then the
//! globals, then the page's own segments. Beyond never panicking, the answer is checked against
//! [`pdf_sandbox::Bilevel`]'s documented shape: packed rows of `ceil(width / 8)` bytes, as many
//! as the height, with `delivered` never above the height and `concealed` zero for this filter.

#![no_main]
#![expect(
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `panic!` is how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_sandbox::{Decoded, Isolation, Request};

fuzz_target!(|data: &[u8]| {
    let Some((head, rest)) = data.split_first_chunk::<2>() else {
        return;
    };
    let split = usize::from(u16::from_be_bytes(*head)).min(rest.len());
    let (globals, segments) = rest.split_at(split);

    pdf_sandbox::set_isolation(Isolation::InProcess);
    let Ok(decoded) = pdf_sandbox::decode(&Request::Jbig2 {
        data: segments,
        globals,
    }) else {
        return;
    };
    let Decoded::Bilevel(image) = decoded else {
        panic!("JBIG2Decode answers a bilevel image");
    };
    let stride = u64::from(image.width).div_ceil(8);
    assert_eq!(
        u64::try_from(image.rows.len()).ok(),
        stride.checked_mul(u64::from(image.height)),
        "the rows are packed, ceil(width / 8) bytes each, one per row of the grid"
    );
    assert!(
        image.delivered <= image.height,
        "no more rows delivered than the grid holds"
    );
    assert_eq!(image.concealed, 0, "only CCITTFaxDecode conceals a row");
});
