//! A fill whose shape encloses no area still paints the pixels it lies in — ISO 32000-2 §10.7.4
//! and §8.5.3.3.1, ADR 2330.
//!
//! # What the expected values come from
//!
//! §10.7.4: "A shape shall be scan-converted by painting any pixel whose half-open square region
//! intersects the shape, no matter how small the intersection is", with NOTE 1's reading of
//! *intersects* for exactly this case — "a filling region is considered to intersect every pixel
//! through which its boundary passes, even if the interior of the filling region is empty" — and
//! §8.5.3.3.1 names the pixel a point is: "If a subpath is degenerate (consists entirely of one or
//! more points at the same coordinates), the subpath shall be considered to enclose the single
//! device pixel lying under that point". The same subclause of §10.7.4 identifies that pixel:
//! "let i = floor( x ) and j = floor( y ). The pixel that contains this point is the one
//! identified as ( i, j )".
//!
//! The page is 100 units square drawn at one device pixel per unit, so user `(x, y)` is device
//! `(x, 100 − y)`: the point `(10.5, 20.25)` is device `(10.5, 79.75)`, pixel column 10 of row 79,
//! painted whole, and no other pixel is.
//!
//! A subpath whose points all lie on one line across the axes encloses no area either, and its
//! mark is the band of one device pixel along that line — the construction a line across the
//! grid already takes when its matrix carries a rectangle onto it (ADR 1360), so its ink is its
//! length in device pixels.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not open should fail loudly, and the arithmetic is \
              over a fixture of a few hundred bytes and a raster this file sized"
)]

use std::fmt::Write as _;

use pdf_model::Unsupported;
use pdf_render::{Rasterizer as _, TargetSpec};
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

/// The page drawn at one device pixel per unit: the ink of every pixel, row by row, and what the
/// interpretation reported.
fn drawn(content: &str) -> (Vec<Vec<f64>>, Vec<Unsupported>) {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the page draws");
    let width = raster.width as usize;
    let pixels = raster
        .data
        .chunks_exact(width * 4)
        .map(|row| {
            row.chunks_exact(4)
                .map(|pixel| f64::from(255 - pixel[0]) / 255.0)
                .collect()
        })
        .collect();
    (pixels, interpretation.unsupported)
}

/// Every pixel of `pixels` is whole where `inked` says and blank everywhere else.
fn only(pixels: &[Vec<f64>], content: &str, inked: &[(usize, usize)]) {
    for (row, line) in pixels.iter().enumerate() {
        for (column, ink) in line.iter().enumerate() {
            let want = if inked.contains(&(column, row)) {
                1.0
            } else {
                0.0
            };
            assert!(
                (ink - want).abs() < 0.5 / 255.0,
                "{content}: pixel ({column}, {row}) carries {ink} where the clause puts {want}"
            );
        }
    }
}

/// A degenerate subpath, however the file states it, paints the one pixel under its point.
#[test]
fn a_point_paints_the_pixel_under_it() {
    for content in [
        "0 g 10.5 20.25 m h f",
        "0 g 10.5 20.25 m 10.5 20.25 l f",
        "0 g 10.5 20.25 m 10.5 20.25 l 10.5 20.25 l h f",
        "0 g 10.5 20.25 0 0 re f",
        "0 g 10.5 20.25 0 0 re f*",
    ] {
        let (pixels, _) = drawn(content);
        only(&pixels, content, &[(10, 79)]);
    }
}

/// A point on a pixel's corner lies in the pixel whose lower boundaries it is on, because the
/// region is half-open: device `(30, 60)` is pixel `(30, 60)`, and not one of its neighbours.
#[test]
fn a_point_on_a_corner_takes_the_pixel_it_floors_to() {
    let content = "0 g 30 40 m h f";
    let (pixels, _) = drawn(content);
    only(&pixels, content, &[(30, 60)]);
}

/// The point beside an area of its own path is one more pixel, and the area keeps its own fill.
#[test]
fn a_point_beside_an_area_is_one_more_pixel() {
    let content = "0 g 0 90 10 10 re 50.5 50.5 m h f";
    let (pixels, _) = drawn(content);
    let mut inked: Vec<(usize, usize)> = (0..10)
        .flat_map(|row| (0..10).map(move |column| (column, row)))
        .collect();
    inked.push((50, 49));
    only(&pixels, content, &inked);
}

/// A trailing `m` is not a subpath, and §8.5.3.3.1 says so outright: "if the last subpath in the
/// path is a single-point open subpath (specified by a trailing m operator), it shall be
/// disregarded and not considered to be part of the path".
#[test]
fn a_trailing_move_is_disregarded() {
    let content = "0 g 10.5 20.25 m f";
    let (pixels, _) = drawn(content);
    only(&pixels, content, &[]);
}

/// A clip is the set of pixels the fill of its path includes, so a degenerate clipping path
/// admits the pixel its fill paints and nothing else (ADR 1060's one decision, both halves).
#[test]
fn a_degenerate_clip_admits_the_pixel_its_fill_paints() {
    let content = "10.5 20.25 0 0 re W n 0 g 0 0 100 100 re f";
    let (pixels, _) = drawn(content);
    only(&pixels, content, &[(10, 79)]);
}

/// A matrix with no extent at all carries every point of the mark onto `(e, f)`, and the shape
/// §10.7.4 scan-converts is that point: the pixel under it, and nothing reported as refused.
#[test]
fn a_mark_its_matrix_makes_a_point_is_the_pixel_under_it() {
    for content in [
        "0 g 0 0 0 0 50.5 50.5 cm 10 20 80 40 re f",
        "0 G 5 w 0 0 0 0 50.5 50.5 cm 20 30 m 60 70 l S",
    ] {
        let (pixels, reported) = drawn(content);
        assert!(
            !reported
                .iter()
                .any(|report| matches!(report, Unsupported::NoninvertibleMatrix { .. })),
            "{content}: the pixel is drawn, so nothing is reported about its matrix: {reported:?}"
        );
        only(&pixels, content, &[(50, 49)]);
    }
}

/// A subpath along a line across the axes encloses no area and is the band of one device pixel
/// along that line: from device `(30, 70)` to `(100, 0)`, `70√2` of ink, `√2` in every row the
/// line crosses and none in a row it does not.
#[test]
fn a_flat_subpath_across_the_axes_is_a_band_along_it() {
    for content in [
        "0 g 30 30 m 100 100 l f",
        "0 g 30 30 m 65 65 l 100 100 l h f",
        "0 g 30 30 m 50 50 60 60 100 100 c f",
        "0 g 100 100 m 30 30 l 65 65 l f*",
    ] {
        let (pixels, _) = drawn(content);
        let rows: Vec<f64> = pixels.iter().map(|row| row.iter().sum()).collect();
        let total: f64 = rows.iter().sum();
        let length = 70.0 * 2f64.sqrt();
        assert!(
            (total - length).abs() < 1.0,
            "{content}: the band carries {total} of ink where its length is {length}"
        );
        for (row, ink) in rows.iter().enumerate() {
            let expected = if row < 70 { 2f64.sqrt() } else { 0.0 };
            assert!(
                (ink - expected).abs() < 0.1 || row == 0 || row == 69 || row == 70,
                "{content}: device row {row} carries {ink} where the band puts {expected}"
            );
        }
    }
}

/// A sliver with an area, however thin, is not a line: it is drawn at the coverage its own area
/// implies (§10.7.4's first departure), so it marks far less than a band of one pixel would.
#[test]
fn a_sliver_with_an_area_is_not_a_band() {
    let content = "0 g 30 30 m 100 100 l 100 100.01 l h f";
    let (pixels, _) = drawn(content);
    let total: f64 = pixels.iter().flatten().sum();
    assert!(
        total < 1.0,
        "{content}: the sliver's area is under a pixel and it carries {total} of ink"
    );
}
