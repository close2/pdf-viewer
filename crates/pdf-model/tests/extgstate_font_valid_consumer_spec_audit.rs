//! Valid Table 57 `/Font` consumer control: setting font and size by `gs` vs `Tf`.
//!
//! Helvetica is one of PDF 1.7's Standard 14 fonts and needs no embedded program here. Its
//! checked-in AFM width for `A` is 667/1000 em, so a 24-unit font advances the second glyph by
//! 16.008 page units. Table 57 specifically requires the first `/Font` array item to be an
//! indirect font reference; the fixture complies and has exact xref/stream lengths.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "fixture construction is bounded and a missing glyph is the failure under test"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R /Courier 7 0 R >> \
         /ExtGState << /Set 6 0 R /Other 8 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /ExtGState /Font [5 0 R 24] >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n\
         8 0 obj\n<< /Type /ExtGState /LW 5 >>\nendobj\n",
        content.len() + 1,
        content,
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
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn placements(content: &str) -> Vec<f32> {
    let document = Document::open(fixture(content)).expect("exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "a supported Standard 14 glyph route should not be reported incomplete: {:?}",
        interpretation.unsupported
    );
    let x: Vec<f32> = interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } | Command::Stroke { transform, .. } => {
                Some(transform.e)
            }
            _ => None,
        })
        .collect();
    assert_eq!(x.len(), 2, "Helvetica draws both A glyphs");
    x
}

#[test]
fn extgstate_font_sets_the_indirect_font_and_text_size_like_tf() {
    let via_gs = placements("/Set gs BT 10 20 Td (AA) Tj ET");
    let via_tf = placements("BT /F1 24 Tf 10 20 Td (AA) Tj ET");
    assert_eq!(via_gs, via_tf, "Table 57 Font must match Tf's two values");
    let overridden = placements("BT /Courier 12 Tf /Set gs 10 20 Td (AA) Tj ET");
    assert_eq!(
        overridden, via_tf,
        "gs replaces both previous font and size"
    );
    let retained = placements("/Set gs /Other gs BT 10 20 Td (AA) Tj ET");
    assert_eq!(
        retained, via_tf,
        "an unrelated gs leaves the selected font and size intact"
    );
    let courier = placements("BT /Courier 12 Tf 10 20 Td (AA) Tj ET");
    assert!(
        ((courier[1] - courier[0]) - 7.2).abs() < 0.01,
        "Courier's 600/1000 advance at size12 distinguishes prior state"
    );
    let advance = via_gs[1] - via_gs[0];
    assert!(
        (advance - 16.008).abs() < 0.01,
        "AFM A width 667 at 24 units gives 16.008 page units, got {advance}"
    );
}
