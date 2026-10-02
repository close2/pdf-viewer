//! Fuzzes `pdf_font::embed::for_embedding`: §9.9.1's `/FontFile2` program, §9.9.2's subset, and
//! the `CFF ` subsetter with its charstring walk (ADRs 1425, 1438, 1449).
//!
//! The face is the machine's rather than the document's — a layout that sets a value no
//! compiled-in face can draw sets it in a face the system offers (ADR 1414) — so its bytes are
//! not an attacker's in the way a stream's are. They are still bytes this program did not write:
//! a damaged font file in a system directory is read by the same walk, and the `CFF ` subsetter's
//! charstring interpreter follows `callsubr` and `callgsubr` through indices the face states.
//!
//! The first byte states how many glyph indices follow (modulo 64), each two bytes big-endian;
//! the rest is the face. Beyond never panicking — overflow checks stay on in this profile — four
//! properties are under test:
//!
//! - **The kept glyphs are renumbered densely**: a written program's glyphs map onto `0 .. n`,
//!   each new index once, and glyph 0 — §9.9.2's `.notdef` — is kept at 0.
//! - **A subset is named as §9.9.2 names one**: six uppercase letters and a plus sign on every
//!   subset — a `glyf` face whose licence permits one, a `CFF ` face written as a bare CID-keyed
//!   program — and the bare PostScript name on a face written whole.
//! - **The answer is a function of the bytes**: asked twice, the same program comes back.
//! - **A written `/FontFile2` program is a program this function writes again**: re-embedding it
//!   by every glyph it kept succeeds and renumbers nothing, so a subset is closed under
//!   subsetting — a component a composite names was kept with it.

#![no_main]
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use std::collections::BTreeSet;

use libfuzzer_sys::fuzz_target;
use pdf_font::embed::{Outlines, for_embedding, permission};

fuzz_target!(|data: &[u8]| {
    let Some((&count, rest)) = data.split_first() else {
        return;
    };
    let count = usize::from(count % 64);
    let Some((indices, program)) = rest.split_at_checked(count.saturating_mul(2)) else {
        return;
    };
    let used: BTreeSet<u16> = indices
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    let Ok(embedded) = for_embedding(program, &used) else {
        return;
    };

    let mut renumbered: Vec<u16> = embedded.glyphs.values().copied().collect();
    renumbered.sort_unstable();
    assert!(
        renumbered
            .iter()
            .copied()
            .eq(0..u16::try_from(renumbered.len()).expect("at most 65536 glyphs")),
        "the kept glyphs are renumbered onto 0 .. n, each once: {renumbered:?}"
    );
    assert_eq!(
        embedded.glyphs.get(&0),
        Some(&0),
        "glyph 0, §9.9.2's .notdef, is kept at 0"
    );

    // A `glyf` face is subset wherever its licence permits; a `CFF ` one is subset as a bare
    // CID-keyed program where the closure can be taken, and otherwise written whole under
    // `OpenType` with no tag (ADRs 1438, 1449).
    let subset = match embedded.outlines {
        Outlines::TrueType => permission(program).subsetting,
        Outlines::CompactCid { .. } => true,
        Outlines::Cff { .. } => false,
    };
    let tagged = embedded
        .name
        .split_once('+')
        .is_some_and(|(tag, _)| tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()));
    assert_eq!(
        tagged, subset,
        "a subset is named with §9.9.2's six-letter tag, and only a subset: {:?}",
        embedded.name
    );

    assert_eq!(
        for_embedding(program, &used).as_ref(),
        Ok(&embedded),
        "the written program is a function of the face and the glyphs"
    );

    if embedded.outlines == Outlines::TrueType {
        let kept: BTreeSet<u16> = embedded.glyphs.values().copied().collect();
        let again = for_embedding(&embedded.program, &kept)
            .unwrap_or_else(|refusal| panic!("a written /FontFile2 program is refused: {refusal}"));
        assert!(
            !again.renumbered(),
            "re-embedding a subset by every glyph it kept renumbers nothing"
        );
        assert_eq!(
            again.glyphs.len(),
            embedded.glyphs.len(),
            "re-embedding a subset by every glyph it kept keeps exactly those"
        );
    }
});
