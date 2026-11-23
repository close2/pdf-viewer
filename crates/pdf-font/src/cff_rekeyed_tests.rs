//! [`super::cid_keyed_by_map`] against a name-keyed program built here: every CID the map sends
//! to a glyph draws that glyph's outline at its width through the written charset, a glyph two
//! CIDs reach is copied, the Local Subr INDEX is still reached, and the refusals name themselves.

use std::collections::BTreeMap;

use skrifa::outline::OutlinePen;

use super::super::{CodeToGlyph, charstring_widths, draw, uses_cid_operators};
use super::cid_keyed_by_map;
use crate::embed::SystemInfo;

/// A CFF INDEX with four-byte offsets (Adobe Technical Note #5176 section 5).
fn index(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = u16::try_from(items.len())
        .expect("a fixture INDEX is small")
        .to_be_bytes()
        .to_vec();
    if items.is_empty() {
        return out;
    }
    out.push(4);
    let mut offset = 1u32;
    out.extend_from_slice(&offset.to_be_bytes());
    for item in items {
        offset = offset.saturating_add(u32::try_from(item.len()).expect("a fixture item is small"));
        out.extend_from_slice(&offset.to_be_bytes());
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

/// A five-byte DICT integer operand (Technical Note #5176 Table 3).
fn int5(value: usize) -> Vec<u8> {
    let mut out = vec![29];
    out.extend_from_slice(
        &i32::try_from(value)
            .expect("a fixture is small")
            .to_be_bytes(),
    );
    out
}

/// A one-byte charstring operand (Technical Note #5177 section 3.2).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture operand is within -107 to 107"
)]
fn small(value: i16) -> u8 {
    u8::try_from(value + 139).expect("a fixture operand is within -107 to 107")
}

/// A name-keyed program of four glyphs under `nominalWidthX 100`: glyph 0 an empty `.notdef`,
/// glyphs 1 and 2 an L of their own width and length, and glyph 3 the same through local
/// subroutine 0.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the fixture is a few dozen bytes"
)]
fn name_keyed() -> Vec<u8> {
    const RMOVETO: u8 = 21;
    const RLINETO: u8 = 5;
    const CALLSUBR: u8 = 10;
    const RETURN: u8 = 11;
    const ENDCHAR: u8 = 14;
    let glyph = |width: i16, length: i16| {
        vec![
            small(width),
            small(0),
            small(0),
            RMOVETO,
            small(length),
            small(0),
            RLINETO,
            small(0),
            small(10),
            RLINETO,
            ENDCHAR,
        ]
    };
    let mut through_subr = glyph(30, 40);
    through_subr.splice(7..10, [small(-107), CALLSUBR]);
    let charstrings = index(&[vec![ENDCHAR], glyph(10, 20), glyph(20, 30), through_subr]);
    let subrs = index(&[vec![small(0), small(10), RLINETO, RETURN]]);
    // `100 nominalWidthX`, then `Subrs` at the DICT's own eight bytes.
    let mut private = vec![small(100), 21];
    private.extend(int5(8));
    private.push(19);
    let name = index(&[b"F".to_vec()]);
    let strings = index(&[]);
    let global = index(&[]);
    // The Top DICT is `CharStrings` and `Private`, 17 bytes, in an INDEX of 2 + 1 + 8 + 17.
    let charstrings_at = 4 + name.len() + 28 + strings.len() + global.len();
    let private_at = charstrings_at + charstrings.len();
    let mut top = int5(charstrings_at);
    top.push(17);
    top.extend(int5(private.len()));
    top.extend(int5(private_at));
    top.push(18);
    let mut out = vec![1, 0, 4, 4];
    for part in [
        name,
        index(&[top]),
        strings,
        global,
        charstrings,
        private,
        subrs,
    ] {
        out.extend(part);
    }
    out
}

fn system() -> SystemInfo {
    SystemInfo {
        registry: b"Adobe".to_vec(),
        ordering: b"Identity".to_vec(),
        supplement: 0,
    }
}

/// One outline as its pen received it.
#[derive(Default, PartialEq, Debug)]
struct Recorded(Vec<(u8, [u32; 6])>);

impl OutlinePen for Recorded {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((0, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((1, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push((2, [cx, cy, x, y, 0.0, 0.0].map(f32::to_bits)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .push((3, [cx0, cy0, cx1, cy1, x, y].map(f32::to_bits)));
    }
    fn close(&mut self) {
        self.0.push((4, [0; 6]));
    }
}

fn outline(program: &[u8], glyph: u16) -> Recorded {
    let mut pen = Recorded::default();
    draw(program, glyph, &mut pen).expect("the fixture draws");
    pen
}

/// ISO 32000-2 §9.7.4.2's first CFF case reaches, through the written charset, the glyph Table
/// 115's map reached: CID 3 maps to glyph 0 and is left to `.notdef`, and glyph 3, reached by
/// CIDs 1 and 5, is copied once for each.
#[test]
fn every_cid_draws_the_glyph_its_map_selected() {
    let source = name_keyed();
    let map = [0u16, 3, 1, 0, 2, 3];
    let written = cid_keyed_by_map(&source, &map, &system()).expect("the map is written");
    assert_eq!(uses_cid_operators(&written), Ok(true));
    let CodeToGlyph::Keyed { by_cid } = CodeToGlyph::read(&written).expect("it reads") else {
        panic!("the written program is CID-keyed");
    };
    assert_eq!(
        by_cid,
        BTreeMap::from([(0, 0), (1, 1), (2, 2), (4, 3), (5, 4)])
    );
    for (cid, glyph) in by_cid {
        let source_glyph = map[usize::from(cid)];
        assert_eq!(
            outline(&written, glyph),
            outline(&source, source_glyph),
            "CID {cid}"
        );
        assert_eq!(
            charstring_widths(&written, &[glyph]),
            charstring_widths(&source, &[source_glyph]),
            "CID {cid}"
        );
    }
    assert_eq!(
        charstring_widths(&written, &[1]),
        Ok(vec![Some(130.0)]),
        "100 nominalWidthX and the operand 30"
    );
    assert!(
        outline(&written, 1).0.len() > 1,
        "the glyph drawn through the Local Subr INDEX has its second line"
    );
}

/// A map past the program's glyphs, and a program already CID-keyed, are refused by name.
#[test]
fn a_map_past_the_glyphs_and_a_cid_keyed_program_are_refused() {
    let source = name_keyed();
    let past = cid_keyed_by_map(&source, &[0, 4], &system()).expect_err("glyph 4 is not held");
    assert!(past.to_string().contains("CID 1 maps to glyph 4"), "{past}");
    let written = cid_keyed_by_map(&source, &[0, 1], &system()).expect("written");
    let again = cid_keyed_by_map(&written, &[0, 1], &system()).expect_err("already CID-keyed");
    assert!(
        again.to_string().contains("already uses CIDFont operators"),
        "{again}"
    );
}
