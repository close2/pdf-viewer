//! §12.5.6.17's movie annotation is drawn as §13.4's poster, and only where Table 306 states one.
//!
//! Table 306's `/Poster` takes three values, and each fixture here states one of them on the same
//! annotation, with no appearance stream so that what reaches the page is the constructed one:
//!
//! > If this value is a stream, it shall contain an image XObject (see 8.9, "Images") to be
//! > displayed as the poster. If it is the boolean value true , the poster image shall be
//! > retrieved from the movie file; if it is false , no poster shall be displayed. Default value:
//! > false .
//!
//! The stream is drawn, `false` and an absent entry draw nothing and owe nothing, and `true` is
//! refused out loud, because retrieving a frame from the movie file is playing the movie, which
//! `CLAUDE.md` principle 5's clause 13 exclusion keeps out (`doc/questions/A33`). Where the image
//! sits in `/Rect` is this program's choice and is reported as one (ADR 1561).

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are small enough that no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 100×100 page these tests build.
const GENEROUS: u64 = 1 << 30;

/// A two-sample image, red then blue, so that a placement's scale and its side are both visible.
///
/// Two samples wide and one high: §8.9.5 paints any image on the unit square, so only the
/// placement decides that the poster keeps this 2:1 shape in the square `/Rect` below.
const RED_THEN_BLUE: &str = "<< /Type /XObject /Subtype /Image /Width 2 /Height 1 \
     /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 14 >>\n\
     stream\nFF00000000FF>\nendstream";

/// A one-page document whose page holds one movie annotation stating `poster` as Table 306's
/// `/Poster`, or no `/Poster` where `poster` is `None`; object 6 is [`RED_THEN_BLUE`].
fn movie_with(poster: Option<&str>) -> Vec<u8> {
    let poster = poster.map_or_else(String::new, |value| format!(" /Poster {value}"));
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 1 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Movie /Rect [10 10 90 90] /F 4 \
         /Movie << /F (clip.mov){poster} >> >>\nendobj\n\
         6 0 obj\n{RED_THEN_BLUE}\nendobj\n"
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

/// The page's interpretation, its reports joined into one string, and its raster at one pixel
/// per unit.
fn draw(bytes: Vec<u8>) -> (String, pdf_render::Raster) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let reports = format!("{:?}", interpretation.unsupported);
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(list, target)
        .expect("supported");
    (reports, raster)
}

/// A pixel's colour and opacity at a point in PDF coordinates (the raster's rows run downward).
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height - 1 - y;
    let index = ((row * raster.width + x) as usize) * 4;
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

/// Whether nothing at all was painted on the page.
fn blank(raster: &pdf_render::Raster) -> bool {
    raster.data.chunks_exact(4).all(|sample| sample[3] == 0)
}

/// A stream `/Poster` is drawn in `/Rect`, scaled proportionally and centred, and says so.
///
/// The image is 2:1 and `/Rect` is 80 by 80, so Table 250's defaults — which ADR 1561 takes for
/// the placement — scale it to 80 by 40 and centre it, leaving 20 units empty above and below.
/// Red is the left half and blue the right, which is what tells a placement from a mirror.
#[test]
fn a_stream_poster_is_drawn_fitted_and_centred_in_the_rectangle() {
    let (reports, raster) = draw(movie_with(Some("6 0 R")));
    assert_eq!(pixel(&raster, 20, 50), [255, 0, 0, 255], "the left sample");
    assert_eq!(pixel(&raster, 80, 50), [0, 0, 255, 255], "the right sample");
    assert_eq!(pixel(&raster, 50, 25)[3], 0, "proportional: nothing below");
    assert_eq!(pixel(&raster, 50, 75)[3], 0, "and nothing above");
    assert_eq!(pixel(&raster, 5, 50)[3], 0, "and nothing outside /Rect");
    assert!(
        reports.contains("a placement this program chose"),
        "the placement is the program's, and the report says so: {reports}"
    );
}

/// `/Poster true` retrieves the poster from the movie file, which is playing it: refused, loudly.
#[test]
fn a_poster_retrieved_from_the_movie_file_is_refused_and_reported() {
    let (reports, raster) = draw(movie_with(Some("true")));
    assert!(blank(&raster), "nothing the file states is drawable");
    assert!(
        reports.contains("retrieves the poster image from the movie file"),
        "the refusal names the boolean form's own reason: {reports}"
    );
}

/// `/Poster false` is "no poster shall be displayed": nothing drawn, and nothing owed.
#[test]
fn a_false_poster_draws_nothing_and_owes_nothing() {
    let (reports, raster) = draw(movie_with(Some("false")));
    assert!(blank(&raster), "the clause says no poster is displayed");
    assert_eq!(reports, "[]", "and that is the whole of what it asks");
}

/// An absent `/Poster` takes Table 306's default, `false`, and is the same page.
#[test]
fn an_absent_poster_is_the_default_false() {
    let (reports, raster) = draw(movie_with(None));
    assert!(blank(&raster), "the default is no poster");
    assert_eq!(reports, "[]", "and nothing is owed for it");
}

/// A `/Poster` stream that is not an image `XObject` is not what the table requires: reported.
#[test]
fn a_poster_stream_that_is_not_an_image_is_reported() {
    let bytes = movie_with(Some("4 0 R"));
    let (reports, raster) = draw(bytes);
    assert!(blank(&raster), "a content stream is not a poster image");
    assert!(
        reports.contains("is not an image XObject"),
        "the report names the table's requirement: {reports}"
    );
}
