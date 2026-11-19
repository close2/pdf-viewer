//! The converter's tests: each Type 1 program is built here in Adobe Type 1 Font Format section
//! 2.4's shape and each charstring in chapter 6's operators, so every expected point is the
//! format's arithmetic and not a reader's.

use skrifa::outline::OutlinePen;

use super::as_cid_keyed_cff;
use crate::embed::SystemInfo;
use crate::type1::tests::{CALLSUBR, ENDCHAR, HSBW, RETURN, number, plain_glyph, program_with};
use crate::type1::{Program, Type1Error};

/// `rlineto`, `rrcurveto`, `closepath`, `rmoveto` (section 6.4).
const RLINETO: u8 = 5;
/// See [`RLINETO`].
const RRCURVETO: u8 = 8;
/// See [`RLINETO`].
const CLOSEPATH: u8 = 9;
/// See [`RLINETO`].
const RMOVETO: u8 = 21;
/// The escape byte, and the second bytes of `seac`, `div`, `callothersubr`, `pop` and
/// `setcurrentpoint` (section 6.4).
const ESCAPE: u8 = 12;
/// See [`ESCAPE`].
const SEAC: u8 = 6;
/// See [`ESCAPE`].
const DIV: u8 = 12;
/// See [`ESCAPE`].
const CALLOTHERSUBR: u8 = 16;
/// See [`ESCAPE`].
const POP: u8 = 17;
/// See [`ESCAPE`].
const SETCURRENTPOINT: u8 = 33;

/// The `CIDFont`'s `CIDSystemInfo` every fixture is written under.
fn identity() -> SystemInfo {
    SystemInfo {
        registry: b"Adobe".to_vec(),
        ordering: b"Identity".to_vec(),
        supplement: 0,
    }
}

/// A charstring of numbers and operators, the operators given as their bytes.
fn charstring(parts: &[Part]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        match part {
            N(value) => out.extend(number(*value)),
            Op(bytes) => out.extend_from_slice(bytes),
        }
    }
    out
}

/// One token of a fixture charstring.
enum Part {
    N(i32),
    Op(&'static [u8]),
}

use Part::{N, Op};

/// A square from (50, 0), 400 wide and 700 high, under `50 600 hsbw`.
fn square() -> Vec<u8> {
    charstring(&[
        N(50),
        N(600),
        Op(&[HSBW]),
        N(0),
        N(0),
        Op(&[RMOVETO]),
        N(400),
        N(0),
        Op(&[RLINETO]),
        N(0),
        N(700),
        Op(&[RLINETO]),
        N(-400),
        N(0),
        Op(&[RLINETO]),
        Op(&[CLOSEPATH]),
        Op(&[ENDCHAR]),
    ])
}

/// What a pen received, each point as numbers.
#[derive(Debug, Default, PartialEq)]
struct Drawn(Vec<(char, Vec<f32>)>);

impl OutlinePen for Drawn {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(('M', vec![x, y]));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(('L', vec![x, y]));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push(('Q', vec![cx, cy, x, y]));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(('C', vec![cx0, cy0, cx1, cy1, x, y]));
    }
    fn close(&mut self) {
        self.0.push(('Z', vec![]));
    }
}

/// Glyph `glyph` of the written CFF.
fn drawn_from_cff(cff: &[u8], glyph: u16) -> Drawn {
    let mut pen = Drawn::default();
    crate::cff::draw(cff, glyph, &mut pen).expect("the written program draws");
    pen
}

/// Glyph `glyph` of the Type 1 program.
fn drawn_from_type1(data: &[u8], glyph: u16) -> Drawn {
    let mut pen = Drawn::default();
    Program::parse(data)
        .expect("the fixture reads")
        .draw(glyph, &mut pen)
        .expect("the fixture draws");
    pen
}

#[test]
fn a_square_is_written_as_the_cid_keyed_program_drawing_it_at_its_cid() {
    let notdef = plain_glyph(500);
    let a = square();
    let data = program_with(&[(".notdef", &notdef), ("A", &a)], &[]);
    let cff = as_cid_keyed_cff(&data, &identity()).expect("the program converts");

    // Technical Note #5176 section 18: a CID-keyed program, its charset sending CID n to glyph n.
    assert!(crate::cff::uses_cid_operators(&cff).expect("the CFF reads"));
    let Ok(crate::cff::CodeToGlyph::Keyed { by_cid }) = crate::cff::CodeToGlyph::read(&cff) else {
        panic!("the program is CID-keyed");
    };
    assert_eq!(by_cid.into_iter().collect::<Vec<_>>(), vec![(0, 0), (1, 1)]);

    // Section 6.4: `hsbw` puts the current point at the side bearing (50, 0); each `rlineto` moves
    // it by its operands; `closepath` closes the subpath.
    let drawn = drawn_from_cff(&cff, 1);
    assert_eq!(
        drawn.0.get(..4),
        Some(
            &[
                ('M', vec![50.0, 0.0]),
                ('L', vec![450.0, 0.0]),
                ('L', vec![450.0, 700.0]),
                ('L', vec![50.0, 700.0]),
            ][..]
        )
    );
    assert_eq!(drawn.0.last().map(|(command, _)| *command), Some('Z'));
    assert_eq!(drawn, drawn_from_type1(&data, 1));
    assert_eq!(
        crate::cff::charstring_widths(&cff, &[0, 1]).expect("the CFF reads"),
        vec![Some(500.0), Some(600.0)]
    );
}

#[test]
fn an_accented_glyph_is_written_as_its_two_components_paths() {
    // Section 6.4's `seac asb adx ady bchar achar`: the base `A` (StandardEncoding 65) and the
    // accent `acute` (194) offset by (adx, ady) = (150, 700), drawn as one glyph.
    let notdef = plain_glyph(500);
    let a = square();
    let acute = charstring(&[
        N(0),
        N(300),
        Op(&[HSBW]),
        N(0),
        N(0),
        Op(&[RMOVETO]),
        N(100),
        N(0),
        Op(&[RLINETO]),
        N(0),
        N(100),
        Op(&[RLINETO]),
        Op(&[CLOSEPATH]),
        Op(&[ENDCHAR]),
    ]);
    let accented = charstring(&[
        N(50),
        N(600),
        Op(&[HSBW]),
        N(0),
        N(150),
        N(700),
        N(65),
        N(194),
        Op(&[ESCAPE, SEAC]),
    ]);
    let data = program_with(
        &[
            (".notdef", &notdef),
            ("A", &a),
            ("acute", &acute),
            ("Aacute", &accented),
        ],
        &[],
    );
    let cff = as_cid_keyed_cff(&data, &identity()).expect("the program converts");
    let drawn = drawn_from_cff(&cff, 3);
    assert_eq!(drawn, drawn_from_type1(&data, 3));
    // The base's subpath is `A`'s own, and the accent's is `acute`'s own moved by one offset:
    // where `seac` places the accent is the interpreter's reading of section 6.4, which the
    // converted program carries as it drew it (the equality above).
    let a_drawn = drawn_from_cff(&cff, 1);
    assert_eq!(drawn.0.get(..a_drawn.0.len()), Some(&a_drawn.0[..]));
    let accent = &drawn.0[a_drawn.0.len()..];
    let acute_drawn = drawn_from_cff(&cff, 2);
    assert_eq!(accent.len(), acute_drawn.0.len());
    let Some(('M', at)) = accent.first() else {
        panic!("the accent opens a subpath: {drawn:?}");
    };
    let (dx, dy) = (at[0] - acute_drawn.0[0].1[0], at[1] - acute_drawn.0[0].1[1]);
    for ((command, points), (own_command, own_points)) in accent.iter().zip(&acute_drawn.0) {
        assert_eq!(command, own_command);
        let moved: Vec<f32> = own_points
            .chunks(2)
            .flat_map(|point| [point[0] + dx, point[1] + dy])
            .collect();
        assert_eq!(points, &moved);
    }
    assert_eq!(
        crate::cff::charstring_widths(&cff, &[3]).expect("the CFF reads"),
        vec![Some(600.0)]
    );
}

/// The four subroutines Adobe Type 1 Font Format section 8.4 gives every program that uses flex
/// and hint replacement, in their standard form.
fn standard_subrs() -> Vec<Vec<u8>> {
    vec![
        charstring(&[
            N(3),
            N(0),
            Op(&[ESCAPE, CALLOTHERSUBR, ESCAPE, POP, ESCAPE, POP]),
            Op(&[ESCAPE, SETCURRENTPOINT, RETURN]),
        ]),
        charstring(&[N(0), N(1), Op(&[ESCAPE, CALLOTHERSUBR, RETURN])]),
        charstring(&[N(0), N(2), Op(&[ESCAPE, CALLOTHERSUBR, RETURN])]),
        charstring(&[N(3), Op(&[RETURN])]),
    ]
}

#[test]
fn a_flex_is_written_as_the_two_curves_it_draws() {
    // Section 8.3: `1 callsubr` opens the flex, seven `rmoveto 2 callsubr` pairs state the
    // reference point and the six control points, and `50 x y 0 callsubr` ends it at (x, y).
    let mut body = vec![N(0), N(600), Op(&[HSBW]), N(0), N(0), Op(&[RMOVETO])];
    body.extend([N(1), Op(&[CALLSUBR])]);
    for (dx, dy) in [
        (100, 0),
        (0, 20),
        (50, 0),
        (50, -20),
        (50, 20),
        (50, 0),
        (0, -20),
    ] {
        body.extend([N(dx), N(dy), Op(&[RMOVETO]), N(2), Op(&[CALLSUBR])]);
    }
    body.extend([N(50), N(300), N(0), N(0), Op(&[CALLSUBR])]);
    body.extend([N(0), N(-100), Op(&[RLINETO]), Op(&[CLOSEPATH, ENDCHAR])]);
    let flexed = charstring(&body);
    let notdef = plain_glyph(500);
    let data = program_with(&[(".notdef", &notdef), ("F", &flexed)], &standard_subrs());
    let cff = as_cid_keyed_cff(&data, &identity()).expect("the program converts");
    let drawn = drawn_from_cff(&cff, 1);
    assert_eq!(drawn, drawn_from_type1(&data, 1));
    // The reference point (100, 0) is not drawn; the points the moves reach in turn are the
    // control points: from (0, 0) through (100, 20) and (150, 20) to (200, 0), and from there
    // through (250, 20) and (300, 20) to (300, 0).
    let curves: Vec<&Vec<f32>> = drawn
        .0
        .iter()
        .filter(|(command, _)| *command == 'C')
        .map(|(_, points)| points)
        .collect();
    assert_eq!(
        curves,
        vec![
            &vec![100.0, 20.0, 150.0, 20.0, 200.0, 0.0],
            &vec![250.0, 20.0, 300.0, 20.0, 300.0, 0.0],
        ]
    );
}

#[test]
fn a_divided_operand_is_carried_as_a_fraction() {
    // Section 6.4's `div`: 100 3 div is 33.333…, which only the 16.16 operand form states.
    let body = charstring(&[
        N(0),
        N(600),
        Op(&[HSBW]),
        N(0),
        N(0),
        Op(&[RMOVETO]),
        N(100),
        N(3),
        Op(&[ESCAPE, DIV]),
        N(0),
        Op(&[RLINETO]),
        N(0),
        N(100),
        Op(&[RLINETO]),
        N(-10),
        N(0),
        N(-10),
        N(-50),
        N(0),
        N(-50),
        Op(&[RRCURVETO, CLOSEPATH, ENDCHAR]),
    ]);
    let notdef = plain_glyph(500);
    let data = program_with(&[(".notdef", &notdef), ("D", &body)], &[]);
    let cff = as_cid_keyed_cff(&data, &identity()).expect("the program converts");
    let drawn = drawn_from_cff(&cff, 1);
    assert_eq!(drawn, drawn_from_type1(&data, 1));
    // The quotient is carried at the precision `div` computed it in, 100/3 as the 16.16 number
    // 0x0021_5555 after the operand prefix 255 (Technical Note #5177 section 3.2); the pens of
    // both readers place a point in whole units, so the outline is compared above.
    assert!(
        cff.windows(5)
            .any(|window| window == [255, 0x00, 0x21, 0x55, 0x55]),
        "the CFF states the quotient as a 16.16 operand"
    );
}

#[test]
fn a_font_matrix_of_other_units_is_refused_rather_than_written_in_these() {
    let notdef = plain_glyph(500);
    let a = square();
    let data = program_with(&[(".notdef", &notdef), ("A", &a)], &[]);
    let mut other = data.clone();
    let from = b"/FontMatrix [0.001 0 0 0.001 0 0]";
    let at = other
        .windows(from.len())
        .position(|window| window == from)
        .expect("the header states the matrix");
    other.splice(
        at..at + from.len(),
        b"/FontMatrix [0.002 0 0 0.002 0 0]".iter().copied(),
    );
    assert!(matches!(
        as_cid_keyed_cff(&other, &identity()),
        Err(Type1Error::NotWritten { detail }) if detail.contains("FontMatrix")
    ));
}

#[test]
fn a_glyph_the_interpreter_refuses_is_refused_by_its_index_and_name() {
    // `5 callsubr` in a program with no subroutine 5.
    let notdef = plain_glyph(500);
    let broken = charstring(&[N(0), N(600), Op(&[HSBW]), N(5), Op(&[CALLSUBR, ENDCHAR])]);
    let data = program_with(&[(".notdef", &notdef), ("B", &broken)], &[]);
    let refusal = as_cid_keyed_cff(&data, &identity()).expect_err("the glyph cannot be drawn");
    let Type1Error::NotWritten { detail } = refusal else {
        panic!("refused as not written: {refusal:?}");
    };
    assert!(detail.contains("glyph 1 (B)"), "{detail}");
}

#[test]
fn a_glyph_stating_no_width_is_refused_rather_than_given_the_default_one() {
    // Section 6.4 opens a charstring with `hsbw`; this one draws without it.
    let notdef = plain_glyph(500);
    let unstated = charstring(&[
        N(0),
        N(0),
        Op(&[RMOVETO]),
        N(10),
        N(0),
        Op(&[RLINETO, ENDCHAR]),
    ]);
    let data = program_with(&[(".notdef", &notdef), ("W", &unstated)], &[]);
    let Err(Type1Error::NotWritten { detail }) = as_cid_keyed_cff(&data, &identity()) else {
        panic!("the glyph is refused");
    };
    assert!(
        detail.contains("glyph 1 (W)") && detail.contains("no width"),
        "{detail}"
    );
}
