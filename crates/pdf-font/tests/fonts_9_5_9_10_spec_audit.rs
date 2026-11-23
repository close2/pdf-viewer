//! Audit 2180 of ISO 32000-2 §9.5 to §9.10 against `pdf-font`: what an inherited `CMap` keeps,
//! and what a symbolic `TrueType` font's ignored `/Encoding` may still decide.
//!
//! Each test pairs a control that passes on main with the case the audit found.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]
#![expect(
    clippy::doc_markdown,
    reason = "the specification's sentences are quoted verbatim, CMap unbackticked as it prints it"
)]

use std::fmt::Write as _;

use pdf_font::LoadedFont;
use pdf_font::cmap::CMap;
use pdf_syntax::{Document, ObjectId};

/// A one-byte codespace and the mappings given, as an embedded `CMap` file states them.
fn cmap_file(usecmap: bool, mappings: &str) -> Vec<u8> {
    let usecmap = if usecmap { "/Base usecmap\n" } else { "" };
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n{usecmap}\
         /CMapName /Fixture def\n/WMode 0 def\n\
         1 begincodespacerange\n<00> <FF>\nendcodespacerange\n{mappings}\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
    )
    .into_bytes()
}

/// Table 118's `/UseCMap`: "If this entry is present, the referencing CMap shall specify only
/// the character mappings that differ from the referenced CMap." What the referencing file
/// states for a code is therefore that code's mapping, whichever operator the referenced file
/// used for it; §9.7.6.2 then looks the code up "in the character code mappings for codes of
/// that length", of which there is one set.
#[test]
fn a_referencing_cmaps_range_overrides_the_referenced_cmaps_single_spec_audit() {
    let code = |map: &CMap, byte: u8| map.cid(map.next_code(&[byte]));

    // Control: the referenced file maps `A` to CID 5 by a range, and the referencing file's own
    // range moves it to CID 10 — a range over a range is honoured.
    let base = CMap::parse(
        &cmap_file(false, "1 begincidrange\n<41> <41> 5\nendcidrange\n"),
        None,
    );
    let over = CMap::parse(
        &cmap_file(true, "1 begincidrange\n<41> <42> 10\nendcidrange\n"),
        Some(&base),
    );
    assert_eq!(
        code(&over, b'A'),
        Some(10),
        "control: a range overrides a range"
    );
    assert_eq!(
        code(&over, b'B'),
        Some(11),
        "control: the range's second code"
    );

    // The same override where the referenced file stated `A` with `cidchar`: the referencing
    // file's range still states the mapping that differs, so `A` is CID 10.
    let base = CMap::parse(
        &cmap_file(false, "1 begincidchar\n<41> 5\nendcidchar\n"),
        None,
    );
    let over = CMap::parse(
        &cmap_file(true, "1 begincidrange\n<41> <42> 10\nendcidrange\n"),
        Some(&base),
    );
    assert_eq!(
        code(&over, b'B'),
        Some(11),
        "the range's second code is the file's own"
    );
    assert_eq!(
        code(&over, b'A'),
        Some(10),
        "the referencing CMap states A -> CID 10, which Table 118 makes the mapping that differs; \
         the referenced CMap's cidchar must not win over it"
    );
}

/// A `head` table stating 1000 units to the em, and nothing else that is read.
fn head() -> Vec<u8> {
    let mut table = vec![0_u8; 54];
    table.splice(0..4, 0x0001_0000_u32.to_be_bytes());
    table.splice(12..16, 0x5F0F_3CF5_u32.to_be_bytes());
    table.splice(18..20, 1000_u16.to_be_bytes());
    table
}

/// A version 0.5 `maxp`, which states the glyph count and nothing else.
fn maxp(glyphs: u16) -> Vec<u8> {
    let mut table = 0x0000_5000_u32.to_be_bytes().to_vec();
    table.extend_from_slice(&glyphs.to_be_bytes());
    table
}

/// A `cmap` with one (3, 1) format 6 subtable: `A` is glyph 1 and `B` glyph 2.
fn unicode_cmap() -> Vec<u8> {
    let mut table = Vec::new();
    for field in [0_u16, 1, 3, 1] {
        table.extend_from_slice(&field.to_be_bytes());
    }
    table.extend_from_slice(&12_u32.to_be_bytes());
    for field in [6_u16, 14, 0, 0x41, 2, 1, 2] {
        table.extend_from_slice(&field.to_be_bytes());
    }
    table
}

/// An sfnt holding the tables given, each on a four-byte boundary.
fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = 0x0001_0000_u32.to_be_bytes().to_vec();
    let count = u16::try_from(tables.len()).expect("a handful of tables");
    for field in [count, 0, 0, 0] {
        out.extend_from_slice(&field.to_be_bytes());
    }
    let directory = 12 + 16 * tables.len();
    let mut body = Vec::new();
    for (tag, data) in tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&0_u32.to_be_bytes());
        let offset = u32::try_from(directory + body.len()).expect("a small file");
        out.extend_from_slice(&offset.to_be_bytes());
        let length = u32::try_from(data.len()).expect("a small table");
        out.extend_from_slice(&length.to_be_bytes());
        body.extend_from_slice(data);
        body.resize(body.len().next_multiple_of(4), 0);
    }
    out.extend_from_slice(&body);
    out
}

/// A symbolic (`/Flags 4`) `TrueType` font embedding a program whose only `cmap` subtable is
/// (3, 1), with the `/Encoding` entry given.
fn symbolic_truetype(encoding: &str) -> LoadedFont {
    let program = sfnt(&[
        (*b"cmap", unicode_cmap()),
        (*b"head", head()),
        (*b"maxp", maxp(3)),
    ]);
    let objects: [Vec<u8>; 2] = [
        format!(
            "1 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Fixture {encoding} \
             /FirstChar 65 /LastChar 66 /Widths [500 500] \
             /FontDescriptor << /Type /FontDescriptor /FontName /Fixture /Flags 4 \
             /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 \
             /CapHeight 700 /StemV 80 /FontFile2 2 0 R >> >>\nendobj\n"
        )
        .into_bytes(),
        [
            format!("2 0 obj\n<< /Length {} >>\nstream\n", program.len()).into_bytes(),
            program,
            b"\nendstream\nendobj\n".to_vec(),
        ]
        .concat(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let mut xref = String::from("xref\n0 3\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());
    let document = Document::open(out).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "the fixture has a complete xref");
    let dict = document
        .get(ObjectId {
            number: 1,
            generation: 0,
        })
        .as_dict()
        .expect("object 1 is the font dictionary")
        .clone();
    LoadedFont::load(&document, &dict, "F1").expect("the fixture font loads")
}

/// §9.6.5.4: "When the font has no Encoding entry, or the font descriptor's Symbolic flag is set
/// (in which case the Encoding entry is ignored), this shall occur" — and the two steps that
/// follow name a (3, 0) and a (1, 0) subtable. A font carrying only (3, 1) reaches neither, so
/// the code is left to "a mapping of its choosing", which `truetype_code_table`'s documentation
/// says is the code offered to the subtable as a character, recorded as reader-chosen. An entry
/// the clause says is ignored decides nothing in either step: the glyph a code reaches is the
/// same whatever `/Encoding` states.
#[test]
fn a_symbolic_truetype_fonts_ignored_encoding_selects_no_glyph_spec_audit() {
    let code = pdf_font::Code::single_byte(b'A');

    // Control: no `/Encoding` at all. Code 65 reaches the (3, 1) subtable's `A`, glyph 1, by the
    // reader-chosen tier, and says so.
    let unencoded = symbolic_truetype("");
    assert_eq!(
        unencoded.glyph_index(code),
        Some(1),
        "control: code 65 offered as U+0041"
    );
    assert!(
        unencoded.glyph_needed_a_reader_chosen_mapping(code),
        "control: no rule of §9.6.5.4 reached the glyph"
    );

    // The same font with an `/Encoding` whose `/Differences` rename code 65 `B`. The symbolic
    // flag is set, so the entry is ignored and code 65 reaches what it reached above.
    let encoded = symbolic_truetype(
        "/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /B] >>",
    );
    assert_eq!(
        encoded.glyph_index(code),
        Some(1),
        "the Symbolic flag is set, so §9.6.5.4 ignores /Encoding: its /Differences name B must \
         not send code 65 to the (3, 1) subtable's B"
    );
    assert!(
        encoded.glyph_needed_a_reader_chosen_mapping(code),
        "no rule of §9.6.5.4 reaches a glyph for code 65 in a symbolic font without a (3, 0) or \
         (1, 0) subtable, so whatever reached it was the reader's choice"
    );
}
