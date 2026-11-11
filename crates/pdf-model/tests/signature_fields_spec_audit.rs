//! Signature-field visibility uses the supplied appearance and the field's own visibility rules.

#![expect(
    clippy::expect_used,
    reason = "complete deterministic PDF and raster fixtures"
)]

use pdf_model::{Pages, content::interpret_with, view::ViewState};
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

fn signature_field(flags: u32, rect: &str) -> Document {
    let paint = "1 0 0 rg 0 0 40 40 re f";
    // Unsigned fields are valid: /V is optional. No cryptographic validity is implied.
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 60] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Sig /T (approval) /Rect [{rect}] /F {flags} /AP << /N 5 0 R >> >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> /Length {} >>\nstream\n{paint}\nendstream",
            paint.len()
        ),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{object}\nendobj", index.saturating_add(1))
            .expect("write string");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 6\n0000000000 65535 f ").expect("xref");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("offset");
    }
    write!(
        bytes,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("trailer");
    let document = Document::open(bytes.into_bytes()).expect("complete signature field");
    assert!(!document.was_recovered(), "exact xref");
    document
}

fn pixels(flags: u32, rect: &str) -> Vec<u8> {
    let document = signature_field(flags, rect);
    let page = Pages::new(&document).get(0).expect("one page");
    let rendered = interpret_with(&document, &page, &ViewState::of(&document));
    assert!(rendered.is_complete(), "{:?}", rendered.unsupported);
    let target = TargetSpec::for_page(&rendered.display_list, 1.0, 1 << 20).expect("small raster");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&rendered.display_list, target)
        .expect("solid appearance")
        .data
}

#[test]
fn a_signature_fields_hidden_no_view_and_zero_rectangle_cases_leave_no_visible_appearance() {
    let visible = pixels(0, "10 10 50 50");
    assert!(
        visible.chunks_exact(4).any(|rgba| rgba == [255, 0, 0, 255]),
        "positive stored signature appearance"
    );
    for (flags, rect) in [(2, "10 10 50 50"), (32, "10 10 50 50"), (0, "10 10 10 10")] {
        let hidden = pixels(flags, rect);
        assert!(
            hidden.chunks_exact(4).all(|rgba| rgba == [0, 0, 0, 0]),
            "signature flags={flags}, Rect={rect} must leave no mark"
        );
    }
}
