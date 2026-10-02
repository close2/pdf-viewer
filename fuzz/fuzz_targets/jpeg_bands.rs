//! Fuzzes the two ways `pdf_model` cuts a baseline `DCTDecode` frame into bands decoded beside
//! each other — at its restart intervals (ADR 1433) and at the rows an entropy pass finds
//! (ADR 1481) — against the whole frame's decoder. A differential target: a band plan that
//! admits a frame and writes one byte the whole decoder does not is a finding.
//!
//! §7.4.8's codestream is the document's, so the frame header that sizes the plan, the `DRI` that
//! places the cuts and the Huffman tables the pass decodes with are all untrusted. Both plans
//! answer `None` for anything they decline, and the caller then decodes the frame whole; what
//! they must never do is answer *differently*. `pdf_model::image::banded_decodes` asks all three
//! below the production floor, at a band height the first byte states (8 to 64 lines), so a frame
//! of a few hundred bytes is cut at all.
//!
//! Beyond never panicking — overflow checks stay on in this profile — three properties:
//!
//! - **A band plan's frame is the whole frame**: where a plan and the whole decoder both answer,
//!   the bytes are identical.
//! - **A band plan answers only where the whole decoder does**: the caller falls back to the
//!   whole decoder on a decline, so a damaged codestream is read by that decoder alone; a plan
//!   that decoded what the decoder refuses would have changed which reading a page gets.
//! - **The two plans agree with each other** wherever both answer.

#![no_main]
#![expect(
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `panic!` is how a violated one reaches libFuzzer, and each message here names the property"
)]

use libfuzzer_sys::fuzz_target;
use pdf_model::image::banded_decodes;

fuzz_target!(|data: &[u8]| {
    let Some((&head, codestream)) = data.split_first() else {
        return;
    };
    let band_lines = u32::from(head % 8).saturating_add(1).saturating_mul(8);
    let decodes = banded_decodes(codestream, band_lines);
    for (name, banded) in [
        ("restart intervals", &decodes.at_restarts),
        ("entropy pass rows", &decodes.at_rows),
    ] {
        let Some(banded) = banded else {
            continue;
        };
        let Some(whole) = &decodes.whole else {
            panic!(
                "bands at the {name} decoded a frame of {} bytes the whole decoder refuses",
                banded.len()
            );
        };
        assert!(
            banded == whole,
            "bands of {band_lines} lines at the {name} moved a byte: {} against {} bytes, first \
             difference at {:?}",
            banded.len(),
            whole.len(),
            banded.iter().zip(whole).position(|(a, b)| a != b)
        );
    }
    if let (Some(a), Some(b)) = (&decodes.at_restarts, &decodes.at_rows) {
        assert!(a == b, "the two band plans disagree");
    }
});
