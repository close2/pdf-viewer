//! A charstring stated again with its x coordinates scaled (`doc/adr/2789`): each expected
//! charstring is derived from Adobe Technical Note #5177's operand rules by hand, section by
//! section, at a scale of one half so that every product is visible.

use super::{Refusal, Restatement, put_number};
use crate::cff::Subroutines;

/// One item of a charstring as these tests write it.
#[derive(Clone, Copy)]
enum Item {
    /// A number, in the shortest form section 3.2 gives it.
    N(f64),
    /// A one-byte operator.
    Op(u8),
    /// An escaped operator, 12 and its second byte.
    Esc(u8),
}
use Item::{Esc, N, Op};

fn charstring(items: &[Item]) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items {
        match *item {
            N(value) => put_number(&mut out, value).expect("a number the tests can state"),
            Op(op) => out.push(op),
            Esc(op) => out.extend_from_slice(&[12, op]),
        }
    }
    out
}

fn restated(items: &[Item], local: &[Vec<u8>]) -> Result<Vec<u8>, Refusal> {
    let subrs = Subroutines { global: &[], local };
    Restatement::new(&subrs, 0.5).run_glyph(&charstring(items))
}

/// Section 4.3: an `hstem`'s edges are y and copied; a `vstem`'s are x, 100 and 150 from 0,
/// which become 50 and 75 — stated `50 25`. Section 4.1: `rmoveto 10 20` and `rlineto 100 0` move
/// the point to (5, 20) and (55, 20). The width, 500, stays the first operand (section 3.1).
/// The plant: a writer that left the hints alone states `100 50 vstem`.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn a_vertical_stem_and_every_point_are_scaled_and_a_horizontal_stem_is_not() {
    let source = [
        N(500.0), N(0.0), N(20.0), Op(1), N(100.0), N(50.0), Op(3),
        N(10.0), N(20.0), Op(21), N(100.0), N(0.0), Op(5), Op(14),
    ];
    let expected = [
        N(500.0), N(0.0), N(20.0), Op(1), N(50.0), N(25.0), Op(3),
        N(5.0), N(20.0), Op(21), N(50.0), N(0.0), Op(5), Op(14),
    ];
    assert_eq!(restated(&source, &[]), Ok(charstring(&expected)));
}

/// Section 2 reserves a stem width of −21 for an edge hint's left edge: the edge at 100 becomes
/// 50 and the flag stays −21, where scaling it as a distance would state −10.5.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn an_edge_hints_flag_is_kept() {
    let source = [N(100.0), N(-21.0), Op(3), N(0.0), N(0.0), Op(21), Op(14)];
    let expected = [N(50.0), N(-21.0), Op(3), N(0.0), N(0.0), Op(21), Op(14)];
    assert_eq!(restated(&source, &[]), Ok(charstring(&expected)));
}

/// Section 4.7: one subroutine, `rlineto return`, is called with `100 0` and then with `0 100`,
/// so its operands are an x the first time and a y the second. Run as the interpreter runs it,
/// the glyph draws lines to (100, 0) and (100, 100), which scaled are (50, 0) and (50, 100):
/// `50 0 rlineto 0 100 rlineto`, with no call left. The plant: scaling by where an operand stands
/// in the caller would halve the second call's 100 as well.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn a_shared_subroutine_is_run_with_the_operands_each_call_gives_it() {
    let subroutine = vec![charstring(&[Op(5), Op(11)])];
    // A Local Subr INDEX of one holds subroutine 0 at the biased number −107 (section 4.7).
    let source = [
        N(0.0), N(0.0), Op(21), N(100.0), N(0.0), N(-107.0), Op(10),
        N(0.0), N(100.0), N(-107.0), Op(10), Op(14),
    ];
    let expected = [
        N(0.0), N(0.0), Op(21), N(50.0), N(0.0), Op(5), N(0.0), N(100.0), Op(5), Op(14),
    ];
    assert_eq!(restated(&source, &subroutine), Ok(charstring(&expected)));
}

/// Points at x = 1, 2 and 3 become 0.5, 1 and 1.5, which round to 1, 1 and 2: the deltas are
/// between the rounded points, so the last is 2, the whole number nearest 1.5, where rounding
/// each delta of 0.5 would carry the contour to 3 (the plant). The line from 1 to 1 has no length
/// and is not stated.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn the_rounding_does_not_accumulate_along_a_contour() {
    let source = [
        N(0.0), N(0.0), Op(21), N(1.0), N(0.0), N(1.0), N(0.0), N(1.0), N(0.0), Op(5), Op(14),
    ];
    let expected = [
        N(0.0), N(0.0), Op(21), N(1.0), N(0.0), Op(5), N(1.0), N(0.0), Op(5), Op(14),
    ];
    assert_eq!(restated(&source, &[]), Ok(charstring(&expected)));
}

/// Section 4.2: `endchar`'s `adx ady bchar achar` place an accent `adx` to the right of the base
/// glyph's origin, so `adx` 100 becomes 50 and the rest are copied; 500 is the width.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn an_accents_offset_is_scaled_with_the_outline() {
    let source = [N(500.0), N(100.0), N(20.0), N(65.0), N(66.0), Op(14)];
    let expected = [N(500.0), N(50.0), N(20.0), N(65.0), N(66.0), Op(14)];
    assert_eq!(restated(&source, &[]), Ok(charstring(&expected)));
}

/// Section 4.1's `hvcurveto` with five operands, `10 20 30 40 5`, starts horizontal: points
/// (10, 0), (30, 30) and (35, 70). Scaled, (5, 0), (15, 30) and (17.5 → 18, 70): stated as one
/// `rrcurveto 5 0 10 30 3 40`.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn an_alternating_curve_is_stated_point_for_point() {
    let source = [
        N(0.0), N(0.0), Op(21), N(10.0), N(20.0), N(30.0), N(40.0), N(5.0), Op(31), Op(14),
    ];
    let expected = [
        N(0.0), N(0.0), Op(21), N(5.0), N(0.0), N(10.0), N(30.0), N(3.0), N(40.0), Op(8), Op(14),
    ];
    assert_eq!(restated(&source, &[]), Ok(charstring(&expected)));
}

/// Section 4.1's `hflex` keeps its depth, 50, as a `flex`: points (10, 0), (20, 5), (30, 5),
/// (40, 5), (50, 0) and (60, 0) become x 5, 10, 15, 20, 25 and 30.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn a_short_flex_is_stated_as_a_flex_with_its_depth() {
    let source = [
        N(0.0), N(0.0), Op(21), N(10.0), N(10.0), N(5.0), N(10.0), N(10.0), N(10.0), N(10.0),
        Esc(34), Op(14),
    ];
    let expected = [
        N(0.0), N(0.0), Op(21), N(5.0), N(0.0), N(5.0), N(5.0), N(5.0), N(0.0), N(5.0), N(0.0),
        N(5.0), N(-5.0), N(5.0), N(0.0), N(50.0), Esc(35), Op(14),
    ];
    assert_eq!(restated(&source, &[]), Ok(charstring(&expected)));
}

/// An arithmetic operator (Appendix A's `12 10`, `add`) changes the stack by a rule this run does
/// not keep, and a reserved one (`2`) is not an operator at all: each is refused by name.
#[test]
#[rustfmt::skip] // One charstring item per word, as section 4.1 writes its operand patterns.
fn an_operator_the_run_does_not_follow_is_refused() {
    assert_eq!(
        restated(&[N(1.0), N(2.0), Esc(10), Op(14)], &[]),
        Err(Refusal::Operator(12, Some(10)))
    );
    assert_eq!(restated(&[Op(2)], &[]), Err(Refusal::Operator(2, None)));
}
