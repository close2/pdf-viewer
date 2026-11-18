//! A simple `TrueType` dictionary over a CFF-flavoured OpenType program becomes the `Type1` font
//! that selects the same glyphs by name (`doc/adr/3078`).
//!
//! ISO 32000-2 §9.9's Table 124 admits an OpenType program whose `CFF ` table uses no `CIDFont`
//! operators under a `Type1` dictionary and under no `TrueType` one, and §9.9.1 selects its glyphs
//! under `Type1` by §9.6.5.2's glyph names. The expected glyphs come from §9.6.5.4: under the
//! source's nonsymbolic `TrueType` dictionary and `WinAnsiEncoding`, codes 50, 65 and 66 are
//! `two`, `A` and `B`, looked up through the program's (3, 1) `cmap` by their Unicode values, which
//! is glyphs 1, 3 and 4 of the fixture's glyph order `.notdef two two.sups A B`.

use super::*;
use pdf_font::LoadedFont;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Object};

const FILED: &str = "fonts/font-programs-filed-by-their-format";

/// The synthetic CFF-flavoured OpenType face `pdf-model`'s selection audit is built on: five
/// glyphs of distinct rectangles, a (3, 1) `cmap` for `two`, `two.sups`, `A` and `B`, and a
/// `post` table naming no glyph.
const CFF_OTF: &[u8] =
    include_bytes!("../../../pdf-model/tests/fixtures/opentype-selection/cff.otf");

/// A page showing codes 50, 65 and 66 in a nonsymbolic `TrueType` font over [`CFF_OTF`] whose
/// encoding is `WinAnsiEncoding`, filed under `/FontFile2` as its producer filed it.
fn a_truetype_font_over_cff() -> Vec<u8> {
    a_truetype_font_over_cff_encoded("/WinAnsiEncoding")
}

/// The same, its `/Encoding` entry `encoding`.
fn a_truetype_font_over_cff_encoded(encoding: &str) -> Vec<u8> {
    let widths = vec!["500"; 17].join(" ");
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (2AB) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /AuditCFF /FirstChar 50 /LastChar \
                 66 /Widths [{widths}] /Encoding {encoding} /FontDescriptor 7 0 R >>"
            ),
            "<< /Type /FontDescriptor /FontName /AuditCFF /Flags 32 /FontBBox [0 0 350 460] \
             /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Length {0} /Length1 {0}", CFF_OTF.len()),
            CFF_OTF,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// Page one's `/F1` in `bytes`, with the document holding it.
fn the_font(bytes: &[u8]) -> (Document, Dictionary) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a page");
    let resources = document.get_key(&page.dict, "Resources");
    let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
    let font = document.get_key(fonts.as_dict().expect("fonts"), "F1");
    let font = font.as_dict().cloned().expect("/F1");
    (document, font)
}

/// The glyph and the advance this reader draws each of the three shown codes with in `bytes`.
fn drawn(bytes: &[u8]) -> Vec<(Option<u16>, f32)> {
    let (document, font) = the_font(bytes);
    let loaded = LoadedFont::load(&document, &font, "F1").expect("the font loads");
    assert!(!loaded.is_substituted(), "drawn from its own program");
    [50_u8, 65, 66]
        .iter()
        .map(|byte| {
            let [code] = loaded.decode(&[*byte])[..] else {
                panic!("one code per byte in a simple font");
            };
            (loaded.glyph_index(code), loaded.advance(code))
        })
        .collect()
}

#[test]
fn the_source_draws_the_glyphs_its_cmap_gives_the_codes_characters() {
    assert_eq!(
        drawn(&a_truetype_font_over_cff()),
        vec![(Some(1), 0.5), (Some(3), 0.5), (Some(4), 0.5)]
    );
}

#[test]
fn a_cff_program_under_a_truetype_dictionary_is_the_type1_font_drawing_the_same_glyphs() {
    let source = a_truetype_font_over_cff();
    let before = drawn(&source);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        // The retyping removes nothing, so every profile carries it, `refuse-any-loss` included.
        for profile in ["as-shown", "as-if-printed", "refuse-any-loss"] {
            let text = pdf_transform::profiles::named(profile)
                .expect("a shipped profile")
                .text;
            let config =
                pdf_transform::archive::Configuration::read(text, target).expect("it reads");
            let plan = ArchivePlan {
                authorised: config.authorisations(target),
                ..plan_from(text, target)
            };
            let (report, output) = convert_with_plan(&source, &plan);
            assert!(
                matches!(
                    decision(&report, FILED),
                    Decision::Mechanical(Rewrite::FontProgramRefiled)
                ),
                "{profile} at {target:?}: {:?}",
                decision(&report, FILED)
            );
            let output = written(&report, output);
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{profile} at {target:?}"
            );
            let (document, font) = the_font(&output);
            assert_eq!(
                document
                    .get_key(&font, "Subtype")
                    .as_name()
                    .map(|name| name.as_bytes().to_vec()),
                Some(b"Type1".to_vec())
            );
            let descriptor = document.get_key(&font, "FontDescriptor");
            let descriptor = descriptor.as_dict().expect("a descriptor");
            assert!(descriptor.get("FontFile2").is_none(), "the program moved");
            let Some(Object::Reference(at)) = descriptor.get("FontFile3").cloned() else {
                panic!("the program is under /FontFile3");
            };
            let Object::Stream(program) = document.get(at) else {
                panic!("a stream");
            };
            assert_eq!(
                document
                    .get_key(&program.dict, "Subtype")
                    .as_name()
                    .map(|name| name.as_bytes().to_vec()),
                Some(b"OpenType".to_vec()),
                "Table 125's name for an OpenType program"
            );
            assert_eq!(
                document.decoded_stream_data(&program).as_deref(),
                Some(CFF_OTF),
                "the program is carried byte for byte"
            );
            assert_eq!(drawn(&output), before, "{profile} at {target:?}");
        }
    }
}

#[test]
fn the_type1_encoding_names_each_code_by_the_glyph_it_drew() {
    let (report, output) = convert(
        &a_truetype_font_over_cff(),
        Target::Two(Level::B),
        Authorisations::default(),
    );
    let output = written(&report, output);
    let (document, font) = the_font(&output);
    let encoding = document.get_key(&font, "Encoding");
    let differences = document.get_key(
        encoding.as_dict().expect("an encoding dictionary"),
        "Differences",
    );
    let names: Vec<Vec<u8>> = differences
        .as_array()
        .expect("a Differences array")
        .iter()
        .skip(1)
        .map(|name| name.as_name().expect("a name").as_bytes().to_vec())
        .collect();
    assert_eq!(names.len(), 256, "every one-byte code, from code 0");
    assert_eq!(names[50], b"two");
    assert_eq!(names[65], b"A");
    assert_eq!(names[66], b"B");
    assert_eq!(names[67], b".notdef", "a code the cmap reaches nothing for");
}

#[test]
fn a_font_a_truetype_row_also_binds_keeps_a_refusal_naming_it() {
    // ISO 19005-2 section 6.2.11.6 asks the names of a nonsymbolic TrueType font's `/Differences`
    // to be in the Adobe Glyph List, and `two.sups` is not, so that row's rewrite would write this
    // font as a TrueType font again; the retyping does not yet end the row, and says so.
    let (report, output) = convert(
        &a_truetype_font_over_cff_encoded(
            "<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [50 /two.sups] >>",
        ),
        Target::Two(Level::B),
        Authorisations::default(),
    );
    assert!(output.is_none());
    let Decision::Refused(Because::NotBuiltYet(why)) = decision(&report, FILED) else {
        panic!("refused: {:?}", decision(&report, FILED));
    };
    assert!(why.contains("states of TrueType fonts alone"), "{why}");
}
