//! A simple font's Type 1 program whose `hsbw` widths disagree with the font dictionary's
//! `/Widths` has the program's widths restated, and nothing else of it (`doc/adr/2685`).
//!
//! ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 require the two statements to
//! agree; ISO 32000-2 §9.2.4 makes the dictionary's the one a reader positions glyphs by, so the
//! program's is the one that moves. Adobe Type 1 Font Format section 6.4 states a glyph's width
//! as the `wx` operand of the `hsbw` that opens its charstring. The fixture's program states 722
//! for `A` and 500 for `B`; the dictionary states 600 for `A` and 500 for `B`, or, under the
//! standard name `Courier`, no widths at all, which `doc/adr/2686` gives the ones it is laid out by.

use std::collections::BTreeMap;

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Level, Rewrite, Target, Verdict, conversion, convert, decision,
    holds, stream,
};

const WIDTHS: &str = "fonts/widths-agree-with-the-program";
const STATED: &str = "fonts/simple-font-states-its-widths";

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

/// A number from -107 to 107, or a two-byte one up to 1131, in section 6.2's encoding.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture's numbers are the few this file writes, each inside the range its arm encodes"
)]
fn number(value: i32) -> Vec<u8> {
    if (-107..=107).contains(&value) {
        vec![u8::try_from(value + 139).expect("in range")]
    } else {
        let v = value - 108;
        vec![
            u8::try_from(v / 256 + 247).expect("in range"),
            u8::try_from(v % 256).expect("in range"),
        ]
    }
}

/// `50 <width> hsbw`, a triangle `size` units across, `closepath endchar`.
fn triangle(width: i32, size: i32) -> Vec<u8> {
    let mut body = [number(50), number(width), vec![13]].concat();
    for (dx, dy) in [(0, 0), (size, 0), (0, size)] {
        body.extend([number(dx), number(dy), vec![5]].concat());
    }
    body.extend([9, 14]);
    body
}

/// The program, its `/Length1` and its `/Length2`; `lenIV` 4 by its default.
fn program() -> (Vec<u8>, usize, usize) {
    let mut private = b"dup /Private 8 dict dup begin\n/RD{string currentfile exch readstring pop}\
        executeonly def\n/ND{noaccess def}executeonly def\n/NP{noaccess put}executeonly def\n\
        /Subrs 0 array\n2 index /CharStrings 3 dict dup begin\n"
        .to_vec();
    for (name, body) in [
        (".notdef", triangle(250, 200)),
        ("A", triangle(722, 600)),
        ("B", triangle(500, 400)),
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

const CONTENT: &[u8] = b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (AB) Tj ET";

fn a_page_whose_widths_disagree_with_its_program() -> Vec<u8> {
    a_page_under(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Fixture /FirstChar 65 /LastChar 66 \
         /Widths [600 500] /FontDescriptor 7 0 R /Encoding /WinAnsiEncoding >>",
    )
}

/// The page, with font object 6 written as `font`.
fn a_page_under(font: &str) -> Vec<u8> {
    a_page_drawing(font, CONTENT)
}

/// The page, with font object 6 written as `font` and `content` its content stream.
fn a_page_drawing(font: &str, content: &[u8]) -> Vec<u8> {
    let (data, length1, length2) = program();
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((String::new(), content.to_vec())),
        objects: vec![
            font.to_owned(),
            "<< /Type /FontDescriptor /FontName /Fixture /Flags 34 /FontBBox [0 0 700 700] \
             /ItalicAngle 0 /Ascent 700 /Descent 0 /CapHeight 700 /StemV 80 /FontFile 8 0 R >>"
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

/// One code's dictionary width, program width and outline, in ems and drawing commands.
type Read = (f32, Option<f32>, Option<Vec<String>>);

/// For each of `codes`: the dictionary's width, the program's, and the outline drawn, in font 6
/// of `bytes`.
fn read(bytes: &[u8], codes: [Code; 2]) -> BTreeMap<u8, Read> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let font = LoadedFont::load(&held, &dict, "").expect("the font loads");
    assert!(!font.is_substituted(), "the program is the file's own");
    codes
        .into_iter()
        .zip(*b"AB")
        .map(|(code, key)| {
            let drawn = font.outline(code).map(|outline| {
                outline
                    .commands()
                    .iter()
                    .map(|command| format!("{command:?}"))
                    .collect()
            });
            (key, (font.advance(code), font.program_advance(code), drawn))
        })
        .collect()
}

/// Whether two widths in ems are the same number, to well inside the thousandth ISO 19005 allows.
fn near(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-5
}

#[test]
fn a_type1_programs_widths_are_restated_to_the_dictionarys_and_nothing_else_moves() {
    let source = a_page_whose_widths_disagree_with_its_program();
    let codes = (*b"AB").map(Code::single_byte);
    let before = read(&source, codes);
    assert!(near(before[&b'A'].0, 0.6), "{before:?}");
    assert!(before[&b'A'].1.is_some_and(|program| near(program, 0.722)));
    assert!(
        holds(&source, Target::Two(Level::B))
            .failures()
            .any(|failed| failed.id == WIDTHS),
        "the fixture fails the width requirement"
    );
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        assert_eq!(
            decision(&report, WIDTHS).rewrite(),
            Some(Rewrite::RestateFontMetrics),
            "{:?}",
            decision(&report, WIDTHS)
        );
        let output = output
            .unwrap_or_else(|| panic!("{:?}\n{:?}", conversion(&report).achieved, report.refused));
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let after = read(&output, codes);
        assert!(
            near(after[&b'A'].0, 0.6),
            "the dictionary's width is untouched"
        );
        assert!(
            after[&b'A'].1.is_some_and(|program| near(program, 0.6)),
            "the program now states it: {after:?}"
        );
        assert_eq!(after[&b'B'], before[&b'B'], "B agreed and is not touched");
        assert_eq!(after[&b'A'].2, before[&b'A'].2, "the outline is the same");
        assert!(
            String::from_utf8_lossy(&output).contains(std::str::from_utf8(CONTENT).expect("ASCII")),
            "the content stream is the producer's bytes"
        );
    }
}

#[test]
fn a_standard_name_stating_a_descriptor_and_no_widths_is_given_the_widths_it_is_laid_out_by() {
    // ISO 32000-1 Table 111, which ISO 19005-2 section 6.2.11.2 holds a font to, makes a standard
    // 14 font's FirstChar, LastChar, Widths and FontDescriptor all present or all absent, and ISO
    // 32000-2 Table 109, part 4's, makes all four required. This one names Courier, states a
    // descriptor and no widths, so ISO 32000-2 §9.6.2.1 has a reader lay it out by Courier's
    // published 600 units; the program states 722 for A and 500 for B (`doc/adr/2686`, 2687).
    let source = a_page_under(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /FontDescriptor 7 0 R \
         /Encoding /WinAnsiEncoding >>",
    );
    let codes = (*b"AB").map(Code::single_byte);
    let before = read(&source, codes);
    assert!(
        near(before[&b'A'].0, 0.6) && near(before[&b'B'].0, 0.6),
        "the published metrics, not MissingWidth's 0: {before:?}"
    );
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&STATED), "{failed:?}");
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        assert_eq!(
            decision(&report, STATED).rewrite(),
            Some(Rewrite::FontWidthsStated),
            "{target:?}: {:?}",
            decision(&report, STATED)
        );
        assert_eq!(
            decision(&report, WIDTHS).rewrite(),
            Some(Rewrite::RestateFontMetrics),
            "{target:?}: {:?}",
            decision(&report, WIDTHS)
        );
        let output = output
            .unwrap_or_else(|| panic!("{:?}\n{:?}", conversion(&report).achieved, report.refused));
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let written = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("opens");
        let font = written
            .get(ObjectId::new(6, 0))
            .as_dict()
            .cloned()
            .expect("the font object");
        assert_eq!(written.get_key(&font, "FirstChar").as_integer(), Some(32));
        let after = read(&output, codes);
        for key in *b"AB" {
            assert!(
                near(after[&key].0, 0.6),
                "{target:?}: the page does not move: {after:?}"
            );
            assert!(
                after[&key].1.is_some_and(|program| near(program, 0.6)),
                "{target:?}: the program states it too: {after:?}"
            );
            assert_eq!(after[&key].2, before[&key].2, "the outline is the same");
        }
    }
}

/// Every code's advance, bit for bit, and the text it reads as, in font 6 of `bytes`: what an
/// invisible layer's extraction is made of.
fn laid_out_and_read(bytes: &[u8]) -> Vec<(u32, String)> {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let font = LoadedFont::load(&held, &dict, "F1").expect("the font loads");
    (0..=u8::MAX)
        .map(Code::single_byte)
        .map(|code| {
            let mut text = String::new();
            font.text(code, &mut text);
            (font.advance(code).to_bits(), text)
        })
        .collect()
}

/// Converts `source`, whose font 6 is drawn only in rendering mode 3 and states none of the four
/// entries, to `target`, and holds the output to the descriptor `doc/adr/2688` writes: Table 120's
/// required entries, `/FontName` the dictionary's `/BaseFont`, no program, `flags` from the face
/// substituted for it, and every code advancing and reading as it did.
fn described_without_a_program(source: &[u8], target: Target, base_font: &[u8], flags: i64) {
    let before = laid_out_and_read(source);
    let (report, output) = convert(source, target, Authorisations::default());
    assert_eq!(
        decision(&report, STATED).rewrite(),
        Some(Rewrite::FontWidthsStated),
        "{target:?}: {:?}",
        decision(&report, STATED)
    );
    let output = output
        .unwrap_or_else(|| panic!("{:?}\n{:?}", conversion(&report).achieved, report.refused));
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let written = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("opens");
    let font = written
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let at = font
        .get("FontDescriptor")
        .and_then(pdf_syntax::Object::as_reference)
        .expect("Table 109: an indirect reference");
    let descriptor = written.get(at).as_dict().cloned().expect("a dictionary");
    let entry = |key: &str| written.get_key(&descriptor, key);
    assert_eq!(
        entry("Type").as_name().map(|n| n.as_bytes().to_vec()),
        Some(b"FontDescriptor".to_vec())
    );
    assert_eq!(
        entry("FontName").as_name().map(|n| n.as_bytes().to_vec()),
        Some(base_font.to_vec())
    );
    assert_eq!(entry("Flags").as_integer(), Some(flags), "{descriptor:?}");
    assert_eq!(
        entry("FontBBox").as_array().map(<[_]>::len),
        Some(4),
        "{descriptor:?}"
    );
    for key in ["ItalicAngle", "Ascent", "Descent", "CapHeight", "StemV"] {
        assert!(
            !entry(key).is_null(),
            "Table 120 requires {key}: {descriptor:?}"
        );
    }
    assert!(
        entry("Descent")
            .as_number()
            .is_some_and(|descent| descent < 0.0),
        "Table 120: the value shall be a negative number: {descriptor:?}"
    );
    for key in ["FontFile", "FontFile2", "FontFile3"] {
        assert!(entry(key).is_null(), "nothing is embedded: {descriptor:?}");
    }
    for key in ["FirstChar", "LastChar", "Widths"] {
        assert!(!written.get_key(&font, key).is_null(), "{key}: {font:?}");
    }
    assert_eq!(
        laid_out_and_read(&output),
        before,
        "{target:?}: every code advances and reads as it did"
    );
}

#[test]
fn a_standard_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_four() {
    // ISO 32000-1 Table 111 lets one of the standard 14 leave out all four entries, so part 2
    // holds nothing against it; ISO 32000-2 Table 109 requires them of every font, and part 4's
    // file is a PDF 2.0 file. ISO 19005-4 section 6.2.10.4.1 does not require the program of a
    // font used only in rendering mode 3, so the four are written with no program: Helvetica's
    // published widths, A and B 667 each (ISO 32000-2 §9.6.2.1), and the descriptor of the face
    // substituted for it, whose Table 121 flags are Nonsymbolic alone (`doc/adr/2688`).
    let source = a_page_drawing(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"BT /F1 12 Tf 3 Tr 10 100 Td (AB) Tj ET",
    );
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(!failed.contains(&STATED), "{failed:?}");
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(
        output.is_some(),
        "part 2: {:?}\n{:?}",
        conversion(&report).achieved,
        report.refused
    );
    let failed: Vec<&str> = holds(&source, Target::Four(pdf_archive::Flavour::Plain))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&STATED), "{failed:?}");
    let before = laid_out_and_read(&source);
    for key in *b"AB" {
        assert!(
            near(f32::from_bits(before[usize::from(key)].0), 0.667),
            "the published metrics: {:?}",
            before[usize::from(key)]
        );
    }
    described_without_a_program(
        &source,
        Target::Four(pdf_archive::Flavour::Plain),
        b"Helvetica",
        32,
    );
}

#[test]
fn a_truetype_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_both_parts() {
    // A `TrueType` /Arial is not one of §9.6.2.2's fourteen, so ISO 32000-1 Table 111 requires
    // all four entries of it at part 2 as well, and an OCR layer written in it is invisible and
    // owes no program (ISO 19005-2 section 6.2.11.4.1). It is laid out by the metrics of the face
    // its name resembles, which the widths written state, and described by that face: a sans
    // serif Latin face, Nonsymbolic alone (`doc/adr/2688`).
    let source = a_page_drawing(
        "<< /Type /Font /Subtype /TrueType /BaseFont /Arial /Encoding /WinAnsiEncoding >>",
        b"BT /F1 12 Tf 3 Tr 10 100 Td (AB) Tj ET",
    );
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let failed: Vec<&str> = holds(&source, target)
            .failures()
            .map(|failed| failed.id)
            .collect();
        assert!(failed.contains(&STATED), "{target:?}: {failed:?}");
        described_without_a_program(&source, target, b"Arial", 32);
    }
}
