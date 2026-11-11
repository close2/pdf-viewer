//! Document-resource writer regression for a concrete subset-tag collision.

use super::{for_a_file, machine_font};
use pdf_syntax::{Dictionary, Name, Object};

fn machine_font_for_collision(glyphs: &[(char, &str)]) -> Dictionary {
    let program: std::sync::Arc<[u8]> = std::sync::Arc::from(
        include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf").as_slice(),
    );
    let glyphs = glyphs
        .iter()
        .map(|(character, meaning)| (*character, (*meaning).to_owned()))
        .collect();
    machine_font(&program, &glyphs)
        .expect("the bundled face covers the stated characters")
        .0
}

fn resources(first: Dictionary, second: Dictionary) -> Dictionary {
    let mut fonts = Dictionary::new();
    fonts.insert(Name::new(b"F1".to_vec()), Object::Dictionary(first));
    fonts.insert(Name::new(b"F2".to_vec()), Object::Dictionary(second));
    let mut resources = Dictionary::new();
    resources.insert(Name::new(b"Font".to_vec()), Object::Dictionary(fonts));
    resources
}

fn base_font_name(font: &Dictionary) -> &[u8] {
    font.get("BaseFont")
        .and_then(Object::as_name)
        .expect("subset BaseFont")
        .as_bytes()
}

fn program_stream(font: &Dictionary) -> &[u8] {
    font.get("DescendantFonts")
        .and_then(Object::as_array)
        .and_then(<[Object]>::first)
        .and_then(Object::as_dict)
        .and_then(|descendant| descendant.get("FontDescriptor"))
        .and_then(Object::as_dict)
        .and_then(|descriptor| descriptor.get("FontFile2"))
        .and_then(Object::as_stream)
        .expect("FontFile2 stream")
        .data
        .as_ref()
}

/// The real appearance-resource writer leaves a concrete tag collision unresolved.
#[test]
fn appearance_writer_allocates_distinct_tags_for_sibling_subsets() {
    // These text characters map to source glyph sets whose composite closures collide under
    // §9.9.2's current six-letter hash. U+011C is G-circumflex.
    let first = machine_font_for_collision(&[('G', "G"), ('Ĝ', "Ĝ"), ('ь', "ь"), ('∑', "∑")]);
    let second = machine_font_for_collision(&[('C', "C"), ('~', "~"), ('¸', "¸"), ('Ç', "Ç")]);
    let written = for_a_file(&resources(first, second)).expect("embed both subsets");
    let fonts = written
        .get("Font")
        .and_then(Object::as_dict)
        .expect("writer kept Font resources");
    let first = fonts.get("F1").and_then(Object::as_dict).expect("F1");
    let second = fonts.get("F2").and_then(Object::as_dict).expect("F2");

    // Positive controls run before the expected-to-fail uniqueness assertion: these are
    // distinct subset programs, and each one's BaseFont agrees with its descriptor name.
    assert_ne!(program_stream(first), program_stream(second));
    for font in [first, second] {
        let descriptor_name = font
            .get("DescendantFonts")
            .and_then(Object::as_array)
            .and_then(<[Object]>::first)
            .and_then(Object::as_dict)
            .and_then(|descendant| descendant.get("FontDescriptor"))
            .and_then(Object::as_dict)
            .and_then(|descriptor| descriptor.get("FontName"))
            .and_then(Object::as_name)
            .expect("descriptor FontName");
        assert_eq!(base_font_name(font), descriptor_name.as_bytes());
    }

    // Regression oracle: the current implementation fails because both are NQYCPV+… .
    assert_ne!(
        base_font_name(first),
        base_font_name(second),
        "different subsets of one source face need different tags in this PDF"
    );
}
