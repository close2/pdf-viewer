//! A code a simple font's Type 1 program sends to `.notdef`, or to no glyph, is given a glyph of
//! its own by writing the program, and the content stream stays the producer's bytes
//! (`doc/adr/2620`, `doc/questions/A694` item 3).
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to reference `.notdef`, and section 6.2.11.4.1 (part 4's 6.2.10.4.1) requires the program to
//! define every glyph shown. The page shows `A`, code 1, which `/Differences` names `uni4E00` and
//! the program has no glyph for and so draws as its `.notdef` (§9.6.5.2), code 10, which
//! `WinAnsiEncoding` names nothing for and which reaches `.notdef` — a triangle here — and `B`. The program is built in
//! Adobe Type 1 Font Format section 2.4's shape and encrypted by section 7's rules.

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Decision, Level, Rewrite, Target, Verdict, conversion, convert,
    decision, holds, stream,
};

const NOTDEF_SHOWN: &str = "fonts/no-notdef-glyph-shown";

const EVERY_GLYPH: &str = "fonts/embedded-programs-define-every-glyph-shown";

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
        ("B", triangle(722, 500)),
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

fn a_page_showing_codes_without_glyphs() -> Vec<u8> {
    let mut each = vec!["722".to_owned(); 66];
    "500".clone_into(&mut each[0]);
    "250".clone_into(&mut each[9]);
    let widths = each.join(" ");
    let (data, length1, length2) = program();
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A\\001\\012B) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /Fixture /FirstChar 1 /LastChar 66 \
                 /Widths [{widths}] /FontDescriptor 7 0 R /Encoding << /Type /Encoding \
                 /BaseEncoding /WinAnsiEncoding /Differences [1 /uni4E00] >> >>"
            ),
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

/// The font object 6 of `bytes`, loaded.
fn font_of(bytes: &[u8]) -> (Document, LoadedFont) {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let font = LoadedFont::load(&held, &dict, "").expect("the font loads");
    (held, font)
}

#[test]
fn a_code_on_notdef_and_a_code_on_no_glyph_are_given_glyphs_of_their_own() {
    let source = a_page_showing_codes_without_glyphs();
    let (_, before) = font_of(&source);
    let [a, one, ten, b] = [b'A', 1, 10, b'B'].map(Code::single_byte);
    assert_eq!(
        before.glyph_index(one),
        None,
        "uni4E00 is not in the program"
    );
    assert_eq!(before.glyph_index(ten), Some(pdf_font::NOTDEF_GLYPH));
    let notdef_drawn = before
        .outline(ten)
        .expect("the .notdef triangle")
        .commands()
        .to_vec();
    assert!(!notdef_drawn.is_empty());
    assert!(before.draws_notdef_in_place(one));
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(
        failed.contains(&NOTDEF_SHOWN) && failed.contains(&EVERY_GLYPH),
        "{failed:?}"
    );
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        for row in [NOTDEF_SHOWN, EVERY_GLYPH] {
            assert!(
                matches!(
                    decision(&report, row),
                    Decision::Stated {
                        rewrite: Rewrite::NotdefGlyphGiven,
                        ..
                    }
                ),
                "{row}: {:?}",
                decision(&report, row)
            );
        }
        let output = output.unwrap_or_else(|| {
            panic!(
                "{:?}\n{:?}\n{:?}",
                conversion(&report).achieved,
                report.refused,
                report.warnings
            )
        });
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let (_, after) = font_of(&output);
        assert_eq!(after.selected_glyph_name(one), Some("uni4E00"));
        assert_eq!(after.selected_glyph_name(ten), Some("notdef"));
        for code in [one, ten] {
            let glyph = after.glyph_index(code).expect("the code reaches a glyph");
            assert_ne!(glyph, pdf_font::NOTDEF_GLYPH);
            let program = after.program_advance(code).expect("a program advance");
            assert!((program - after.advance(code)).abs() <= 0.001, "{program}");
        }
        // Both drew `.notdef`'s triangle — code 1 by §9.6.5.2's substitution for a name the
        // program lacks — and both draw it from a glyph of their own.
        for code in [one, ten] {
            assert_eq!(
                after
                    .outline(code)
                    .map(|outline| outline.commands().to_vec()),
                Some(notdef_drawn.clone())
            );
        }
        assert_eq!(after.glyph_index(a), before.glyph_index(a));
        assert_eq!(after.glyph_index(b), before.glyph_index(b));
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains("BT /F1 12 Tf 10 100 Td (A\\001\\012B) Tj ET"),
            "the content stream is the producer's bytes"
        );
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(
            packet.contains("the Type 1 program of 1 simple font"),
            "{packet}"
        );
    }
}
