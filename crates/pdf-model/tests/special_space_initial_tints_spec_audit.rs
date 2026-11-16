//! Public §8.6.6.4–.5 regression: selecting `Separation` and `DeviceN` initializes all tints to 1.
//!
//! Source: ISO 32000-2:2020(E) §8.6.6.4 printed p.201 / physical p.216, and §8.6.6.5
//! printed p.205 / physical p.220. The clauses explicitly set `Separation` initial tint to
//! 1.0 and each `DeviceN` component to 1.0. Both initial paints are paired with explicit
//! full-tint controls and alternate-tint negative controls; expected RGB values follow from
//! the supplied valid function definitions, not a second invocation of the evaluator.

#![expect(
    clippy::indexing_slicing,
    reason = "standalone test uses a fixed exact-xref fixture and fixed-size raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

#[test]
fn separation_and_devicen_start_at_full_tint_after_cs() {
    let content = concat!(
        "/Sep cs 0 0 20 20 re f ",
        "/Sep cs 1 scn 20 0 20 20 re f ",
        "/Sep cs 0 scn 40 0 20 20 re f ",
        "/DN cs 60 0 20 20 re f ",
        "/DN cs 1 1 scn 80 0 20 20 re f ",
        "/DN cs 0 0 scn 100 0 20 20 re f",
    );
    let objects = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 20] \
         /Resources << /ColorSpace << /Sep 5 0 R /DN 7 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n[/Separation /Ink /DeviceRGB 6 0 R]\nendobj\n\
         6 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [0 0 1] /C1 [1 0 0] /N 1 >>\nendobj\n\
         7 0 obj\n[/DeviceN [/RedInk /GreenInk] /DeviceRGB 8 0 R]\nendobj\n\
         8 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        content.len() + 1,
        content,
        "{ exch 0.25 mul exch 0.5 mul 0 }".len() + 1,
        "{ exch 0.25 mul exch 0.5 mul 0 }",
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
    assert_eq!((raster.width, raster.height), (120, 20));
    let pixel = |x: u32| {
        let at = ((10 * raster.width + x) * 4) as usize;
        [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
    };
    assert_eq!(pixel(10), [255, 0, 0], "implicit Separation tint 1");
    assert_eq!(pixel(30), [255, 0, 0], "explicit Separation tint 1");
    assert_eq!(pixel(50), [0, 0, 255], "explicit zero is the lightest tint");
    assert_eq!(pixel(70), [64, 128, 0], "both implicit DeviceN tints are 1");
    assert_eq!(pixel(90), [64, 128, 0], "explicit DeviceN 1,1 control");
    assert_eq!(pixel(110), [0, 0, 0], "explicit DeviceN 0,0 control");
}
