//! A name-keyed CFF program under a `CIDFontType0` that states a `/CIDToGIDMap` is written as the
//! CID-keyed program whose charset states the map, end to end (`doc/adr/3446`).
//!
//! ISO 32000-2 §9.7.4.2 selects a `CIDFontType0`'s glyphs through a CID-keyed program's charset,
//! and for a program with no `CIDFont` operators uses the CIDs "directly as GID values"; Table 115
//! gives `/CIDToGIDMap` to a Type 2 `CIDFont` alone. ISO 19005-2 section 6.2.11.2 and ISO 19005-4
//! section 6.2.10.2 hold the font to those clauses. The fixture's map sends CID 1 to glyph 2 and
//! CID 2 to glyph 1, so the clause's reading and the map's draw different boxes, and the written
//! program has to draw at each CID the box the map chose.

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Decision, Level, Rewrite, Target, Verdict, conversion, convert,
    decision, holds, stream,
};

const SELECTED_BY_CID: &str = "fonts/cidfonttype0-glyphs-selected-by-cid";

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

/// A name-keyed program of three glyphs and an empty Private DICT (`nominalWidthX` 0): glyph 0,
/// `.notdef`, a 700 by 700 box of width 100; glyph 1, `A` (SID 34), a 500 by 600 box of width
/// 500; glyph 2, `B` (SID 35), a 300 by 400 box of width 400. The charset in format 0.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture builder: every offset is a few hundred bytes"
)]
fn program() -> Vec<u8> {
    // `100 100 100 rmoveto 700 hlineto 700 vlineto -700 hlineto endchar`.
    let notdef: &[u8] = &[239, 239, 239, 21, 249, 80, 6, 249, 80, 7, 253, 80, 6, 14];
    // `500 50 0 rmoveto 500 hlineto 600 vlineto -500 hlineto endchar`.
    let a: &[u8] = &[
        248, 136, 189, 139, 21, 248, 136, 6, 248, 236, 7, 252, 136, 6, 14,
    ];
    // `400 50 0 rmoveto 300 hlineto 400 vlineto -300 hlineto endchar`.
    let b: &[u8] = &[
        248, 36, 189, 139, 21, 247, 192, 6, 248, 36, 7, 251, 192, 6, 14,
    ];
    let header = [1u8, 0, 4, 4];
    let names = index(&[b"F"]);
    let strings = vec![0u8, 0];
    let gsubrs = vec![0u8, 0];
    let charset = [0u8, 0, 34, 0, 35];
    let charstrings = index(&[notdef, a, b]);
    let top_len = 23;
    let charset_at = header.len() + names.len() + 3 + 8 + top_len + strings.len() + gsubrs.len();
    let charstrings_at = charset_at + charset.len();
    let private_at = charstrings_at + charstrings.len();
    let mut top = Vec::new();
    top.extend_from_slice(&int5(charset_at));
    top.push(15);
    top.extend_from_slice(&int5(charstrings_at));
    top.push(17);
    top.extend_from_slice(&int5(0));
    top.extend_from_slice(&int5(private_at));
    top.push(18);
    assert_eq!(top.len(), top_len);
    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&names);
    out.extend_from_slice(&index(&[&top]));
    out.extend_from_slice(&strings);
    out.extend_from_slice(&gsubrs);
    out.extend_from_slice(&charset);
    out.extend_from_slice(&charstrings);
    out
}

/// A page showing CIDs 1 and 2 under `Identity-H` in a `CIDFontType0` over [`program`], filed as
/// `/CIDFontType0C`, whose `/CIDToGIDMap` stream sends CID 1 to glyph 2 and CID 2 to glyph 1 and
/// whose `/W` states the widths of the glyphs the map chose.
fn a_page_drawn_through_a_map() -> Vec<u8> {
    let program = program();
    let map: &[u8] = &[0, 0, 0, 2, 0, 1];
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <00010002> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /F /Encoding /Identity-H \
             /DescendantFonts [7 0 R] >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /F /CIDSystemInfo << /Registry \
             (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R /DW 100 \
             /W [1 [400 500]] /CIDToGIDMap 10 0 R >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /F /Flags 4 /FontBBox [0 0 800 800] \
             /ItalicAngle 0 /Ascent 800 /Descent 0 /CapHeight 700 /StemV 80 /FontFile3 9 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![
            stream(
                &format!("/Subtype /CIDFontType0C /Length {}", program.len()),
                &program,
            ),
            stream(&format!("/Length {}", map.len()), map),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// The font object 6 of `bytes`, loaded, with the document holding it.
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
fn a_name_keyed_program_drawn_through_a_map_is_written_cid_keyed_drawing_what_the_map_chose() {
    let source = a_page_drawn_through_a_map();
    let (_, before) = font_of(&source);
    let [one, two]: [Code; 2] = before
        .decode(&[0, 1, 0, 2])
        .try_into()
        .expect("two codes under Identity-H");
    assert_eq!(
        before.glyph_index(one),
        Some(2),
        "the map sends CID 1 to glyph 2"
    );
    assert_eq!(before.glyph_index(two), Some(1), "and CID 2 to glyph 1");
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&SELECTED_BY_CID), "{failed:?}");
    let outline = |font: &LoadedFont, code| {
        font.outline(code)
            .map_or_default(|outline| outline.commands().to_vec())
    };
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, SELECTED_BY_CID),
                Decision::Mechanical(Rewrite::CidKeyedProgramWritten)
            ),
            "{target:?}: {:?}",
            decision(&report, SELECTED_BY_CID)
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
        let (written, after) = font_of(&output);
        let cid_font = written
            .get(ObjectId::new(7, 0))
            .as_dict()
            .cloned()
            .expect("the CIDFont is written at its own number");
        assert!(
            written.get_key(&cid_font, "CIDToGIDMap").is_null(),
            "Table 115 gives the map to Type 2 alone, so it goes"
        );
        let descriptor = written.get_key(&cid_font, "FontDescriptor");
        let descriptor = descriptor.as_dict().expect("a descriptor");
        let Object::Stream(file) = written.get_key(descriptor, "FontFile3") else {
            panic!("the program is filed under FontFile3");
        };
        assert_eq!(
            file.dict
                .get("Subtype")
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec()),
            Some(b"CIDFontType0C".to_vec()),
            "Table 124's subtype for a CID-keyed CFF"
        );
        for (code, cid) in [(one, 1), (two, 2)] {
            assert_ne!(outline(&before, code), [] as [pdf_render::PathCommand; 0]);
            assert_eq!(
                outline(&after, code),
                outline(&before, code),
                "CID {cid} draws the box the map chose"
            );
            assert!(
                (after.advance(code) - before.advance(code)).abs() <= 0.001,
                "CID {cid} keeps its width"
            );
        }
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains("BT /F1 12 Tf 10 100 Td <00010002> Tj ET"),
            "the content stream is the producer's bytes"
        );
    }
}
