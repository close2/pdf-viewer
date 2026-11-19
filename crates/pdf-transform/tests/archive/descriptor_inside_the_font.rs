//! A font descriptor written directly inside a font that embeds no program, given the program the
//! substitution embeds, and a font name that is no UTF-8 written as that program's name
//! (`doc/adr/3209`, `doc/adr/3210`).
//!
//! The expected values come from ISO 32000-2 §9.6.2.1's Table 109, whose `/FontDescriptor` entry is
//! an indirect reference to a font descriptor; §9.6.3, which reads a TrueType font's `/BaseFont`
//! off its program's `name` table; §9.8.1's Table 120, whose `/FontName` is the same name as that
//! `/BaseFont`; and ISO 19005-2 section 6.1.8, which holds font names to valid UTF-8. The bytes
//! `BA DA CC E5` are a face name in GBK and no UTF-8 sequence, as in `pdf.js`'s `SimFang-variant`
//! and `XiaoBiaoSong`, whose shape the fixture copies: a symbolic TrueType font under
//! `WinAnsiEncoding` with no program and its descriptor written inside it.

use super::*;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::{Dictionary, Object};

/// The row the font's name fails.
const NAMES: &str = "file-structure/bound-names-are-valid-utf8";

/// A page showing `ABC` in a TrueType font named in GBK's bytes, embedding no program, whose
/// descriptor is written inside the font dictionary rather than as an object of its own.
fn a_descriptor_inside_the_font() -> Vec<u8> {
    a_descriptor_inside_the_font_named("#ba#da#cc#e5")
}

/// The same font, its `/BaseFont` and `/FontName` the name `name` spells.
fn a_descriptor_inside_the_font_named(name: &str) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (ABC) Tj ET".to_vec(),
        )),
        objects: vec![format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /{name} /Encoding /WinAnsiEncoding \
             /FirstChar 65 /LastChar 67 /Widths [600 600 600] /FontDescriptor << /Type \
             /FontDescriptor /FontName /{name} /Flags 4 /FontBBox [0 -200 1000 1000] \
             /ItalicAngle 0 /Ascent 1000 /Descent -200 /CapHeight 1000 /StemV 100 >> >>"
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The conversion of `source` under the shipped profile `name` at `target`.
fn under(source: &[u8], name: &str, target: Target) -> (Report, Option<Vec<u8>>) {
    let text = pdf_transform::profiles::named(name)
        .expect("a shipped profile")
        .text;
    let config = pdf_transform::archive::Configuration::read(text, target).expect("it reads");
    let plan = ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    };
    convert_with_plan(source, &plan)
}

/// A name entry's bytes.
fn name_of(document: &Document, dict: &Dictionary, key: &str) -> Vec<u8> {
    document.get_key(dict, key).as_name().map_or_else(
        || panic!("/{key} is a name"),
        |name| name.as_bytes().to_vec(),
    )
}

#[test]
fn the_source_writes_its_descriptor_inside_the_font_and_fails_the_name_row() {
    let source = a_descriptor_inside_the_font();
    let held = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    let font = font_resource(&held);
    assert!(
        matches!(font.get("FontDescriptor"), Some(Object::Dictionary(_))),
        "the descriptor is a direct dictionary"
    );
    assert!(
        holds(&source, Target::Two(Level::B))
            .judgements
            .iter()
            .any(|judgement| judgement.id == NAMES
                && matches!(judgement.outcome, Outcome::Failed { .. })),
        "the font's name is no UTF-8"
    );
}

#[test]
fn a_descriptor_inside_the_font_is_given_the_substituted_program_as_an_object_of_its_own() {
    let source = a_descriptor_inside_the_font();
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        for profile in ["as-shown", "as-if-printed"] {
            let (report, output) = under(&source, profile, target);
            let output = written(&report, output);
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{profile} at {target:?}"
            );
            // The page keeps its text: the name goes with the program embedded, not with a
            // photograph of the page (ADR 3127 photographs a font whose own program is named so).
            assert!(
                conversion(&report).photographed.is_empty(),
                "{profile} at {target:?}: no page is photographed"
            );
            let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
            let font = font_resource(&held);
            assert!(
                matches!(font.get("FontDescriptor"), Some(Object::Reference(_))),
                "{profile} at {target:?}: Table 109's indirect reference"
            );
            let descriptor = held.get_key(&font, "FontDescriptor");
            let descriptor = descriptor.as_dict().expect("a descriptor");
            assert!(
                ["FontFile", "FontFile2", "FontFile3"]
                    .iter()
                    .any(|key| descriptor.get(key).is_some()),
                "{profile} at {target:?}: the descriptor holds the program"
            );
            let base_font = name_of(&held, &font, "BaseFont");
            assert!(
                std::str::from_utf8(&base_font).is_ok(),
                "{profile} at {target:?}: section 6.1.8's UTF-8"
            );
            assert_eq!(
                name_of(&held, descriptor, "FontName"),
                base_font,
                "{profile} at {target:?}: Table 120's same name"
            );
            // Every shown code still advances by the 600 the file states (Table 109's /Widths).
            let loaded = LoadedFont::load(&held, &font, "F1").expect("the font loads");
            assert!(!loaded.is_substituted(), "drawn from the program embedded");
            for byte in [65_u8, 66, 67] {
                let advance = loaded.advance(Code::single_byte(byte));
                assert!(
                    (advance - 0.6).abs() < 1e-3,
                    "{profile} at {target:?}: code {byte} advances {advance}"
                );
            }
        }
    }
}

#[test]
fn a_valid_font_name_is_kept_as_the_producer_wrote_it() {
    // The control: the same font named in ASCII keeps its name, since only a name section 6.1.8
    // finds no UTF-8 is a request the program embedded answers (ADR 3210).
    let source = a_descriptor_inside_the_font_named("Kept");
    let target = Target::Two(Level::B);
    let (report, output) = under(&source, "as-shown", target);
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let font = font_resource(&held);
    assert_eq!(name_of(&held, &font, "BaseFont"), b"Kept".to_vec());
    assert!(matches!(
        font.get("FontDescriptor"),
        Some(Object::Reference(_))
    ));
}
