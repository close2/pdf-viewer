//! A code a Type 3 font shows and describes no glyph for is given a glyph procedure that paints
//! nothing at the font's own width (`doc/adr/2918`).
//!
//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 require an embedded font to
//! define every glyph a file references for rendering. A Type 3 font's definitions are its
//! `/CharProcs` streams, reached by the names its encoding gives (ISO 32000-2 §9.6.4), and step b)
//! of that clause paints nothing for a name `/CharProcs` does not hold — so the procedure given
//! draws what the page already drew.

use super::*;
use pdf_model::{Pages, interpret};
use pdf_syntax::object::Object;

const DEFINED: &str = "fonts/embedded-programs-define-every-glyph-shown";

/// A page showing `ab` in a Type 3 font whose `/Widths` states 600 and 700 and whose encoding
/// names the two codes `names` (after code 97); `/CharProcs` describes `/square` alone.
fn a_type3_font_naming(names: &str) -> Vec<u8> {
    let glyph = b"600 0 d0 0 0 500 500 re f";
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (ab) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 7 0 R /Encoding 8 0 R /FirstChar 97 \
             /LastChar 98 /Widths [600 700] /Resources << >> >>"
                .to_owned(),
            "<< /square 9 0 R >>".to_owned(),
            format!("<< /Type /Encoding /Differences [97 {names}] >>"),
        ],
        binary_objects: vec![stream(&format!("/Length {}", glyph.len()), glyph)],
        ..Conforming::part_two()
    }
    .build()
}

/// Where each character of page one begins, and the text it reads as.
fn laid_out(bytes: &[u8]) -> (Vec<f32>, String) {
    let document = Document::open(bytes.to_vec()).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a page");
    let drawn = interpret(&document, &page);
    (
        drawn
            .text_layer
            .iter()
            .map(|placed| placed.quad[0])
            .collect(),
        drawn.text,
    )
}

/// The font page one's resources name `/F1`, with its `/CharProcs` and its encoding's names.
fn the_font(bytes: &[u8]) -> (Document, pdf_syntax::object::Dictionary) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a page");
    let resources = document.get_key(&page.dict, "Resources");
    let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
    let font = document.get_key(fonts.as_dict().expect("fonts"), "F1");
    let font = font.as_dict().cloned().expect("/F1");
    (document, font)
}

/// The decoded procedure `/CharProcs` names `glyph` by, in `bytes`'s `/F1`.
fn procedure_of(bytes: &[u8], glyph: &str) -> String {
    let (document, font) = the_font(bytes);
    let procedures = document.get_key(&font, "CharProcs");
    let Some(Object::Reference(at)) = procedures.as_dict().and_then(|p| p.get(glyph)).cloned()
    else {
        panic!("/{glyph} names a procedure");
    };
    let Object::Stream(stream) = document.get(at) else {
        panic!("a stream");
    };
    String::from_utf8_lossy(&document.decoded_stream_data(&stream).expect("it decodes"))
        .trim()
        .to_owned()
}

/// Every target the requirement is stated at, and the profiles: no loss is taken, so every
/// profile carries the rewrite, `refuse-any-loss` included.
fn each_conversion(source: &[u8], check: impl Fn(&str, Target, &Report, Vec<u8>)) {
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        for profile in ["as-shown", "refuse-any-loss"] {
            let text = pdf_transform::profiles::named(profile)
                .expect("a shipped profile")
                .text;
            let config =
                pdf_transform::archive::Configuration::read(text, target).expect("it reads");
            let plan = ArchivePlan {
                authorised: config.authorisations(target),
                ..plan_from(text, target)
            };
            let (report, output) = convert_with_plan(source, &plan);
            assert!(
                matches!(
                    decision(&report, DEFINED),
                    Decision::Mechanical(Rewrite::Type3GlyphDescribed)
                ),
                "{profile} at {target:?}: {:?}",
                decision(&report, DEFINED)
            );
            let output = written(&report, output);
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{profile} at {target:?}"
            );
            check(profile, target, &report, output);
        }
    }
}

#[test]
fn a_code_its_encoding_names_no_glyph_for_is_named_and_given_a_procedure_painting_nothing() {
    // Code 98 has no `/Differences` entry, and Table 110 makes that array the complete encoding:
    // the code reaches no glyph. It is named `/empty`, whose procedure states the font's width
    // for it, 700, and paints nothing; code 97 keeps the producer's `/square`.
    let source = a_type3_font_naming("/square");
    assert_eq!(
        pdf_archive::type3_encoding(&the_font(&source).0, &the_font(&source).1),
        [(97, "square".to_owned())]
    );
    each_conversion(&source, |profile, target, _, output| {
        let (document, font) = the_font(&output);
        assert_eq!(
            pdf_archive::type3_encoding(&document, &font),
            [(97, "square".to_owned()), (98, "empty".to_owned())],
            "{profile} at {target:?}"
        );
        assert_eq!(procedure_of(&output, "empty"), "700 0 d0");
        assert_eq!(
            procedure_of(&output, "square"),
            "600 0 d0 0 0 500 500 re f",
            "the producer's procedure is untouched"
        );
        assert_eq!(
            laid_out(&output),
            laid_out(&source),
            "no glyph moves, no text changes"
        );
    });
}

#[test]
fn a_name_its_char_procs_does_not_describe_keeps_its_name_and_is_given_a_procedure() {
    // Code 98 is named `/circle`, which `/CharProcs` has no key for: §9.6.4 b) paints nothing.
    // The name stays, so the code reads as it did, and `/circle` is given the procedure.
    let source = a_type3_font_naming("/square /circle");
    each_conversion(&source, |profile, target, _, output| {
        let (document, font) = the_font(&output);
        assert_eq!(
            pdf_archive::type3_encoding(&document, &font),
            [(97, "square".to_owned()), (98, "circle".to_owned())],
            "{profile} at {target:?}"
        );
        assert_eq!(procedure_of(&output, "circle"), "700 0 d0");
        assert_eq!(
            laid_out(&output),
            laid_out(&source),
            "no glyph moves, no text changes"
        );
    });
}
