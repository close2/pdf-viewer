//! An `OpenType` program with `CFF ` outlines given empty glyphs (`pdf_font::embedding::
//! with_empty_cff_glyphs`, ADR 3493): the `CFF ` table gains them as a bare program does, and the
//! sfnt's glyph count and horizontal metrics count and state them, every glyph it held unchanged.
//!
//! The advances are OpenType's `hmtx` (one long metric per glyph, `hhea`'s `numberOfHMetrics`
//! counting them) and the Type 2 charstring's width operand (Adobe Technical Note #5177); ISO
//! 19005-2 section 6.2.11.5 holds a program's advance for a glyph to the width the font dictionary
//! states, which is what the advances asked for are.

use pdf_font::embedding::{
    glyph_count, with_empty_cff_glyphs, wrapped_cff_glyph_names, wrapped_cff_units_per_em,
};
use pdf_font::substitute::Format;

const CFF: &[u8] = include_bytes!("../../pdf-model/tests/fixtures/opentype-selection/cff.otf");

#[test]
fn each_advance_is_a_glyph_counted_named_and_measured() {
    let held = glyph_count(CFF, Format::Sfnt).expect("the fixture's maxp");
    let before: Vec<Option<f32>> = (0..held)
        .map(|glyph| pdf_font::restate::advance(CFF, Format::Sfnt, glyph))
        .collect();
    let (program, added) = with_empty_cff_glyphs(CFF, &[500, 250]).expect("the glyphs are given");
    assert_eq!(added.len(), 2);
    assert_eq!(glyph_count(&program, Format::Sfnt), Some(held + 2));
    let names = wrapped_cff_glyph_names(&program).expect("a name-keyed CFF table");
    // `restate::advance` answers in ems, and only where `hmtx` and the charstring agree.
    let units = wrapped_cff_units_per_em(&program).expect("the CFF table's FontMatrix");
    for (glyph, advance) in added.iter().zip([500.0_f32, 250.0]) {
        assert_eq!(names.get(&glyph.glyph), Some(&glyph.name));
        assert!(glyph.name.starts_with("notdef"), "{}", glyph.name);
        assert_eq!(
            pdf_font::restate::advance(&program, Format::Sfnt, glyph.glyph),
            Some(advance / units)
        );
    }
    for (glyph, advance) in (0..held).zip(before) {
        assert_eq!(
            pdf_font::restate::advance(&program, Format::Sfnt, glyph),
            advance
        );
    }
}

#[test]
fn a_program_without_a_cff_table_is_refused() {
    let glyf = include_bytes!("../../pdf-model/tests/fixtures/opentype-selection/glyf.ttf");
    assert!(with_empty_cff_glyphs(glyf, &[500]).is_none());
}
