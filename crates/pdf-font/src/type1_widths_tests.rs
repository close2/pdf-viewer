//! The Type 1 width restatement's tests: each program is built in Adobe Type 1 Font Format section
//! 2.4's shape by the writer's own fixtures, so every expected value is the format's.

use std::collections::BTreeMap;

use super::super::tests::{PLAIN, Shape, boxed, empty, index_of, program};
use super::super::{ENDCHAR, ESCAPE, SBW, number, outline};
use super::with_advances;
use crate::type1::{Program, Type1Error};

#[test]
fn a_glyphs_advance_is_restated_and_its_outline_and_every_other_glyph_kept() {
    // `.notdef` second, so `read-fonts` swaps it with `A` and the index map is exercised.
    let (data, lengths) = program(
        &[
            ("A", boxed(700)),
            (".notdef", empty(250)),
            ("B", boxed(500)),
        ],
        &PLAIN,
    );
    let before = Program::parse(&data).expect("the fixture reads");
    let (a, b) = (index_of(&before, "A"), index_of(&before, "B"));
    // 5000 is a five-byte number where 700 was two, so the entry's byte count has to change.
    let restated =
        with_advances(&data, lengths, &BTreeMap::from([(a, 5000), (0, 260)])).expect("restated");
    assert_eq!(restated.lengths[0], lengths.0, "the header is not touched");
    assert_eq!(restated.lengths[2], 0);
    assert_eq!(
        restated.lengths.iter().sum::<usize>(),
        restated.program.len()
    );
    let after = Program::parse(&restated.program).expect("the restated program reads");
    assert_eq!(after.glyph_names(), before.glyph_names());
    assert_eq!(after.advance(a), Ok(Some(5000.0)));
    assert_eq!(after.advance(0), Ok(Some(260.0)));
    assert_eq!(
        after.advance(b),
        Ok(Some(500.0)),
        "a glyph not named keeps its width"
    );
    for glyph in [0, a, b] {
        assert_eq!(outline(&after, glyph), outline(&before, glyph));
    }
    assert!(!outline(&after, a).expect("drawn").is_empty());
}

#[test]
fn every_len_iv_and_spelling_is_kept_and_the_trailer_with_it() {
    for len_iv in [0, 1, 4, -1] {
        let shape = Shape {
            len_iv,
            reader: "-|",
            definer: "|-",
            trailer: b"\n0000000000000000\ncleartomark\n",
        };
        let (data, lengths) = program(&[(".notdef", empty(250)), ("A", boxed(700))], &shape);
        let a = index_of(&Program::parse(&data).expect("reads"), "A");
        let restated = with_advances(&data, lengths, &BTreeMap::from([(a, 611)]))
            .unwrap_or_else(|error| panic!("lenIV {len_iv}: {error}"));
        assert!(restated.program.ends_with(shape.trailer));
        assert_eq!(restated.lengths[2], shape.trailer.len());
        let read = Program::parse(&restated.program).expect("reads");
        assert_eq!(read.advance(a), Ok(Some(611.0)), "lenIV {len_iv}");
        assert_eq!(read.advance(0), Ok(Some(250.0)), "lenIV {len_iv}");
    }
}

#[test]
fn an_sbw_width_is_restated_and_a_charstring_that_opens_otherwise_is_refused() {
    let mut sbw = vec![];
    for value in [10, 0, 500, 0] {
        sbw.extend(number(value));
    }
    sbw.extend([ESCAPE, SBW, ENDCHAR]);
    let (data, lengths) = program(&[(".notdef", empty(250)), ("A", sbw)], &PLAIN);
    let a = index_of(&Program::parse(&data).expect("reads"), "A");
    let restated = with_advances(&data, lengths, &BTreeMap::from([(a, 480)])).expect("restated");
    let read = Program::parse(&restated.program).expect("reads");
    assert_eq!(read.advance(a), Ok(Some(480.0)));

    let (data, lengths) = program(&[(".notdef", empty(250)), ("A", vec![ENDCHAR])], &PLAIN);
    assert!(matches!(
        with_advances(&data, lengths, &BTreeMap::from([(1, 480)])),
        Err(Type1Error::NotWritten { .. })
    ));
}

#[test]
fn a_glyph_index_past_the_program_is_refused() {
    let (data, lengths) = program(&[(".notdef", empty(250)), ("A", boxed(700))], &PLAIN);
    assert!(matches!(
        with_advances(&data, lengths, &BTreeMap::from([(2, 480)])),
        Err(Type1Error::NotWritten { .. })
    ));
}

#[test]
fn a_width_is_restated_through_a_matrix_that_extends_the_face() {
    // `[0.0005 0 0 0.001 0 0]` makes a width unit half as wide as a height unit, so an advance of
    // 0.615 em is 1230 units of `wx` (Adobe Type 1 Font Format section 6.4; the `FontMatrix`).
    let (data, (header, section)) = program(&[(".notdef", empty(250)), ("A", boxed(700))], &PLAIN);
    let (extended, wide) = (
        b"/FontMatrix [0.0005 0 0 0.001",
        b"/FontMatrix [0.001 0 0 0.001",
    );
    let at = data
        .windows(wide.len())
        .position(|window| window == wide)
        .expect("the fixture states its matrix");
    let data = [&data[..at], &extended[..], &data[at + wide.len()..]].concat();
    let lengths = (header + 1, section);
    let program = Program::parse(&data).expect("reads");
    let a = index_of(&program, "A");
    assert_eq!(program.horizontal_scale(), Some(0.5));
    assert_eq!(program.horizontal_advance(a), Ok(Some(350.0)));
    let restated = crate::restate::type1_with_widths(&data, lengths, &BTreeMap::from([(a, 0.615)]))
        .expect("restated");
    let read = Program::parse(&restated.program).expect("reads");
    assert_eq!(read.advance(a), Ok(Some(1230.0)), "the charstring's own wx");
    let across = crate::restate::type1_advance(&restated.program, a).expect("an advance");
    assert!((across - 0.615).abs() < 1e-6, "{across}");
}
