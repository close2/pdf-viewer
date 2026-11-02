//! The `cff_write` fuzz target's stops that remained after the DICT operand rule (ADR 2610), one
//! minimised test each: offsets a rewrite would shift onto bytes the reader never read, and a
//! charstring that pops an empty operand stack after the width written in front of it, a
//! subroutine spliced in past its first `return`, the reserved operator 12 6, a width that would
//! be a forty-ninth operand, and a Local Subr INDEX ending past the program (ADR 2628).
#![expect(
    clippy::arithmetic_side_effects,
    reason = "fixtures over a font of two glyphs: every quantity is a few dozen"
)]

use std::collections::BTreeMap;

use super::font_dict_repair::cid_keyed;
use super::operand_tests::name_keyed;
use super::{
    CHARSTRING_OPERATIONS, CffError, CidToGive, GivenOutline, GlyphToGive, Layout, Misstated,
    NotRestated, OP_CHARSET, Subroutines, calls_local_subr, charstring_with_advance, dict_entries,
    dict_int, first_reserved_seac, glyph_count, index_items, int5, leading_operands, open,
    pops_an_empty_stack, subroutine_body, with_advances, with_cids_given, with_glyphs_given,
};

/// The Private DICT every fixture here holds: `nominalWidthX 20`.
const PRIVATE: [u8; 3] = [0xf7, 0xc0, 20];

/// `program` with its Top DICT's five-byte `Private` operands restated as `size` and `at`.
fn private_restated(program: &[u8], size: i64, at: i64) -> Vec<u8> {
    let private_at = i64::try_from(program.len() - PRIVATE.len()).expect("a fixture offset");
    let mut needle = int5(3).to_vec();
    needle.extend_from_slice(&int5(private_at));
    needle.push(18);
    let found = program
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the Private entry's two operands");
    let mut out = program.to_vec();
    out[found..found + 5].copy_from_slice(&int5(size));
    out[found + 5..found + 10].copy_from_slice(&int5(at));
    out
}

/// A glyph asked for under a name no string holds, so that the String INDEX is restated.
fn one_glyph() -> Vec<GlyphToGive> {
    vec![GlyphToGive {
        draws: GivenOutline::Nothing,
        name: Some("given.one".to_owned()),
        advance: 500,
    }]
}

/// A `Private` of 64 bytes at offset -48 (ADR 2628, decision 1). `read-fonts` drops the entry,
/// its end overflowing, and a rewrite that shifted it past zero named the Top DICT's own bytes as
/// a Private DICT.
#[test]
fn a_private_below_zero_is_refused_by_name() {
    let program = private_restated(&name_keyed(&PRIVATE), 64, -48);
    assert!(glyph_count(&program).is_ok(), "the reader drops the entry");
    assert_eq!(
        Layout::read(&program).and_then(|layout| layout.misstated),
        Some(Misstated::Outside {
            dict: "Top",
            operator: "Private"
        })
    );
    let refused = |result: &Result<Vec<u8>, CffError>| {
        matches!(result, Err(CffError::Malformed { detail })
            if detail.contains("Private offset points before the program's start"))
    };
    assert!(refused(&with_advances(
        &program,
        &BTreeMap::from([(1, 500)])
    )));
    assert!(refused(
        &with_glyphs_given(&program, &one_glyph()).map(|(written, _)| written)
    ));
}

/// A `Private` of negative size (ADR 2628, decision 2), which the reader drops or refuses by
/// where its offset sits, is refused by name whatever the offset.
#[test]
fn a_private_of_negative_size_is_refused_by_name() {
    let program = name_keyed(&PRIVATE);
    let private_at = i64::try_from(program.len() - PRIVATE.len()).expect("a fixture offset");
    let negative = private_restated(&program, -1, private_at);
    assert_eq!(
        Layout::read(&negative).and_then(|layout| layout.misstated),
        Some(Misstated::NegativeSize { dict: "Top" })
    );
    assert!(matches!(
        with_advances(&negative, &BTreeMap::from([(1, 500)])),
        Err(CffError::Malformed { detail }) if detail.contains("states a negative size")
    ));
}

/// A `Private` inside the String INDEX (ADR 2628, decision 3). A writer that adds strings re-encodes that INDEX, so it refuses the offset by name; a
/// writer that copies it shifts the offset with the bytes and goes on.
#[test]
fn a_private_inside_the_string_index_is_refused_where_the_index_is_restated() {
    let program = name_keyed(&PRIVATE);
    let strings_at =
        i64::try_from(Layout::read(&program).expect("a layout").top_index.end).expect("small");
    let inside = private_restated(&program, 2, strings_at);
    assert_eq!(
        Layout::read(&inside).and_then(|layout| layout.misstated),
        None
    );
    assert!(matches!(
        with_glyphs_given(&inside, &one_glyph()),
        Err(CffError::Malformed { detail })
            if detail.contains("Private offset points into the String INDEX")
    ));
    assert!(with_advances(&inside, &BTreeMap::from([(1, 500)])).is_ok());
}

/// An operator that takes operands (ADR 2628, decision 4), stated with the stack empty after the
/// first operator, reads the bottom slot the width is written into.
#[test]
fn an_operator_on_an_empty_stack_after_the_first_is_found() {
    // 0 0 rmoveto callsubr
    assert!(pops_an_empty_stack(&[139, 139, 21, 10]));
    // 0 0 rmoveto 0 callsubr: the operand overwrites the bottom slot first.
    assert!(!pops_an_empty_stack(&[139, 139, 21, 139, 10]));
    // 0 1 hstem hintmask <one byte> seac: the mask is passed over, the escape found.
    assert!(pops_an_empty_stack(&[139, 140, 1, 19, 0x80, 12, 6]));
    // 0 0 rmoveto endchar
    assert!(!pops_an_empty_stack(&[139, 139, 21, 14]));
}

/// The refusal reaches every writer through [`charstring_with_advance`], by its own name.
#[test]
fn a_width_before_an_empty_stack_pop_is_not_written() {
    let none = Subroutines {
        global: &[],
        local: &[],
    };
    assert_eq!(
        charstring_with_advance(&[139, 139, 21, 10], 500, 0, &none),
        Err(NotRestated::EmptyStack)
    );
    assert!(matches!(
        NotRestated::EmptyStack.refusal(1, "placement"),
        CffError::AdvanceNotRestatable { glyph: 1, detail } if detail.contains("operand stack empty")
    ));
    assert!(charstring_with_advance(&[139, 139, 21, 139, 10], 500, 0, &none).is_ok());
}

/// A subroutine's body ends at its first `return` (ADR 2628, decision 5): the bytes after it are
/// never run, and a width restated through the call at the head splices in none of them.
#[test]
fn a_subroutine_is_spliced_up_to_its_first_return() {
    // 0 0 rmoveto return 0 0 rmoveto return
    let subr = [139, 139, 21, 11, 139, 139, 21, 11];
    assert_eq!(subroutine_body(&subr), Some(3));
    assert_eq!(subroutine_body(&[139, 139, 21]), Some(3));
    assert_eq!(subroutine_body(&[]), None);
    let local = [subr.to_vec()];
    let subrs = Subroutines {
        global: &[],
        local: &local,
    };
    // -107 callsubr endchar: the bias of a small INDEX makes -107 subroutine 0.
    let restated = charstring_with_advance(&[32, 10, 14], 500, 0, &subrs).expect("restated");
    assert_eq!(restated, [28, 1, 244, 139, 139, 21, 14]);
}

/// A glyph that reaches 12 6, itself or through a call, or whose call cannot be told, is the
/// glyph [`first_reserved_seac`] names (ADR 2628, decision 6); a mask byte of 12 is data.
#[test]
fn the_reserved_seac_is_found_where_the_reader_would_run_it() {
    let local = [vec![12, 6]];
    let subrs = || Subroutines {
        global: &[],
        local: &local,
    };
    let found = |charstrings: &[Vec<u8>]| first_reserved_seac(charstrings, |_| subrs());
    // 0 0 0 0 seac, glyph 1.
    assert_eq!(found(&[vec![14], vec![139, 139, 139, 139, 12, 6]]), Some(1));
    // -107 callsubr, the subroutine stating 12 6.
    assert_eq!(found(&[vec![14], vec![32, 10]]), Some(1));
    // 0 0 rmoveto callsubr: the call's operand is a stale slot.
    assert_eq!(found(&[vec![139, 139, 21, 10]]), Some(0));
    // 0 1 hstem hintmask <12> hlineto endchar: the mask byte is not an escape.
    assert_eq!(found(&[vec![139, 140, 1, 19, 12, 6, 14]]), None);
    // 0 hsbw callsubr: Type 1's `hsbw` may leave its operand, so the call cannot be told.
    assert_eq!(found(&[vec![139, 13, 10]]), Some(0));
}

/// A Font DICT's `Private` whose bytes run past the program's end (ADR 2628, decision 7):
/// whether a rewrite's output holds the bytes it states turns on the output's length, so a
/// second rewrite read a Private DICT the first had not, and it is refused by name.
#[test]
fn a_private_ending_past_the_program_is_refused_by_name() {
    let program = cid_keyed(&[14], None, true);
    // The fixture's one Font DICT states `0 <offset> Private` in five-byte operands.
    let at = i64::try_from(program.len()).expect("a fixture offset");
    let found = program
        .windows(11)
        .position(|window| window[..5] == int5(0) && window[5] == 29 && window[10] == 18)
        .expect("the Font DICT's Private entry");
    let mut past = program.clone();
    past[found..found + 5].copy_from_slice(&int5(64));
    past[found + 5..found + 10].copy_from_slice(&int5(at - 16));
    assert!(glyph_count(&past).is_ok(), "the reader opens the program");
    assert_eq!(
        Layout::read(&past).and_then(|layout| layout.misstated),
        Some(Misstated::Outside {
            dict: "Font",
            operator: "Private"
        })
    );
    let refused = with_advances(&past, &BTreeMap::from([(1, 500)]));
    assert!(
        matches!(&refused, Err(CffError::Malformed { detail })
            if detail.contains("Private DICT ending past it")),
        "{refused:?}"
    );
    let mut held = program;
    held[found..found + 5].copy_from_slice(&int5(16));
    held[found + 5..found + 10].copy_from_slice(&int5(at - 16));
    assert_eq!(
        Layout::read(&held).and_then(|layout| layout.misstated),
        None
    );
}

/// A charstring whose first operator takes the 48 operands Appendix B allows and states no width
/// (ADR 2628, decision 8): a width written in front would be a forty-ninth operand, which the walk
/// over the output reads as a point it cannot tell past, so the writer refuses it by name.
#[test]
fn a_width_that_would_be_a_forty_ninth_operand_is_not_written() {
    let none = || Subroutines {
        global: &[],
        local: &[],
    };
    // 48 zeros hstem endchar: 24 stems, no width.
    let mut full = vec![139; 48];
    full.extend_from_slice(&[1, 14]);
    assert_eq!(leading_operands(&full), 48);
    assert_eq!(first_reserved_seac(&[full.clone()], |_| none()), None);
    assert_eq!(
        charstring_with_advance(&full, 500, 0, &none()),
        Err(NotRestated::FullStack)
    );
    let mut written = vec![28, 1, 244];
    written.extend_from_slice(&full);
    assert_eq!(first_reserved_seac(&[written], |_| none()), Some(0));
    assert!(matches!(
        NotRestated::FullStack.refusal(0, "placement"),
        CffError::AdvanceNotRestatable { glyph: 0, detail } if detail.contains("forty-ninth")
    ));
    // 46 zeros hstem endchar: the width makes 47 operands, and is written.
    let fits = [&[139; 46][..], &[1, 14]].concat();
    assert!(charstring_with_advance(&fits, 500, 0, &none()).is_ok());
}

/// A Private DICT's `Subrs` whose INDEX ends past the program (ADR 2628, decision 9): the walk
/// over the input read no subroutines and the walk over a longer output read them, so it is
/// refused by name; an INDEX the program holds is read.
#[test]
fn a_local_subr_index_ending_past_the_program_is_refused_by_name() {
    // nominalWidthX 20, Subrs 5: the INDEX would start where the program ends.
    let past = name_keyed(&[0xf7, 0xc0, 20, 144, 19]);
    assert_eq!(
        Layout::read(&past).and_then(|layout| layout.misstated),
        Some(Misstated::SubrsUnread)
    );
    let refused = with_advances(&past, &BTreeMap::from([(1, 500)]));
    assert!(
        matches!(&refused, Err(CffError::Malformed { detail })
            if detail.contains("Local Subr INDEX")),
        "{refused:?}"
    );
    // The same Private with an INDEX of no subroutines after it.
    let mut held = past;
    held.extend_from_slice(&[0, 0]);
    assert_eq!(
        Layout::read(&held).and_then(|layout| layout.misstated),
        None
    );
    assert!(with_advances(&held, &BTreeMap::from([(1, 500)])).is_ok());
}

/// The `charset` offset `program`'s first Font DICT states, where it states one.
fn font_dict_charset(program: &[u8]) -> Option<i64> {
    let at = Layout::read(program)?.fd_array_at?;
    let dict = *index_items(program, at)?.first()?;
    dict_entries(dict)?
        .iter()
        .find(|(_, op)| *op == OP_CHARSET)
        .and_then(|(operands, _)| dict_int(operands))
}

/// A Font DICT that states a `charset` past the String INDEX, as artifact `64656d0d`'s copy of its
/// Top DICT did (ADR 2648): the writer that restates the String INDEX shifts it with the bytes it
/// names, so the output's Font DICT names its input's charset, and the output is one the writer
/// extends again rather than refusing an offset left inside the re-encoded String INDEX.
#[test]
fn a_font_dict_offset_is_shifted_with_the_bytes_it_names() {
    let program = cid_keyed(&[14], None, true);
    let layout = Layout::read(&program).expect("the fixture's layout");
    let fd_array_at = layout.fd_array_at.expect("the fixture's FDArray");
    let charset_at = layout
        .entries
        .iter()
        .find(|(_, op)| *op == OP_CHARSET)
        .and_then(|(operands, _)| dict_int(operands))
        .expect("the fixture's charset");
    // The fixture's FDArray ends the program: an INDEX of one Font DICT, `0 <offset> Private`,
    // restated with a `charset` that names the Top DICT's own.
    let mut font_dict = int5(0).to_vec();
    font_dict.extend_from_slice(&int5(i64::try_from(fd_array_at).expect("a fixture offset")));
    font_dict.push(18);
    font_dict.extend_from_slice(&int5(charset_at));
    font_dict.push(15);
    let mut stated = program[..fd_array_at].to_vec();
    stated.extend_from_slice(&[
        0,
        1,
        1,
        1,
        1 + u8::try_from(font_dict.len()).expect("small"),
    ]);
    stated.extend_from_slice(&font_dict);
    assert!(glyph_count(&stated).is_ok(), "the reader opens the program");
    assert_eq!(font_dict_charset(&stated), Some(charset_at));
    assert_eq!(
        Layout::read(&stated).and_then(|layout| layout.misstated),
        None
    );
    let named = |program: &[u8], at: i64| {
        let at = usize::try_from(at).expect("an offset the program holds");
        program[at..at + 3].to_vec()
    };

    let (written, _) = with_cids_given(
        &stated,
        &[CidToGive {
            cid: 9,
            advance: 500,
        }],
    )
    .expect("the first rewrite");
    let shifted = font_dict_charset(&written).expect("the output's Font DICT states a charset");
    assert_ne!(shifted, charset_at, "the String INDEX grew under it");
    assert_eq!(named(&written, shifted), named(&stated, charset_at));
    assert_eq!(
        Layout::read(&written).and_then(|layout| layout.misstated),
        None
    );
    let again = with_cids_given(
        &written,
        &[CidToGive {
            cid: 10,
            advance: 500,
        }],
    );
    assert!(
        again.is_ok(),
        "the writer extends its own output: {again:?}"
    );
}

/// Whether glyph 1 of a CID-keyed fixture whose one global subroutine is `gsubr` reaches a
/// `callsubr`, by [`calls_local_subr`]'s walk.
fn glyph_calls_local_subr(glyph: &[u8], gsubr: &[u8]) -> bool {
    let program = cid_keyed(glyph, Some(gsubr), true);
    let font = open(&program).expect("the fixture opens");
    let glyph = font.charstrings().get(1).expect("the fixture's glyph");
    calls_local_subr(glyph, &font, 0, &mut { CHARSTRING_OPERATIONS })
}

/// A `hintmask` after a global subroutine that declares the stems (ADR 2649): Technical Note
/// #5177 section 4.7 keeps one argument stack across the call and section 4.3 sizes the mask by
/// every stem declared, so nine stems make the mask two bytes, and the `callsubr` after it is
/// reached.
/// Counted from no stems in the caller, the mask was no bytes, its first byte read as a 32-bit
/// operand, and the `callsubr` it swallowed was missed.
#[test]
fn a_mask_after_a_global_subroutine_counts_the_stems_it_declared() {
    // Eighteen zeros hstemhm return: nine stems.
    let stems = [&[139; 18][..], &[18, 11]].concat();
    // -107 callgsubr hintmask <0xff 0x80> -107 callsubr endchar.
    let glyph = [32, 29, 19, 0xff, 0x80, 32, 10, 14];
    assert!(glyph_calls_local_subr(&glyph, &stems));
    // The same mask with no `callsubr` after it is not counted as one.
    assert!(!glyph_calls_local_subr(
        &[32, 29, 19, 0xff, 0x80, 14],
        &stems
    ));
    // The stems stated in the caller and the operands the subroutine leaves count together:
    // two stems in the caller, fifteen by the subroutine's thirty operands and the `hstemhm` the
    // caller states after it, so the mask is three bytes; the two of the caller's stems alone
    // would make it one, and its second byte a 32-bit operand swallowing the `callsubr`.
    let operands = [&[139; 30][..], &[11]].concat();
    let split = [
        &[139; 4][..],
        &[18, 32, 29, 18, 19, 0xff, 0xff, 0x80, 32, 10, 14],
    ]
    .concat();
    assert!(glyph_calls_local_subr(&split, &operands));
}

/// An `endchar` in a global subroutine ends the glyph (Technical Note #5177 section 4.2), so a
/// `callsubr` the caller states after the call is never run and is not counted as reached.
#[test]
fn a_callsubr_after_a_subroutine_s_endchar_is_not_reached() {
    // -107 callgsubr -107 callsubr endchar, the subroutine `endchar`.
    assert!(!glyph_calls_local_subr(&[32, 29, 32, 10, 14], &[14]));
    // The subroutine returning instead, the `callsubr` is reached.
    assert!(glyph_calls_local_subr(&[32, 29, 32, 10, 14], &[11]));
}
