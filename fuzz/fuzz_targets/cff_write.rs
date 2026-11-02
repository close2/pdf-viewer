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
//!
//! A CID-keyed program goes to `pdf_font::cff::with_cids_given` instead (ADR 2598), which restates
//! the charset, the `FDSelect` and the `FDArray` for the CIDs asked for (modulo 300 again, against
//! the writer's byte bound), under the same three properties.
//!
//! Every program, name-keyed or CID-keyed, also goes to `pdf_font::cff::with_advances`, the
//! writer that restates glyphs' advances, with up to sixteen glyphs asked for and their advances
//! shaped by the third byte. Its own property is the one the DICT operand rule exists for (ADR
//! 2610): **the reader reads what the writer wrote** — `charstring_widths` finds every glyph asked
//! for at the advance asked and every other at the advance it had, so a writer that followed a
//! DICT entry otherwise than `read-fonts` does shows up as a width the reader does not find.

#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use pdf_font::cff::{
    CidToGive, GivenOutline, GlyphToGive, charstring_widths, glyph_count, uses_cid_operators,
    with_advances, with_cids_given, with_glyphs_given,
};

/// The most glyphs whose widths are read back, so that a program of 65 535 glyphs costs an
/// execution no more than one of 512.
const READ_BACK: u16 = 512;

/// A program through `with_advances`: the reader reads what the writer wrote, the answer is a
/// function of the bytes, and the program written is one the writer restates again.
fn advances_restated(program: &[u8], count: usize, shape: u8) {
    let Ok(held) = glyph_count(program) else {
        return;
    };
    let held = u16::try_from(held).unwrap_or(u16::MAX);
    if held == 0 {
        return;
    }
    let asked: BTreeMap<u16, i64> = (0..count.min(16))
        .map(|at| {
            let at = u32::try_from(at).unwrap_or(0);
            let glyph = at
                .saturating_mul(u32::from(shape) | 1)
                .checked_rem(u32::from(held))
                .unwrap_or(0);
            let advance = i64::from(shape)
                .saturating_mul(37)
                .saturating_add(i64::from(at).saturating_mul(101))
                % 2000;
            (u16::try_from(glyph).unwrap_or(0), advance)
        })
        .collect();
    let Ok(written) = with_advances(program, &asked) else {
        return;
    };
    let glyphs: Vec<u16> = (0..held.min(READ_BACK))
        .chain(asked.keys().copied())
        .collect();
    let before = charstring_widths(program, &glyphs);
    let after = charstring_widths(&written, &glyphs);
    assert!(
        after.is_ok(),
        "the program written is one the reader opens: {after:?}"
    );
    if let (Ok(before), Ok(after)) = (before, after) {
        for ((glyph, had), has) in glyphs.iter().zip(before).zip(after) {
            // A glyph the reader could not evaluate in the program given is not the writer's.
            if had.is_none() {
                continue;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "an advance asked for is below 2000, which an f32 holds exactly"
            )]
            let expected = asked
                .get(glyph)
                .map_or(had, |advance| Some(*advance as f32));
            assert_eq!(
                has, expected,
                "glyph {glyph} is read at the advance the writer was asked to state, or kept"
            );
        }
    }
    let again = with_advances(program, &asked);
    assert!(
        again.as_ref().is_ok_and(|again| *again == written),
        "the program written is a function of the bytes: {again:?}"
    );
    let restated = with_advances(&written, &asked);
    assert!(
        restated.is_ok(),
        "the writer's own output is refused by its reader: {restated:?}"
    );
}

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

/// The CIDs `shape` asks for: `count` distinct ones, spread over the 16-bit range from `first`.
fn cids_asked(count: usize, shape: u8, first: u16) -> Vec<CidToGive> {
    (0..count)
        .map(|at| {
            let step = u32::from(shape).saturating_mul(2).saturating_add(1);
            let spread = u32::try_from(at).unwrap_or(0).saturating_mul(step);
            let cid = u32::from(first).saturating_add(spread) % 0xFFFF;
            CidToGive {
                cid: u16::try_from(cid).unwrap_or(u16::MAX),
                advance: i64::from(shape).saturating_mul(613).rem_euclid(3000),
            }
        })
        .collect()
}

/// A CID-keyed program through `with_cids_given`, under the target's three properties.
fn cid_keyed(program: &[u8], count: usize, shape: u8) {
    let cids = cids_asked(count, shape, 1);
    let Ok((written, added)) = with_cids_given(program, &cids) else {
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
    for (at, glyph) in added.iter().enumerate() {
        let expected = u64::from(held).saturating_add(u64::try_from(at).unwrap_or(u64::MAX));
        assert_eq!(
            u64::from(*glyph),
            expected,
            "the glyphs follow the program's own, in order"
        );
    }
    let grown = u64::from(held).saturating_add(u64::try_from(cids.len()).unwrap_or(u64::MAX));
    assert_eq!(
        glyph_count(&written).map(u64::from).ok(),
        Some(grown),
        "the program written holds the glyphs given"
    );
    let again = with_cids_given(program, &cids);
    assert!(
        again.as_ref().is_ok_and(|again| again.0 == written),
        "the program written is a function of the bytes: {again:?}"
    );
    // One more CID, past every CID asked for, which the charset may hold already.
    let one_more = cids_asked(1, shape, 0xFFFE);
    if let Err(error) = with_cids_given(&written, &one_more) {
        let by_name = matches!(&error, pdf_font::cff::CffError::Malformed { detail }
            if detail.contains("charset holds") || detail.contains("INDEX counts")
                || detail.contains("bound"));
        assert!(
            by_name || matches!(error, pdf_font::cff::CffError::AdvanceNotRestatable { .. }),
            "the writer's own output is refused by its reader: {error:?}"
        );
    }
}

fuzz_target!(|data: &[u8]| {
    let [high, low, shape, program @ ..] = data else {
        return;
    };
    advances_restated(
        program,
        usize::from(u16::from_be_bytes([*high, *low])) % 300,
        *shape,
    );
    if uses_cid_operators(program).unwrap_or(false) {
        cid_keyed(
            program,
            usize::from(u16::from_be_bytes([*high, *low])) % 300,
            *shape,
        );
        return;
    }
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
