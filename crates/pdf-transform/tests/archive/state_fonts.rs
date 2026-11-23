//! A font a graphics state's `/Font` sets is used for rendering as one `Tf` selects (ADR 3411).
//!
//! ISO 32000-2 §8.4.5's Table 57 makes a graphics state parameter dictionary's `/Font` an array of
//! a font and a size corresponding to `Tf`'s operands, so text shown after the `gs` is drawn in it,
//! and ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 bind its program.

use super::*;

/// A page that selects Courier by `gs` alone, never by `Tf`.
fn a_font_set_by_a_graphics_state() -> Vec<u8> {
    Conforming {
        resources: "/ExtGState << /GS1 << /Type /ExtGState /Font [6 0 R 12] >> >> \
                    /ColorSpace << /CS0 [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /GS1 gs 10 100 Td (Hello) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn a_font_a_graphics_state_sets_owes_its_program_and_is_given_one() {
    let source = a_font_set_by_a_graphics_state();
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        assert!(
            holds(&source, target)
                .failures()
                .any(|failed| failed.id == "fonts/font-programs-embedded"),
            "{target}: the font the text is drawn in embeds no program"
        );
        let (_, output) = convert(&source, target, the_face_losses_authorised());
        let output = output.expect("the document converts");
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        assert!(
            String::from_utf8_lossy(&output).contains("/FontFile"),
            "{target}: a program is embedded for the font"
        );
    }
}
