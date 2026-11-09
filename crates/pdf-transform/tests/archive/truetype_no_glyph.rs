//! A shown code a simple TrueType font's route sends to no glyph is given a glyph of its own that
//! draws nothing, at the code's width, and the content stream stays the producer's bytes
//! (`doc/adr/2753`, `doc/questions/A694` item 3).
//!
//! ISO 19005-2 section 6.2.11.4.1 (ISO 19005-4 section 6.2.10.4.1) asks the embedded program to
//! define every glyph the file renders, and a code that reaches no glyph is one it does not
//! define. This reader advances by the code's width and draws no mark for it, so the glyph the
//! code is given has no outline; the expected values below are those two facts, read from the
//! source before it is converted, and the program's own glyph 0 for a code that reached it
//! (`doc/adr/2543`).

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Decision, LIBERATION_SANS, Level, Rewrite, Target, Verdict,
    conversion, convert, decision, holds, stream,
};

const DEFINED: &str = "fonts/embedded-programs-define-every-glyph-shown";

/// Liberation Sans with its `cmap` replaced by one (1, 0) format 6 subtable over codes 3 to 66,
/// the shape `issue17333.pdf`'s subset carries: code 3 to glyph 0, `A` and `B` (65 and 66) to
/// the glyphs the face's own (3, 1) subtable gives U+0041 and U+0042, and every code between to
/// glyph 0. Codes 1 and 2 lie below the subtable's first code, so no subtable maps them.
fn a_program_mapping_from_code_3() -> Vec<u8> {
    const FIRST: u16 = 3;
    const LAST: u16 = 66;
    // Sixty-four codes: the subtable's length is its ten header bytes and two a code.
    const COUNT: u16 = LAST - FIRST + 1;
    const LENGTH: u16 = 10 + 2 * COUNT;
    let mut subtable = Vec::new();
    for value in [6, LENGTH, 0, FIRST, COUNT] {
        subtable.extend_from_slice(&value.to_be_bytes());
    }
    for code in FIRST..=LAST {
        let glyph: u16 = match code {
            65 => 36,
            66 => 37,
            _ => 0,
        };
        subtable.extend_from_slice(&glyph.to_be_bytes());
    }
    let mut cmap = Vec::new();
    for value in [0_u16, 1, 1, 0] {
        cmap.extend_from_slice(&value.to_be_bytes());
    }
    cmap.extend_from_slice(&12_u32.to_be_bytes());
    cmap.extend_from_slice(&subtable);
    pdf_font::embedding::with_tables(LIBERATION_SANS, &[(*b"cmap", cmap)])
        .expect("the face takes the table")
}

/// A page showing `A`, codes 1 and 2, code 3 and `B` in one non-symbolic TrueType font over
/// [`a_program_mapping_from_code_3`]: `WinAnsiEncoding` and `StandardEncoding` name nothing for
/// codes 1 and 2 and no subtable maps them, so they reach no glyph (ISO 32000-2 §9.6.5.4); the
/// `Differences` name code 3 `.notdef`, which the program's `post` table gives glyph 0. `/Widths`
/// gives codes 1, 2 and 3 `widths`.
fn a_page_showing_codes_on_no_glyph(widths: [u16; 3]) -> Vec<u8> {
    let program = a_program_mapping_from_code_3();
    let mut each = vec!["667".to_owned(); 66];
    for (at, width) in widths.iter().enumerate() {
        each[at] = width.to_string();
    }
    let widths = each.join(" ");
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A\\001\\002\\003B) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 1 \
                 /LastChar 66 /Widths [{widths}] /FontDescriptor 7 0 R /Encoding << /Type \
                 /Encoding /BaseEncoding /WinAnsiEncoding /Differences [3 /.notdef] >> >>"
            ),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-543 -303 \
             1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 \
             /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(&format!("/Length {}", program.len()), &program)],
        ..Conforming::part_two()
    }
    .build()
}

/// Font object 6 of `bytes`, loaded as this reader draws it.
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

/// The glyph each of codes 1, 2, 3, 65 and 66 reaches in font object 6 of `bytes`.
fn glyphs_of(bytes: &[u8]) -> [Option<u16>; 5] {
    let (_, font) = font_of(bytes);
    [1, 2, 3, 65, 66].map(|byte| font.glyph_index(Code::single_byte(byte)))
}

#[test]
fn codes_on_no_glyph_at_one_width_share_one_empty_glyph_beside_the_notdef_copy() {
    given_glyphs(&a_page_showing_codes_on_no_glyph([500, 500, 667]), true);
}

#[test]
fn codes_on_no_glyph_at_two_widths_take_two_empty_glyphs_each_at_its_width() {
    given_glyphs(&a_page_showing_codes_on_no_glyph([250, 1000, 667]), false);
}

/// Converts `source` at 2b and part 4 and holds each output to the construction: codes 1 and 2
/// reach a glyph that is not glyph 0 and draws nothing, at their widths (one glyph where
/// `shared`, two otherwise); code 3 reaches a copy of glyph 0's outline; `A` and `B` their own.
fn given_glyphs(source: &[u8], shared: bool) {
    let before = glyphs_of(source);
    assert_eq!(
        before[..3],
        [None, None, Some(0)],
        "codes 1 and 2 reach no glyph, code 3 reaches .notdef: {before:?}"
    );
    let (_, drawn) = font_of(source);
    for byte in [1, 2] {
        assert!(
            drawn.outline(Code::single_byte(byte)).is_none(),
            "this reader draws no mark for code {byte}"
        );
    }
    let failed: Vec<&str> = holds(source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&DEFINED), "{failed:?}");
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, DEFINED),
                Decision::Stated {
                    rewrite: Rewrite::NotdefGlyphGiven,
                    ..
                }
            ),
            "{:?}",
            decision(&report, DEFINED)
        );
        // The first pass answers it: the second (ADR 2534) repairs nothing here.
        assert!(
            conversion(&report).settled.is_empty(),
            "{:?}",
            conversion(&report).settled
        );
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
        let after = glyphs_of(&output);
        let (_, font) = font_of(&output);
        let empty = after[0].expect("code 1 reaches a glyph");
        assert_ne!(empty, 0, "code 1 does not reach .notdef");
        if shared {
            assert_eq!(after[1], Some(empty), "the two codes share the one glyph");
        } else {
            assert!(
                after[1].is_some_and(|other| other != empty && other != 0),
                "two widths, two glyphs: {after:?}"
            );
        }
        for byte in [1, 2] {
            let code = Code::single_byte(byte);
            assert!(
                font.outline(code)
                    .is_none_or(|path| path.commands().is_empty()),
                "code {byte}'s glyph draws nothing"
            );
            let width = drawn.advance(code);
            let program = font.program_advance(code).expect("the program states one");
            assert!(
                (program - width).abs() <= 0.001,
                "code {byte}'s program advance {program} is its width {width}"
            );
        }
        let copy = after[2].expect("code 3 reaches a glyph");
        assert!(
            copy != 0 && copy != empty && Some(copy) != after[1],
            "code 3 reaches its own copy of .notdef: {after:?}"
        );
        assert_eq!(
            font.outline(Code::single_byte(3))
                .map(|path| path.commands().to_vec()),
            drawn
                .outline(Code::single_byte(3))
                .map(|path| path.commands().to_vec()),
            "code 3 draws .notdef's outline, as it did"
        );
        assert_eq!(
            after[3..],
            before[3..],
            "A and B reach the glyphs they reached"
        );
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains("BT /F1 12 Tf 10 100 Td (A\\001\\002\\003B) Tj ET"),
            "the content stream is the producer's bytes"
        );
    }
}
