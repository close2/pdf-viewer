//! Saved `/Tx` stream replacement, including nested marked content.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a complete fixture and its pixel oracle must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;

/// The blue band before `/Tx` contains the text `/Tx BMC` as a literal-string property value.
/// It is legal BDC property-list data and must not be mistaken for the marked-content tag.
fn source() -> Vec<u8> {
    let stream = concat!(
        "q\n",
        "1 0 0 rg 0 25 20 5 re f\n",
        "/Span << /ActualText (/Tx BMC) >> BDC\n",
        "0 0 1 rg 20 25 20 5 re f\n",
        "EMC\n",
        "/Tx BMC\n",
        "BT /Helv 12 Tf 0 g 1 0 0 1 2 12 Tm (old) Tj ET\n",
        "0 1 0 rg 40 25 20 5 re f\n",
        "/Span BMC\n",
        "1 0 1 rg 60 25 20 5 re f\n",
        "EMC\n",
        "0 0 1 rg 80 25 20 5 re f\n",
        "EMC\n",
        "0 1 1 rg 100 25 20 5 re f\n",
        "Q\n",
    );
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [4 0 R] /DA (/Helv 12 Tf 0 g) \
         /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 7 0 R /Annots [4 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 \
         /P 3 0 R /FT /Tx /T (field) /V (old) /AP << /N 5 0 R >> \
         /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 160 30] \
         /Resources << /Font << /Helv 6 0 R >> >> /Length {} >>\nstream\n{}endstream\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n\
         7 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        stream.len(),
        stream
    );
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }
    let xref = pdf.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn raster(document: &Document, value: &str) -> pdf_render::Raster {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the complete page tree");
    let view = ViewState::of(document);
    let interpreted = pdf_model::content::interpret_with(document, &page, &view);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    assert!(
        interpreted.text.contains(value),
        "appearance matches the field value"
    );
    if value == "new" {
        assert!(!interpreted.text.contains("old"), "old text was replaced");
    }
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, GENEROUS)
        .expect("page fits the raster budget");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("the fixture uses supported paint operators")
}

fn pixel(raster: &pdf_render::Raster, x: u32, pdf_y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(pdf_y);
    let at = (row
        .saturating_mul(raster.width)
        .saturating_add(x)
        .saturating_mul(4)) as usize;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn opaque_color(pixel: [u8; 4], r: u8, g: u8, b: u8, label: &str) {
    assert!(pixel[3] > 240, "{label} should remain opaque: {pixel:?}");
    assert!(
        pixel[0].abs_diff(r) < 8 && pixel[1].abs_diff(g) < 8 && pixel[2].abs_diff(b) < 8,
        "{label} should be RGB({r},{g},{b}), got {pixel:?}"
    );
}

#[test]
fn saved_tx_replacement_reaches_matching_emc_and_preserves_outside_paint() {
    let source = Document::open(source()).expect("valid complete PDF");
    assert!(!source.was_recovered(), "source xref is exact");
    let original = raster(&source, "old");
    // Positive controls establish the literal-string trap and each painted band before editing.
    opaque_color(
        pixel(&original, 30, 67),
        255,
        0,
        0,
        "leading red outside band",
    );
    opaque_color(
        pixel(&original, 50, 67),
        0,
        0,
        255,
        "literal-string BDC blue band",
    );
    opaque_color(pixel(&original, 70, 67), 0, 255, 0, "Tx green band");
    opaque_color(pixel(&original, 90, 67), 255, 0, 255, "nested magenta band");
    opaque_color(
        pixel(&original, 110, 67),
        0,
        0,
        255,
        "Tx trailing blue band",
    );
    opaque_color(
        pixel(&original, 130, 67),
        0,
        255,
        255,
        "trailing cyan outside band",
    );

    let mut view = ViewState::of(&source);
    assert_eq!(
        view.set_field(&source, "field", &Entered::Text("new".to_owned())),
        1,
        "the fixture has one editable text widget"
    );
    let written = view.save(&source).expect("the edit can be saved");
    assert!(
        written.unconstructed.is_empty(),
        "{:?}",
        written.unconstructed
    );
    let saved = Document::open(written.bytes).expect("saved incremental PDF");
    assert!(!saved.was_recovered(), "saved xref is exact");
    let rendered = raster(&saved, "new");

    // Content before the real tag and after the matching EMC survives. If the writer stops at
    // the nested EMC, the later blue band inside `/Tx` is left behind.
    opaque_color(
        pixel(&rendered, 30, 67),
        255,
        0,
        0,
        "leading red outside band",
    );
    opaque_color(
        pixel(&rendered, 50, 67),
        0,
        0,
        255,
        "literal-string BDC blue band",
    );
    opaque_color(
        pixel(&rendered, 130, 67),
        0,
        255,
        255,
        "trailing cyan outside band",
    );
    for (x, label) in [
        (70, "Tx green band"),
        (90, "nested magenta band"),
        (110, "content after inner EMC but before matching EMC"),
    ] {
        assert_eq!(pixel(&rendered, x, 67)[3], 0, "{label} was replaced");
    }
}
