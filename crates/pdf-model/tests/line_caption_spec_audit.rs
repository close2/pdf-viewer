//! Discriminating `/CO` axes/default regression for the constructed line-appearance path.
//! This is a complete, exact-xref PDF but deliberately omits `/AP` so it tests this reader's
//! constructed-appearance fallback policy; it is not offered as a producer-conforming fixture.

#![expect(
    clippy::expect_used,
    reason = "a fixed PDF and exact transform assertions must fail loudly on regression"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Transform};
use pdf_syntax::Document;

fn line_with(extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 \
         /L [10 20 70 80] /C [0 0 0] /BS << /W 1 >> \
         /Cap true /Contents (same caption) {extra} >>\nendobj\n"
    );
    assemble(&body)
}

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn caption_glyph_transforms(extra: &str) -> Vec<Transform> {
    let document = Document::open(line_with(extra)).expect("complete PDF syntax");
    assert!(!document.was_recovered(), "exact cross references");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the supported caption should be interpreted: {:?}",
        interpretation.unsupported
    );
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => Some(*transform),
            _ => None,
        })
        .collect()
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "actual {actual} differs from expected {expected}"
    );
}

#[test]
fn caption_co_defaults_and_offsets_follow_the_diagonal_line_axes() {
    let omitted = caption_glyph_transforms("");
    let explicit_default = caption_glyph_transforms("/CO [0 0]");
    let shifted = caption_glyph_transforms("/CO [10 -8]");
    assert!(!omitted.is_empty(), "the caption produces glyph fill paths");
    assert_eq!(omitted.len(), explicit_default.len());
    assert_eq!(omitted.len(), shifted.len());

    // Table 178 defaults CO to [0,0]. A missing key and its explicit default must put every
    // corresponding glyph outline at the same transform.
    assert_eq!(omitted, explicit_default);

    // The line vector is (60,60), hence its unit direction is (1/√2,1/√2), and its positive
    // perpendicular is (-1/√2,1/√2). For CO [10,-8], the independently computed translation is
    // (10 - 8*(-1), 10 + -8) / √2 = (18/√2, 2/√2). Treating CO as page-axis offsets would instead
    // be (10,-8); either wrong reading fails both component assertions.
    let expected_x = 18.0_f32 / 2.0_f32.sqrt();
    let expected_y = 2.0_f32 / 2.0_f32.sqrt();
    for (base, moved) in omitted.iter().zip(&shifted) {
        close(moved.a, base.a);
        close(moved.b, base.b);
        close(moved.c, base.c);
        close(moved.d, base.d);
        close(moved.e - base.e, expected_x);
        close(moved.f - base.f, expected_y);
    }
}
