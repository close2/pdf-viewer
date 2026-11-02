//! The Type 1 writer's tests: each program is built here in Adobe Type 1 Font Format section
//! 2.4's shape, so every expected value is the format's and not a reader's.

use super::{
    CHARSTRING_KEY, EEXEC_KEY, ENDCHAR, ESCAPE, HSBW, MOST_CHARSTRING_BYTES, SBW, encrypt, number,
    with_glyphs_given,
};
use crate::cff::{GivenOutline, GlyphToGive, MOST_GLYPHS_GIVEN};
use crate::type1::{Program, Type1Error};

/// `rlineto` (section 6.4).
const RLINETO: u8 = 5;
/// `closepath` (section 6.4).
const CLOSEPATH: u8 = 9;

/// How a fixture spells its program.
struct Shape {
    len_iv: i64,
    reader: &'static str,
    definer: &'static str,
    trailer: &'static [u8],
}

const PLAIN: Shape = Shape {
    len_iv: 4,
    reader: "RD",
    definer: "ND",
    trailer: b"",
};

/// A charstring stored under `len_iv` leading bytes, as section 7.3 stores it.
fn stored(body: &[u8], len_iv: i64) -> Vec<u8> {
    let Ok(leading) = usize::try_from(len_iv) else {
        return body.to_vec();
    };
    let mut plain = vec![0x5a; leading];
    plain.extend_from_slice(body);
    encrypt(&plain, CHARSTRING_KEY)
}

/// A program holding `glyphs` by name, with its `/Length1` and `/Length2`.
fn program(glyphs: &[(&str, Vec<u8>)], shape: &Shape) -> (Vec<u8>, (usize, usize)) {
    let mut private = format!(
        "dup /Private 8 dict dup begin\n/{r}{{string currentfile exch readstring pop}}executeonly def\n\
         /{d}{{noaccess def}}executeonly def\n/NP{{noaccess put}}executeonly def\n/lenIV {l} def\n\
         /Subrs 0 array\n2 index /CharStrings {n} dict dup begin\n",
        r = shape.reader,
        d = shape.definer,
        l = shape.len_iv,
        n = glyphs.len(),
    )
    .into_bytes();
    for (name, body) in glyphs {
        let body = stored(body, shape.len_iv);
        private.extend_from_slice(format!("/{name} {} {} ", body.len(), shape.reader).as_bytes());
        private.extend_from_slice(&body);
        private.extend_from_slice(format!(" {}\n", shape.definer).as_bytes());
    }
    private
        .extend_from_slice(b"end\nend\nreadonly put\nnoaccess put\nmark currentfile closefile\n");
    let mut plain = b"\xff\xff\xff\xff".to_vec();
    plain.extend_from_slice(&private);
    let mut data = b"%!PS-AdobeFont-1.0: Fixture 001\n/FontName /Fixture def\n\
        /FontMatrix [0.001 0 0 0.001 0 0] readonly def\n/Encoding StandardEncoding def\n\
        currentfile eexec\n"
        .to_vec();
    let length1 = data.len();
    let section = encrypt(&plain, EEXEC_KEY);
    data.extend_from_slice(&section);
    data.extend_from_slice(shape.trailer);
    (data, (length1, section.len()))
}

/// `sbx width hsbw`, then a triangle, `closepath` and `endchar`: a `.notdef` that draws.
fn boxed(width: i32) -> Vec<u8> {
    let mut body = number(50);
    body.extend(number(width));
    body.push(HSBW);
    for (dx, dy) in [(0, 0), (400, 0), (0, 600)] {
        body.extend(number(dx));
        body.extend(number(dy));
        body.push(RLINETO);
    }
    body.extend([CLOSEPATH, ENDCHAR]);
    body
}

fn empty(width: i32) -> Vec<u8> {
    let mut body = number(0);
    body.extend(number(width));
    body.extend([HSBW, ENDCHAR]);
    body
}

fn asked(draws: GivenOutline, name: Option<&str>, advance: i64) -> GlyphToGive {
    GlyphToGive {
        draws,
        name: name.map(str::to_owned),
        advance,
    }
}

fn index_of(program: &Program, name: &str) -> u16 {
    let at = program
        .glyph_names()
        .iter()
        .position(|held| held == name)
        .expect("the program holds the name");
    u16::try_from(at).expect("a fixture's index fits")
}

#[test]
fn a_code_on_notdef_is_given_a_copy_at_its_own_width_and_every_glyph_held_is_kept() {
    let (data, lengths) = program(&[(".notdef", boxed(250)), ("A", empty(700))], &PLAIN);
    let given = with_glyphs_given(
        &data,
        lengths,
        &[
            asked(GivenOutline::NotdefsOwn, None, 600),
            asked(GivenOutline::Nothing, Some("space"), 278),
        ],
    )
    .expect("written");
    assert_eq!(given.added.len(), 2);
    assert_eq!(
        (given.added[0].glyph, given.added[0].name.as_str()),
        (2, "notdef")
    );
    assert_eq!(
        (given.added[1].glyph, given.added[1].name.as_str()),
        (3, "space")
    );
    // The header and the empty trailer are copied, so Length1 and Length3 keep their values.
    assert_eq!(given.lengths[0], lengths.0);
    assert_eq!(given.lengths[2], 0);
    assert_eq!(given.program[..lengths.0], data[..lengths.0]);
    assert_eq!(given.lengths.iter().sum::<usize>(), given.program.len());

    let read = Program::parse(&given.program).expect("read");
    assert_eq!(read.advance(2), Ok(Some(600.0)));
    assert_eq!(read.advance(3), Ok(Some(278.0)));
    assert_eq!(read.advance(index_of(&read, "A")), Ok(Some(700.0)));
    assert_eq!(read.advance(0), Ok(Some(250.0)));
    assert_eq!(super::outline(&read, 2), super::outline(&read, 0));
    assert!(!super::outline(&read, 0).expect("drawn").is_empty());
    assert_eq!(super::outline(&read, 3), Ok(Vec::new()));
}

#[test]
fn len_iv_is_honoured_and_the_trailer_and_the_spellings_are_kept() {
    for len_iv in [0, 1, 4, -1] {
        let shape = Shape {
            len_iv,
            reader: "-|",
            definer: "|-",
            trailer: b"\n0000000000000000\ncleartomark\n",
        };
        let (data, lengths) = program(&[(".notdef", boxed(250))], &shape);
        let given = with_glyphs_given(
            &data,
            lengths,
            &[asked(GivenOutline::NotdefsOwn, Some("caron"), 333)],
        )
        .unwrap_or_else(|error| panic!("lenIV {len_iv}: {error}"));
        assert_eq!(given.lengths[2], shape.trailer.len());
        assert!(given.program.ends_with(shape.trailer));
        let read = Program::parse(&given.program).expect("read");
        assert_eq!(read.advance(1), Ok(Some(333.0)), "lenIV {len_iv}");
        assert_eq!(super::outline(&read, 1), super::outline(&read, 0));
    }
}

#[test]
fn an_sbw_width_is_restated_and_a_charstring_that_opens_otherwise_is_refused() {
    let mut sbw = vec![];
    for value in [10, 0, 500, 0] {
        sbw.extend(number(value));
    }
    sbw.extend([ESCAPE, SBW, ENDCHAR]);
    let (data, lengths) = program(&[(".notdef", sbw)], &PLAIN);
    let given = with_glyphs_given(
        &data,
        lengths,
        &[asked(GivenOutline::NotdefsOwn, None, 900)],
    )
    .expect("written");
    let read = Program::parse(&given.program).expect("read");
    assert_eq!(read.advance(1), Ok(Some(900.0)));

    let (data, lengths) = program(&[(".notdef", vec![ENDCHAR])], &PLAIN);
    assert!(matches!(
        with_glyphs_given(
            &data,
            lengths,
            &[asked(GivenOutline::NotdefsOwn, None, 900)]
        ),
        Err(Type1Error::NotWritten { .. })
    ));
}

#[test]
fn a_name_held_or_unreadable_is_refused_and_fresh_names_skip_the_held_ones() {
    let (data, lengths) = program(&[(".notdef", boxed(250)), ("notdef", empty(1))], &PLAIN);
    for name in ["notdef", "a b", "a/b", ""] {
        assert!(
            with_glyphs_given(
                &data,
                lengths,
                &[asked(GivenOutline::Nothing, Some(name), 1)]
            )
            .is_err(),
            "{name:?}"
        );
    }
    let given = with_glyphs_given(
        &data,
        lengths,
        &[
            asked(GivenOutline::Nothing, None, 1),
            asked(GivenOutline::Nothing, None, 2),
        ],
    )
    .expect("written");
    let names: Vec<_> = given
        .added
        .iter()
        .map(|glyph| glyph.name.as_str())
        .collect();
    assert_eq!(names, ["notdef.1", "notdef.2"]);
}

#[test]
fn the_glyph_bound_is_checked_before_anything_is_written() {
    let (data, lengths) = program(&[(".notdef", boxed(250))], &PLAIN);
    let held = asked(GivenOutline::Nothing, None, 500);
    let at_bound = vec![held.clone(); MOST_GLYPHS_GIVEN];
    let given = with_glyphs_given(&data, lengths, &at_bound).expect("one at the bound is written");
    assert_eq!(given.added.len(), MOST_GLYPHS_GIVEN);
    let past = vec![held; MOST_GLYPHS_GIVEN + 1];
    assert!(matches!(
        with_glyphs_given(&data, lengths, &past),
        Err(Type1Error::NotWritten { .. })
    ));
}

/// A `.notdef` of `segments` one-unit `rlineto`s after `0 500 hsbw`, closed by `endchar`: five
/// bytes and three a segment.
fn notdef_of(segments: usize) -> Vec<u8> {
    let mut notdef = number(0);
    notdef.extend(number(500));
    notdef.push(HSBW);
    let segment = [number(1), number(0), vec![RLINETO]].concat();
    for _ in 0..segments {
        notdef.extend_from_slice(&segment);
    }
    notdef.push(ENDCHAR);
    notdef
}

#[test]
fn a_copy_of_notdef_is_written_up_to_the_charstring_bound_and_refused_past_it() {
    // 5 + 3 × 21 842 bytes and lenIV's 4 are MOST_CHARSTRING_BYTES exactly; one segment more is
    // past it, and the refusal comes before any glyph is built.
    for (segments, written) in [(21_842, true), (21_843, false)] {
        let notdef = notdef_of(segments);
        assert_eq!(notdef.len() + 4 <= MOST_CHARSTRING_BYTES, written);
        let (data, lengths) = program(&[(".notdef", notdef)], &PLAIN);
        let all = vec![asked(GivenOutline::NotdefsOwn, None, 500); MOST_GLYPHS_GIVEN];
        let result = with_glyphs_given(&data, lengths, &all);
        if written {
            assert_eq!(result.expect("at the bound").added.len(), MOST_GLYPHS_GIVEN);
        } else {
            assert!(
                matches!(&result, Err(Type1Error::NotWritten { detail })
                    if detail.contains("65 535")),
                "{result:?}"
            );
        }
    }
}

/// `type1_write`'s finding `defcb9b3`, minimised by `cargo fuzz tmin`: a header ending `eexec`
/// and a carriage return.
const CR_HEADER: &[u8] = b"%!PS-AdobeFont-01\n/Fo\xcd eexec\r";

/// The same finding's section as it decrypts: four leading bytes, the Private dictionary and
/// three charstrings under `lenIV` 4, and the bytes the finding left after `end`.
const CR_SECTION: &[u8] = b"\
    \xff\xff\xff\xffdup /Private 8 dict dup begin\n\
    /RD{string currentfile exch readstring pop}executeonly def\n\
    /ND{noaccess def}executeonly def\n\
    /NP{noaccess put}executeonly def\n\
    /lenIV 4 def\n\
    /Subrs 0 array\n\
    2 index /CharStrings 3 dict dup begin\n\
    /.notdef 21 RD \x10\xbf1pO\xa5\x97\xdboZJ9\x11\xb4\xb3\xcen\x9e\x9b\n\x19 ND\n\
    /A 21 RD \x10\xbf1pO\xab\x89'1biI\xb2\xe9r\xdc\xe0\xf9\x9eDA ND\n\
    /B 21 RD \x10\xbf1pO\xab\x89'1biI\xd6\xe2\x8c\x14O\xe9\x9ai\x1c ND\n\
    end\n\
    \xb9\xda";

#[test]
fn a_carriage_return_after_eexec_is_written_as_a_line_feed_and_the_program_written_again() {
    // Adobe Type 1 Font Format section 7.2 starts the section at the first byte after the white
    // space after `eexec`; `read-fonts` skips a carriage return there only where a line feed
    // occurs later in the data. This program's ciphertext holds one, so `read-fonts` finds its
    // section; the program written from the program written from it holds none, so only the
    // line feed written for the carriage return lets its section be found.
    let section = encrypt(CR_SECTION, EEXEC_KEY);
    assert!(section.contains(&b'\n'));
    let data = [CR_HEADER, &section].concat();
    let length1 = CR_HEADER.len();
    // The three glyphs `type1_write` asks for under the finding's first two bytes, 3 and 180.
    let glyphs = [
        asked(GivenOutline::Nothing, Some("g0"), 660),
        asked(GivenOutline::Nothing, None, 761),
        asked(GivenOutline::NotdefsOwn, None, 862),
    ];
    let given = with_glyphs_given(&data, (length1, section.len()), &glyphs).expect("written");
    let again = with_glyphs_given(
        &given.program,
        (given.lengths[0], given.lengths[1]),
        &[asked(GivenOutline::Nothing, None, 500)],
    )
    .expect("written again");
    assert!(!again.program[length1..].contains(&b'\n'));
    assert_eq!(given.lengths[0], length1);
    assert_eq!(given.program[..length1 - 1], CR_HEADER[..length1 - 1]);
    assert_eq!(given.program[length1 - 1], b'\n');
}

#[test]
fn lengths_that_do_not_divide_the_program_at_eexec_and_a_hex_section_are_refused() {
    let (data, (length1, length2)) = program(&[(".notdef", boxed(250))], &PLAIN);
    let one = [asked(GivenOutline::Nothing, None, 1)];
    for lengths in [
        (length1 - 1, length2),
        (length1, length2 + 1),
        (usize::MAX, 1),
    ] {
        assert!(
            with_glyphs_given(&data, lengths, &one).is_err(),
            "{lengths:?}"
        );
    }
    let mut hex = data[..length1].to_vec();
    hex.extend_from_slice(b"ABCD0123");
    assert!(with_glyphs_given(&hex, (length1, 8), &one).is_err());
}

#[test]
fn a_program_with_no_notdef_is_refused() {
    let (data, lengths) = program(&[("A", empty(700))], &PLAIN);
    assert!(matches!(
        with_glyphs_given(&data, lengths, &[asked(GivenOutline::Nothing, None, 1)]),
        Err(Type1Error::NotWritten { .. })
    ));
}
