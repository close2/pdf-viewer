//! §9.6.4 step c and Table 111 d1: restore page state after a shape-only glyph.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small fixed valid PDF fixture"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

fn type3_after_d1_page() -> pdf_model::Interpretation {
    let glyph = "1000 0 0 0 750 750 d1\n/Half gs\n0 0 750 750 re f";
    let content = "BT /F3 10 Tf (a) Tj ET\n0 0 1 rg /Half gs\n100 100 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F3 5 0 R >> /ExtGState << /Half 7 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R >> \
         /Encoding << /Type /Encoding /Differences [97 /A] >> \
         /FirstChar 97 /LastChar 97 /Widths [1000] >>\nendobj\n\
         6 0 obj\n<< /Length {} >>\nstream\n{glyph}\nendstream\nendobj\n\
         7 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 >>\nendobj\n",
        content.len() + 1,
        glyph.len() + 1,
    );

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    let document = Document::open(bytes.into_bytes()).expect("exact-xref fixture is valid");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one page exists");
    pdf_model::interpret(&document, &page)
}

#[test]
fn d1_shape_only_state_is_restored_before_page_extgstate() {
    let result = type3_after_d1_page();
    assert!(
        result.is_complete(),
        "valid fixture: {:?}",
        result.unsupported
    );

    let fills: Vec<_> = result
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some(*colour),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills.len(),
        2,
        "d1 glyph and following page rectangle both paint"
    );
    assert_eq!(
        fills[0].a.to_bits(),
        1.0_f32.to_bits(),
        "d1 ignores its internal /Half"
    );
    assert_eq!(
        fills[1].a.to_bits(),
        0.5_f32.to_bits(),
        "page /Half applies after the glyph returns"
    );
    assert_eq!(
        [
            fills[0].r.to_bits(),
            fills[0].g.to_bits(),
            fills[0].b.to_bits()
        ],
        [0.0_f32.to_bits(); 3],
        "glyph uses inherited black"
    );
    assert_eq!(
        [
            fills[1].r.to_bits(),
            fills[1].g.to_bits(),
            fills[1].b.to_bits()
        ],
        [0.0_f32.to_bits(), 0.0_f32.to_bits(), 1.0_f32.to_bits()],
        "following page rectangle uses its explicit blue"
    );
}
