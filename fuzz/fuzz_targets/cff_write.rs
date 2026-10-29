//! Fuzzes `pdf_font::cff::with_glyphs_given`, the writer that grows a simple font's bare CFF
//! program by glyphs of its own for codes on `.notdef` (ISO 19005-2 section 6.2.11.8, ADR 2578).
//!
//! The program is a stream in the document, so its bytes are as untrusted as any others, and the
//! writer reads its INDEXes, its charset and `.notdef`'s charstring through every subroutine it
//! calls, then *writes* a CFF from them (Adobe Technical Note #5176, sections 5, 8 and 13). The
//! first two bytes state how many glyphs are asked for (modulo 300, so the writer's own bound of
//! 256 is crossed), the third shapes their advances and outlines; the rest is the program. Beyond
//! never panicking — overflow checks stay on in this profile — three properties are under test:
//!
//! - **The glyphs come after the program's own**: every glyph given is numbered past the glyphs
//!   the program held, in the order asked, and the program written holds exactly that many more.
//! - **The answer is a function of the bytes**: asked twice, the same program comes back.
//! - **A written program is a program this function writes again**: growing it by one more glyph
//!   succeeds, so the writer's output is never a CFF its own reader refuses.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_font::cff::{GivenOutline, GlyphToGive, glyph_count, with_glyphs_given};

/// The glyphs `shape` asks for: `count` of them, alternating outlines by its bits, at advances
/// spread over and past the range a charstring's width operand holds.
fn asked(count: usize, shape: u8) -> Vec<GlyphToGive> {
    (0..count)
        .map(|at| {
            let spread = i64::try_from(at)
                .unwrap_or(0)
                .saturating_mul(157)
                .saturating_add(i64::from(shape).saturating_mul(4099));
            GlyphToGive {
                draws: if (usize::from(shape) >> (at % 8)) & 1 == 1 {
                    GivenOutline::NotdefsOwn
                } else {
                    GivenOutline::Nothing
                },
                name: None,
                advance: spread.rem_euclid(80_000).saturating_sub(40_000),
            }
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let [high, low, shape, program @ ..] = data else {
        return;
    };
    let count = usize::from(u16::from_be_bytes([*high, *low])) % 300;
    let glyphs = asked(count, *shape);
    let Ok((written, added)) = with_glyphs_given(program, &glyphs) else {
        return;
    };
    let held = glyph_count(program);
    assert!(
        held.is_ok(),
        "a program the writer extended is one it could read: {held:?}"
    );
    let Ok(held) = held else {
        return;
    };
    assert_eq!(
        added.len(),
        glyphs.len(),
        "one glyph is given for each asked"
    );
    for (at, glyph) in added.iter().enumerate() {
        let expected = u64::from(held).saturating_add(u64::try_from(at).unwrap_or(u64::MAX));
        assert_eq!(
            u64::from(glyph.glyph),
            expected,
            "the glyphs follow the program's own, in order"
        );
    }
    let grown = u64::from(held).saturating_add(u64::try_from(glyphs.len()).unwrap_or(u64::MAX));
    assert_eq!(
        glyph_count(&written).map(u64::from).ok(),
        Some(grown),
        "the program written holds the glyphs given"
    );
    let again = with_glyphs_given(program, &glyphs);
    assert!(
        again.is_ok(),
        "the same bytes are answered the same: {again:?}"
    );
    let Ok(again) = again else {
        return;
    };
    assert_eq!(
        again.0, written,
        "the program written is a function of the bytes"
    );
    if held < u32::from(u16::MAX).saturating_sub(1) {
        let one_more = asked(1, *shape);
        if let Err(error) = with_glyphs_given(&written, &one_more) {
            // The width operand may refuse a glyph the first pass gave at another advance.
            // A program at the String INDEX's or the glyph count's 16-bit end is refused by name.
            let at_an_end = matches!(&error, pdf_font::cff::CffError::Malformed { detail }
                if detail.contains("as many"));
            assert!(
                at_an_end || matches!(error, pdf_font::cff::CffError::AdvanceNotRestatable { .. }),
                "the writer's own output is refused by its reader: {error:?}"
            );
        }
    }
});
