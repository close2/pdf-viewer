//! A CID a `CIDFontType0`'s CID-keyed CFF program has no glyph for is given a glyph of its own by
//! adding the CID to the program's charset, and the content stream stays the producer's bytes
//! (`doc/adr/2598`, `doc/questions/A694` item 3).
//!
//! ISO 32000-2 §9.7.4.2 selects a Type 0 `CIDFont`'s glyph through the program's charset, and
//! §9.7.6.3 substitutes the glyph for CID 0 for a CID that selects none, which ISO 19005-2 section
//! 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to reference. The
//! page shows CID 1, which the charset holds, and CID 7, which it does not; the program's glyph 0
//! draws a box, under the second of its two Font DICTs.

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Because, Conforming, Decision, Level, Rewrite, Target, Verdict, conversion,
    convert, decision, holds, stream,
};

const NOTDEF_SHOWN: &str = "fonts/no-notdef-glyph-shown";

const EVERY_GLYPH: &str = "fonts/embedded-programs-define-every-glyph-shown";

/// An INDEX of `items`, offset size four (Adobe Technical Note #5176, section 5).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture builder: every offset is a few hundred bytes"
)]
fn index(items: &[&[u8]]) -> Vec<u8> {
    let mut out = u16::try_from(items.len())
        .expect("a small INDEX")
        .to_be_bytes()
        .to_vec();
    out.push(4);
    let mut offset = 1u32;
    out.extend_from_slice(&offset.to_be_bytes());
    for item in items {
        offset += u32::try_from(item.len()).expect("a small item");
        out.extend_from_slice(&offset.to_be_bytes());
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

/// A DICT integer in the five-byte form (Adobe Technical Note #5176, Table 3).
fn int5(value: usize) -> Vec<u8> {
    let mut out = vec![29];
    out.extend_from_slice(&u32::try_from(value).expect("a small offset").to_be_bytes());
    out
}

/// A CID-keyed program of two glyphs: glyph 0, CID 0, a 700 by 700 box under FD 1, whose Private
/// DICT states `nominalWidthX` 500; glyph 1, CID 1, a 500 by 600 box under FD 0. `FDSelect` in
/// format 3, the charset in format 0.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture builder: every offset is a few hundred bytes"
)]
fn program() -> Vec<u8> {
    // `100 100 100 rmoveto 700 hlineto 700 vlineto -700 hlineto endchar`: width 600.
    let notdef: &[u8] = &[239, 239, 239, 21, 249, 80, 6, 249, 80, 7, 253, 80, 6, 14];
    // `500 50 0 rmoveto 500 hlineto 600 vlineto -500 hlineto endchar`: width 500.
    let a: &[u8] = &[
        248, 136, 189, 139, 21, 248, 136, 6, 248, 236, 7, 252, 136, 6, 14,
    ];
    let header = [1u8, 0, 4, 4];
    let names = index(&[b"F"]);
    let strings = index(&[b"Adobe", b"Identity"]);
    let gsubrs = vec![0u8, 0];
    let charset = [0u8, 0, 1];
    let fd_select = [3u8, 0, 2, 0, 0, 1, 0, 1, 0, 0, 2];
    let charstrings = index(&[notdef, a]);
    let top_len = 43;
    let charset_at = header.len() + names.len() + 3 + 8 + top_len + strings.len() + gsubrs.len();
    let fd_select_at = charset_at + charset.len();
    let charstrings_at = fd_select_at + fd_select.len();
    let private_at = charstrings_at + charstrings.len();
    let mut private = int5(500);
    private.push(21);
    let fd_array_at = private_at + private.len();
    let mut top = Vec::new();
    for operand in [391, 392, 0] {
        top.extend_from_slice(&int5(operand));
    }
    top.extend_from_slice(&[12, 30]);
    top.extend_from_slice(&int5(charset_at));
    top.push(15);
    top.extend_from_slice(&int5(fd_select_at));
    top.extend_from_slice(&[12, 37]);
    top.extend_from_slice(&int5(charstrings_at));
    top.push(17);
    top.extend_from_slice(&int5(fd_array_at));
    top.extend_from_slice(&[12, 36]);
    assert_eq!(top.len(), top_len);
    let font_dict = |size: usize| {
        let mut dict = int5(size);
        dict.extend_from_slice(&int5(private_at));
        dict.push(18);
        dict
    };
    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&names);
    out.extend_from_slice(&index(&[&top]));
    out.extend_from_slice(&strings);
    out.extend_from_slice(&gsubrs);
    out.extend_from_slice(&charset);
    out.extend_from_slice(&fd_select);
    out.extend_from_slice(&charstrings);
    out.extend_from_slice(&private);
    out.extend_from_slice(&index(&[&font_dict(0), &font_dict(private.len())]));
    out
}

/// A page showing `codes` (hex, two bytes a code under `Identity-H`) in a `Type0` font over
/// [`program`], whose `/W` gives CID 1 the program's 500 and CID 7 the 600 glyph 0 has.
fn a_page_showing(codes: &str) -> Vec<u8> {
    let program = program();
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <{codes}> Tj ET").into_bytes(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /F /Encoding /Identity-H \
             /DescendantFonts [7 0 R] >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /F /CIDSystemInfo << /Registry \
             (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R /DW 600 \
             /W [1 [500]] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /F /Flags 4 /FontBBox [0 0 800 800] \
             /ItalicAngle 0 /Ascent 800 /Descent 0 /CapHeight 700 /StemV 80 /FontFile3 9 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Subtype /CIDFontType0C /Length {}", program.len()),
            &program,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The font object 6 of `bytes`, loaded.
fn font_of(bytes: &[u8]) -> LoadedFont {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    LoadedFont::load(&held, &dict, "").expect("the font loads")
}

#[test]
fn a_cid_the_charset_does_not_hold_is_given_a_copy_of_notdef_under_its_own_cid() {
    let source = a_page_showing("00010007");
    let before = font_of(&source);
    let [one, seven]: [Code; 2] = before
        .decode(&[0, 1, 0, 7])
        .try_into()
        .expect("two codes under Identity-H");
    assert_eq!(before.glyph_index(one), Some(1));
    assert_eq!(
        before.glyph_index(seven),
        Some(0),
        "CID 7 falls back to CID 0"
    );
    assert_eq!(before.drawn_cid(seven), Some(0));
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&NOTDEF_SHOWN), "{failed:?}");
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        for row in [NOTDEF_SHOWN, EVERY_GLYPH] {
            let decided = decision(&report, row);
            assert!(
                matches!(
                    decided,
                    Decision::Stated {
                        rewrite: Rewrite::NotdefGlyphGiven,
                        ..
                    }
                ) || !failed.contains(&row),
                "{row}: {decided:?}"
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
        let after = font_of(&output);
        assert_eq!(after.glyph_index(one), Some(1), "CID 1 keeps its glyph");
        assert_eq!(
            after.glyph_index(seven),
            Some(2),
            "CID 7 reaches its own glyph"
        );
        assert_eq!(after.drawn_cid(seven), Some(7));
        let outline = |font: &LoadedFont, code| {
            font.outline(code)
                .map(|outline| outline.commands().to_vec())
                .unwrap_or_default()
        };
        assert!(!outline(&before, seven).is_empty());
        assert_eq!(
            outline(&after, seven),
            outline(&before, seven),
            "the box it drew"
        );
        let program = after.program_advance(seven).expect("a program advance");
        assert!((program - after.advance(seven)).abs() <= 0.001, "{program}");
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains("BT /F1 12 Tf 10 100 Td <00010007> Tj ET"),
            "the content stream is the producer's bytes"
        );
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(
            packet.contains("the CID-keyed CFF program of 1 CIDFontType0 font"),
            "{packet}"
        );
    }
}

#[test]
fn a_code_whose_own_cid_is_0_keeps_the_refusal_and_no_file_is_written() {
    // ISO 32000-2 §9.7.4.2: CID 0 is the `CIDFont`'s `.notdef` character, so a copy has nothing
    // to give the code `<0000>` (`doc/adr/2557`).
    let source = a_page_showing("00000007");
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        let row = decision(&report, NOTDEF_SHOWN);
        assert!(
            matches!(
                row,
                Decision::Refused(Because::NotBuiltYet(why))
                    if why.contains("shows CID 0 of a CIDFontType0 font")
            ),
            "{row:?}"
        );
        assert!(output.is_none(), "and no file is written");
    }
}
