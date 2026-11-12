//! ISO 32000-2 §9.9.2 across the appearances of one written file: one [`SubsetNames`] for the
//! file, and the names the file already states taken too (ADR 2832).
//!
//! The two glyph sets are the owner's audit's (`variable_text_subset_audit.rs`): cut from the
//! bundled face, their subsets are different programs whose six-letter digests agree, which is
//! the collision the clause's "different subsets of the same font in the same PDF file shall
//! have different tags" forbids.

use super::{SubsetNames, for_a_file_in, is_subset_tagged, machine_font};
use pdf_syntax::{Dictionary, Document, Name, Object};
use std::fmt::Write as _;

/// A machine font for `glyphs`, cut from the bundled face.
fn machine_font_for(glyphs: &[(char, &str)]) -> Dictionary {
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

/// The first of the audit's two colliding sets.
fn first() -> Dictionary {
    machine_font_for(&[('G', "G"), ('Ĝ', "Ĝ"), ('ь', "ь"), ('∑', "∑")])
}

/// The second, a different subset whose tag digests alike.
fn second() -> Dictionary {
    machine_font_for(&[('C', "C"), ('~', "~"), ('¸', "¸"), ('Ç', "Ç")])
}

/// One appearance's resources, holding one font.
fn resources(font: Dictionary) -> Dictionary {
    let mut fonts = Dictionary::new();
    fonts.insert(Name::new(b"F1".to_vec()), Object::Dictionary(font));
    let mut resources = Dictionary::new();
    resources.insert(Name::new(b"Font".to_vec()), Object::Dictionary(fonts));
    resources
}

/// The `/BaseFont` one appearance's resources were written with.
fn written_name(written: &Dictionary) -> Vec<u8> {
    written
        .get("Font")
        .and_then(Object::as_dict)
        .and_then(|fonts| fonts.get("F1"))
        .and_then(Object::as_dict)
        .and_then(|font| font.get("BaseFont"))
        .and_then(Object::as_name)
        .expect("a written /BaseFont")
        .as_bytes()
        .to_vec()
}

/// A file whose one object is a font dictionary named `name`, as an earlier save would leave it.
fn a_file_stating(name: &[u8]) -> Document {
    let name = String::from_utf8(name.to_vec()).expect("a tag and a face name are ASCII");
    let object = format!(
        "1 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H >>\nendobj\n"
    );
    let mut out = String::from("%PDF-1.7\n");
    let offset = out.len();
    out.push_str(&object);
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 2\n0000000000 65535 f \n{offset:010} 00000 n \ntrailer\n<< /Size 2 >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture is a valid file")
}

/// Two appearances of one file, written through one registry, carry the two different subsets
/// under two tags.
///
/// §9.9.2: "different subsets of the same font in the same PDF file shall have different tags".
/// The control writes each appearance with a registry of its own, as a writer that checked only
/// one resource dictionary at a time would, and finds the collision the clause forbids — which
/// is what makes the first assertion a test of the registry and not of the digest.
#[test]
fn two_appearances_of_one_file_give_two_subsets_two_tags() {
    let document = Document::empty();
    let alone = |font| {
        written_name(
            &for_a_file_in(&document, &resources(font), &mut SubsetNames::default())
                .expect("the bundled face may be embedded"),
        )
    };
    assert_eq!(
        alone(first()),
        alone(second()),
        "the control: the digests collide"
    );

    let mut names = SubsetNames::default();
    let one = for_a_file_in(&document, &resources(first()), &mut names).expect("embedded");
    let other = for_a_file_in(&document, &resources(second()), &mut names).expect("embedded");
    let (one, other) = (written_name(&one), written_name(&other));
    assert_ne!(one, other);
    assert!(is_subset_tagged(&one) && is_subset_tagged(&other));
    // The face's name after the plus sign is the same font's, so only the tag tells them apart.
    assert_eq!(one.get(6..), other.get(6..));
}

/// The same subset written into two appearances of one file keeps one name: §9.9.2 forbids one
/// tag for two subsets and asks nothing of one subset written twice.
#[test]
fn one_subset_in_two_appearances_keeps_one_name() {
    let document = Document::empty();
    let mut names = SubsetNames::default();
    let once = for_a_file_in(&document, &resources(first()), &mut names).expect("embedded");
    let twice = for_a_file_in(&document, &resources(first()), &mut names).expect("embedded");
    assert_eq!(written_name(&once), written_name(&twice));
}

/// A name the file already states is not given to a subset written into it.
///
/// An incremental update (§7.5.6) keeps every font the file held, so a subset written now under
/// the name an earlier one carries would be two subsets under one tag "in the same PDF file"
/// (§9.9.2). The control writes into an empty document and takes the name.
#[test]
fn a_name_the_file_states_is_passed_over() {
    let taken = written_name(
        &for_a_file_in(
            &Document::empty(),
            &resources(first()),
            &mut SubsetNames::default(),
        )
        .expect("embedded"),
    );
    let document = a_file_stating(&taken);
    let written = written_name(
        &for_a_file_in(&document, &resources(first()), &mut SubsetNames::default())
            .expect("embedded"),
    );
    assert_ne!(written, taken);
    assert!(is_subset_tagged(&written));
    assert_eq!(written.get(6..), taken.get(6..));
}

/// §9.9.2's tag is "exactly six uppercase letters" and a plus sign; a name that merely holds a
/// plus sign is not one, and is not taken from the file as a subset's.
#[test]
fn a_tag_is_six_uppercase_letters_and_a_plus_sign() {
    assert!(is_subset_tagged(b"EOODIA+Poetica"));
    assert!(!is_subset_tagged(b"EOODIa+Poetica"));
    assert!(!is_subset_tagged(b"EOODI+Poetica"));
    assert!(!is_subset_tagged(b"Arial+Bold"));
}
