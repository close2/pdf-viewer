//! A code the file's own encoding sends to `.notdef`, shown in a font that states its own
//! descriptor and embeds no program, is given a glyph that draws nothing: under a `glyf` face the
//! font is made the symbolic font ISO 32000-2 §9.6.5.4 describes (`doc/adr/3125`), under a
//! Compact Font Format face the encoding names the glyph (`doc/adr/3126`). Both are
//! `doc/questions/A48`'s permission, and the expected values are the clauses': ISO 19005-2
//! section 6.2.11.8 forbids the `.notdef` reference, Table 121 makes Symbolic and Nonsymbolic
//! exclusive, and section 6.2.11.6 asks a symbolic `TrueType` font to state no `/Encoding`.

use super::*;
use pdf_syntax::object::{Dictionary, Object};

/// A font of `subtype` naming `base_font` with a descriptor and no program, under
/// `WinAnsiEncoding`, showing `H`, the tab (`\011`, a code `WinAnsiEncoding` names nothing for,
/// Annex D.2) and `H`; the tab's width is 278 and `H`'s 722.
fn with_a_descriptor(subtype: &str, base_font: &str) -> Vec<u8> {
    let mut widths = vec!["0"; 64];
    widths[0] = "278";
    widths[63] = "722";
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (H\\011H) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /{subtype} /BaseFont /{base_font} /Encoding \
                 /WinAnsiEncoding /FirstChar 9 /LastChar 72 /Widths [{}] /FontDescriptor 7 0 R >>",
                widths.join(" ")
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /{base_font} /Flags 32 /FontBBox [-665 -325 \
                 2000 1006] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 >>"
            ),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// The converted font, its descriptor and its `/ToUnicode` text per code, and the codes the
/// conversion reports given an empty glyph, after holding the output to `target`.
fn converted(bytes: &[u8], target: Target) -> (Document, Dictionary, Vec<u8>) {
    let (report, output) = convert(bytes, target, Authorisations::default());
    let output = written(&report, output);
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{target}"
    );
    let substituted = &conversion(&report).substituted;
    assert_eq!(substituted.len(), 1, "{substituted:?}");
    let codes = substituted
        .first()
        .map_or_default(|font| font.empty_glyphs.clone());
    let (document, font) = described_font(&output);
    (document, font, codes)
}

/// Whether extracted `text` states no character: nothing, or only Private Use values, which is
/// what this reader reads off a symbolic font's (3, 0) subtable for a code `/ToUnicode` leaves
/// out — Unicode assigns them no meaning, and the file states none (`doc/adr/3125`).
pub(super) fn states_no_character(text: &str) -> bool {
    text.chars().all(|c| ('\u{E000}'..='\u{F8FF}').contains(&c))
}

/// The tab reaches a glyph of the program's own that is not `.notdef` and draws nothing, at its
/// width, and states no character; `H` still draws its letter and says it is an `H`.
fn the_tab_is_empty(document: &Document, font: &Dictionary) {
    let loaded = pdf_font::LoadedFont::load(document, font, "F1").expect("the font loads");
    assert!(
        loaded.substitute_program().is_none(),
        "drawn from its own program"
    );
    let tab = pdf_font::Code::single_byte(9);
    let glyph = loaded.glyph_index(tab).expect("the tab reaches a glyph");
    assert_ne!(glyph, pdf_font::NOTDEF_GLYPH);
    assert!(
        loaded
            .outline(tab)
            .is_none_or(|outline| outline.commands().is_empty()),
        "the empty glyph draws nothing"
    );
    // ISO 19005-2 section 6.2.11.5's tolerance: the program's advance, in its own units, within a
    // thousandth of an em of the dictionary's 278.
    let advance = loaded
        .program_advance(tab)
        .expect("the empty glyph has an advance");
    assert!((advance - 0.278).abs() <= 0.001, "{advance}");
    let mut text = String::new();
    loaded.text(tab, &mut text);
    assert!(states_no_character(&text), "{text:?}");
    let letter = pdf_font::Code::single_byte(b'H');
    assert!(
        loaded
            .outline(letter)
            .is_some_and(|outline| !outline.commands().is_empty())
    );
    let mut text = String::new();
    loaded.text(letter, &mut text);
    assert_eq!(text, "H");
}

#[test]
fn under_a_glyf_face_the_font_is_made_symbolic_and_the_code_reaches_an_empty_glyph() {
    let source = with_a_descriptor("TrueType", "Arial");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (document, font, codes) = converted(&source, target);
        assert_eq!(codes, vec![9], "{target}");
        // ISO 19005-2 section 6.2.11.6: a symbolic TrueType font states no Encoding.
        assert!(font.get("Encoding").is_none(), "{target}");
        let descriptor = document.get_key(&font, "FontDescriptor");
        let flags = document
            .resolve(&descriptor)
            .as_dict()
            .and_then(|descriptor| descriptor.get("Flags").and_then(Object::as_integer))
            .expect("the descriptor states its flags");
        // Table 121: Symbolic is bit 3, Nonsymbolic bit 6, and exactly one is set.
        assert_eq!(flags & (1 << 2), 1 << 2, "{target}");
        assert_eq!(flags & (1 << 5), 0, "{target}");
        assert!(
            matches!(document.get_key(&font, "ToUnicode"), Object::Stream(_)),
            "the encoding's characters are kept"
        );
        the_tab_is_empty(&document, &font);
    }
}

#[test]
fn under_a_compact_font_face_the_encoding_names_the_empty_glyph() {
    let source = with_a_descriptor("Type1", "Times-Roman");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (document, font, codes) = converted(&source, target);
        assert_eq!(codes, vec![9], "{target}");
        let loaded = pdf_font::LoadedFont::load(&document, &font, "F1").expect("the font loads");
        assert_eq!(
            loaded.selected_glyph_name(pdf_font::Code::single_byte(9)),
            Some("notdef")
        );
        the_tab_is_empty(&document, &font);
    }
}

#[test]
fn levels_u_and_a_refuse_because_the_glyph_states_no_character() {
    let source = with_a_descriptor("TrueType", "Arial");
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
