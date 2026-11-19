//! A Type 1 program under a `CIDFontType0` is written as the CID-keyed CFF ISO 32000-2 §9.9's
//! Table 124 files there, every CID drawing the outline it drew (`doc/adr/3173`).
//!
//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 require a program embedded as
//! §9.9 defines, and Table 124 gives a `CIDFontType0` a CFF under `/FontFile3` `/CIDFontType0C` or
//! an OpenType program, never `/FontFile`. The fixture's program is built in Adobe Type 1 Font
//! Format section 2.4's shape with five glyphs, so the conversion carries each of chapter 6's
//! constructions it has to: a plain outline, a `seac` accent over its base, and a flex through
//! section 8.4's standard subroutines.

use super::*;
use pdf_font::LoadedFont;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Object};

const FILED: &str = "fonts/font-programs-filed-by-their-format";

/// Section 7.1's encryption under the initial key `r`.
fn encrypt(plain: &[u8], mut r: u16) -> Vec<u8> {
    plain
        .iter()
        .map(|&byte| {
            let cipher = byte ^ r.to_be_bytes()[0];
            r = u16::from(cipher)
                .wrapping_add(r)
                .wrapping_mul(52845)
                .wrapping_add(22719);
            cipher
        })
        .collect()
}

/// A number from -107 to 107, or a two-byte one from 108 to 1131, in section 6.2's encoding.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture's numbers are the few this file writes, each inside the range its arm encodes"
)]
fn number(value: i32) -> Vec<u8> {
    match value {
        -107..=107 => vec![u8::try_from(value + 139).expect("in range")],
        108..=1131 => {
            let v = value - 108;
            vec![
                u8::try_from(v / 256 + 247).expect("in range"),
                u8::try_from(v % 256).expect("in range"),
            ]
        }
        _ => {
            let v = -value - 108;
            vec![
                u8::try_from(v / 256 + 251).expect("in range"),
                u8::try_from(v % 256).expect("in range"),
            ]
        }
    }
}

/// A charstring: numbers, each operator given as its bytes after them (section 6.4).
fn charstring(parts: &[(&[i32], &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (numbers, operator) in parts {
        for value in *numbers {
            out.extend(number(*value));
        }
        out.extend_from_slice(operator);
    }
    out
}

/// `hsbw`, `rlineto`, `rmoveto`, `closepath`, `endchar`, `callsubr`, `return`, and the escaped
/// `seac`, `callothersubr`, `pop` and `setcurrentpoint` (section 6.4).
const HSBW: &[u8] = &[13];
const RLINETO: &[u8] = &[5];
const RMOVETO: &[u8] = &[21];
const CLOSE_END: &[u8] = &[9, 14];
const CALLSUBR: &[u8] = &[10];
const SEAC: &[u8] = &[12, 6];

/// A rectangle `w` by `h` from the side bearing `sb`, under `sb width hsbw`.
fn rectangle(sb: i32, width: i32, w: i32, h: i32) -> Vec<u8> {
    charstring(&[
        (&[sb, width], HSBW),
        (&[0, 0], RMOVETO),
        (&[w, 0], RLINETO),
        (&[0, h], RLINETO),
        (&[w.saturating_neg(), 0], RLINETO),
        (&[], CLOSE_END),
    ])
}

/// Section 8.4's four standard subroutines: flex's end, its start, its points, hint replacement.
fn standard_subrs() -> Vec<Vec<u8>> {
    vec![
        charstring(&[(&[3, 0], &[12, 16, 12, 17, 12, 17, 12, 33, 11])]),
        charstring(&[(&[0, 1], &[12, 16, 11])]),
        charstring(&[(&[0, 2], &[12, 16, 11])]),
        charstring(&[(&[3], &[11])]),
    ]
}

/// A flex from (50, 0) to (350, 0) bulging 20 units up, closed by a line 100 units down.
fn flexed() -> Vec<u8> {
    let mut parts: Vec<(&[i32], &[u8])> = vec![(&[50, 600], HSBW), (&[0, 0], RMOVETO)];
    parts.push((&[1], CALLSUBR));
    let points: [&[i32]; 7] = [
        &[100, 0],
        &[0, 20],
        &[50, 0],
        &[50, -20],
        &[50, 20],
        &[50, 0],
        &[0, -20],
    ];
    for point in points {
        parts.push((point, RMOVETO));
        parts.push((&[2], CALLSUBR));
    }
    parts.push((&[50, 350, 0, 0], CALLSUBR));
    parts.push((&[0, -100], RLINETO));
    parts.push((&[], CLOSE_END));
    charstring(&parts)
}

/// The program: `.notdef`, `A`, `acute`, `Aacute` (a `seac` of the two) and `F` (a flex), in that
/// order, so CIDs 1 to 4 select them under §9.7.4.2's rule; with its `/Length1` and `/Length2`.
fn program() -> (Vec<u8>, usize, usize) {
    let mut private = b"dup /Private 8 dict dup begin\n/RD{string currentfile exch readstring pop}\
        executeonly def\n/ND{noaccess def}executeonly def\n/NP{noaccess put}executeonly def\n"
        .to_vec();
    let subrs = standard_subrs();
    private.extend_from_slice(format!("/Subrs {} array\n", subrs.len()).as_bytes());
    for (index, subr) in subrs.iter().enumerate() {
        let subr = encrypt(&[&[0u8; 4][..], subr].concat(), 4330);
        private.extend_from_slice(format!("dup {index} {} RD ", subr.len()).as_bytes());
        private.extend_from_slice(&subr);
        private.extend_from_slice(b" NP\n");
    }
    private.extend_from_slice(b"2 index /CharStrings 5 dict dup begin\n");
    let aacute = charstring(&[(&[50, 600], HSBW), (&[0, 150, 700, 65, 194], SEAC)]);
    for (name, body) in [
        (".notdef", rectangle(0, 500, 400, 600)),
        ("A", rectangle(50, 600, 500, 700)),
        ("acute", rectangle(0, 300, 100, 100)),
        ("Aacute", aacute),
        ("F", flexed()),
    ] {
        let body = encrypt(&[vec![0; 4], body].concat(), 4330);
        private.extend_from_slice(format!("/{name} {} RD ", body.len()).as_bytes());
        private.extend_from_slice(&body);
        private.extend_from_slice(b" ND\n");
    }
    private
        .extend_from_slice(b"end\nend\nreadonly put\nnoaccess put\nmark currentfile closefile\n");
    let section = encrypt(&[b"\xff\xff\xff\xff".to_vec(), private].concat(), 55665);
    let mut data = b"%!PS-AdobeFont-1.0: Fixture 001\n/FontName /Fixture def\n\
        /FontMatrix [0.001 0 0 0.001 0 0] readonly def\n/Encoding StandardEncoding def\n\
        currentfile eexec\n"
        .to_vec();
    let length1 = data.len();
    data.extend_from_slice(&section);
    (data, length1, section.len())
}

/// A page showing CIDs 1 to 4 through `Identity-H` in a `Type0` font (object 6) over a
/// `CIDFontType0` (object 7) whose descriptor (object 8) files [`program`] (object 9) under
/// `/FontFile`, as `issue11740_reduced` does.
fn a_type1_program_under_a_cid_font() -> Vec<u8> {
    let (data, length1, length2) = program();
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <0001000200030004> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /Fixture /Encoding /Identity-H \
             /DescendantFonts [7 0 R] >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /Fixture /CIDSystemInfo << \
             /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R \
             /CIDToGIDMap /Identity /DW 1000 /W [0 [500 600 300 600 600]] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /Fixture /Flags 32 /FontBBox [0 0 700 800] \
             /ItalicAngle 0 /Ascent 800 /Descent 0 /CapHeight 700 /StemV 80 /FontFile 9 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!(
                "/Length {} /Length1 {length1} /Length2 {length2} /Length3 0",
                data.len()
            ),
            &data,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// Page one's `/F1` in `bytes`, with the document holding it.
fn the_font(bytes: &[u8]) -> (Document, Dictionary) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a page");
    let resources = document.get_key(&page.dict, "Resources");
    let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
    let font = document.get_key(fonts.as_dict().expect("fonts"), "F1");
    let font = font.as_dict().cloned().expect("/F1");
    (document, font)
}

/// For CIDs 0 to 4: the advance and the outline's commands this reader draws in `bytes`.
fn drawn(bytes: &[u8]) -> Vec<(f32, Option<Vec<String>>)> {
    let (document, font) = the_font(bytes);
    let loaded = LoadedFont::load(&document, &font, "F1").expect("the font loads");
    assert!(!loaded.is_substituted(), "drawn from its own program");
    (0_u8..5)
        .map(|cid| {
            let [code] = loaded.decode(&[0, cid])[..] else {
                panic!("two bytes are one code under Identity-H");
            };
            let outline = loaded.outline(code).map(|outline| {
                outline
                    .commands()
                    .iter()
                    .map(|command| format!("{command:?}"))
                    .collect()
            });
            (loaded.advance(code), outline)
        })
        .collect()
}

#[test]
fn a_type1_program_under_a_cid_font_is_the_cid_keyed_cff_drawing_every_cid_as_before() {
    let source = a_type1_program_under_a_cid_font();
    assert!(
        holds(&source, Target::Two(Level::B))
            .failures()
            .any(|failed| failed.id == FILED),
        "the fixture fails the filing requirement"
    );
    let before = drawn(&source);
    assert!(
        before.iter().all(|(_, outline)| outline.is_some()),
        "every CID draws: {before:?}"
    );
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, FILED),
                Decision::Mechanical(Rewrite::FontProgramRefiled)
            ),
            "{target:?}: {:?}",
            decision(&report, FILED)
        );
        let output = output
            .unwrap_or_else(|| panic!("{:?}\n{:?}", conversion(&report).achieved, report.refused));
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        assert_eq!(
            drawn(&output),
            before,
            "every CID draws as before at {target:?}"
        );

        let (document, font) = the_font(&output);
        let descendants = document.get_key(&font, "DescendantFonts");
        let descendant = document.resolve(
            descendants
                .as_array()
                .and_then(|array| array.first())
                .expect("one descendant"),
        );
        let descendant = descendant.as_dict().expect("a CIDFont");
        assert!(
            descendant.get("CIDToGIDMap").is_none(),
            "Table 115 states the entry of a Type 2 CIDFont alone"
        );
        let descriptor = document.get_key(descendant, "FontDescriptor");
        let descriptor = descriptor.as_dict().expect("a descriptor");
        assert!(descriptor.get("FontFile").is_none(), "the program moved");
        let Some(Object::Reference(at)) = descriptor.get("FontFile3").cloned() else {
            panic!("the program is under /FontFile3");
        };
        let Object::Stream(program) = document.get(at) else {
            panic!("a stream");
        };
        assert_eq!(
            document
                .get_key(&program.dict, "Subtype")
                .as_name()
                .map(|name| name.as_bytes().to_vec()),
            Some(b"CIDFontType0C".to_vec()),
            "Table 125's name for a CID-keyed CFF"
        );
        for length in ["Length1", "Length2", "Length3"] {
            assert!(program.dict.get(length).is_none(), "{length} is a Type 1's");
        }
        let bytes = document
            .decoded_stream_data(&program)
            .expect("the program decodes");
        assert!(
            pdf_font::cff::uses_cid_operators(&bytes).expect("a CFF"),
            "the program is CID-keyed"
        );
    }
}
