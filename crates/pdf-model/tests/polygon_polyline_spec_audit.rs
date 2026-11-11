//! Focused §12.5.6.9 regressions. These complete PDFs intentionally omit `/AP` to exercise
//! this reader's constructed-appearance fallback policy; they are not conforming producer
//! examples for nonempty annotation rectangles.

#![expect(
    clippy::expect_used,
    reason = "fixed complete PDFs and public interpretation must fail loudly on regression"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const BUDGET: u64 = 1 << 24;

fn document(header: &str, annotation: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n"
    );
    let mut out = String::from(header);
    out.push('\n');
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    if header.contains("2.0") {
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R \
             /ID [<0123456789ABCDEF0123456789ABCDEF> \
                  <0123456789ABCDEF0123456789ABCDEF>] >>\n"
        );
    } else {
        let _ = write!(out, "trailer\n<< /Size {size} /Root 1 0 R >>\n");
    }
    let _ = write!(out, "startxref\n{xref_at}\n%%EOF\n");
    out.into_bytes()
}

fn raster(header: &str, annotation: &str) -> pdf_render::Raster {
    let bytes = document(header, annotation);
    let parsed = Document::open(bytes).expect("complete PDF syntax");
    assert!(!parsed.was_recovered(), "exact xref offsets");
    let page = pdf_model::Pages::new(&parsed).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&parsed, &page);
    assert!(
        interpretation.is_complete(),
        "supported shape interpretation: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, BUDGET).expect("valid raster target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterization")
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "offsets are bounded by the fixed 100 by 100 raster"
)]
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let offset = ((row.saturating_mul(raster.width).saturating_add(x)) as usize).saturating_mul(4);
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

#[test]
fn polyline_ic_fills_closed_endings_but_not_open_endings() {
    // A line is the positive control for Table179 Square's established `/IC` fill path.
    // The pixel lies inside the square, away from its stroked boundary; the fill covers the line.
    let line = raster(
        "%PDF-1.7",
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 \
         /L [20 50 80 50] /C [0 0 0] /IC [1 0 0] /BS << /W 2 >> \
         /LE [/None /Square] >>",
    );
    // The second endpoint's square covers its center, after the line is stroked.
    assert_eq!(pixel(&line, 80, 50), [255, 0, 0, 255]);

    let polyline = |ending: &str, interior: &str| {
        raster(
            "%PDF-1.7",
            &format!(
                "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] /F 4 \
                 /Vertices [20 50 50 50 80 50] /C [0 0 0] {interior} \
                 /BS << /W 2 >> /LE [/None /{ending}] >>"
            ),
        )
    };

    // The open-style control comes before the expected failing closed-style behavior. `/IC` is
    // only a fill and must not alter an OpenArrow's unfilled shape.
    assert_eq!(
        polyline("OpenArrow", "/IC [1 0 0]").data,
        polyline("OpenArrow", "").data
    );
    let filled = polyline("Square", "/IC [1 0 0]");
    let unfilled = polyline("Square", "");
    assert!(
        filled.data != unfilled.data,
        "Table181 requires PolyLine /IC to fill its Square ending"
    );
    assert_eq!(pixel(&filled, 80, 50), [255, 0, 0, 255]);
}

#[test]
fn polygon_path_strokes_its_closing_edge_while_polyline_path_stays_open() {
    // PDF 2.0 makes Path a valid geometry form here. Two segments make a triangle only when
    // Polygon closes it. The diagonal midpoint (50,50) is off both explicit segments.
    let path = "/Vertices [5 5 10 5 10 10] /Path [[20 20] [80 20] [80 80]]";
    let polyline = raster(
        "%PDF-2.0",
        &format!(
            "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] /F 4 \
             /C [0 0 0] /BS << /W 2 >> {path} >>"
        ),
    );
    // EC3 #444 changes both cells to ignore Vertices when Path exists; coexistence is
    // not forbidden. The distinct decoy triangle must not appear.
    assert_eq!(pixel(&polyline, 7, 5)[3], 0);
    assert_eq!(pixel(&polyline, 50, 20), [0, 0, 0, 255]);
    assert_eq!(pixel(&polyline, 80, 50), [0, 0, 0, 255]);
    assert_eq!(pixel(&polyline, 50, 50)[3], 0, "the polyline stays open");

    let polygon = raster(
        "%PDF-2.0",
        &format!(
            "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] /F 4 \
             /C [0 0 0] /BS << /W 2 >> {path} >>"
        ),
    );
    assert_eq!(
        pixel(&polygon, 50, 50),
        [0, 0, 0, 255],
        "Polygon is a closed outline; its closing edge must be stroked"
    );
}

#[test]
fn polyline_first_ending_is_at_first_vertex_not_second_vertex() {
    let draw = |subtype: &str, geometry: &str, endings: &str| {
        raster(
            "%PDF-1.7",
            &format!(
                "<< /Type /Annot /Subtype /{subtype} /Rect [0 0 100 100]             /F 4 {geometry} /C [0 0 0] /BS << /W 2 >> /LE {endings} >>"
            ),
        )
    };
    let plain = draw("PolyLine", "/Vertices [20 50 50 50 80 50]", "[/None /None]");
    let first = draw(
        "PolyLine",
        "/Vertices [20 50 50 50 80 50]",
        "[/Square /None]",
    );
    let second = draw(
        "PolyLine",
        "/Vertices [20 50 50 50 80 50]",
        "[/None /Square]",
    );
    let line_first = draw("Line", "/L [20 50 80 50]", "[/Square /None]");
    // Three collinear vertices make the line geometry identical. Whole-raster comparison
    // avoids making the chosen square dimensions part of the normative assertion.
    let line_plain = draw("Line", "/L [20 50 80 50]", "[/None /None]");
    assert_eq!(plain.data, line_plain.data);
    assert_ne!(first.data, plain.data, "the first ending exists");
    assert_eq!(
        second.data,
        draw("Line", "/L [20 50 80 50]", "[/None /Square]").data,
        "the last endpoint is an independent positive control"
    );
    assert!(
        first.data == line_first.data,
        "Table181 locates the first ending at the first Vertices pair, not the interior vertex"
    );
}
