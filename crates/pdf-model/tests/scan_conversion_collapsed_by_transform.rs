//! A fill whose matrix carries it onto a line is §10.7.4's line — ISO 32000-2 §10.7.4 against
//! §8.3.4's third NOTE, `doc/todo/11` item 8.
//!
//! # What the expected values come from
//!
//! §10.7.4 states its rule for a shape whose "coordinates are mapped into device space": "A shape
//! shall be scan-converted by painting any pixel whose half-open square region intersects the
//! shape, no matter how small the intersection is", NOTE 1 adds that "a filling region is
//! considered to intersect every pixel through which its boundary passes, even if the interior of
//! the filling region is empty", and its EXAMPLE is the shape here: "A zero-width or zero-height
//! rectangle paints a line 1 pixel wide". `80 × 40` under `1 0 0 0 0 50.3 cm` is every point of
//! the rectangle carried onto the line `y = 50.3` from `x = 10` to `x = 90`, so on a page drawn at
//! one device pixel per unit the mark is device row `floor(100 − 50.3)` = 49, columns 10 to 89,
//! whole — eighty pixels of ink, and none anywhere else.
//!
//! What stays refused is stated by the clauses too: a matrix that carries the path onto one point
//! is §8.5.3.3.1's point, which this tree records as a departure, and a line across the axes is
//! `pdf_render::collapsed`'s stated absence. Both are still counted as
//! `Unsupported::NoninvertibleMatrix`, and the line is not.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not open should fail loudly, and the arithmetic is \
              over a fixture of a few hundred bytes and a raster this file sized"
)]

use std::fmt::Write as _;

use pdf_model::Unsupported;
use pdf_render::{Command, Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything here.
const GENEROUS: u64 = 1 << 30;

/// Builds a one-page fixture, 100 units square, whose content stream is `content`.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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

/// The page interpreted, and drawn at one device pixel per unit: its ink per device row, and what
/// it reported.
fn drawn(content: &str) -> (Vec<f64>, Vec<Unsupported>, Vec<Command>) {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the page draws");
    let width = raster.width as usize;
    let rows = raster
        .data
        .chunks_exact(width * 4)
        .map(|row| {
            row.chunks_exact(4)
                .map(|pixel| f64::from(255 - pixel[0]) / 255.0)
                .sum()
        })
        .collect();
    (rows, interpretation.unsupported, list.commands().to_vec())
}

/// A rectangle carried onto a horizontal line is the line: row 49, eighty whole pixels.
#[test]
fn a_rectangle_its_matrix_flattens_is_a_line_one_pixel_wide() {
    let (rows, reported, commands) = drawn("0 g 1 0 0 0 0 50.3 cm 10 20 80 40 re f");
    assert!(
        !reported
            .iter()
            .any(|report| matches!(report, Unsupported::NoninvertibleMatrix { .. })),
        "the line is drawn, so nothing is reported about its matrix: {reported:?}"
    );
    assert!(
        matches!(commands.as_slice(), [Command::Fill { transform, .. }] if *transform == Transform::IDENTITY),
        "the fill is restated in page space"
    );
    for (row, ink) in rows.iter().enumerate() {
        let expected = if row == 49 { 80.0 } else { 0.0 };
        assert!(
            (ink - expected).abs() < 0.5 / 255.0 * 100.0,
            "device row {row} carries {ink} of ink where the line puts {expected}"
        );
    }
}

/// A matrix carrying the rectangle onto a vertical line draws the column.
#[test]
fn a_rectangle_flattened_onto_a_vertical_line_is_a_column() {
    let (rows, _, _) = drawn("0 g 0 0 0 1 30.6 0 cm 10 20 80 40 re f");
    // `x = 30.6` for every point, `y` from 20 to 60: device column 30, rows 40 to 79, one pixel
    // of ink in each.
    for (row, ink) in rows.iter().enumerate() {
        let expected = if (40..80).contains(&row) { 1.0 } else { 0.0 };
        assert!(
            (ink - expected).abs() < 2.0 / 255.0,
            "device row {row} carries {ink} where the column puts {expected}"
        );
    }
}

/// A matrix carrying everything onto one point, and one carrying it onto a line across the axes,
/// stay refused and counted.
#[test]
fn a_point_and_a_line_across_the_axes_stay_refused() {
    for content in [
        "0 g 0 0 0 0 50 50 cm 10 20 80 40 re f",
        "0 g 1 1 1 1 0 0 cm 10 20 30 40 re f",
    ] {
        let (rows, reported, _) = drawn(content);
        assert!(
            reported
                .iter()
                .any(|report| matches!(report, Unsupported::NoninvertibleMatrix { commands: 1 })),
            "{content}: the mark is refused and counted: {reported:?}"
        );
        assert!(
            rows.iter().all(|ink| *ink == 0.0),
            "{content}: nothing is drawn"
        );
    }
}
