//! ISO 32000-2 §9.10.2 and Adobe Glyph List Specification v2.9 §2.
//! The independent oracles are the AGL table row and the specified uniXXXX grouping.

use pdf_font::encoding::text_for;

fn controls() {
    assert_eq!(text_for("A").as_deref(), Some("A"));
    assert_eq!(text_for("A.sc").as_deref(), Some("A"));
    assert_eq!(text_for("f_f_i").as_deref(), Some("ffi"));
    assert_eq!(text_for("uni0042").as_deref(), Some("B"));
    assert_eq!(text_for("uniD801DC0C"), None);
}

#[test]
fn an_agl_table_row_keeps_all_its_unicode_scalars() {
    controls();
    let actual = ["dalethatafpatah", "dalethatafpatahhebrew"].map(text_for);
    assert_eq!(
        actual,
        [
            Some("\u{05D3}\u{05B2}".into()),
            Some("\u{05D3}\u{05B2}".into())
        ]
    );
}

#[test]
fn a_uni_name_keeps_every_four_digit_group() {
    controls();
    assert_eq!(text_for("uni00420043").as_deref(), Some("BC"));
}

#[test]
fn an_unrecognized_component_contributes_an_empty_string() {
    controls();
    // Adobe AGL §2 maps each component separately, including its empty fallback,
    // before concatenation. The representation of a wholly empty result is not at issue.
    assert_eq!(text_for("A_notaglyph").as_deref(), Some("A"));
}
