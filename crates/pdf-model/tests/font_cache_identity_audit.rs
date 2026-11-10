//! Discriminating test for §7.8.3 resource-local names and ADR 0115.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the fixture is a small, complete PDF with three asserted glyph paints"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

fn stream(id: usize, dictionary: &str, data: &str) -> String {
    format!(
        "{id} 0 obj\n<< {dictionary} /Length {} >>\nstream\n{data}\nendstream\nendobj\n",
        data.len().saturating_add(1)
    )
}

fn type3_font(id: usize, encoding_id: usize, charprocs_id: usize, glyph_id: usize) -> String {
    format!(
        "{id} 0 obj\n<< /Type /Font /Subtype /Type3 \
         /FontBBox [0 0 750 750] /FontMatrix [0.001 0 0 0.001 0 0] \
         /CharProcs {charprocs_id} 0 R /Encoding {encoding_id} 0 R \
         /FirstChar 65 /LastChar 65 /Widths [1000] /Resources << >> >>\nendobj\n"
    ) + &format!(
        "{encoding_id} 0 obj\n<< /Type /Encoding /Differences [65 /A] >>\nendobj\n\
         {charprocs_id} 0 obj\n<< /A {glyph_id} 0 R >>\nendobj\n"
    )
}

fn pdf() -> Vec<u8> {
    let page_content = "BT /F1 20 Tf 10 10 Td (A) Tj ET /Fm Do BT /F1 20 Tf 60 10 Td (A) Tj ET";
    let form_content = "BT /F1 20 Tf 10 10 Td (A) Tj ET";
    let red_glyph = "1000 0 d0\n1 0 0 rg\n0 0 750 750 re f";
    let blue_glyph = "1000 0 d0\n0 0 1 rg\n0 0 m 750 0 l 375 750 l h f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 60] \
         /Resources << /Font << /F1 5 0 R >> /XObject << /Fm 9 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         {}{}{}{}{}{}",
        stream(4, "", page_content),
        type3_font(5, 6, 7, 8),
        stream(8, "", red_glyph),
        stream(
            9,
            "/Type /XObject /Subtype /Form /BBox [0 0 100 50] \
             /Resources << /Font << /F1 10 0 R >> >>",
            form_content,
        ),
        type3_font(10, 11, 12, 13),
        stream(13, "", blue_glyph),
    );

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn solid_fills() -> Vec<[u32; 4]> {
    let document = Document::open(pdf()).expect("complete fixture opens");
    assert!(
        !document.was_recovered(),
        "fixture has a complete xref table"
    );
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("one page exists");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.unsupported.is_empty(),
        "both in-file Type 3 fonts and their glyph programs are valid: {:?}",
        interpretation.unsupported
    );
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some([
                colour.r.to_bits(),
                colour.g.to_bits(),
                colour.b.to_bits(),
                colour.a.to_bits(),
            ]),
            _ => None,
        })
        .collect()
}

#[test]
fn same_named_type3_font_is_scoped_to_each_dictionary_and_page_scope_restores() {
    let fills = solid_fills();
    assert_eq!(
        fills.len(),
        3,
        "the page, form, then page show one glyph each"
    );
    assert_eq!(
        fills[0],
        [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
        "the page's /F1 resolves to its red Type 3 font"
    );
    assert_eq!(
        fills[1],
        [0.0_f32, 0.0, 1.0, 1.0].map(f32::to_bits),
        "the form's same-named /F1 resolves to its blue Type 3 font"
    );
    assert_eq!(
        fills[2],
        [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
        "after Do returns, the page's red /F1 is selected again"
    );
}
