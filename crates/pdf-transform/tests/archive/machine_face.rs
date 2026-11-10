//! The machine's face the viewer drew a font from is embedded where the profile withholds the loss
//! `machine-face` and the face's own licence permits (`doc/adr/2773`, `doc/adr/2774`).
//!
//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 require the program of every
//! rendered font in the file. Where the file names a face this machine has installed, the viewer
//! draws that face, so the program that keeps the page as it was shown is that face's, and the
//! shipped face embedded in its place is the loss `machine-face`.
//!
//! **Which faces a machine has is a fact of the machine**, so each test asks the viewer first,
//! through `pdf_font::LoadedFont` and not through the conversion under test, whether it draws the
//! fixture's font from a face of this machine's, and says so where it does not: what `as-shown`
//! converts is what this machine showed.

use super::*;

/// A page showing `A` in a simple font of `subtype` named `base_font`, stating a descriptor
/// without a program, `/WinAnsiEncoding` and the width `667` (Annex D's `A` of Helvetica).
fn a_font_named(subtype: &str, base_font: &str) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /{subtype} /BaseFont /{base_font} /FirstChar 65 \
                 /LastChar 65 /Widths [667] /FontDescriptor 7 0 R /Encoding /WinAnsiEncoding >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /{base_font} /Flags 32 \
                 /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
                 /CapHeight 716 /StemV 80 >>"
            ),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// The fixtures: a dictionary of each kind named for a face Table 124 admits under it as it stands
/// or once the dictionary is restated, one `glyf` face and one carrying a `CFF ` table.
const NAMED: [(&str, &str); 4] = [
    ("TrueType", "DejaVuSans"),
    ("Type1", "DejaVuSans"),
    ("TrueType", "NimbusSans-Regular"),
    ("Type1", "NimbusSans-Regular"),
];

/// The outline the viewer draws code 65 of the font `F1` (object 6) of `bytes` with, wound one way
/// (the viewer winds a substitute's glyphs counter-clockwise and leaves an embedded program's as
/// drawn, ADR 0396, and §8.5.3.3's two rules paint a lone glyph alike either way), and whether the
/// font is a substitute.
fn drawn_a(bytes: &[u8]) -> (Option<Vec<pdf_render::PathCommand>>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = document.get(ObjectId::new(6, 0));
    let font = font.as_dict().expect("object 6 is the font");
    let loaded = pdf_font::LoadedFont::load(&document, font, "F1").expect("the font loads");
    let wound = loaded.outline(pdf_font::Code::single_byte(65)).map(|path| {
        if path.signed_area() < 0.0 {
            path.reversed().commands().to_vec()
        } else {
            path.commands().to_vec()
        }
    });
    (wound, loaded.is_substituted())
}

#[test]
fn the_machines_face_the_viewer_drew_is_embedded_where_the_profile_withholds_its_loss() {
    let mut converted = 0;
    for (subtype, base_font) in NAMED {
        let source = a_font_named(subtype, base_font);
        if !drawn_from_a_machine_face(&source) {
            eprintln!("{subtype} {base_font}: this machine has no such face, so nothing to embed");
            continue;
        }
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            // Every loss withheld, as `as-shown` withholds `machine-face`.
            let (report, output) = convert(&source, target, Authorisations::default());
            let output = written(&report, output);
            assert!(
                conversion(&report).settled.is_empty(),
                "{subtype} {base_font} {target}: the first pass embeds it"
            );
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{subtype} {base_font} {target}: what was written is held to the target again"
            );
            let font = conversion(&report).substituted.first().expect("one font");
            assert_eq!(
                (font.authority, font.losses),
                (
                    pdf_transform::archive::FaceAuthority::Machine,
                    Losses::default()
                ),
                "{subtype} {base_font} {target}: the machine's face, and nothing lost"
            );
            assert!(
                font.face.contains("a face of the converting machine's"),
                "{}",
                font.face
            );
            // ISO 32000-2 §9.6.5: the embedded program now answers the code, and it draws the
            // outline the window drew.
            let (before, substituted_before) = drawn_a(&source);
            let (after, substituted_after) = drawn_a(&output);
            assert!(substituted_before && !substituted_after);
            assert!(before.is_some(), "the window drew a glyph for A");
            assert_eq!(
                before, after,
                "{subtype} {base_font} {target}: the output draws what the window drew"
            );
            converted += 1;
        }
    }
    eprintln!("{converted} conversions embedded a face of this machine's");
}

#[test]
fn where_the_profile_authorises_the_loss_the_shipped_face_goes_in_and_names_it() {
    for (subtype, base_font) in NAMED {
        let source = a_font_named(subtype, base_font);
        if !drawn_from_a_machine_face(&source) {
            eprintln!("{subtype} {base_font}: this machine has no such face");
            continue;
        }
        let (report, output) =
            convert(&source, Target::Two(Level::B), the_face_losses_authorised());
        written(&report, output);
        let font = conversion(&report).substituted.first().expect("one font");
        assert_eq!(
            font.authority,
            pdf_transform::archive::FaceAuthority::Shipped,
            "{subtype} {base_font}: as-if-printed keeps the shipped face (doc/adr/2774)"
        );
        assert!(
            font.losses.contains(Loss::MachineFace),
            "{subtype} {base_font}: {}",
            font.losses.words()
        );
    }
}
