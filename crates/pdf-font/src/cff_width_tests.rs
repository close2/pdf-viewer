//! A Type 2 charstring's width as Adobe Technical Note #5177 section 3.1 and Technical Note
//! #5176 section 15 state it (ADR 3185): the operand added to the Private DICT's `nominalWidthX`
//! where the charstring states one, the Private DICT's `defaultWidthX` where it does not, both
//! 0 where the DICT is silent (Table 23), and both read from the Font DICT `FDSelect` names for
//! the glyph in a CID-keyed program.

use super::font_dict_repair::index;
use super::{advances, charstring_widths, int5};

/// `endchar` alone: a charstring that omits its width (the note's section 3.1, Note 7).
const OMITTED: &[u8] = &[14];

/// `100 endchar`: a width operand of 100 (`0xef` is 239, a one-byte integer of 239 − 139).
const OPERAND_100: &[u8] = &[0xef, 14];

/// The Private DICT entries `defaultWidthX` (operator 20) and `nominalWidthX` (21), each written
/// only where it is given.
fn private(default_width: Option<i64>, nominal_width: Option<i64>) -> Vec<u8> {
    let mut dict = Vec::new();
    for (value, operator) in [(default_width, 20u8), (nominal_width, 21)] {
        if let Some(value) = value {
            dict.extend_from_slice(&int5(value));
            dict.push(operator);
        }
    }
    dict
}

/// A name-keyed program of the given charstrings under one Private DICT. It states no charset,
/// so Technical Note #5176's predefined `ISOAdobe` charset (offset 0) names glyph 1 `space`.
#[expect(
    clippy::cast_possible_wrap,
    clippy::arithmetic_side_effects,
    reason = "a fixture builder over a font of a few glyphs: every quantity is a few hundred, and \
              a wrong table is what the assertions exist to catch"
)]
pub(crate) fn name_keyed_with(charstrings: &[&[u8]], private: &[u8]) -> Vec<u8> {
    let header = [1u8, 0, 4, 4];
    let names = index(&[b"F"]);
    let empty = index(&[]);
    let charstrings = index(charstrings);
    // CharStrings, then Private: 6 + 11 bytes, in an INDEX of one item and one-byte offsets.
    let top_len = 17;
    let charstrings_at = header.len() + names.len() + index(&[&[0; 17]]).len() + 2 * empty.len();
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

/// A four-glyph CID-keyed program of two Font DICTs: glyphs 0 and 1 omit their widths under
/// FD 0, glyph 2 omits its width and glyph 3 states the operand 100 under FD 1. The charset is
/// format 2 (CIDs 1 to 3 for glyphs 1 to 3) and `FDSelect` format 0.
#[expect(
    clippy::cast_possible_wrap,
    clippy::arithmetic_side_effects,
    reason = "a fixture builder over a font of four glyphs: every quantity is a few hundred, and \
              a wrong table is what the assertions exist to catch"
)]
fn cid_keyed_two_font_dicts(private_0: &[u8], private_1: &[u8]) -> Vec<u8> {
    let header = [1u8, 0, 4, 4];
    let names = index(&[b"F"]);
    let strings = index(&[b"Adobe", b"Identity"]);
    let gsubrs = index(&[]);
    let charset = [2u8, 0, 1, 0, 2];
    let fd_select = [0u8, 0, 0, 1, 1];
    let charstrings = index(&[OMITTED, OMITTED, OMITTED, OPERAND_100]);
    // ROS, charset, FDSelect, CharStrings, FDArray: 17 + 6 + 7 + 6 + 7 bytes.
    let top_len = 43;
    let strings_at = header.len() + names.len() + index(&[&[0; 43]]).len();
    let charset_at = strings_at + strings.len() + gsubrs.len();
    let fd_select_at = charset_at + charset.len();
    let charstrings_at = fd_select_at + fd_select.len();
    let private_0_at = charstrings_at + charstrings.len();
    let private_1_at = private_0_at + private_0.len();
    let fd_array_at = private_1_at + private_1.len();
    let mut top = Vec::new();
    for operand in [391, 392, 0] {
        top.extend_from_slice(&int5(operand));
    }
    top.extend_from_slice(&[12, 30]);
    top.extend_from_slice(&int5(charset_at as i64));
    top.push(15);
    top.extend_from_slice(&int5(fd_select_at as i64));
    top.extend_from_slice(&[12, 37]);
    top.extend_from_slice(&int5(charstrings_at as i64));
    top.push(17);
    top.extend_from_slice(&int5(fd_array_at as i64));
    top.extend_from_slice(&[12, 36]);
    assert_eq!(top.len(), top_len);
    let font_dict = |private: &[u8], at: usize| {
        let mut dict = int5(private.len() as i64).to_vec();
        dict.extend_from_slice(&int5(at as i64));
        dict.push(18);
        dict
    };
    let fd_0 = font_dict(private_0, private_0_at);
    let fd_1 = font_dict(private_1, private_1_at);

    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&names);
    out.extend_from_slice(&index(&[&top]));
    out.extend_from_slice(&strings);
    out.extend_from_slice(&gsubrs);
    out.extend_from_slice(&charset);
    out.extend_from_slice(&fd_select);
    out.extend_from_slice(&charstrings);
    out.extend_from_slice(private_0);
    out.extend_from_slice(private_1);
    out.extend_from_slice(&index(&[&fd_0, &fd_1]));
    out
}

/// The defect ADR 3173 section 5 found: a charstring omitting its width under a Private DICT
/// that states `nominalWidthX` and no `defaultWidthX` states the default 0 — not "no width", and
/// not the nominal. The glyph stating its operand is read against the nominal, the control.
#[test]
fn an_omitted_width_under_no_default_width_is_zero() {
    let program = name_keyed_with(&[OMITTED, OMITTED, OPERAND_100], &private(None, Some(100)));
    let expected = Ok(vec![Some(0.0), Some(0.0), Some(200.0)]);
    assert_eq!(charstring_widths(&program, &[0, 1, 2]), expected);
    assert_eq!(advances(&program, &[0, 1, 2]), expected);
    // And under an empty Private DICT both operators are 0.
    let program = name_keyed_with(&[OMITTED, OPERAND_100], &[]);
    assert_eq!(
        charstring_widths(&program, &[0, 1]),
        Ok(vec![Some(0.0), Some(100.0)])
    );
}

/// A non-zero `defaultWidthX` is the omitted width, and the nominal is not added to it.
#[test]
fn an_omitted_width_is_the_default_width() {
    let program = name_keyed_with(&[OMITTED, OPERAND_100], &private(Some(300), Some(100)));
    let expected = Ok(vec![Some(300.0), Some(200.0)]);
    assert_eq!(charstring_widths(&program, &[0, 1]), expected);
    assert_eq!(advances(&program, &[0, 1]), expected);
}

/// Each glyph of a CID-keyed program reads the Private DICT of the Font DICT `FDSelect` names
/// for it (Technical Note #5176 sections 18 and 19): FD 0's `defaultWidthX` 250 for glyphs 0
/// and 1, FD 1's 600 for glyph 2, and FD 1's `nominalWidthX` 400 under glyph 3's operand 100.
/// Where FD 1 states no `defaultWidthX`, glyph 2 is 0 while FD 0's glyphs keep 250.
#[test]
fn each_font_dict_supplies_its_own_default_width() {
    let program =
        cid_keyed_two_font_dicts(&private(Some(250), None), &private(Some(600), Some(400)));
    let expected = Ok(vec![Some(250.0), Some(250.0), Some(600.0), Some(500.0)]);
    assert_eq!(charstring_widths(&program, &[0, 1, 2, 3]), expected);
    assert_eq!(advances(&program, &[0, 1, 2, 3]), expected);

    let program = cid_keyed_two_font_dicts(&private(Some(250), None), &private(None, Some(400)));
    assert_eq!(
        charstring_widths(&program, &[0, 1, 2, 3]),
        Ok(vec![Some(250.0), Some(250.0), Some(0.0), Some(500.0)])
    );
}

/// `None` stays the answer for a glyph the program does not hold: the `CharStrings` INDEX has
/// no such entry, so there is no charstring to state a width.
#[test]
fn a_glyph_past_the_charstrings_still_has_no_width() {
    let program = name_keyed_with(&[OMITTED], &private(None, Some(100)));
    assert_eq!(
        charstring_widths(&program, &[0, 1]),
        Ok(vec![Some(0.0), None])
    );
}
