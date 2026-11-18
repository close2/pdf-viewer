//! A substituted composite font draws each character it shows from a face that draws it.
//!
//! ISO 32000-2 §9.7.4.2 leaves a composite font whose program the document did not embed
//! reachable only by what its codes mean, and §9.10.2 says what they mean: characters. The face
//! this crate chooses for such a font is chosen for the character collection's script sample — あ
//! for `Adobe-Japan1` — and a Japanese font may still show Latin: `issue15053.pdf` shows "This
//! Message can be toggled." through `KozMinPr6N-Regular` and `UniJIS-UTF16-H`. A face chosen for
//! あ that maps no Latin letter drew none of the 29, and a face that drew every one was on the
//! machine. ADR 3067: each character the chosen face does not map is asked for on its own.
//!
//! **The machine is a fixture here, not this computer.** The process states it reads no font
//! files and is armed with a resource port answering from two faces built below — one mapping あ
//! and no Latin letter, one mapping Latin and no あ — so the answer does not depend on what fonts
//! are installed, and the test runs in a process of its own because both statements are for the
//! life of the process.
//!
//! **Calibrated** (trap 13): with `substituted_glyph`'s borrowing arm removed — the face chosen for
//! the sample drawing every character or none — `uncovered_character` answers `T` and the first
//! assertion fails.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the sums are offsets into a \
              fixture of a dozen tables"
)]

use std::fmt::Write as _;
use std::sync::OnceLock;

use pdf_font::LoadedFont;
use pdf_syntax::{Document, ObjectId};
use skrifa::MetadataProvider as _;

/// Liberation Sans, whose tables both fixture faces are built from: a real face, with outlines.
const LATIN: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

/// The two faces the port offers, in the order it offers them: the kana face first.
fn faces() -> &'static [(&'static str, Vec<u8>)] {
    static FACES: OnceLock<Vec<(&'static str, Vec<u8>)>> = OnceLock::new();
    FACES.get_or_init(|| vec![("kana face", kana_face()), ("Latin face", LATIN.to_vec())])
}

/// Liberation Sans with its `cmap` replaced by one mapping あ, い and う and nothing else.
///
/// The glyphs are Liberation's (`A`, `B`, `C`, by its own `cmap`), because what is under test is
/// which face is asked, not what the glyph looks like.
fn kana_face() -> Vec<u8> {
    let latin = read_fonts::FontRef::new(LATIN).expect("Liberation Sans reads");
    let charmap = skrifa::FontRef::new(LATIN)
        .expect("Liberation Sans reads")
        .charmap();
    let glyph = |c: char| u16::try_from(charmap.map(c).expect("mapped").to_u32()).expect("u16");
    let mapped = [
        ('\u{3042}', glyph('A')),
        ('\u{3044}', glyph('B')),
        ('\u{3046}', glyph('C')),
    ];
    let tables: Vec<([u8; 4], Vec<u8>)> = latin
        .table_directory()
        .table_records()
        .iter()
        .filter_map(|record| {
            let tag = record.tag().to_be_bytes();
            if &tag == b"cmap" {
                return Some((tag, cmap(&mapped)));
            }
            let data = latin.table_data(record.tag())?;
            Some((tag, data.as_bytes().to_vec()))
        })
        .collect();
    sfnt(&tables)
}

/// A `cmap` with one Windows Unicode subtable in format 12, one group per character.
fn cmap(mapped: &[(char, u16)]) -> Vec<u8> {
    let groups = u32::try_from(mapped.len()).expect("few");
    let mut subtable = Vec::new();
    subtable.extend_from_slice(&12_u16.to_be_bytes());
    subtable.extend_from_slice(&0_u16.to_be_bytes());
    subtable.extend_from_slice(&(16 + 12 * groups).to_be_bytes());
    subtable.extend_from_slice(&0_u32.to_be_bytes());
    subtable.extend_from_slice(&groups.to_be_bytes());
    for (character, glyph) in mapped {
        let value = u32::from(*character);
        subtable.extend_from_slice(&value.to_be_bytes());
        subtable.extend_from_slice(&value.to_be_bytes());
        subtable.extend_from_slice(&u32::from(*glyph).to_be_bytes());
    }
    let mut table = Vec::new();
    table.extend_from_slice(&0_u16.to_be_bytes());
    table.extend_from_slice(&1_u16.to_be_bytes());
    table.extend_from_slice(&3_u16.to_be_bytes());
    table.extend_from_slice(&10_u16.to_be_bytes());
    table.extend_from_slice(&12_u32.to_be_bytes());
    table.extend_from_slice(&subtable);
    table
}

/// An `sfnt` assembled from tables, each on a four-byte boundary.
fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    out.extend_from_slice(&u16::try_from(tables.len()).expect("few").to_be_bytes());
    out.extend_from_slice(&[0; 6]);
    let mut offset = 12 + 16 * tables.len();
    let mut body = Vec::new();
    for (tag, data) in tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&u32::try_from(offset).expect("small").to_be_bytes());
        out.extend_from_slice(&u32::try_from(data.len()).expect("small").to_be_bytes());
        body.extend_from_slice(data);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        offset = 12 + 16 * tables.len() + body.len();
    }
    out.extend_from_slice(&body);
    out
}

/// The broker of this fixture machine: the `skip`th of its faces that maps every character asked.
fn ask(request: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (_, wanted, skip) = pdf_font::provider::decode_request(request)?;
    let (name, face) = faces()
        .iter()
        .filter(|(_, face)| {
            let font = skrifa::FontRef::new(face).expect("the fixture faces read");
            wanted.iter().all(|c| font.charmap().map(*c).is_some())
        })
        .nth(usize::try_from(skip).ok()?)?;
    Some((pdf_font::provider::encode_identity(name), face.clone()))
}

/// `KozMinPr6N-Regular` as `issue15053.pdf` states it: `Adobe-Japan1` through `UniJIS-UTF16-H`,
/// no program and no `/ToUnicode`, so §9.10.2's third method gives every code its character.
fn japanese_font() -> LoadedFont {
    let objects: [&[u8]; 2] = [
        b"1 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /KozMinPr6N-Regular \
          /Encoding /UniJIS-UTF16-H /DescendantFonts [2 0 R] >>\nendobj\n",
        b"2 0 obj\n<< /Type /Font /Subtype /CIDFontType0 /BaseFont /KozMinPr6N-Regular \
          /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >> \
          /FontDescriptor << /Type /FontDescriptor /FontName /KozMinPr6N-Regular /Flags 4 \
          /FontBBox [0 -120 1000 880] /ItalicAngle 0 /Ascent 880 /Descent -120 \
          /CapHeight 700 /StemV 80 >> /DW 1000 >>\nendobj\n",
    ];
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut trailer = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        writeln!(trailer, "{offset:010} 00000 n ").expect("a String takes every write");
    }
    write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    )
    .expect("a String takes every write");
    out.extend_from_slice(trailer.as_bytes());
    let document = Document::open(out).expect("the fixture is a valid PDF");
    let dict = document
        .get(ObjectId {
            number: 1,
            generation: 0,
        })
        .as_dict()
        .expect("object 1 is the font dictionary")
        .clone();
    LoadedFont::load(&document, &dict, "F1").expect("the fixture machine offers a face")
}

/// A Latin string through a Japanese font: the face chosen for あ draws none of it, and each
/// letter is drawn from the face this machine offers for the letter.
#[test]
fn a_character_the_scripts_face_does_not_draw_is_drawn_from_a_face_that_does() {
    pdf_font::substitute::no_machine_fonts();
    pdf_font::provider::faces_come_from(ask);
    let font = japanese_font();

    // The script still chooses the font's own face, which is the kana face's.
    assert_eq!(
        font.substitute_program(),
        Some(faces()[0].1.as_slice()),
        "the face chosen for Adobe-Japan1's sample is the one that maps あ"
    );

    let shown = "This Message";
    let bytes: Vec<u8> = shown.encode_utf16().flat_map(u16::to_be_bytes).collect();
    let codes = font.decode(&bytes);
    assert_eq!(
        codes.len(),
        shown.chars().count(),
        "UTF-16 is two bytes a code here"
    );
    for (code, character) in codes.iter().zip(shown.chars()) {
        assert_eq!(
            font.uncovered_character(*code),
            None,
            "{character:?} is drawn by the Latin face, so it is not uncovered"
        );
        if character != ' ' {
            assert!(
                font.glyph_index(*code).is_some_and(|glyph| glyph != 0),
                "{character:?} reaches a glyph"
            );
        }
    }

    // And あ is still the kana face's: borrowing is for what the chosen face does not draw.
    let kana = font.decode(&[0x30, 0x42]);
    assert_eq!(font.uncovered_character(kana[0]), None);
}
