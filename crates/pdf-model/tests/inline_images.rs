//! Inline images, against ISO 32000-2 §8.9.7's own rules.
//!
//! An inline image is the one image whose *extent* is a question. Everything else about it —
//! the samples, the colour space, the filters — is an ordinary image, and this file pins that
//! by rendering pages and reading pixels rather than by inspecting a dictionary: the mapping
//! from Table 91's abbreviated keys to real ones is only worth anything if a page comes out
//! right at the end of it.
//!
//! Three of the tests here are about the extent, and each is a case where a reader that gets
//! it wrong still draws *something*:
//!
//! - data holding a whitespace-delimited `EI` of its own, which a search stops at early;
//! - a `/Length` that says where the data ends, which is the only answer for filtered data;
//! - what runs after `EI`, which is what a reader that mislocates the end silently loses.
//!
//! The last is the reason the fixtures paint a marker rectangle after the image. A content
//! stream that resumes in the middle of image data does not fail — it executes whatever the
//! samples happen to spell, which is usually nothing, and the page comes out missing its
//! remaining content with no report at all.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are 40x40 pages where no index can overflow"
)]
#![expect(
    clippy::doc_markdown,
    reason = "these comments quote the standard, and a quotation with backticks added to \
              please a lint is no longer a quotation"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF whose content stream is `content`, with `resources` as its resources.
///
/// The page is 40 units square and the content is written as raw bytes, because an inline
/// image's data is not text and several of these fixtures depend on exactly which bytes are
/// between `ID` and `EI`.
fn fixture(content: &[u8], resources: &str) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
             /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream\nendobj\n");

    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body.as_slice();
    while let Some(at) = position_after(rest, b"endobj\n") {
        offsets.push(out.len());
        out.extend_from_slice(&rest[..at]);
        rest = &rest[at..];
    }

    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// The offset just past the first occurrence of `needle`.
fn position_after(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at + needle.len())
}

/// A 2×2 `DeviceRGB` image scaled over the whole page, written with Table 91's abbreviations.
///
/// Red and green on the top row, blue and white on the bottom, so no quadrant can be
/// confused with another under a flip in either axis — the mistake trap 2 in
/// `doc/HANDOVER.md` is about.
const QUADRANTS: &[u8] = b"\xff\x00\x00\x00\xff\x00\x00\x00\xff\xff\xff\xff";

/// Renders a fixture at one pixel per unit, and returns what could not be drawn with it.
fn render(bytes: Vec<u8>) -> (pdf_render::Raster, Vec<pdf_model::Unsupported>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let unsupported = interpretation.unsupported.clone();
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_background(pdf_render::Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported");
    (raster, unsupported)
}

/// Renders a fixture that must draw completely.
fn render_complete(bytes: Vec<u8>) -> pdf_render::Raster {
    let (raster, unsupported) = render(bytes);
    assert!(
        unsupported.is_empty(),
        "the fixture should draw completely: {unsupported:?}"
    );
    raster
}

/// The RGBA at a point given in PDF coordinates, whose y runs the other way from a raster's.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// Asserts a pixel is the given colour, allowing for the sampler's ramp between texels.
fn assert_colour(raster: &pdf_render::Raster, x: u32, y: u32, expected: [u8; 3], what: &str) {
    let [red, green, blue, alpha] = pixel(raster, x, y);
    assert!(alpha > 200, "{what}: nothing was drawn at {x},{y}");
    for (got, want) in [red, green, blue].into_iter().zip(expected) {
        assert!(
            got.abs_diff(want) < 24,
            "{what}: expected {expected:?} at {x},{y}, got {red},{green},{blue}"
        );
    }
}

/// The content stream after an image: a black square in the page's bottom-left corner.
///
/// Its whole job is to be *missing* when interpretation resumes inside the image data. The
/// square is 6×6 at the origin, clear of every quadrant test point — and it comes after the
/// `Q` that ends the image's own scaling, or the same six units would cover the page.
const MARKER: &[u8] = b" 0 0 0 rg 0 0 6 6 re f";

/// The marker square is there, which means interpretation resumed past `EI`.
fn assert_marker(raster: &pdf_render::Raster) {
    assert_colour(raster, 3, 3, [0, 0, 0], "the content after EI");
}

/// An inline image draws, with Table 91's and Table 92's abbreviations expanded.
///
/// `/W`, `/H`, `/BPC`, `/CS` and `/RGB` are all abbreviations, and a reader that does not
/// expand them has no width, no height and no colour space — so this is the whole of §8.9.7's
/// first half in one page.
#[test]
fn an_inline_image_draws_from_its_abbreviated_keys() {
    let mut content: Vec<u8> = b"q 40 0 0 40 0 0 cm BI /W 2 /H 2 /BPC 8 /CS /RGB ID ".to_vec();
    content.extend_from_slice(QUADRANTS);
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let raster = render_complete(fixture(&content, ""));
    assert_colour(&raster, 5, 35, [255, 0, 0], "top left");
    assert_colour(&raster, 35, 35, [0, 255, 0], "top right");
    assert_colour(&raster, 5, 15, [0, 0, 255], "bottom left");
    assert_colour(&raster, 35, 15, [255, 255, 255], "bottom right");
    assert_marker(&raster);
}

/// Sample data that spells ` EI ` does not end the image.
///
/// §8.9.3 fixes the layout of unfiltered samples, so a reader that computes the length from
/// `/W`, `/H`, `/BPC` and the colour space never has to look at the data at all. One that
/// searches for `EI` instead stops at the third sample here, draws a torn image, and resumes
/// interpretation inside the remaining samples — losing the marker square with no report.
#[test]
fn data_that_contains_ei_is_not_cut_short_by_it() {
    // Six `DeviceGray` samples, the middle four spelling ` EI ` — 0x20, 0x45, 0x49, 0x20.
    let samples: &[u8] = b"\xff EI \x00";
    let mut content: Vec<u8> = b"q 40 0 0 40 0 0 cm BI /W 6 /H 1 /BPC 8 /CS /G ID ".to_vec();
    content.extend_from_slice(samples);
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let raster = render_complete(fixture(&content, ""));
    assert_colour(&raster, 3, 20, [255, 255, 255], "the first sample");
    assert_colour(&raster, 36, 20, [0, 0, 0], "the sixth sample");
    assert_marker(&raster);
}

/// A document to hand the scanner, which needs one for its resource bounds.
///
/// The three tests below ask `crate::inline_image` where the data ended rather than reading
/// pixels, because that is the question, and a page cannot answer it: filtered data whose
/// end is misplaced usually still decodes to *something*, and the difference between the two
/// answers is bytes rather than colours.
fn scanning_document() -> Document {
    Document::open(fixture(b"", "")).expect("the fixture is a valid PDF")
}

/// Scans the inline image in `content`, whose `BI` is the first two bytes.
fn scan(content: &[u8]) -> pdf_model::inline_image::Scan {
    pdf_model::inline_image::scan(
        &scanning_document(),
        content,
        2,
        &pdf_syntax::Dictionary::new(),
    )
}

/// `/L` says where filtered data ends, and is used in preference to searching for `EI`.
///
/// §8.9.7: the value of `/L` "is the length of the data between the ID and EI operators
/// excluding the white-space delimiting those operators". It is the only answer available for
/// compressed data, whose bytes can spell anything — including the `EI` a search would stop
/// at, which is what this data does.
#[test]
fn a_stated_length_locates_the_end_of_filtered_data() {
    let content = b"BI /W 8 /H 1 /BPC 8 /CS /G /F /Fl /L 7 ID \x01 EI \x02\x03 EI Q";
    let scanned = scan(content);

    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), b"\x01 EI \x02\x03");
    // Past the second `EI`, which is the real one: two bytes of ` Q` are left.
    assert_eq!(scanned.resume, content.len() - 2);
}

/// A `/L` that does not predict an `EI` is not believed.
///
/// The clause requires the entry to be right; a file where it is not would otherwise take
/// the rest of the page's content stream as image data. Checking it against the terminator
/// it predicts costs one comparison and turns a wrong length into a fallback rather than
/// into a blank page.
#[test]
fn a_length_that_does_not_predict_the_terminator_falls_back_to_the_search() {
    let content = b"BI /W 2 /H 1 /BPC 8 /CS /G /F /Fl /L 4000 ID \x01\x02 EI Q";
    let scanned = scan(content);

    let stream = scanned.image.expect("an image");
    assert_eq!(stream.data.as_ref(), b"\x01\x02");
    assert_eq!(scanned.resume, content.len() - 2);
}

/// Filtered data with no terminator at all is reported, and consumes the rest of the stream.
///
/// The rest of the stream is the only safe place to resume: the bytes after `ID` are not a
/// program, so handing any of them back to the lexer risks drawing whatever they spell.
#[test]
fn filtered_data_with_no_terminator_is_reported() {
    let content = b"BI /W 2 /H 1 /BPC 8 /CS /G /F /Fl ID \x01\x02\x03";
    let scanned = scan(content);

    assert!(scanned.image.is_err(), "there is no EI to find");
    assert_eq!(scanned.resume, content.len());
}

/// A `/CS` name that is not a device space names a resource.
///
/// §8.9.7: "the value of the ColorSpace entry may also be the name of a colour space in the
/// ColorSpace subdictionary of the current resource dictionary". The space here is
/// `[/Indexed /DeviceRGB 1 <ff000000ff00>]`, whose two entries are red and green, so a reader
/// that ignored the resource and guessed a device space would draw grey.
#[test]
fn a_colour_space_name_is_looked_up_in_the_resources() {
    let mut content: Vec<u8> = b"q 40 0 0 40 0 0 cm BI /W 2 /H 1 /BPC 8 /CS /Cs1 ID ".to_vec();
    content.extend_from_slice(b"\x00\x01");
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let raster = render_complete(fixture(
        &content,
        "/ColorSpace << /Cs1 [/Indexed /DeviceRGB 1 <ff000000ff00>] >>",
    ));
    assert_colour(&raster, 5, 20, [255, 0, 0], "index 0 is red");
    assert_colour(&raster, 35, 20, [0, 255, 0], "index 1 is green");
    assert_marker(&raster);
}

/// An inline image that cannot be decoded is reported, and the rest of the page still draws.
///
/// The filter is `CCITTFaxDecode`, which this tree does not implement — so the image is
/// named as undrawn rather than silently skipped, which is trap 5, and the content after
/// `EI` still runs, which is what says the data was stepped over rather than executed.
#[test]
fn an_inline_image_we_cannot_decode_is_reported_and_the_page_continues() {
    let mut content: Vec<u8> =
        b"q 40 0 0 40 0 0 cm BI /W 2 /H 1 /BPC 8 /CS /G /F /CCF ID ".to_vec();
    content.extend_from_slice(b"\x01\x02\x03");
    content.extend_from_slice(b" EI Q");
    content.extend_from_slice(MARKER);

    let (raster, unsupported) = render(fixture(&content, ""));
    assert!(
        unsupported.iter().any(|item| matches!(
            item,
            pdf_model::Unsupported::Image { name } if name.starts_with("<inline>")
        )),
        "an undecodable inline image should be reported: {unsupported:?}"
    );
    assert_marker(&raster);
}
