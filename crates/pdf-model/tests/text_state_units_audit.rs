//! Regression for ISO 32000-2 §§9.3.1–9.3.4.
//!
//! This is a complete integration-test source. It uses a deterministic embedded Type 3 font,
//! whose space glyph has no outline and whose `A` glyph is a filled square. The glyph placement
//! is measured directly from the display list, so the assertion does not depend on host fonts or
//! rasterizer antialiasing.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture and display-list assumptions should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

fn fixture(content: &str) -> Vec<u8> {
    const SPACE: &str = "250 0 d0";
    const A_GLYPH: &str = "500 0 0 0 500 500 d1\n0 0 500 500 re f";
    // FirstChar 32 through LastChar 65: code 32 has width 250, codes 33..64 are
    // zero-width, and code 65 has width 500.
    let widths = format!("[250 {} 500]", "0 ".repeat(32));
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 \
         /FontBBox [0 0 500 500] /FontMatrix [0.001 0 0 0.001 0 0] \
         /CharProcs 6 0 R /Encoding 7 0 R /FirstChar 32 /LastChar 65 \
         /Widths {widths} >>\nendobj\n\
         6 0 obj\n<< /sp 8 0 R /A 9 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /Encoding /Differences [32 /sp 65 /A] >>\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{SPACE}\nendstream\nendobj\n\
         9 0 obj\n<< /Length {} >>\nstream\n{A_GLYPH}\nendstream\nendobj\n",
        content.len() + 1,
        SPACE.len() + 1,
        A_GLYPH.len() + 1,
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn a_glyph_x(content: &str) -> f32 {
    let document = Document::open(fixture(content)).expect("valid generated PDF");
    assert!(!document.was_recovered(), "fixture has a complete xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.unsupported.is_empty(),
        "{:?}",
        interpretation.unsupported
    );
    assert_eq!(
        interpretation
            .display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::Fill { .. }))
            .count(),
        1,
        "only the A glyph paints; space contributes advance only"
    );
    interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill { transform, .. } => Some(transform.e),
            _ => None,
        })
        .expect("embedded Type 3 A glyph is painted")
}

#[test]
fn tc_and_tw_are_unscaled_text_units_then_horizontal_tz_scales_them() {
    // At each font size, compare the A after an empty-outline single-byte SPACE with a
    // zero-spacing control. Tc and Tw are unscaled text-space values, so either adds exactly
    // five page units at 5pt and 25pt; horizontal Th=0.5 then halves that addition.
    for size in [5_u8, 25] {
        let plain = a_glyph_x(&format!(
            "BT /F1 {size} Tf 0 Tc 0 Tw 100 Tz 0 0 Td (\\040A) Tj ET"
        ));
        let plain_half = a_glyph_x(&format!(
            "BT /F1 {size} Tf 0 Tc 0 Tw 50 Tz 0 0 Td (\\040A) Tj ET"
        ));
        assert!(
            (plain - f32::from(size) * 0.25).abs() < 0.01,
            "space width at full scale"
        );
        assert!(
            (plain_half - f32::from(size) * 0.125).abs() < 0.01,
            "space width at half scale"
        );
        let tc = a_glyph_x(&format!(
            "BT /F1 {size} Tf 5 Tc 0 Tw 100 Tz 0 0 Td (\\040A) Tj ET"
        ));
        let tw = a_glyph_x(&format!(
            "BT /F1 {size} Tf 0 Tc 5 Tw 100 Tz 0 0 Td (\\040A) Tj ET"
        ));
        let character_half = a_glyph_x(&format!(
            "BT /F1 {size} Tf 5 Tc 0 Tw 50 Tz 0 0 Td (\\040A) Tj ET"
        ));
        let word_half = a_glyph_x(&format!(
            "BT /F1 {size} Tf 0 Tc 5 Tw 50 Tz 0 0 Td (\\040A) Tj ET"
        ));

        assert!(
            (tc - plain - 5.0).abs() < 0.01,
            "Tc must be 5 unscaled units at {size}pt: {tc} vs {plain}"
        );
        assert!(
            (tw - plain - 5.0).abs() < 0.01,
            "single-byte code 32 Tw must be 5 unscaled units at {size}pt: {tw} vs {plain}"
        );
        assert!(
            (character_half - plain_half - 2.5).abs() < 0.01,
            "Th must scale horizontal Tc to 2.5 at {size}pt"
        );
        assert!(
            (word_half - plain_half - 2.5).abs() < 0.01,
            "Th must scale horizontal Tw to 2.5 at {size}pt"
        );
    }
}
