//! Regression for the Link-specific `/H` effect in ISO 32000-2 §12.5.6.5, Table 176.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "a broken fixture or pixel coordinate must fail the audit test"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    writeln!(out, "xref\n0 {size}").expect("string write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("string write");
    }
    writeln!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF"
    )
    .expect("string write");
    out.into_bytes()
}

fn fixture(highlight: &str) -> Vec<u8> {
    let appearance = "1 0 0 rg 0 0 80 80 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 10 90 90] /F 4 {highlight} \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> \
         /Length {} >>\nstream\n{appearance}\nendstream\nendobj\n",
        appearance.len() + 1
    );
    assemble(&body)
}

fn centre_colour(highlight: &str, pointer: Option<pdf_model::view::Pointer>) -> (u8, u8, u8) {
    let document = Document::open(fixture(highlight)).expect("complete valid PDF fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = pdf_model::view::ViewState::of(&document);
    if let Some(pointer) = pointer {
        state.set_pointer(Some((
            ObjectId {
                number: 5,
                generation: 0,
            },
            pointer,
        )));
    }
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(interpretation.is_complete(), "fixture is fully interpreted");
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("target fits");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports this page");
    // The annotation appearance paints red over user-space [10,90]². (50,50) is
    // strictly inside it; raster rows are inverted relative to PDF's upward y axis.
    let row = raster.height - 1 - 50;
    let index = ((row * raster.width + 50) * 4) as usize;
    (
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
    )
}

#[test]
fn link_highlighting_modes_apply_only_while_pressed() {
    use pdf_model::view::Pointer;
    assert_eq!(centre_colour("/H /I", None), (255, 0, 0), "normal red link");
    assert_eq!(
        centre_colour("/H /I", Some(Pointer::Down)),
        (0, 255, 255),
        "Table176 I inverts RGB"
    );
    assert_eq!(
        centre_colour("/H /N", Some(Pointer::Down)),
        (255, 0, 0),
        "N suppresses highlight"
    );
    assert_eq!(
        centre_colour("/H /I", Some(Pointer::Over)),
        (255, 0, 0),
        "hovering does not invert"
    );
    assert_eq!(
        centre_colour("", Some(Pointer::Down)),
        (0, 255, 255),
        "absent H defaults to I when no down appearance exists"
    );
}
