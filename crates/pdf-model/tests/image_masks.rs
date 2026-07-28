//! Stencil masking, checked against ISO 32000-2 §8.9.6.2's own three differences.
//!
//! An image mask paints the *current colour* through its set bits, which makes it unlike
//! every other image: it carries no colour of its own, one bit per sample, and its `/Decode`
//! array decides which of the two bit values marks the page. Those three sentences are the
//! whole of what distinguishes it, and until this file existed none of them was held by a
//! test — the corpus and the oracle covered it, in the sense that a page drawn through a
//! stencil would have looked wrong, which is not the same as a rule that fails by name.
//!
//! The gap was found by reading §8.9.6 as a family while filling the conformance ledger, not
//! by anything that renders a page. It is a small instance of the argument for the ledger:
//! the two gates ask what documents need, and neither can notice that a rule everyone
//! believes is implemented has nothing pinning it.
//!
//! Explicit masking (§8.9.6.3) and colour key masking (§8.9.6.4) are *not* implemented, and
//! `render_real_pdf.rs` holds the test that they are reported. When either lands, its tests
//! belong here beside these.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are 40x40 pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF drawing one 4×2 image mask over the whole page, in red.
///
/// The page is 40 units square so that each of the mask's eight cells covers 10×20 pixels.
/// A cell the size of a pixel or two would be judged on values `tiny-skia`'s bilinear
/// filter blends across the cell boundary, and the test would be measuring the filter.
///
/// The samples are two rows of four bits, each padded out to a byte. ISO 32000-2 §8.9.3:
///
/// > Byte boundaries shall be ignored, except that each row of sample data shall begin on a
/// > byte boundary.
///
/// The first row is `0 1 1 1`, the second `0 0 0 1`. With the default `/Decode` that marks one
/// cell of the top row and three of the bottom — an asymmetric shape in both axes, which is
/// what distinguishes a correct read from one that is mirrored either way. A symmetric
/// pattern would pass while flipped, which is the mistake trap 2 in `doc/HANDOVER.md` is
/// about.
fn stencil(decode: &str) -> Vec<u8> {
    // `0b0111_0000` and `0b0001_0000`, the four significant bits at the top of each byte.
    let samples = "\x70\x10";
    let content = "1 0 0 rg 40 0 0 40 0 0 cm /Im Do";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
         /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true \
         {decode} /Length 2 >>\nstream\n{samples}\nendstream\nendobj\n",
        content.len().saturating_add(1)
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

/// Renders a fixture at one pixel per unit onto a transparent background.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_background(pdf_render::Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported")
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

/// Whether a point was marked, and — when it was — that it was marked in the fill colour.
///
/// A threshold rather than an equality, and the reason is the fixture rather than the code:
/// the mask is four texels wide stretched across forty pixels, so `tiny-skia`'s bilinear
/// filter ramps between neighbouring cells and a point half a pixel off a texel centre
/// carries a few percent of its neighbour. What these tests ask is *which cells mark the
/// page*, not how the sampler interpolates between them — and the colour assertion still
/// holds exactly, because a stencil paints one colour or nothing.
fn marked(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let [red, green, blue, alpha] = pixel(raster, x, y);
    assert!(
        alpha < 32 || (red > 200 && green < 32 && blue < 32),
        "a stencil marks the page in the current colour, not in {red},{green},{blue}"
    );
    alpha > 128
}

/// A sample of 0 marks the page with the current colour; a 1 leaves it alone.
///
/// §8.9.6.2: "If the Decode array is [0 1] (the default for an image mask), a sample value of
/// 0 shall mark the page with the current colour, and a 1 shall leave the previous contents
/// unchanged." The image's first row is the *top* of the unit square, which is where the y
/// flip could quietly go wrong.
#[test]
fn a_zero_sample_paints_the_current_colour_and_a_one_leaves_the_page_alone() {
    let raster = render(stencil(""));
    assert!(marked(&raster, 5, 30), "top row, first cell");
    assert!(!marked(&raster, 25, 30), "top row, third cell");
    assert!(marked(&raster, 5, 5), "bottom row, first cell");
    assert!(!marked(&raster, 35, 5), "bottom row, fourth cell");
}

/// `/Decode [1 0]` reverses which bit marks the page.
///
/// §8.9.6.2: "If the Decode array is [1 0], these meanings shall be reversed." Every pixel
/// swaps, which is what makes this test worth having separately: a reader that ignores
/// `/Decode` entirely passes the test above.
#[test]
fn a_decode_array_of_one_zero_reverses_the_stencil() {
    let raster = render(stencil("/Decode [1 0]"));
    assert!(!marked(&raster, 5, 30), "top row, first cell, now clear");
    assert!(marked(&raster, 25, 30), "top row, third cell, now painted");
    assert!(!marked(&raster, 5, 5), "bottom row, first cell, now clear");
    assert!(
        marked(&raster, 35, 5),
        "bottom row, fourth cell, now painted"
    );
}
