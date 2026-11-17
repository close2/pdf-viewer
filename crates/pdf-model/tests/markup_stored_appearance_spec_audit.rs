//! Supplied Stamp and Caret appearances remain authoritative even though the reader declines to
//! construct missing stamp artwork. The no-AP policy test is separate and uses producer-invalid
//! input under Table 166; this fixture exercises the stored-appearance reader path.

#![expect(
    clippy::expect_used,
    reason = "the fixed exact-xref fixture must fail loudly if interpretation or pixel sampling regresses"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(annotation: &str, marks: &str) -> Vec<u8> {
    let appearance = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /Length {} >>\nstream\n{marks}\nendstream",
        marks.len()
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!("<< /Type /Annot {annotation} /Rect [10 10 90 90] /F 4 /AP << /N 6 0 R >> >>"),
        appearance,
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (number, object) in (1_usize..).zip(objects) {
        offsets.push(bytes.len());
        writeln!(bytes, "{number} 0 obj\n{object}\nendobj").expect("write object");
    }
    let xref = bytes.len();
    let size = offsets.len().saturating_add(1);
    writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ").expect("write xref header");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("write trailer");
    bytes.into_bytes()
}

fn stored_pixel(annotation: &str, marks: &str) -> [u8; 4] {
    let document = Document::open(fixture(annotation, marks)).expect("the complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture has exact xref offsets"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "Table 166's supplied appearance is usable: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterization");

    let row = raster.height.saturating_sub(1).saturating_sub(50);
    let offset = ((row.saturating_mul(raster.width).saturating_add(50)) as usize).saturating_mul(4);
    raster.data[offset..offset.saturating_add(4)]
        .try_into()
        .expect("four channels")
}

fn supplied_colours(annotation: &str) {
    assert_eq!(
        stored_pixel(annotation, "1 0 0 rg 0 0 10 10 re f"),
        [255, 0, 0, 255]
    );
    assert_eq!(
        stored_pixel(annotation, "0 0 1 rg 0 0 10 10 re f"),
        [0, 0, 255, 255]
    );
}

#[test]
fn a_stamp_with_a_stored_appearance_draws_its_actual_stream() {
    supplied_colours("/Subtype /Stamp /Name /Approved");
}

#[test]
fn a_caret_with_a_stored_appearance_draws_its_actual_stream() {
    supplied_colours("/Subtype /Caret /Sy /P");
}
