//! A code Annex D encodes twice, at a width of its own, reaches a copy of its twin's glyph
//! (`doc/adr/2723`); and a font whose descriptor, or whose `CIDFont`, is written inside another
//! object is reached all the same (`doc/adr/2722`).

use super::*;

/// `Helvetica` as a `TrueType` font with a descriptor and no program, showing the hyphen and
/// `WinAnsiEncoding`'s code 173 at the widths `hyphen` and `soft` — the shape of the corpora's
/// `486832` and `499039`, whose producers state 0 for the soft hyphen.
fn a_hyphen_and_a_soft_hyphen(hyphen: u16, soft: u16) -> Vec<u8> {
    let mut widths = vec![0_u16; 129];
    widths[0] = hyphen;
    widths[128] = soft;
    let widths: Vec<String> = widths.iter().map(ToString::to_string).collect();
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (-\\255-) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 45 \
                 /LastChar 173 /Widths [{}] /FontDescriptor 7 0 R /Encoding /WinAnsiEncoding >>",
                widths.join(" ")
            ),
            "<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn a_soft_hyphen_narrower_than_the_hyphen_reaches_a_copy_of_its_glyph() {
    // ISO 32000-1 Annex D note 5 makes code 173 the soft hyphen, typographically the same as
    // the hyphen. Named so, it reaches the face's glyph for U+00AD,
    // which Liberation Sans shares with the hyphen; the copy is the hyphen's outline at the
    // soft hyphen's own width, and ISO 19005-2 section 6.2.11.5 then holds for both codes.
    let source = a_hyphen_and_a_soft_hyphen(333, 0);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert(&source, target, the_face_losses_authorised());
        let output = written(&report, output);
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        let substituted = &conversion(&report).substituted;
        assert_eq!(
            substituted.first().map(|font| font.twin_codes.clone()),
            Some(vec![173]),
            "{target}: {substituted:?}"
        );
        let document =
            Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
        let font = document.get(ObjectId::new(6, 0));
        let font = font.as_dict().expect("object 6 is the font");
        let loaded = pdf_font::LoadedFont::load(&document, font, "F1").expect("the font loads");
        let (hyphen, soft) = (
            pdf_font::Code::single_byte(45),
            pdf_font::Code::single_byte(173),
        );
        assert_ne!(loaded.glyph_index(hyphen), loaded.glyph_index(soft));
        let commands = |code| {
            loaded
                .outline(code)
                .map(|outline| outline.commands().to_vec())
        };
        assert!(commands(hyphen).is_some_and(|drawn| !drawn.is_empty()));
        assert_eq!(commands(hyphen), commands(soft), "typographically the same");
        assert_eq!(loaded.program_advance(soft), Some(0.0));
        let mut text = String::new();
        loaded.text(soft, &mut text);
        assert_eq!(
            text, "\u{ad}",
            "the character ISO 32000-1 says the code means"
        );
    }
}

#[test]
fn a_soft_hyphen_at_the_hyphens_width_keeps_the_encoding() {
    // Two codes the dictionary states at one width already agree with one glyph's advance.
    let source = a_hyphen_and_a_soft_hyphen(333, 333);
    let (report, output) = convert(&source, Target::Two(Level::B), the_face_losses_authorised());
    let _ = written(&report, output);
    assert_eq!(
        conversion(&report)
            .substituted
            .first()
            .map(|font| font.twin_codes.clone()),
        Some(Vec::new())
    );
}

#[test]
fn a_program_behind_a_descriptor_written_inside_its_font_is_restated() {
    // ISO 32000-2 §7.3.10: a direct value and a reference to it are one value, so a descriptor
    // written inside its font names the program as one that is an object does. The program
    // makes `A` 667 wide and the dictionary states 500; ISO 19005-2 section 6.2.11.5 is answered
    // by restating the program, which is an object of its own wherever the descriptor is.
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 \
             /LastChar 65 /Widths [500] /Encoding /WinAnsiEncoding /FontDescriptor << \
             /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 7 0 R >> >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let decided = decision(&report, "fonts/widths-agree-with-the-program");
    assert_eq!(
        decided.rewrite(),
        Some(Rewrite::RestateFontMetrics),
        "{decided:?}"
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms
    );
}

#[test]
fn a_cid_font_inside_a_type0_font_written_in_a_page_is_given_the_identity_map() {
    // ISO 32000-1's Table 117 gives /CIDToGIDMap the default Identity, and where the CIDFont is
    // written does not change what its absent entry said: one inside a Type 0 font that is itself
    // written in the page's resources is given the entry there.
    let source = Conforming {
        resources: "/Font << /F1 << /Type /Font /Subtype /Type0 /BaseFont /LiberationSans \
                    /Encoding /Identity-H /DescendantFonts [<< /Type /Font /Subtype \
                    /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry \
                    (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 6 0 R \
                    /DW 1000 /W [36 [667]] >>] >> >> /ColorSpace << /CS0 [/CalGray << \
                    /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <0024> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 7 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let decided = decision(&report, "fonts/cid-to-gid-map-present");
    assert_eq!(
        decided.rewrite(),
        Some(Rewrite::CidToGidIdentity),
        "{decided:?}"
    );
    let output = output.expect("the document converts");
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms
    );
}
