//! Fuzzes `pdf_font::type1::with_glyphs_given`, the writer that grows a simple font's Type 1
//! program by glyphs of its own for codes on `.notdef` (ISO 19005-2 section 6.2.11.8, ADR 2620).
//!
//! The program is a stream in the document, so its bytes are as untrusted as any others: the
//! writer decrypts its `eexec` section (Adobe Type 1 Font Format section 7.2), reads its
//! `CharStrings` dictionary and `.notdef`'s charstring under the Private dictionary's `lenIV`
//! (sections 2.4, 5.12 and 7.3), and *writes* the section again. The first byte states how many
//! glyphs are asked for (modulo 300, so the writer's own bound of 256 is crossed), the second
//! shapes their advances and outlines; the rest is the program, divided where its cleartext names
//! `eexec`, as `/Length1` and `/Length2` divide it. Beyond never panicking — overflow checks stay
//! on in this profile — four properties are under test:
//!
//! - **The glyphs come after the program's own**: every glyph given is numbered past the glyphs
//!   the program held, in the order asked.
//! - **The lengths divide what was written**: `/Length1` is the header's, unchanged, and the
//!   three sum to the program's length.
//! - **The answer is a function of the bytes**: asked twice, the same program comes back.
//! - **A written program is a program this function writes again**: growing it by one more glyph
//!   under the lengths it returned succeeds.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_font::cff::{GivenOutline, GlyphToGive};
use pdf_font::type1::with_glyphs_given;

/// `/Length1` and `/Length2` as a producer states them: the header up to the white space after
/// `eexec`, the rest the section.
fn lengths(program: &[u8]) -> Option<(usize, usize)> {
    // Positions inside `program`, a slice in memory, so no sum or difference here saturates.
    let at = program
        .windows(5)
        .position(|window| window == b"eexec")?
        .saturating_add(5);
    let white = program[at..]
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        .count();
    let length1 = at.saturating_add(white);
    Some((length1, program.len().saturating_sub(length1)))
}

fuzz_target!(|data: &[u8]| {
    let [count, shape, program @ ..] = data else {
        return;
    };
    let Some(divided) = lengths(program) else {
        return;
    };
    let glyphs: Vec<GlyphToGive> = (0..usize::from(*count) * 300 / 256)
        .map(|at| GlyphToGive {
            draws: if (usize::from(*shape) >> (at % 8)) & 1 == 1 {
                GivenOutline::NotdefsOwn
            } else {
                GivenOutline::Nothing
            },
            name: (at % 3 == 0).then(|| format!("g{at}")),
            // Both factors are below 300, so the sum is far inside an i64.
            advance: i64::from(*shape)
                .saturating_mul(37)
                .saturating_add(i64::try_from(at).unwrap_or(0).saturating_mul(101))
                % 2000,
        })
        .collect();
    let Ok(given) = with_glyphs_given(program, divided, &glyphs) else {
        return;
    };
    assert_eq!(given.added.len(), glyphs.len());
    let first = given.added.first().map(|glyph| glyph.glyph);
    for (at, glyph) in given.added.iter().enumerate() {
        assert_eq!(
            Some(glyph.glyph),
            first.and_then(|first| first.checked_add(u16::try_from(at).ok()?)),
            "the glyphs are numbered in the order asked"
        );
    }
    assert_eq!(given.lengths[0], divided.0, "the header is copied");
    assert_eq!(given.program[..divided.0], program[..divided.0]);
    assert_eq!(given.lengths.iter().sum::<usize>(), given.program.len());
    let again = with_glyphs_given(program, divided, &glyphs);
    assert_eq!(again.as_ref().ok(), Some(&given), "a function of the bytes");
    let more = GlyphToGive {
        draws: GivenOutline::Nothing,
        name: None,
        advance: 500,
    };
    // One more glyph is asked for only where the bounds leave room for it.
    let room = given.program.len().saturating_add(4096) < pdf_font::type1::MOST_BYTES_WRITTEN;
    if glyphs.len() < pdf_font::cff::MOST_GLYPHS_GIVEN && room {
        let grown = with_glyphs_given(
            &given.program,
            (given.lengths[0], given.lengths[1]),
            &[more],
        );
        assert!(
            grown.is_ok(),
            "the written program is written again: {grown:?}"
        );
    }
});
