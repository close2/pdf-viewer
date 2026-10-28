//! The operation budget a charstring is evaluated under (ADR 2572): a subroutine fan-out that
//! nesting alone cannot bound is refused at every evaluation this crate makes, and a glyph that
//! reads exactly [`CHARSTRING_OPERATIONS`] operands and operators is drawn.

use super::{CHARSTRING_OPERATIONS, advances, calls_local_subr, draw, open};
use skrifa::outline::OutlinePen;

/// A CFF INDEX with four-byte offsets, which holds an item of any size the budget asks for.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every fixture is a few megabytes at most"
)]
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
        offset += u32::try_from(item.len()).expect("a fixture item is under 4 GiB");
        out.extend_from_slice(&offset.to_be_bytes());
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

/// A one-glyph bare CFF whose Global Subr INDEX is `global_subrs` and whose glyph is `glyph`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the header, name and Top DICT are a few dozen bytes"
)]
fn bare_cff(global_subrs: &[Vec<u8>], glyph: Vec<u8>) -> Vec<u8> {
    let name = index(&[b"F".to_vec()]);
    let strings = index(&[]);
    let global = index(global_subrs);
    // The Top DICT is one item of `29 <offset> 17`: a count, an offset size, two offsets and
    // six bytes.
    let charstrings_at = 4 + name.len() + 2 + 1 + 2 * 4 + 6 + strings.len() + global.len();
    let mut top = vec![29];
    top.extend_from_slice(
        &i32::try_from(charstrings_at)
            .expect("a fixture is small")
            .to_be_bytes(),
    );
    top.push(17);
    let mut out = vec![1, 0, 4, 4];
    out.extend_from_slice(&name);
    out.extend_from_slice(&index(&[top]));
    out.extend_from_slice(&strings);
    out.extend_from_slice(&global);
    assert_eq!(
        out.len(),
        charstrings_at,
        "the Top DICT's offset is the INDEX's"
    );
    out.extend_from_slice(&index(&[glyph]));
    out
}

/// `levels` global subroutines, each calling the next `k` times, and a glyph that calls the
/// first: Adobe Technical Note #5177's ten levels of nesting, entered about `k`⁹ times.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "ten subroutines of a few calls each"
)]
fn fan_out(k: usize, levels: u8) -> Vec<u8> {
    const CALLGSUBR: u8 = 29;
    const RETURN: u8 = 11;
    const ENDCHAR: u8 = 14;
    // Fewer than 1240 subroutines: section 4.7's bias is 107, and a one-byte operand `v` is
    // written as `v + 139`, so subroutine `i` is named by the byte `i + 32`.
    let subrs = (0..levels)
        .map(|level| {
            let mut subr = Vec::new();
            if level + 1 < levels {
                for _ in 0..k {
                    subr.extend([level + 1 + 32, CALLGSUBR]);
                }
            }
            subr.push(RETURN);
            subr
        })
        .collect::<Vec<_>>();
    bare_cff(&subrs, vec![32, CALLGSUBR, ENDCHAR])
}

/// Counts the line segments a glyph draws.
#[derive(Default)]
struct Lines(usize);

impl OutlinePen for Lines {
    fn move_to(&mut self, _: f32, _: f32) {}
    fn line_to(&mut self, _: f32, _: f32) {
        self.0 = self.0.saturating_add(1);
    }
    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
    fn close(&mut self) {}
}

/// A glyph that draws one line and then reads reserved operators until it has read `operations`
/// operands and operators in all, `endchar` included.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the fixture's arithmetic is on a count near a million"
)]
fn reading(operations: u32) -> Vec<u8> {
    // `0 0 rmoveto 100 0 rlineto`: six, and `endchar` is the seventh.
    let mut glyph = vec![139, 139, 21, 239, 139, 5];
    let filler = usize::try_from(operations - 7).expect("a count fits");
    // Operator 0 is reserved; the evaluator clears its stack and counts it.
    glyph.resize(glyph.len() + filler, 0);
    glyph.push(14);
    bare_cff(&[], glyph)
}

#[test]
fn a_glyph_at_the_budget_is_drawn_and_one_operation_past_it_is_refused() {
    let at = reading(CHARSTRING_OPERATIONS);
    let mut lines = Lines::default();
    draw(&at, 0, &mut lines).expect("a glyph at the budget is drawn");
    assert_eq!(lines.0, 1, "and its line is there");
    let past = reading(CHARSTRING_OPERATIONS + 1);
    assert!(draw(&past, 0, &mut Lines::default()).is_err());
    assert_eq!(advances(&past, &[0]), Ok(vec![None]));
}

#[test]
fn a_fan_out_of_five_at_ten_levels_is_refused_by_every_walk_of_a_charstring() {
    let hostile = fan_out(5, 10);
    // The evaluator, through both of this module's entries to it.
    assert!(draw(&hostile, 0, &mut Lines::default()).is_err());
    assert_eq!(advances(&hostile, &[0]), Ok(vec![None]));
    // This module's own walk, which spends the same budget and errs on the side of reporting.
    let font = open(&hostile).expect("the fixture opens");
    let glyph = font.charstrings().get(0).expect("the fixture has a glyph");
    let mut operations = CHARSTRING_OPERATIONS;
    assert!(calls_local_subr(glyph, &font, 0, &mut operations));
    assert_eq!(
        operations, 0,
        "the walk stopped because it spent the budget"
    );
    // Below the budget the same shape is drawn: three calls a level is about 3⁹ entries.
    let modest = fan_out(3, 10);
    draw(&modest, 0, &mut Lines::default()).expect("a fan-out under the budget is drawn");
}
