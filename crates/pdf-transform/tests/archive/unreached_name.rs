//! A code whose `/Differences` name no route reaches, in a font whose `/FontFile2` this reader does
//! not load, is given the empty glyph under the OpenType face with `CFF ` outlines the window drew
//! the font from (`doc/adr/3493`).
//!
//! ISO 32000-2 §9.6.5.4 takes a simple TrueType font's code by its encoding's glyph name through
//! the Adobe Glyph List to a character, or to a glyph the face's `post` names; a name neither
//! reaches ends the route on any machine, so the window draws nothing for the code (ADR 2880).
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid showing `.notdef`, so the
//! code is given a glyph of its own that draws nothing, at the width the dictionary states, and
//! every other shown code keeps the glyph the window drew.
//!
//! **The face is the machine's, not one built here**: the route under test is the one that embeds
//! the face the window drew a font from (`doc/adr/2773`), and which face that is is a fact of the
//! machine — `NimbusSans-Regular`, URW's OpenType `CFF ` face, on the machines these tests run on.
//! So the test asks the viewer first whether it draws the fixture from a face of this machine's,
//! as `machine_face` does, and says so where it does not.

use super::cut_program::zlib_stored;
use super::*;
use pdf_transform::profiles::{Contribution, named};

/// A TrueType font named for the machine's face, its `/FontFile2` a zlib stream whose Adler-32
/// disowns it, its `/Differences` naming code 65 `/g5167`, showing `A` and `B`.
fn an_unreadable_program_naming_a_name_no_route_reaches() -> Vec<u8> {
    let mut disowned = zlib_stored(LIBERATION_SANS);
    *disowned
        .last_mut()
        .expect("a zlib stream ends in its check value") ^= 0xFF;
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (AB) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /TrueType /BaseFont /NimbusSans-Regular /FirstChar 65 \
             /LastChar 66 /Widths [1000 667] /FontDescriptor 7 0 R /Encoding << /BaseEncoding \
             /WinAnsiEncoding /Differences [65 /g5167] >> >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /NimbusSans-Regular /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!(
                "/Filter /FlateDecode /Length {} /Length1 {}",
                disowned.len(),
                LIBERATION_SANS.len()
            ),
            &disowned,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// Whether the window draws the font from a face of this machine's: a program this reader does not
/// load is drawn as the font with no program is (ADR 2870), so the question is asked of that font.
fn drawn_from_a_machine_face_in_its_programs_place(source: &[u8]) -> bool {
    let document = Document::open_with_limits(source.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = document.get(ObjectId::new(6, 0));
    let font = font.as_dict().expect("object 6 is the font");
    let dropped = pdf_font::without_program(&document, font).expect("a descriptor to copy");
    let drawn = pdf_font::LoadedFont::load(&document, &dropped, "F1").expect("the font loads");
    let shipped =
        pdf_font::standard::shipped_face(&document, &dropped, "F1").expect("a face is shipped");
    drawn
        .substitute_program()
        .is_some_and(|program| program != shipped.program)
}

#[test]
fn a_name_no_route_reaches_is_given_the_empty_glyph_under_a_machines_compact_font_face() {
    let source = an_unreadable_program_naming_a_name_no_route_reaches();
    if !drawn_from_a_machine_face_in_its_programs_place(&source) {
        eprintln!("NimbusSans-Regular: this machine has no such face");
        return;
    }
    let unreached = pdf_font::Code::single_byte(b'A');
    let reached = pdf_font::Code::single_byte(b'B');
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        // `as-shown`'s answers: `machine-face` withheld, so the face the window drew is the one
        // embedded, as it is for `issue13316_reduced`.
        let authorised = Contribution::read(named("as-shown").expect("shipped").text, target)
            .expect("a shipped profile reads")
            .authorised;
        let (report, output) = convert(&source, target, authorised);
        let output = written(&report, output);
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}: what was written is held to the target again"
        );
        let document = Document::open(output).expect("it opens");
        let dict = document
            .get(ObjectId::new(6, 0))
            .as_dict()
            .cloned()
            .expect("object 6 is the font");
        let font = pdf_font::LoadedFont::load(&document, &dict, "F1").expect("it loads");
        assert!(!font.is_substituted(), "{target}: its own program");
        // The face is written as the compact font program it is (ADR 2773).
        let descriptor = document.get_key(&dict, "FontDescriptor");
        assert!(
            descriptor
                .as_dict()
                .is_some_and(|descriptor| descriptor.get("FontFile3").is_some()),
            "{target}: under FontFile3"
        );
        assert!(
            font.glyph_index(unreached)
                .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH),
            "{target}: the code reaches a glyph of its own"
        );
        assert!(
            font.outline(unreached)
                .is_none_or(|outline| outline.commands().is_empty()),
            "{target}: which draws nothing, as the window drew nothing"
        );
        assert!(
            (font.advance(unreached) - 1.0).abs() < 1e-6,
            "{target}: at the width the dictionary states"
        );
        assert!(
            font.outline(reached)
                .is_some_and(|outline| !outline.commands().is_empty()),
            "{target}: and the code the route reaches keeps the glyph the window drew"
        );
    }
}
