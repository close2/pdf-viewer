//! Masked images: ISO 32000-2 §8.9.6, one mechanism at a time.
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
//! Explicit masking (§8.9.6.3) and colour key masking (§8.9.6.4) landed in the fourteenth
//! session (ADR 0023) and their tests are below, beside the stencil ones the file started
//! with — which is the right arrangement for a family whose three mechanisms share one
//! clause and, in two cases, one dictionary key. §11.6.4.3's precedence between an image's
//! `/SMask` and its `/Mask` is tested here too, because the only place it can be seen is an
//! image that carries both.

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
    page_with_image(
        &format!("/Width 4 /Height 2 /ImageMask true {decode}"),
        PATTERN,
        &[],
    )
}

/// The four-by-two bit pattern every stencil in this file uses, packed a row to a byte.
///
/// `0 1 1 1` over `0 0 0 1`, asymmetric in both axes for the reason [`stencil`] gives.
const PATTERN: &[u8] = b"\x70\x10";

/// A one-page PDF, 40 units square, drawing one image over the whole of it.
///
/// `dict` is what goes inside the image `XObject`'s dictionary beyond its size, `data` is
/// its stream, and `extra` are further objects it refers to — numbered from 6, since this
/// builder uses 1 to 5. Everything is assembled as bytes rather than as a string, because
/// image samples are not text and a `String` would silently re-encode any byte above 127.
fn page_with_image(dict: &str, data: &[u8], extra: &[Vec<u8>]) -> Vec<u8> {
    let content = "1 0 0 rg 40 0 0 40 0 0 cm /Im Do";
    let mut objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        stream_object(5, &format!("/Type /XObject /Subtype /Image {dict}"), data),
    ];
    objects.extend_from_slice(extra);
    assemble(&objects)
}

/// One numbered stream object, with the `/Length` its data actually has.
fn stream_object(number: usize, dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// Assembles numbered objects into a file with a cross-reference table that points at them.
fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
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

/// Interprets a fixture without demanding that it drew completely.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// Four by two `DeviceRGB` samples, a byte a component, as §8.9.3 lays them out.
fn rgb_image(cells: [[u8; 3]; 8]) -> Vec<u8> {
    cells.into_iter().flatten().collect()
}

/// Whether a point carries no ink, which for these fixtures means the mask cut it out.
fn cut_out(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    pixel(raster, x, y)[3] < 32
}

/// The colour at a point, ignoring the few percent `tiny-skia` blends in from a neighbour.
///
/// The same tolerance [`marked`] takes, and for the same reason: these fixtures stretch four
/// texels across forty pixels, so the question a test may ask is which cell a point is in,
/// not what the sampler did between cells.
fn about(raster: &pdf_render::Raster, x: u32, y: u32, expected: [u8; 3]) -> bool {
    let [red, green, blue, alpha] = pixel(raster, x, y);
    alpha > 128
        && red.abs_diff(expected[0]) < 24
        && green.abs_diff(expected[1]) < 24
        && blue.abs_diff(expected[2]) < 24
}

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];

/// §8.9.6.4: a sample inside every range is not painted, and one outside any of them is.
///
/// > An image sample shall be masked (not painted) if all of its colour components before
/// > decoding, c 1 … c n , fall within the specified ranges (that is, if min i ≤ c i ≤ max i
/// > for all 1 ≤ i ≤ n ).
///
/// The fixture masks pure red out of a four-by-two image whose other cells are green and
/// blue, so a reader that applied the ranges to one component, or to the wrong one, paints a
/// cell this test demands is gone. `colorkeymask.pdf` is the same shape at page scale: three
/// bands, the red one masked, and we drew all three until the fourteenth session.
#[test]
fn colour_key_masking_removes_the_samples_inside_the_range() {
    let raster = render(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 \
         /Mask [255 255 0 0 0 0]",
        &rgb_image([RED, GREEN, BLUE, RED, GREEN, BLUE, RED, GREEN]),
        &[],
    ));

    assert!(cut_out(&raster, 5, 30), "top row, first cell, pure red");
    assert!(cut_out(&raster, 35, 30), "top row, fourth cell, pure red");
    assert!(cut_out(&raster, 25, 5), "bottom row, third cell, pure red");

    assert!(about(&raster, 15, 30, GREEN), "top row, second cell");
    assert!(about(&raster, 25, 30, BLUE), "top row, third cell");
    assert!(about(&raster, 5, 5, GREEN), "bottom row, first cell");
    assert!(about(&raster, 35, 5, GREEN), "bottom row, fourth cell");
}

/// §8.9.6.4's bounds include their endpoints, and one step outside is painted.
///
/// The clause writes the test as `min i ≤ c i ≤ max i`, so a range of 200 to 255 masks a red
/// of exactly 200 and exactly 255 and leaves 199 alone. Worth its own fixture because the
/// test above passes whether the comparison is inclusive or not — every value in it is at a
/// bound or far from one.
#[test]
fn the_colour_key_bounds_are_inclusive() {
    let raster = render(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 \
         /Mask [200 255 0 0 0 0]",
        &rgb_image([
            [199, 0, 0],
            [200, 0, 0],
            [255, 0, 0],
            BLUE,
            BLUE,
            BLUE,
            BLUE,
            BLUE,
        ]),
        &[],
    ));

    assert!(
        about(&raster, 5, 30, [199, 0, 0]),
        "199 is below the range and shall be painted"
    );
    assert!(cut_out(&raster, 15, 30), "200 is the lower bound");
    assert!(cut_out(&raster, 25, 30), "255 is the upper bound");
    assert!(about(&raster, 35, 30, BLUE), "a blue sample is untouched");
}

/// §8.9.6.3: the base image is painted where the mask marks and nowhere else.
///
/// > The image mask indicates which places on the page shall be painted and which shall be
/// > masked out (left unchanged). Unmasked areas shall be painted with the corresponding
/// > portions of the base image; masked areas shall not be.
///
/// Which bit marks is §8.9.6.2's rule, tested above for a stencil painting on its own; this
/// pins that the same reading governs when the stencil is somebody else's `/Mask`. The
/// pattern is the asymmetric one, so a mask applied mirrored in either axis fails.
#[test]
fn an_explicit_mask_paints_the_base_image_only_where_it_marks() {
    let raster = render(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R",
        &rgb_image([GREEN; 8]),
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true",
            PATTERN,
        )],
    ));

    assert!(about(&raster, 5, 30, GREEN), "top row, first cell, marked");
    assert!(cut_out(&raster, 25, 30), "top row, third cell, masked out");
    assert!(
        about(&raster, 5, 5, GREEN),
        "bottom row, first cell, marked"
    );
    assert!(
        cut_out(&raster, 35, 5),
        "bottom row, fourth cell, masked out"
    );
}

/// A mask finer than its image keeps its own detail.
///
/// §8.9.6.3 says the two "need not have the same resolution" and that both are defined on the
/// unit square, so the mask's eight cells still cut a one-sample image into eight. Combining
/// on the *image's* grid — the obvious implementation, and the one `/SMask` still uses —
/// would ask a single sample one question and paint the page all or nothing, which is what
/// this fixture is built to catch. `issue4246.pdf` is the corpus instance, at a ratio of
/// twenty: a 50×40 gradient behind a 1000×800 stencil that spells "Image Mask Example".
#[test]
fn a_mask_finer_than_its_image_keeps_its_own_resolution() {
    let raster = render(page_with_image(
        "/Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R",
        &BLUE,
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true",
            PATTERN,
        )],
    ));

    assert!(about(&raster, 5, 30, BLUE), "top row, first cell, marked");
    assert!(cut_out(&raster, 25, 30), "top row, third cell, masked out");
    assert!(about(&raster, 5, 5, BLUE), "bottom row, first cell, marked");
    assert!(
        cut_out(&raster, 35, 5),
        "bottom row, fourth cell, masked out"
    );
}

/// §11.6.4.3: an image's `/SMask` overrides its `/Mask`, so the `/Mask` does nothing.
///
/// > This mask, if present, shall override any explicit or colour key mask specified by the
/// > image dictionary's Mask entry.
///
/// The fixture gives an image an opaque soft mask and an explicit mask that marks nothing at
/// all, so the whole page rests on the precedence: honour it and the image is drawn entire,
/// drop it and the page is blank. It is the kind of rule that is invisible until the day a
/// file writes both, which is why it is pinned rather than trusted.
#[test]
fn a_soft_mask_overrides_an_explicit_mask() {
    let raster = render(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 \
         /SMask 6 0 R /Mask 7 0 R",
        &rgb_image([GREEN; 8]),
        &[
            stream_object(
                6,
                "/Type /XObject /Subtype /Image /Width 4 /Height 2 \
                 /ColorSpace /DeviceGray /BitsPerComponent 8",
                &[255; 8],
            ),
            // Every bit set, which under the default `/Decode` marks nothing.
            stream_object(
                7,
                "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true",
                b"\xf0\xf0",
            ),
        ],
    ));

    assert!(about(&raster, 5, 30, GREEN), "top row, first cell");
    assert!(about(&raster, 35, 5, GREEN), "bottom row, fourth cell");
}

/// A `/Mask` stream that is not an image mask is reported, not guessed at.
///
/// Table 87 and §8.9.6.3 both say the entry holds an image mask, and §8.9.6.2 defines that as
/// an image `XObject` whose `/ImageMask` entry is true. A one-bit greyscale image is not one,
/// and the two readings available for it are opposite — §8.9.6.2's, where a zero sample
/// marks, and §11.6.5.2's, where luminosity is opacity and white marks. `issue6621.pdf`
/// writes exactly this and the first reading blanked its court seal, so nothing is chosen:
/// the base image is drawn whole and the omission is named.
#[test]
fn a_mask_that_is_not_an_image_mask_is_reported() {
    let interpretation = interpret(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R",
        &rgb_image([GREEN; 8]),
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 \
             /ColorSpace /DeviceGray /BitsPerComponent 1",
            PATTERN,
        )],
    ));

    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("not an image mask"),
        "a /Mask this cannot read must say so: {reported}"
    );
}

/// A colour-key array of the wrong length is reported rather than applied to what fits.
///
/// §8.9.6.4 fixes the length at twice the number of components, and the failure mode this
/// guards is quiet: four entries against a three-component image would mask on red and green
/// alone and paint a colour the file asked to hide.
#[test]
fn a_colour_key_array_of_the_wrong_length_is_reported() {
    let interpretation = interpret(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask [255 255 0 0]",
        &rgb_image([RED; 8]),
        &[],
    ));

    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("4 entries against a 3-component image"),
        "a malformed colour key must say so: {reported}"
    );
}
