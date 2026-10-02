//! Composite fonts, ISO 32000-2 §9.7, on Type 0 fonts this file builds.
//!
//! `tests/composite_fonts.rs` holds the same rules against corpus documents, and reads them from
//! the optional `doc/pdf.js` checkout — so on a machine without it every one of those tests returns
//! having read nothing, and ten ledger rows of clause 9 were held by them alone (ADR 1497). The
//! fonts here are written out in the test: a Type 0 dictionary, its `CIDFontType2` descendant over
//! the tree's own `LiberationSans-Regular.ttf` with an `Identity` `/CIDToGIDMap`, and — where the
//! rule is about one — an embedded `CMap` stream.
//!
//! The identity `/CIDToGIDMap` is what makes the `CMap` observable: §9.7.4.2 then makes the glyph
//! index *equal* to the CID, so [`pdf_font::LoadedFont::glyph_index`] reports the CID a code was
//! decoded to, and every expected CID below is read off the `CMap`'s own lines. The metrics are
//! §9.7.4.3's three EXAMPLEs, encoded as written.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    reason = "test code: a fixture that stops loading should fail loudly; every width compared is \
              a value the test wrote divided by 1000, compared against the same division; the \
              arithmetic counts a handful of objects; and the comments quote the clause verbatim, \
              where a name is not marked up"
)]

use std::fmt::Write as _;

use pdf_font::LoadedFont;
use pdf_syntax::{Dictionary, Document, ObjectId};

/// The font program every descendant embeds: the tree's own, so no machine supplies a face.
const PROGRAM: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

/// A Type 0 font over an embedded `TrueType` program.
///
/// `encoding` is the Type 0 font's `/Encoding` when `cmap` is `None`; with a `CMap`, the
/// encoding is that stream. `descendant` is added to the `CIDFontType2` dictionary as written.
fn type0(encoding: &str, cmap: Option<&str>, descendant: &str) -> (Document, Dictionary) {
    let encoding = if cmap.is_some() { "4 0 R" } else { encoding };
    let mut objects: Vec<Vec<u8>> = vec![
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding {encoding} \
             /DescendantFonts [2 0 R] >>"
        )
        .into_bytes(),
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             /FontDescriptor 3 0 R /CIDToGIDMap /Identity {descendant} >>"
        )
        .into_bytes(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
           /FontBBox [-203 -303 1050 910] /ItalicAngle 0 /Ascent 905 /Descent -212 \
           /CapHeight 729 /StemV 80 /FontFile2 5 0 R >>"
            .to_vec(),
    ];
    let cmap = cmap.unwrap_or("");
    let mut stream = format!(
        "<< /Type /CMap /CMapName /Fixture /CIDSystemInfo << /Registry (Adobe) /Ordering \
         (Identity) /Supplement 0 >> /Length {} >>\nstream\n{cmap}\nendstream",
        cmap.len()
    )
    .into_bytes();
    objects.push(std::mem::take(&mut stream));
    let mut program = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        PROGRAM.len(),
        PROGRAM.len()
    )
    .into_bytes();
    program.extend_from_slice(PROGRAM);
    program.extend_from_slice(b"\nendstream");
    objects.push(program);

    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let table_at = out.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{table_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(table.as_bytes());

    let document = Document::open(out).expect("the fixture is a well-formed file");
    let dict = document
        .get(ObjectId {
            number: 1,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("object 1 is the Type 0 font");
    (document, dict)
}

/// Loads the fixture's font, failing loudly.
fn load(document: &Document, dict: &Dictionary) -> LoadedFont {
    LoadedFont::load(document, dict, "F1")
        .unwrap_or_else(|error| panic!("the fixture font does not load: {error}"))
}

/// Each code a string decodes to: its length in bytes and the CID it selected.
fn codes(font: &LoadedFont, bytes: &[u8]) -> Vec<(u8, Option<u16>)> {
    font.decode(bytes)
        .into_iter()
        .map(|code| (code.length(), font.glyph_index(code)))
        .collect()
}

/// An embedded `CMap`'s text around its codespace and CID mappings.
fn cmap(codespace: &str, mappings: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
         /CMapName /Fixture def\n/CMapType 1 def\n{codespace}\n{mappings}\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend"
    )
}

/// §9.7.5.3: "For character encodings that are not predefined, the PDF file shall contain a
/// stream that defines the CMap" — and Table 119's `/Encoding` names that stream.
///
/// The codespace is one one-byte range, so §9.7.1's "sequence of one or more bytes" is one byte
/// here: `<41>` is `begincidchar`'s CID 5, `<42>` and `<43>` are `begincidrange`'s 6 and 7.
#[test]
fn an_embedded_cmap_with_one_byte_ranges_makes_one_byte_one_code() {
    let (document, dict) = type0(
        "",
        Some(&cmap(
            "1 begincodespacerange\n<20> <7E>\nendcodespacerange",
            "1 begincidchar\n<41> 5\nendcidchar\n1 begincidrange\n<42> <43> 6\nendcidrange",
        )),
        "",
    );
    let font = load(&document, &dict);
    assert_eq!(
        codes(&font, b"ABC"),
        vec![(1, Some(5)), (1, Some(6)), (1, Some(7))]
    );
}

/// §9.7.1: "for composite fonts, a sequence of one or more bytes are decoded to select a glyph
/// from the descendant CIDFont" — and how many bytes is the codespace's to say, code by code.
///
/// A UTF-8-shaped codespace puts a one-byte code and a three-byte code in the same string.
#[test]
fn a_codespace_of_several_lengths_splits_a_string_where_its_ranges_say() {
    let (document, dict) = type0(
        "",
        Some(&cmap(
            "3 begincodespacerange\n<00> <7F>\n<C080> <DFBF>\n<E08080> <EFBFBF>\n\
             endcodespacerange",
            "2 begincidrange\n<61> <62> 40\n<E4B880> <E4B8FF> 300\nendcidrange",
        )),
        "",
    );
    let font = load(&document, &dict);
    assert_eq!(
        codes(&font, b"a\xE4\xB8\x81b"),
        vec![(1, Some(40)), (3, Some(301)), (1, Some(41))]
    );
}

/// Table 119's `/Encoding`: "The name of a predefined CMap, or a stream containing a CMap".
///
/// `90ms-RKSJ-H` is the name, and the CIDs are that resource's own lines: `<20> <7e> 231` and
/// `<8140> <817e> 633`. A two-byte code and a one-byte code in one string say the named `CMap`'s
/// codespace was consulted rather than Identity's.
#[test]
fn a_predefined_cmap_named_by_the_type_0_font_is_resolved() {
    let (document, dict) = type0("/90ms-RKSJ-H", None, "");
    let font = load(&document, &dict);
    assert_eq!(
        codes(&font, b"\x81\x40\x41"),
        vec![(2, Some(633)), (1, Some(231 + 0x41 - 0x20))]
    );
}

/// §9.7.4.3's EXAMPLE 1, `/W [120 [400 325 500] 7080 8032 1000]`, with a `/DW` beside it.
///
/// "the glyphs having CIDs 120, 121, and 122 are 400, 325, and 500 units wide, respectively.
/// CIDs in the range 7080 through 8032 inclusive all have a width of 1000 units." A CID neither
/// group names takes Table 115's `/DW`; and a CID named twice takes the first, because "[i]n the
/// case where it is done, the first specification is the one that shall be used".
#[test]
fn the_w_array_of_example_1_states_each_width() {
    let (document, dict) = type0(
        "/Identity-H",
        None,
        "/DW 777 /W [120 [400 325 500] 7080 8032 1000 121 [999]]",
    );
    let font = load(&document, &dict);
    for (cid, width) in [
        (120u16, 400.0f32),
        (121, 325.0),
        (122, 500.0),
        (7080, 1000.0),
        (8032, 1000.0),
        (8033, 777.0),
        (119, 777.0),
    ] {
        let decoded = font.decode(&cid.to_be_bytes());
        let code = *decoded.first().expect("a two-byte code");
        assert_eq!(font.advance(code), width / 1000.0, "CID {cid}");
    }
    assert!(!font.is_vertical(), "Identity-H is writing mode 0");
}

/// §9.7.4.3's EXAMPLE 3, `/W2 [120 [-1000 250 772] 7080 8032 -1000 500 900]`.
///
/// CID 120's "vertical displacement vector … as (0, -1000) and the position vector as (250,
/// 772)", and every CID from 7080 to 8032 displaced by (0, −1000) with the position vector
/// (500, 900). The CMap is `Identity-V`, whose writing mode is what selects these metrics
/// (§9.7.5.1: "The writing mode determines which metrics shall be used").
#[test]
fn the_w2_array_of_example_3_states_each_vertical_metric() {
    let (document, dict) = type0(
        "/Identity-V",
        None,
        "/W [120 [400] 7080 8032 1000] /W2 [120 [-1000 250 772] 7080 8032 -1000 500 900]",
    );
    let font = load(&document, &dict);
    assert!(font.is_vertical(), "Identity-V is writing mode 1");
    for (cid, displacement, position) in [
        (120u16, [0.0f32, -1.0], [0.25f32, 0.772]),
        (7080, [0.0, -1.0], [0.5, 0.9]),
        (8032, [0.0, -1.0], [0.5, 0.9]),
    ] {
        let decoded = font.decode(&cid.to_be_bytes());
        let code = *decoded.first().expect("a two-byte code");
        assert_eq!(
            font.vertical_metrics(code),
            (displacement, position),
            "CID {cid}"
        );
    }
}

/// §9.7.4.3's EXAMPLE 2: with `/DW2 [880 -1000]` "a glyph's position vector and vertical
/// displacement vector are v = (w0 ÷2,880) w1 = (0, -1000)".
///
/// Stated and left to Table 115's default, which is the same pair, it gives the same answer;
/// and a different `/DW2` moves both, which is what says the entry is read at all. "The
/// horizontal component of the position vector shall be half the glyph width" — `/W`'s 600.
#[test]
fn dw2_of_example_2_places_a_glyph_no_w2_entry_names() {
    for (entries, displacement, position) in [
        ("/DW2 [880 -1000]", [0.0f32, -1.0], [0.3f32, 0.88]),
        ("", [0.0, -1.0], [0.3, 0.88]),
        ("/DW2 [700 -900]", [0.0, -0.9], [0.3, 0.7]),
    ] {
        let (document, dict) = type0("/Identity-V", None, &format!("/W [5 [600]] {entries}"));
        let font = load(&document, &dict);
        let decoded = font.decode(&[0, 5]);
        let code = *decoded.first().expect("a two-byte code");
        assert_eq!(
            font.vertical_metrics(code),
            (displacement, position),
            "{entries:?}"
        );
    }
}
