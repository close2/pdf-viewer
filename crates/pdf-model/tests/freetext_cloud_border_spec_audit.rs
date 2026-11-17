//! Public consumer regression for Table 169's BS width/dash duty on `FreeText`.
//!
//! This is a standalone PDF 1.7 annotation (not a field widget), with a complete classic xref.
//! It asserts only differential output and border ink, not a fixed cloud radius or cusp placement.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "small fixed fixtures: malformed input and invalid raster coordinates should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

fn assemble(body: &str) -> Vec<u8> {
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }
    let xref_at = pdf.len();
    let size = offsets.len().saturating_add(1);
    writeln!(pdf, "xref\n0 {size}").expect("write xref header");
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write xref row");
    }
    write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    )
    .expect("write trailer");
    pdf.into_bytes()
}

fn pdf(bs: &str) -> Vec<u8> {
    let annotation = format!(
        "<< /Type /Annot /Subtype /FreeText /Rect [20 20 180 80] /F 4 \
         /Contents () /DA (/F1 12 Tf 0 g) /BS {bs} /BE << /S /C /I 1 >> >>"
    );
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [] /DA (/F1 12 Tf 0 g) /DR << /Font << /F1 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n"
    ))
}

fn draw(bs: &str) -> pdf_render::Raster {
    let document = Document::open(pdf(bs)).expect("valid PDF 1.7");
    assert!(
        !document.was_recovered(),
        "complete exact cross-reference table"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid FreeText and standard-14 resource: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid raster target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list")
}

fn border_ink(raster: &pdf_render::Raster) -> usize {
    // Empty Contents and absent background color leave only the border as possible ink.
    raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] > 0)
        .count()
}

/// Table 169 requires the `FreeText` cloud to honor the BS dash and width parameters.
#[test]
fn a_freetext_cloud_uses_its_bs_dash_and_width() {
    let suppressed = draw("<< /W 0 /S /D /D [10 5] >>");
    assert_eq!(
        border_ink(&suppressed),
        0,
        "zero width suppresses the border"
    );
    let long_dash = draw("<< /W 2 /S /D /D [10 5] >>");
    let short_dash = draw("<< /W 2 /S /D /D [2 3] >>");
    let wider = draw("<< /W 4 /S /D /D [10 5] >>");

    assert!(border_ink(&long_dash) > 0, "long-dash cloud has border ink");
    assert!(
        border_ink(&short_dash) > 0,
        "short-dash cloud has border ink"
    );
    assert!(border_ink(&wider) > 0, "wide cloud has border ink");
    assert_ne!(
        long_dash, short_dash,
        "changing BS /D changes the cloud stroke"
    );
    assert_ne!(long_dash, wider, "changing BS /W changes the cloud stroke");
}
