//! The one operand rule the DICT readers and every CFF writer here share (ADR 2610): an entry a
//! writer follows is stated once and with the operands Adobe Technical Note #5176's Tables 9, 10
//! and 23 give it, and a program stating one otherwise is refused by name on every writer path
//! rather than read at its first operands — `read-fonts` reads a one-operand entry's last.

use std::collections::BTreeMap;

use super::font_dict_repair::{cid_keyed, index};
use super::{
    CffError, GivenOutline, GlyphToGive, Layout, Misstated, OP_CHARSTRINGS, OP_FD_ARRAY, Parts,
    charstring_widths, dict_entries, dict_int, dict_two_ints, glyph_count, int5, misstated_entry,
    operand_count, readable_font_dicts, with_advances, with_empty_glyphs, with_glyphs_given,
};

/// The `cff_write` fuzz target's first finding, minimised: a Top DICT whose `CharStrings` entry
/// carries three operands. `read-fonts` reads the last and finds an INDEX of no glyph; the first
/// names an INDEX of one. Its `Private` entry, earlier in the DICT, states one operand where
/// Table 9 gives two, and the refusal names the first entry misstated.
const CHARSTRINGS_OF_THREE: &[u8] = &[
    0x01, 0x00, 0x04, 0x01, 0x00, 0x01, 0x01, 0x01, 0x08, 0x48, 0x6f, 0x73, 0x74, 0x69, 0x6c, 0x65,
    0x00, 0x01, 0x01, 0x01, 0x10, 0xf8, 0x1d, 0x02, 0x8b, 0x8b, 0xef, 0xef, 0x05, 0xc3, 0x12, 0x0f,
    0xc7, 0x8b, 0xef, 0x11, 0x00, 0x03, 0x01, 0x01, 0x03, 0x05, 0x0c, 0x67, 0x31, 0x67, 0x32, 0x48,
    0x6f, 0x73, 0x74, 0x69, 0x5c, 0x65, 0x00, 0x00, 0x01, 0x01, 0x87, 0x01, 0x00, 0x01, 0x01, 0x10,
    0x1c, 0x1c, 0xdd, 0xff, 0xff, 0xff, 0x1c, 0x0d, 0x1d, 0x02, 0x8b, 0x8b, 0xef, 0xef, 0x05, 0xc3,
    0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a,
];

/// The fuzz target's second finding, minimised: a Top DICT whose `Private` entry carries five
/// operands.
const PRIVATE_OF_FIVE: &[u8] = &[
    0x01, 0x00, 0x04, 0x01, 0x00, 0x01, 0x01, 0x01, 0x08, 0x48, 0x6f, 0x73, 0x74, 0x69, 0x6c, 0x65,
    0x00, 0x01, 0x01, 0x01, 0x10, 0xd8, 0x02, 0x8b, 0x00, 0x00, 0x01, 0x01, 0x43, 0x43, 0x80, 0x8b,
    0xef, 0x12, 0xc7, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x6f, 0x74, 0x64, 0x65, 0x66, 0x2e,
    0x30, 0x39, 0x6e, 0x6f, 0x74, 0x64, 0x65, 0x66, 0x5b, 0x03, 0x55, 0x01, 0x00, 0x04, 0x01, 0x00,
    0x00, 0x01, 0x01, 0x01, 0x4e, 0x43, 0xff, 0xff, 0xff, 0x00, 0x0e, 0x0a,
];

/// Whether `result` is the named refusal of `operator` in a `dict` DICT.
fn refused<T: std::fmt::Debug>(result: &Result<T, CffError>, dict: &str, operator: &str) -> bool {
    matches!(result, Err(CffError::Malformed { detail })
        if detail.contains(&format!("the {dict} DICT states {operator} more than once")))
}

/// A name-keyed CFF of two glyphs — `.notdef`, stating no width, and glyph 1, stating a width
/// operand of 100 — whose Top DICT states `CharStrings` and `Private` in the five-byte form and
/// whose Private DICT is `private`, placed last.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_wrap,
    reason = "a fixture builder over a font of two glyphs: every quantity is a few dozen"
)]
pub(super) fn name_keyed(private: &[u8]) -> Vec<u8> {
    let header = [1u8, 0, 4, 4];
    let names = index(&[b"F"]);
    let empty = index(&[]);
    let charstrings = index(&[&[14], &[0xef, 14]]);
    // CharStrings, then Private: 6 + 11 bytes, in an INDEX of one item and one-byte offsets.
    let top_len = 17;
    let charstrings_at = header.len() + names.len() + 2 + 1 + 2 + top_len + 2 * empty.len();
    let private_at = charstrings_at + charstrings.len();
    let mut top = Vec::new();
    top.extend_from_slice(&int5(charstrings_at as i64));
    top.push(17);
    top.extend_from_slice(&int5(private.len() as i64));
    top.extend_from_slice(&int5(private_at as i64));
    top.push(18);
    assert_eq!(top.len(), top_len);
    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&names);
    out.extend_from_slice(&index(&[&top]));
    out.extend_from_slice(&empty);
    out.extend_from_slice(&empty);
    out.extend_from_slice(&charstrings);
    out.extend_from_slice(private);
    out
}

/// `program` with the five-byte operand of the entry `operator` states restated as three
/// operands in the same five bytes, the last of them the offset — so every offset stays true and
/// only the entry's operand count changes, which is the shape the fuzz target found.
fn three_operands(program: &[u8], operator: u16, operator_bytes: &[u8]) -> Vec<u8> {
    let value = Layout::read(program)
        .expect("a Top DICT")
        .entries
        .iter()
        .find(|(_, op)| *op == operator)
        .and_then(|(operands, _)| dict_int(operands))
        .expect("the entry, stated once");
    let mut needle = int5(value).to_vec();
    needle.extend_from_slice(operator_bytes);
    let at = program
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the entry's five-byte form");
    let short = i16::try_from(value)
        .expect("a fixture offset")
        .to_be_bytes();
    let mut out = program.to_vec();
    out[at..at.checked_add(5).expect("inside the program")]
        .copy_from_slice(&[0x8b, 0x8b, 28, short[0], short[1]]);
    out
}

/// An offset is read under the one rule: exactly the operands Table 9 gives it, each an integer,
/// or no value at all — never the first of several, never the last.
#[test]
fn an_offset_is_read_only_where_it_is_stated_with_its_own_operands() {
    assert_eq!(dict_int(&[0x8c]), Some(1));
    assert_eq!(dict_int(&[0x8b, 0x8c]), None, "two operands for one");
    assert_eq!(dict_int(&[]), None, "no operand");
    assert_eq!(dict_int(&[0x1e, 0x1a, 0x5f]), None, "1.5 is not an offset");
    assert_eq!(dict_two_ints(&[0x8c, 0x8d]), Some((1, 2)));
    assert_eq!(dict_two_ints(&[0x8c, 0x8d, 0x8e]), None, "three for two");
    assert_eq!(
        dict_two_ints(&[0x8c, 0x1e, 0x1a, 0x5f]),
        None,
        "a real size"
    );
}

/// Both minimised programs through every writer path: each refuses them and names the entry, and
/// a writer that never reaches a DICT for a name-keyed program says why it has nothing to do.
#[test]
fn the_fuzz_targets_two_findings_are_refused_by_name_on_every_writer_path() {
    assert_eq!(
        glyph_count(CHARSTRINGS_OF_THREE).ok(),
        Some(0),
        "the reader opens it, and finds no glyph"
    );
    assert!(glyph_count(PRIVATE_OF_FIVE).is_ok(), "the reader opens it");
    let one = GlyphToGive {
        draws: GivenOutline::NotdefsOwn,
        name: None,
        advance: 500,
    };
    let advance = BTreeMap::from([(0u16, 500i64)]);
    for (program, operator) in [
        (CHARSTRINGS_OF_THREE, "Private"),
        (PRIVATE_OF_FIVE, "Private"),
    ] {
        assert!(
            refused(
                &with_glyphs_given(program, std::slice::from_ref(&one)),
                "Top",
                operator
            ),
            "{operator}: the glyph writer"
        );
        assert!(
            refused(&with_empty_glyphs(program, &[500]), "Top", operator),
            "{operator}: the empty-glyph writer"
        );
        assert!(
            refused(&with_advances(program, &advance), "Top", operator),
            "{operator}: the advance writer"
        );
        assert!(
            Parts::read(program).is_none(),
            "{operator}: the embedding writer's reader"
        );
        assert_eq!(
            readable_font_dicts(program),
            Ok(None),
            "{operator}: a name-keyed program has no Font DICT to repair, so nothing is rewritten"
        );
    }
}

/// A CID-keyed program whose `FDArray` entry states three operands, the last the offset
/// `read-fonts` reads: the Font DICT repair, the advance writer and the embedding writer's reader
/// all refuse it, where each read the first operand — an offset of zero — before.
#[test]
fn an_fd_array_of_three_operands_is_refused_by_every_cid_keyed_writer() {
    let unreadable = three_operands(&cid_keyed(&[14], None, false), OP_FD_ARRAY, &[12, 36]);
    assert!(
        refused(&readable_font_dicts(&unreadable), "Top", "FDArray"),
        "{:?}",
        readable_font_dicts(&unreadable)
    );
    let readable = three_operands(&cid_keyed(&[14], None, true), OP_FD_ARRAY, &[12, 36]);
    assert_eq!(glyph_count(&readable), Ok(2), "the reader opens it");
    assert!(refused(
        &with_advances(&readable, &BTreeMap::from([(1u16, 500i64)])),
        "Top",
        "FDArray"
    ));
    assert!(Parts::read(&readable).is_none());
}

/// A Font DICT whose `Private` entry states six operands is refused by the advance writer and
/// the embedding writer's reader, which follow it to the Private DICT.
#[test]
fn a_font_dicts_private_of_six_operands_is_refused() {
    let program = cid_keyed(&[14], None, true);
    let at = program
        .windows(6)
        .position(|window| window == [29, 0, 0, 0, 0, 29])
        .expect("the Font DICT's Private size, then its offset");
    let mut six = program.clone();
    six[at..at.checked_add(5).expect("inside the program")].copy_from_slice(&[0x8b; 5]);
    assert!(refused(
        &with_advances(&six, &BTreeMap::from([(1u16, 500i64)])),
        "Font",
        "Private"
    ));
    assert!(Parts::read(&six).is_none());
    assert!(
        Parts::read(&program).is_some(),
        "the program as built reads"
    );
}

/// A Private DICT stating `nominalWidthX` twice is read by `read-fonts` at the second, so a
/// writer reckoning a width against the first would state another advance than it was asked
/// for; it is refused by name, as an over-long `Subrs` is.
#[test]
fn a_private_dict_entry_stated_twice_or_over_long_is_refused_by_name() {
    // nominalWidthX 100, then nominalWidthX 0.
    let twice = name_keyed(&[0xef, 21, 0x8b, 21]);
    assert_eq!(
        charstring_widths(&twice, &[1]),
        Ok(vec![Some(100.0)]),
        "the reader reckons glyph 1's operand of 100 against the second statement, 0"
    );
    let advance = BTreeMap::from([(1u16, 500i64)]);
    assert!(refused(
        &with_advances(&twice, &advance),
        "Private",
        "nominalWidthX"
    ));
    let subrs = name_keyed(&[0x8b, 0x8b, 19]);
    assert!(refused(
        &with_advances(&subrs, &advance),
        "Private",
        "Subrs"
    ));
    assert!(Parts::read(&subrs).is_none());
}

/// The `cff_write` fuzz target's `advances_restated` finding, minimised: a Private DICT holding
/// a byte section 4 reserves between two `nominalWidthX` entries. The reader passes over the
/// reserved byte and reckons widths against the second statement, 0; a writer that took the
/// DICT as unreadable and the width as the first statement, 100, wrote glyph 1's 500 as 400,
/// which the reader then found at 400. Such a DICT is refused by name on every writer path.
#[test]
fn a_private_dict_holding_a_reserved_byte_is_refused_by_name() {
    // nominalWidthX 100, the reserved byte 31, nominalWidthX 0.
    let reserved = name_keyed(&[0xef, 21, 0x8b, 31, 0x8b, 21]);
    assert_eq!(
        charstring_widths(&reserved, &[1]),
        Ok(vec![Some(100.0)]),
        "the reader reckons glyph 1's operand of 100 against the second statement, 0"
    );
    let untokenised = |result: &Result<(), CffError>| {
        matches!(result, Err(CffError::Malformed { detail })
            if detail.contains("a Private DICT the program names holds a byte"))
    };
    let advance = BTreeMap::from([(1u16, 500i64)]);
    assert!(untokenised(&with_advances(&reserved, &advance).map(|_| ())));
    assert!(untokenised(
        &with_empty_glyphs(&reserved, &[500]).map(|_| ())
    ));
    assert!(Parts::read(&reserved).is_none());
}

/// The `cff_write` fuzz target's second `advances_restated` finding, minimised: a charstring
/// whose first operator is `rmoveto` stated with one operand. Technical Note #5177 section 3.1
/// reads a width before `rmoveto` only where it holds more than two operands, so a width
/// inserted before the one would be read as `rmoveto`'s first argument and the glyph would keep
/// `defaultWidthX`; the glyph is refused by name instead, and `hmoveto` with its one operand is
/// restated.
#[test]
fn a_width_is_inserted_only_where_the_count_reads_it_as_one() {
    // defaultWidthX 300; glyph 1 is `-74 rmoveto`.
    let mut program = name_keyed(&[0xf7, 0xc0, 20]);
    let at = program
        .windows(2)
        .position(|pair| pair == [0xef, 14])
        .expect("glyph 1's charstring");
    program[at..at + 2].copy_from_slice(&[0x41, 21]);
    assert_eq!(charstring_widths(&program, &[1]), Ok(vec![Some(300.0)]));
    let advance = BTreeMap::from([(1u16, 500i64)]);
    assert!(matches!(
        with_advances(&program, &advance),
        Err(CffError::AdvanceNotRestatable { glyph: 1, .. })
    ));
    // The control: `-74 hmoveto`, whose one operand is the one it takes, is given a width.
    program[at..at + 2].copy_from_slice(&[0x41, 22]);
    let written = with_advances(&program, &advance).expect("a width before a full hmoveto");
    assert_eq!(charstring_widths(&written, &[1]), Ok(vec![Some(500.0)]));
}

/// The `cff_write` fuzz target's third `advances_restated` finding, minimised: a Top DICT whose
/// `Private` offset points into the Name INDEX, and its fourth, an offset past the
/// program's end. The rewrite re-encodes the Top DICT, shifts every offset past it and appends
/// what it rebuilt, so either would name other bytes after it; each is refused by name, and
/// the program pointing where its Private DICT is is written.
#[test]
fn an_offset_the_rewrite_cannot_shift_is_refused_by_name() {
    let private = [0xf7, 0xc0, 20];
    let program = name_keyed(&private);
    let private_at = i64::try_from(program.len() - private.len()).expect("a fixture offset");
    let mut needle = int5(private_at).to_vec();
    needle.push(18);
    let at = program
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the Private entry's offset");
    let mut inside = program.clone();
    inside[at..at + 5].copy_from_slice(&int5(4));
    let advance = BTreeMap::from([(1u16, 500i64)]);
    assert!(matches!(
        with_advances(&inside, &advance),
        Err(CffError::Malformed { detail })
            if detail.contains("the Top DICT's Private offset points before the program's start, into the header")
    ));
    let mut past = program.clone();
    past[at..at + 5].copy_from_slice(&int5(private_at + 3));
    // `read-fonts` itself refuses a Private DICT past the end, so the finding is asked of the
    // layout every writer reads; the fuzz target's own was a `charset`, which it reads lazily.
    assert_eq!(
        Layout::read(&past).and_then(|layout| layout.misstated),
        Some(Misstated::Outside {
            dict: "Top",
            operator: "Private"
        })
    );
    assert!(with_advances(&past, &advance).is_err());
    let written = with_advances(&program, &advance).expect("the Private DICT where it belongs");
    assert_eq!(charstring_widths(&written, &[1]), Ok(vec![Some(500.0)]));
}

/// The `cff_write` fuzz target's fifth `advances_restated` finding, minimised: a Top DICT whose
/// `FDArray` names bytes that do not read as an INDEX. A rewrite restates the `FDArray` from its
/// items, and one it cannot read it would write as an INDEX of none, so it is refused by name.
#[test]
fn an_fd_array_that_does_not_read_is_refused_by_name() {
    let program = name_keyed(&[0xf7, 0xc0, 20]);
    let layout = Layout::read(&program).expect("a Top DICT");
    let mut top = layout.entries.clone();
    // An INDEX of 579 items whose offsets run past the program.
    let mut data = program.clone();
    let at = data.len();
    data.extend_from_slice(&[0x02, 0x43, 0x02, 0x4a, 0x02]);
    top.push((
        int5(i64::try_from(at).expect("a fixture offset")).to_vec(),
        OP_FD_ARRAY,
    ));
    assert_eq!(misstated_entry(&data, &top), Some(Misstated::FdArrayUnread));
    top.pop();
    assert_eq!(misstated_entry(&data, &top), None);
}

/// The control: a program stating each entry once — `nominalWidthX` as a real, which Table 23's
/// `number` admits — is written, and the reader reads every advance it was asked to state.
#[test]
fn a_well_stated_program_reads_back_every_advance_it_was_given() {
    // nominalWidthX 1.5.
    let program = name_keyed(&[0x1e, 0x1a, 0x5f, 21]);
    let asked = BTreeMap::from([(0u16, 400i64), (1, 500)]);
    let written = with_advances(&program, &asked).expect("a well-stated program");
    assert_eq!(
        charstring_widths(&written, &[0, 1]),
        Ok(vec![Some(400.0), Some(500.0)])
    );
    assert!(Parts::read(&program).is_some());
}

/// The Private DICTs are read once each, and only while together they hold no more bytes than
/// the program: at exactly the program's length they are read, one byte past it the program is
/// refused before any is.
#[test]
fn private_dicts_are_read_up_to_the_programs_length_and_refused_past_it() {
    fn font_dict(size: i64, at: i64) -> Vec<u8> {
        let mut dict = int5(size).to_vec();
        dict.extend_from_slice(&int5(at));
        dict.push(18);
        dict
    }
    let length = 64usize;
    let with_ranges = |second: (i64, i64)| -> Option<Misstated> {
        let (first, second) = (font_dict(32, 0), font_dict(second.0, second.1));
        let mut data = index(&[&first, &second]);
        data.resize(length, 0);
        let top = vec![(int5(0).to_vec(), OP_FD_ARRAY)];
        misstated_entry(&data, &top)
    };
    assert_ne!(
        with_ranges((32, 32)),
        Some(Misstated::Overlapping),
        "two ranges holding the program's 64 bytes"
    );
    assert_eq!(
        with_ranges((33, 31)),
        Some(Misstated::Overlapping),
        "65 bytes of a 64-byte program"
    );
    let shared = {
        let dict = font_dict(40, 0);
        let mut data = index(&[&dict, &dict]);
        data.resize(length, 0);
        misstated_entry(&data, &[(int5(0).to_vec(), OP_FD_ARRAY)])
    };
    assert_ne!(
        shared,
        Some(Misstated::Overlapping),
        "one Private DICT two Font DICTs share is counted once"
    );
}

/// A `CharStrings` entry of three operands, the last the offset `read-fonts` reads and the first
/// an offset of zero, is refused by name by every name-keyed writer — the first finding's own
/// shape, without the `Private` entry it also misstates.
#[test]
fn a_charstrings_entry_of_three_operands_is_refused_by_name() {
    let program = three_operands(&name_keyed(&[0x8b, 21]), OP_CHARSTRINGS, &[17]);
    assert_eq!(
        glyph_count(&program),
        Ok(2),
        "the reader reads the last operand"
    );
    let one = GlyphToGive {
        draws: GivenOutline::Nothing,
        name: None,
        advance: 500,
    };
    assert!(refused(
        &with_glyphs_given(&program, &[one]),
        "Top",
        "CharStrings"
    ));
    assert!(refused(
        &with_advances(&program, &BTreeMap::from([(1u16, 500i64)])),
        "Top",
        "CharStrings"
    ));
    assert!(Parts::read(&program).is_none());
}

/// The hostile shape the bound exists for: a thousand Font DICTs, each naming a Private DICT of
/// about a mebibyte that overlaps the next, which a writer reading every Private DICT once per
/// Font DICT would parse a thousand times over. It is refused as overlapping before any is read.
#[test]
fn a_thousand_overlapping_private_dicts_are_refused_before_any_is_read() {
    const FONT_DICTS: usize = 1024;
    const REGION: usize = 1 << 20;
    let mut program = cid_keyed(&[14], None, true);
    let fd_array_at = Layout::read(&program)
        .and_then(|layout| layout.fd_array_at)
        .expect("an FDArray");
    let region_at = program.len();
    // `defaultWidthX 0`, over and over: entries a DICT parser allocates for one by one.
    program.extend(std::iter::repeat_n([0x8b, 20], REGION / 2).flatten());
    let dicts: Vec<Vec<u8>> = (0..FONT_DICTS)
        .map(|at| {
            let mut dict = int5(i64::try_from(REGION - at).expect("a size")).to_vec();
            dict.extend_from_slice(&int5(i64::try_from(region_at + at).expect("an offset")));
            dict.push(18);
            dict
        })
        .collect();
    let new_fd_array_at = program.len();
    program.extend_from_slice(&u16::try_from(FONT_DICTS).expect("a count").to_be_bytes());
    program.push(4);
    let mut offset = 1u32;
    program.extend_from_slice(&offset.to_be_bytes());
    for dict in &dicts {
        offset += u32::try_from(dict.len()).expect("a small DICT");
        program.extend_from_slice(&offset.to_be_bytes());
    }
    for dict in &dicts {
        program.extend_from_slice(dict);
    }
    let mut needle = int5(i64::try_from(fd_array_at).expect("an offset")).to_vec();
    needle.extend_from_slice(&[12, 36]);
    let at = program
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the FDArray entry");
    program[at..at + 5].copy_from_slice(&int5(i64::try_from(new_fd_array_at).expect("an offset")));
    assert_eq!(
        Layout::read(&program).and_then(|layout| layout.misstated),
        Some(Misstated::Overlapping)
    );
    assert!(matches!(
        with_advances(&program, &BTreeMap::from([(1u16, 500i64)])),
        Err(CffError::Malformed { detail }) if detail.contains("overlap")
    ));
}

/// The `cff_write` fuzz target's sixth `advances_restated` finding, minimised: a Private DICT
/// whose first `nominalWidthX` is stated after a real holding the reserved nibble 0xd. The
/// reader ends the real with that nibble's byte and reads two `nominalWidthX` entries, the last
/// -76; a tokeniser ending the real only at a 0xf nibble took the first entry into the number,
/// saw one statement, and the writer reckoned against the reader's first, 1.
#[test]
fn a_real_holding_a_reserved_nibble_ends_where_the_reader_ends_it() {
    // The real `d2` (no digit), 1, nominalWidthX; -76, nominalWidthX.
    let private = [0x1e, 0xd2, 0x8c, 21, 0x3f, 21];
    assert_eq!(
        dict_entries(&private),
        Some(vec![(vec![0x1e, 0xd2, 0x8c], 21), (vec![0x3f], 21)])
    );
    let program = name_keyed(&private);
    assert_eq!(
        charstring_widths(&program, &[1]),
        Ok(vec![Some(24.0)]),
        "the reader reckons glyph 1's operand of 100 against the second statement, -76"
    );
    let advance = BTreeMap::from([(1u16, 500i64)]);
    assert!(refused(
        &with_advances(&program, &advance),
        "Private",
        "nominalWidthX"
    ));
    // An exponent of 1001 meeting another digit ends the real with that digit's byte, as the
    // reader ends it; at 1000 the digit is taken and the next byte is still the number's.
    assert_eq!(operand_count(&[0x1e, 0xb1, 0x00, 0x15, 0x8b]), 2);
    assert_eq!(operand_count(&[0x1e, 0xb1, 0x00, 0x05, 0x8b]), 1);
}
