//! A code a simple font's bare CFF program sends to `.notdef`, or to no glyph, is given a glyph of
//! its own by writing the program, and the content stream stays the producer's bytes
//! (`doc/adr/2578`, `doc/questions/A694` item 3).
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to reference `.notdef`, and section 6.2.11.4.1 (part 4's 6.2.10.4.1) requires the program to
//! define every glyph shown. The page shows `A`, code 1, which `/Differences` names `uni4E00` and
//! the program has no glyph for, code 10, which `WinAnsiEncoding` names nothing for and the
//! program therefore draws as `.notdef`, and `B`.

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Decision, Level, Rewrite, Target, Verdict, conversion, convert,
    decision, holds, stream,
};

const NOTDEF_SHOWN: &str = "fonts/no-notdef-glyph-shown";

const EVERY_GLYPH: &str = "fonts/embedded-programs-define-every-glyph-shown";

/// A bare CFF program whose `.notdef` draws nothing, as every shipped face's does.
const FACE: &[u8] = include_bytes!("../../../../data/standard-fonts/FoxitSerif.pfb");

fn a_page_showing_codes_without_glyphs() -> Vec<u8> {
    let mut each = vec!["722".to_owned(); 66];
    "500".clone_into(&mut each[0]);
    "250".clone_into(&mut each[9]);
    let widths = each.join(" ");
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A\\001\\012B) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /ChromSerifOTF /FirstChar 1 \
                 /LastChar 66 /Widths [{widths}] /FontDescriptor 7 0 R /Encoding << /Type \
                 /Encoding /BaseEncoding /WinAnsiEncoding /Differences [1 /uni4E00] >> >>"
            ),
            "<< /Type /FontDescriptor /FontName /ChromSerifOTF /Flags 34 /FontBBox [-200 -300 \
             1200 900] /ItalicAngle 0 /Ascent 900 /Descent -250 /CapHeight 700 /StemV 80 \
             /FontFile3 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Subtype /Type1C /Length {}", FACE.len()),
            FACE,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The font object 6 of `bytes`, loaded.
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
fn a_code_on_notdef_and_a_code_on_no_glyph_are_given_glyphs_of_their_own() {
    let source = a_page_showing_codes_without_glyphs();
    let (_, before) = font_of(&source);
    let [a, one, ten, b] = [b'A', 1, 10, b'B'].map(Code::single_byte);
    assert_eq!(
        before.glyph_index(one),
        None,
        "uni4E00 is not in the program"
    );
    assert_eq!(before.glyph_index(ten), Some(pdf_font::NOTDEF_GLYPH));
    let failed: Vec<&str> = holds(&source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(
        failed.contains(&NOTDEF_SHOWN) && failed.contains(&EVERY_GLYPH),
        "{failed:?}"
    );
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(&source, target, Authorisations::default());
        for row in [NOTDEF_SHOWN, EVERY_GLYPH] {
            assert!(
                matches!(
                    decision(&report, row),
                    Decision::Stated {
                        rewrite: Rewrite::NotdefGlyphGiven,
                        ..
                    }
                ),
                "{row}: {:?}",
                decision(&report, row)
            );
        }
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
        let (_, after) = font_of(&output);
        // Code 1 keeps the name the file gave it and reaches a glyph that draws nothing, as the
        // reader drew nothing; code 10 reaches a fresh glyph named outside the Adobe Glyph List.
        assert_eq!(after.selected_glyph_name(one), Some("uni4E00"));
        assert_eq!(after.selected_glyph_name(ten), Some("notdef"));
        for code in [one, ten] {
            let glyph = after.glyph_index(code).expect("the code reaches a glyph");
            assert_ne!(glyph, pdf_font::NOTDEF_GLYPH);
            assert!(
                after
                    .outline(code)
                    .is_none_or(|outline| outline.commands().is_empty()),
                "it draws what it drew: nothing"
            );
            let program = after.program_advance(code).expect("a program advance");
            assert!((program - after.advance(code)).abs() <= 0.001, "{program}");
        }
        assert_eq!(after.glyph_index(a), before.glyph_index(a));
        assert_eq!(after.glyph_index(b), before.glyph_index(b));
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains("BT /F1 12 Tf 10 100 Td (A\\001\\012B) Tj ET"),
            "the content stream is the producer's bytes"
        );
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(
            packet.contains("the CFF program of 1 simple font"),
            "{packet}"
        );
    }
}
