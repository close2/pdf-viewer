//! Public §8.6.6.3 regression: exact-xref `/Indexed` initial index and half-up tie.
//!
//! Source: ISO 32000-2:2020(E) §8.6.6.3, printed pp.199–200 / physical pp.214–215.
//! The reader shall initialize current colour to 0 when `/Indexed` is selected. Real
//! operands are rounded to the nearest integer and exact 0.5 ties go up. Palette entries
//! are independently encoded bytes: entry 0 is RGB red; entry 1 is RGB blue. Therefore
//! implicit index 0 and explicit 0/0.49 are red, while 0.5 and 1 are blue. No external
//! renderer, corpus fixture, or implementation helper supplies the expected values.

#![expect(
    clippy::indexing_slicing,
    reason = "standalone test uses a fixed exact-xref fixture and fixed-size raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

#[test]
fn indexed_selection_starts_at_zero_and_a_real_half_tie_rounds_up() {
    let content = concat!(
        "/Idx cs 0 0 20 20 re f ",
        "/Idx cs 0 scn 20 0 20 20 re f ",
        "/Idx cs 0.49 scn 40 0 20 20 re f ",
        "/Idx cs 0.5 scn 60 0 20 20 re f ",
        "/Idx cs 1 scn 80 0 20 20 re f",
    );
    let objects = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 20] \
         /Resources << /ColorSpace << /Idx [/Indexed /DeviceRGB 1 <FF00000000FF>] >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        content.len() + 1,
        content,
    );
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in objects.split_inclusive("endobj\n") {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }
    let xref_at = pdf.len();
    let size = offsets.len() + 1;
    writeln!(pdf, "xref\n0 {size}").expect("write xref header");
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write exact object offset");
    }
    write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    )
    .expect("write exact startxref");

    let document = Document::open(pdf.into_bytes()).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "exact xref required");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("fixed page target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported page");
    assert_eq!((raster.width, raster.height), (100, 20));
    let pixel = |x: u32| {
        let at = ((10 * raster.width + x) * 4) as usize;
        [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
    };
    assert_eq!(pixel(10), [255, 0, 0], "cs initializes Indexed to index 0");
    assert_eq!(pixel(30), [255, 0, 0], "explicit index 0 control");
    assert_eq!(pixel(50), [255, 0, 0], "0.49 rounds down");
    assert_eq!(
        pixel(70),
        [0, 0, 255],
        "the source's exact 0.5 tie rounds up"
    );
    assert_eq!(pixel(90), [0, 0, 255], "explicit index 1 control");
}
