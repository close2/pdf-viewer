//! Consumer regressions for §11.3.7.2's tiling-pattern shape/opacity rule.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "complete tiny fixtures and bounded raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;

/// A complete one-page PDF: a white page invokes an isolated knockout form. The form first
/// paints red, then paints the full page with the tiling pattern, making the pattern paint one
/// later element whose separate shape and opacity determine what happens to the red beneath.
fn pattern_in_knockout_cell(cell: &str, cell_resources: &str) -> Vec<u8> {
    let page = "1 1 1 rg 0 0 100 100 re f /Fm Do";
    let form = "1 0 0 rg 0 0 100 100 re f /Pattern cs /P0 scn 0 0 100 100 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources {cell_resources} /Length {} >>\nstream\n{cell}\nendstream",
        cell.len().saturating_add(1)
    );
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm 6 0 R >> /Pattern << /P0 5 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n{pattern}\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I true /K true >> \
         /Resources << /Pattern << /P0 5 0 R >> >> /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n",
        page.len().saturating_add(1),
        form.len().saturating_add(1),
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "exact source xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture should interpret completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid raster target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU consumer supports the fixture")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let start = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel index");
    [
        raster.data[start],
        raster.data[start + 1],
        raster.data[start + 2],
        raster.data[start + 3],
    ]
}

fn near(got: [u8; 4], want: [u8; 4], context: &str) {
    assert!(
        got.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 2),
        "{context}: got {got:?}, expected {want:?}"
    );
}

/// A pattern cell paints only its left half. Within an isolated knockout group that pattern is
/// one element: its painted region knocks red out; the cell hole has zero object shape and must
/// preserve the red element below. Treating the path filled by the pattern as shape 1 everywhere
/// incorrectly knocks the red out in each cell's blank half.
#[test]
fn tiling_pattern_holes_do_not_knock_out_the_prior_element() {
    let raster = render(pattern_in_knockout_cell("0 g 0 0 10 20 re f", "<< >>"));

    // Cell (0,0): x=5 is covered by black ink; x=15 is the empty half-cell. Raster y=94
    // corresponds to PDF user-space y=5.
    near(pixel(&raster, 5, 94), [0, 0, 0, 255], "painted half-cell");
    near(
        pixel(&raster, 15, 94),
        [255, 0, 0, 255],
        "cell hole retains the prior red element rather than knocking it out",
    );
    // Adjacent tile proves the lattice repeats the same shape, rather than one local mask.
    near(
        pixel(&raster, 25, 94),
        [0, 0, 0, 255],
        "next tile's painted half",
    );
    near(pixel(&raster, 35, 94), [255, 0, 0, 255], "next tile's hole");
}

/// The cell's explicit /ca makes painted points half-opacity, but its geometric shape remains
/// one over those points. Knockout therefore replaces red with half-alpha black there; the
/// isolated form is then composited on white, giving neutral 128 gray. Empty cell points have
/// shape zero and retain red. Conflating opacity with shape gives a different knockout blend.
#[test]
fn tiling_pattern_cell_opacity_does_not_become_its_knockout_shape() {
    let raster = render(pattern_in_knockout_cell(
        "/Half gs 0 g 0 0 10 20 re f",
        "<< /ExtGState << /Half << /ca 0.5 >> >> >>",
    ));

    near(
        pixel(&raster, 5, 94),
        [128, 128, 128, 255],
        "half-opacity black with full geometric shape replaces red, then composites on white",
    );
    near(
        pixel(&raster, 15, 94),
        [255, 0, 0, 255],
        "no cell shape leaves the earlier knockout-group element unchanged",
    );
}
