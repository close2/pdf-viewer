//! Fuzzes `viewer_core::find_in_text`: a needle typed into the find bar matched against a page's
//! readback, which folds presentation forms (ISO 32000-2 §9.10.2 hands a `/ToUnicode` that names
//! them through as stated), compares canonical decompositions, and lets a needle typed without
//! marks find a word printed with them (ADRs 1465, 1477).
//!
//! The readback is the document's to state, so every character, mark run and stray combining
//! sequence in it is a producer's. The first byte states the needle's length in bytes (modulo
//! 32); the needle and the readback are read lossily so that every input is one. This target is
//! the one that reaches `pdf_font::shaping::{fold, decompose, mark_class}` with untrusted text —
//! the `shaping` target reaches `Label` and the joining and bidirectional halves, and these three
//! are table lookups that only `find` calls.
//!
//! Beyond never panicking — overflow checks stay on in this profile — three properties:
//!
//! - **A match is a range of the readback**: non-empty, inside it, on character boundaries.
//! - **Matches are reported once, in order**: each starts at or after the previous one's end.
//! - **A needle the readback holds literally is found there**: where the needle is one or more
//!   lowercase ASCII letters and digits and occurs in the readback byte for byte, a match overlaps
//!   that occurrence — folding may widen what is found, and must never lose what is plainly there.

#![no_main]

use libfuzzer_sys::fuzz_target;
use viewer_core::find_in_text;

fuzz_target!(|data: &[u8]| {
    let Some((&head, rest)) = data.split_first() else {
        return;
    };
    let Some((needle, text)) = rest.split_at_checked(usize::from(head % 32).min(rest.len())) else {
        return;
    };
    let needle = String::from_utf8_lossy(needle);
    let text = String::from_utf8_lossy(text);
    let found = find_in_text(&text, &needle);
    let mut end = 0;
    for &(from, to) in &found {
        assert!(
            from < to && to <= text.len(),
            "a match is a non-empty range of the readback: {from}..{to} of {}",
            text.len()
        );
        assert!(
            text.is_char_boundary(from) && text.is_char_boundary(to),
            "a match starts and ends on character boundaries: {from}..{to}"
        );
        assert!(
            from >= end,
            "matches are reported once, in order: {found:?}"
        );
        end = to;
    }
    let literal = !needle.is_empty()
        && needle
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if literal {
        for (at, _) in text.match_indices(needle.as_ref()) {
            let until = at.saturating_add(needle.len());
            assert!(
                found.iter().any(|&(from, to)| from < until && at < to),
                "{needle:?} occurs at {at} in the readback and no match overlaps it: {found:?}"
            );
        }
    }
});
