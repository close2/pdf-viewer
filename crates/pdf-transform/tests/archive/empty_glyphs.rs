//! A code the file's own encoding sends to `.notdef` is given an empty glyph where the face is a
//! Compact Font Format one (`doc/questions/A48`, `doc/adr/2458`), and refused by the fence's
//! sentence where it is a `glyf` one.

use super::*;

/// One of the standard 14 by name alone, showing `text` — a content stream string's bytes.
fn showing(base_font: &str, encoding: &str, text: &str) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td ({text}) Tj ET").into_bytes(),
        )),
        objects: vec![format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{base_font} {encoding} >>"
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The converted font as this reader loads it, and the conversion's entry for it.
fn converted(bytes: &[u8], target: Target) -> (pdf_font::LoadedFont, Vec<u8>, Document) {
    let (report, output) = convert(bytes, target, Authorisations::default());
    let output = written(&report, output);
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{target}"
    );
    let substituted = &conversion(&report).substituted;
    assert_eq!(substituted.len(), 1, "{substituted:?}");
    let codes = substituted.first().map(|font| font.empty_glyphs.clone());
    let (document, font) = described_font(&output);
    let loaded = pdf_font::LoadedFont::load(&document, &font, "F1").expect("the font loads");
    (loaded, codes.unwrap_or_default(), document)
}

/// The code shown reaches a glyph of the program's own that is not `.notdef`, draws nothing, and
/// states no character; every other code still draws its letter.
fn the_code_is_empty(font: &pdf_font::LoadedFont, code: u8) {
    let code = pdf_font::Code::single_byte(code);
    let glyph = font.glyph_index(code).expect("the code reaches a glyph");
    assert_ne!(glyph, pdf_font::NOTDEF_GLYPH);
    assert_eq!(font.selected_glyph_name(code), Some("notdef"));
    assert!(
        font.outline(code)
            .is_none_or(|outline| outline.commands().is_empty()),
        "the empty glyph draws nothing"
    );
    let mut text = String::new();
    font.text(code, &mut text);
    assert_eq!(text, "", "no character is stated for it");
    let letter = pdf_font::Code::single_byte(b'H');
    assert!(
        font.outline(letter)
            .is_some_and(|outline| !outline.commands().is_empty())
    );
}

#[test]
fn a_code_no_encoding_names_is_given_an_empty_glyph_in_a_compact_font_face() {
    for base_font in ["Times-Roman", "Courier-Bold", "Symbol", "ZapfDingbats"] {
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            // `\012`, the line feed: StandardEncoding and Annex D.5's and D.6's tables name nothing
            // there, so §9.6.5 sends it to `.notdef` in any program.
            let source = showing(base_font, "", "H\\012H");
            let (font, codes, _) = converted(&source, target);
            assert_eq!(codes, vec![0x0a], "{base_font} {target}");
            the_code_is_empty(&font, 0x0a);
        }
    }
}

#[test]
fn a_code_differences_names_notdef_is_given_one_too() {
    let source = showing(
        "Times-Roman",
        "/Encoding << /Type /Encoding /Differences [32 /.notdef] >>",
        "H H",
    );
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (font, codes, _) = converted(&source, target);
        assert_eq!(codes, vec![0x20], "{target}");
        the_code_is_empty(&font, 0x20);
    }
}

#[test]
fn codes_of_two_widths_are_given_two_glyphs() {
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (H\\001\\002H) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /FirstChar 0 /LastChar 72 \
             /Widths [0 250 500 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 \
             0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 722] >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build();
    let (font, codes, _) = converted(&source, Target::Two(Level::B));
    assert_eq!(codes, vec![1, 2]);
    let (one, two) = (
        pdf_font::Code::single_byte(1),
        pdf_font::Code::single_byte(2),
    );
    assert_ne!(font.glyph_index(one), font.glyph_index(two));
    assert_eq!(font.selected_glyph_name(two), Some("notdef.1"));
    assert_eq!(font.program_advance(one), Some(0.25));
    assert_eq!(font.program_advance(two), Some(0.5));
}

#[test]
fn a_glyf_face_refuses_by_the_fences_sentence() {
    let source = showing("Helvetica", "", "H\\012H");
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(output.is_none());
    match decision(&report, "fonts/font-programs-embedded") {
        Decision::Refused(Because::TheFence(sentence)) => {
            assert!(sentence.contains("glyf one"), "{sentence}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn levels_u_and_a_refuse_because_the_glyph_states_no_character() {
    let source = showing("Times-Roman", "", "H\\012H");
    for target in [Target::Two(Level::U), Target::Two(Level::A)] {
        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(output.is_none(), "{target}");
        match decision(&report, "fonts/font-programs-embedded") {
            Decision::Refused(Because::NotThisTarget(sentence)) => {
                assert!(sentence.contains("Levels U and A"), "{sentence}");
            }
            other => panic!("{target}: {other:?}"),
        }
    }
}
