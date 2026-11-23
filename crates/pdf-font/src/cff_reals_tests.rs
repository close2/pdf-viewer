//! The digitless real, found in the Top and the Private DICT, and restated as what this reader read.

use skrifa::raw::ps::cff::dict::{Entry, entries};

use super::{DictKind, digitless_reals, with_digitless_reals_restated};

/// A name-keyed program of one name, one Top DICT and the Private DICT it locates — enough of
/// Adobe Technical Note #5176's sections 6 to 9 for the DICTs to be located; no glyph is read.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture builder over a program of a few dozen bytes: every quantity is small, and \
              a wrong offset is what the assertions exist to catch"
)]
fn program(top_operands: &[u8], private: &[u8]) -> Vec<u8> {
    let header = [1u8, 0, 4, 1];
    let name = [0u8, 1, 1, 1, 2, b'A'];
    // The Top DICT: the given entries, then `Private` as two five-byte integers and 18.
    let top_len = top_operands.len() + 11;
    let top_index_len = 2 + 1 + 2 + top_len;
    let private_at = header.len() + name.len() + top_index_len;
    let mut top = top_operands.to_vec();
    top.push(29);
    top.extend_from_slice(&i32::try_from(private.len()).unwrap_or(0).to_be_bytes());
    top.push(29);
    top.extend_from_slice(&i32::try_from(private_at).unwrap_or(0).to_be_bytes());
    top.push(18);
    let mut out = header.to_vec();
    out.extend_from_slice(&name);
    out.extend_from_slice(&[0, 1, 1, 1, u8::try_from(top_len + 1).unwrap_or(0)]);
    out.extend_from_slice(&top);
    out.extend_from_slice(private);
    out
}

/// Where [`program`] puts the Top DICT: after the four-byte header, the six-byte Name INDEX and
/// the five-byte head of the Top DICT INDEX.
const TOP_AT: usize = 15;

/// `ItalicAngle` (12 2) stated as `1e ff`, a real with no nibble before its end.
const ITALIC_ANGLE_EMPTY: [u8; 4] = [0x1e, 0xff, 12, 2];

/// `BlueValues` (6) as a digitless real `1e fc` and an exponent-only one `1e c5 77 02` that this
/// reader ends at the fifth exponent digit, followed by `63` and `7f`, which it reads as the
/// integers -40 and -12: `bug1308536`'s shape.
const BLUES: [u8; 10] = [0x1e, 0xfc, 0x1e, 0xc5, 0x77, 0x02, 0x63, 0x7f, 0x8b, 6];

#[test]
fn every_digitless_real_of_the_top_and_the_private_dict_is_found() {
    let data = program(&ITALIC_ANGLE_EMPTY, &BLUES);
    let found = digitless_reals(&data).expect("the fixture's DICTs are located");
    let kinds: Vec<(DictKind, Option<u16>, usize)> = found
        .iter()
        .map(|real| (real.dict, real.operator, real.bytes.len()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (DictKind::Top, Some(0x0c02), 2),
            (DictKind::Private, Some(6), 2),
            (DictKind::Private, Some(6), 4),
        ],
        "the exponent-only real is as digitless as the empty one, and is as wide as this reader \
         reads it"
    );
}

#[test]
fn a_real_that_states_a_digit_is_not_found() {
    // 0.5 as `1e a5 ff`, and -0 as `1e e0 ff`: each states a digit, so each is a number.
    let data = program(&[0x1e, 0xa5, 0xff, 12, 2], &[0x1e, 0xe0, 0xff, 6]);
    assert_eq!(digitless_reals(&data).expect("located"), Vec::new());
    assert_eq!(with_digitless_reals_restated(&data).expect("located"), None);
}

#[test]
fn the_restated_program_reads_as_this_reader_read_the_original_and_changes_only_those_bytes() {
    let data = program(&ITALIC_ANGLE_EMPTY, &BLUES);
    let restated = with_digitless_reals_restated(&data)
        .expect("located")
        .expect("three reals to restate");
    assert_eq!(restated.len(), data.len(), "no byte is added or removed");
    let changed: Vec<usize> = data
        .iter()
        .zip(&restated)
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(at, _)| at)
        .collect();
    let found = digitless_reals(&data).expect("located");
    assert!(
        changed
            .iter()
            .all(|at| found.iter().any(|real| real.bytes.contains(at))),
        "every changed byte is inside a restated operand: {changed:?}"
    );
    assert_eq!(
        digitless_reals(&restated).expect("located"),
        Vec::new(),
        "and the restatement states a digit in each"
    );
    let private_at = data.len().saturating_sub(BLUES.len());
    let read = |bytes: &[u8]| -> Vec<String> {
        entries(bytes.get(private_at..).unwrap_or_default(), None)
            .map(|entry| format!("{entry:?}"))
            .collect()
    };
    assert_eq!(
        read(&restated),
        read(&data),
        "read-fonts reads every Private DICT entry as it did"
    );
    let top_italic = |bytes: &[u8]| {
        let top = bytes
            .get(TOP_AT..TOP_AT.saturating_add(ITALIC_ANGLE_EMPTY.len()))
            .unwrap_or_default();
        entries(top, None).find_map(|entry| match entry {
            Ok(Entry::ItalicAngle(angle)) => Some(angle),
            _ => None,
        })
    };
    assert_eq!(top_italic(&restated), top_italic(&data));
    assert_eq!(
        top_italic(&data),
        Some(skrifa::raw::types::Fixed::ZERO),
        "and the value this reader took for the digitless real is zero"
    );
}
