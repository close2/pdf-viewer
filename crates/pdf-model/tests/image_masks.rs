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
    clippy::panic,
    reason = "a test helper that cannot start the sandbox must fail loudly"
)]
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

/// [`page_with_image`] with a `/ColorSpace` subdictionary in the page's resources.
///
/// The one thing §8.6.5.6's default colour space mechanism needs to bite, and the only
/// reason this builder exists beside the one above.
fn page_with_image_under_defaults(
    defaults: &str,
    dict: &str,
    data: &[u8],
    extra: &[Vec<u8>],
) -> Vec<u8> {
    let content = "1 0 0 rg 40 0 0 40 0 0 cm /Im Do";
    let mut objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
             /Resources << /XObject << /Im 5 0 R >> /ColorSpace << {defaults} >> >> \
             /Contents 4 0 R >>\nendobj\n"
        )
        .into_bytes(),
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
    let interpretation = interpret(bytes);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    rasterise(interpretation)
}

/// Rasterises what an interpretation drew, at one pixel per unit.
///
/// Split out of [`render`] because a fixture whose whole subject is a refusal draws *and*
/// reports, so the two halves of its answer have to be read together — [`render`]'s assertion
/// that nothing was reported is exactly what such a test is checking the opposite of.
fn rasterise(interpretation: pdf_model::Interpretation) -> pdf_render::Raster {
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
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

/// The current colour a stencil paints may be a *pattern*, and then it is the pattern.
///
/// §8.9.6.2 says what a stencil does — "determines which areas of the page to paint with the
/// current colour" — and §8.7.2 says what the current colour may be:
///
/// > All patterns shall be treated as colours; a Pattern colour space shall be established
/// > with the CS or cs operator just like other colour spaces, and a particular pattern shall
/// > be installed as the current colour with the SCN or scn operator
///
/// A pattern is not a colour any image sample can carry, so the two are recomposed: the
/// stencil becomes a §11.5.2 alpha soft mask and the pattern fills the image's unit square
/// through it. `issue13372.pdf` is the corpus witness — a CCITT stencil over an axial shading
/// — and this reader drew **nothing** for it, silently, because `image::decode` was handed a
/// fill colour a pattern never sets. ADR 0151.
///
/// The gradient is what makes this discriminating: a reader that painted the stencil in some
/// single colour would satisfy every "is it marked" assertion and fail the last two.
#[test]
fn a_stencil_whose_current_colour_is_a_pattern_is_painted_with_the_pattern() {
    let content = "/Pattern cs /P0 scn 40 0 0 40 0 0 cm /Im Do";
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> /Pattern << /P0 6 0 R >> >> \
          /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        stream_object(
            5,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true",
            PATTERN,
        ),
        b"6 0 obj\n<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB \
          /Coords [0 0 40 0] /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] \
          /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >> >>\nendobj\n"
            .to_vec(),
    ];
    let raster = render(assemble(&objects));

    // The cells the stencil marks, and the ones it does not, are unchanged by the pattern.
    assert!(
        pixel(&raster, 5, 30)[3] > 128,
        "top row, first cell, marked"
    );
    assert!(
        pixel(&raster, 25, 30)[3] < 32,
        "top row, third cell, unmarked"
    );
    assert!(
        pixel(&raster, 35, 5)[3] < 32,
        "bottom row, fourth cell, unmarked"
    );

    // And what marks them is the *gradient*: red at the left edge, well into blue three
    // cells along, where a solid colour would give the same pixel twice.
    let left = pixel(&raster, 5, 5);
    let right = pixel(&raster, 25, 5);
    assert!(
        left[0] > 200 && left[2] < 64,
        "the left of the shading is its first colour, got {left:?}"
    );
    assert!(
        right[2] > left[2] + 64,
        "three cells along the shading has run towards its second colour: {left:?} then \
         {right:?}"
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

/// A sixteen-bit colour key separates two samples the eight-bit raster cannot, because the
/// test is on the samples and the raster is downstream of it.
///
/// §8.9.6.4 bounds its integers by the depth the samples arrive in — "[e]ach integer shall be
/// in the range 0 to 2 BitsPerComponent  - 1, representing colour values before decoding with
/// the Decode array" — and `unpack` applies them there, before any conversion. So the domain
/// the producer wrote its range in survives to the comparison whatever the raster holds
/// afterwards, and this fixture is the measurement of that: the second and third samples are
/// `0x8000` and `0x8001`, one unit apart in sixteen bits and **the same byte** in eight, with
/// a range naming the first of them alone.
///
/// The two assertions therefore say different things. The masked pair says the comparison ran
/// at sixteen bits: at eight, `0x8000` and `0x8001` are one value and the range would have
/// taken both cells or neither. The unmasked pair is the eight-bit raster's own cost, measured
/// rather than assumed — the same two samples come out as one colour, so the precision this
/// clause needs is precisely the precision the raster does not carry, and the reason the
/// `JPXDecode` arm of this clause stays owed (ADR 1121) is that there the scaling happens in
/// the decoder, before `unpack` ever sees a sample.
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §8.9.6.4 verbatim, and a quotation is not marked up"
)]
#[test]
fn a_sixteen_bit_colour_key_separates_samples_the_eight_bit_raster_cannot() {
    // 0x0000, 0x8000, 0x8001, 0xFFFF — a four-step grey ramp, most significant byte first.
    let samples: &[u8] = &[0x00, 0x00, 0x80, 0x00, 0x80, 0x01, 0xFF, 0xFF];
    let dict = "/Width 4 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 16";

    let masked = render(page_with_image(
        &format!("{dict} /Mask [32768 32768]"),
        samples,
        &[],
    ));
    assert!(
        cut_out(&masked, 15, 20),
        "the sample the range names is not painted"
    );
    assert!(
        !cut_out(&masked, 25, 20),
        "the sample one unit above it is painted, which eight bits could not have told apart"
    );

    let plain = render(page_with_image(dict, samples, &[]));
    let [second, ..] = pixel(&plain, 15, 20);
    let [third, ..] = pixel(&plain, 25, 20);
    assert_eq!(
        second, third,
        "and the raster holds one colour for both, which is what eight bits cost"
    );
}

/// The one sample value [`JPX_ONE_COMPONENT`] carries, in every pixel.
const JPX_SAMPLE: u8 = 200;

/// An 8×8 one-component JPEG 2000 codestream, every pixel [`JPX_SAMPLE`].
///
/// A bare codestream — SOC, SIZ, COD, QCD, SOT, SOD, EOC — with no JP2 boxes, so the dictionary
/// is the only thing that can say what a sample means. Its SIZ states one component of eight
/// unsigned bits and its COD the reversible 5/3 wavelet, so it is lossless and the value above
/// comes back exactly. The same bytes `tests/jpx_decode_array.rs` carries, generated rather than
/// written because a JPEG 2000 codestream cannot be written by hand legibly:
///
/// ```sh
/// python3 -c "import numpy as np; np.full(64, 200, np.uint8).tofile('gray.raw')"
/// opj_compress -i gray.raw -o gray.j2k -F 8,8,1,8,u -n 1 -r 1
/// ```
const JPX_ONE_COMPONENT: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00, 0x01, 0xff, 0x93, 0xcf,
    0xb4, 0x48, 0x14, 0x00, 0x5c, 0xa3, 0x65, 0x5d, 0xb0, 0x00, 0x03, 0x09, 0x08, 0xd5, 0x0a, 0x18,
    0x48, 0x4b, 0xff, 0x7f, 0xff, 0xd9,
];

/// An 8×8 one-component JPEG 2000 codestream whose `SIZ` states **twelve** unsigned bits.
///
/// Every sample is [`JPX_TWELVE_BIT_SAMPLE`], which is what makes the depth measurable rather
/// than merely declared: the value sits in the middle of a twelve-bit domain and nowhere near
/// the ends, so a range stated in twelve bits and the same range stated in eight pick it out
/// differently. Generated the same way as [`JPX_ONE_COMPONENT`] — lossless, reversible 5/3, so
/// the value comes back exactly — with the `COM` comment marker removed so that no encoder
/// version is baked in, and the samples written **big-endian**, which is the order
/// `opj_compress` reads a raw file in:
///
/// ```sh
/// python3 -c "import struct; open('g.raw','wb').write(struct.pack('>64H', *([3000]*64)))"
/// opj_compress -i g.raw -o g.j2k -F 8,8,1,12,u -n 1 -r 1
/// ```
const JPX_TWELVE_BIT: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x0b, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x60, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x01, 0xff, 0x93, 0xc7,
    0xec, 0x30, 0x14, 0x00, 0x5c, 0xaf, 0x82, 0x3d, 0x48, 0x00, 0x01, 0x84, 0x84, 0x6a, 0x85, 0x0c,
    0x24, 0x25, 0xf6, 0x71, 0x60, 0x00, 0x03, 0x09, 0x09, 0x7f, 0xff, 0xd9,
];

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

/// §8.9.6.4 over an `Indexed` space ranges over the *index*, not over the colour it selects.
///
/// The clause states one range per component of "the image's colour space", and §8.6.6.3 makes
/// an Indexed space's one component an index into its table: "a PDF reader shall treat each
/// sample value as an index into the colour table and shall use the colour value it finds
/// there", and "the PDF file can now specify colours as single-component values in the range 0
/// to 255". So a four-entry palette takes a two-integer `/Mask`, and `[2 2]` names table entry
/// 2 whatever colour stands there.
///
/// **The fixture discriminates between the two readings rather than merely exercising one.**
/// Entry 0 is `(2, 200, 200)`, whose *red* component is 2: a reader that tested the base space's
/// components instead of the index would mask the first cell and paint the third, which is the
/// exact opposite of what this asserts. `issue15629.pdf` is the corpus witness for the
/// construction — `[/Indexed /DeviceRGB 255 …]` under `/Mask [251 251]` — and it could not
/// distinguish the two readings, because a three-integer range over a one-component space is
/// refused for its length before either reading is reached.
#[test]
fn a_colour_key_over_an_indexed_space_ranges_over_the_index() {
    // `02C8C8` `00FF00` `0000FF` `FFFFFF`: entry 0's red is 2, which is inside the range below
    // if the range is misread as naming base-space components.
    let raster = render(page_with_image(
        "/Width 4 /Height 2 /BitsPerComponent 8 \
         /ColorSpace [/Indexed /DeviceRGB 3 <02C8C800FF000000FFFFFFFF>] /Mask [2 2]",
        &[0, 1, 2, 3, 3, 2, 1, 0],
        &[],
    ));

    assert!(cut_out(&raster, 25, 30), "top row, third cell, index 2");
    assert!(cut_out(&raster, 15, 5), "bottom row, second cell, index 2");

    assert!(
        about(&raster, 5, 30, [2, 200, 200]),
        "index 0 is outside the range and is painted, red component of 2 notwithstanding"
    );
    assert!(
        about(&raster, 15, 30, GREEN),
        "top row, second cell, index 1"
    );
    assert!(
        about(&raster, 35, 30, [255, 255, 255]),
        "top row, fourth cell, index 3"
    );
}

/// §8.9.6.4 over an eight-bit JPEG 2000 image, whose domain its own data states.
///
/// Table 87 withdraws `/BitsPerComponent` for this filter — "this entry is optional and shall be
/// ignored if present. The bit depth is determined by the PDF processor in the process of
/// decoding the JPEG 2000 image" — and §7.4.9 says where it is determined from: "These
/// packagings contain all the information needed to properly interpret the image data, including
/// the colour space, bits per component, and image dimensions." [`JPX_ONE_COMPONENT`]'s `SIZ`
/// states eight unsigned bits, which is the domain its samples reach this crate in, so the
/// clause's test is exact and the range is applied.
///
/// **The dictionary states `/BitsPerComponent 4`, which is the half of Table 87 this asserts.**
/// A reader that believed the entry would bound these integers by 15 and refuse `[190 210]` as
/// out of range; ignoring it, as the sentence requires, leaves the sample of 200 inside the
/// range and the picture gone. The control underneath is the same image under a range its
/// sample misses, without which "the mask was applied" and "the image never drew" are the
/// same observation.
#[test]
fn a_colour_key_over_an_eight_bit_jpeg_2000_image_is_applied() {
    sandbox_or_panic();

    let masked = interpret(page_with_image(
        "/Width 8 /Height 8 /Filter /JPXDecode /ColorSpace /DeviceGray /BitsPerComponent 4 \
         /Mask [190 210]",
        JPX_ONE_COMPONENT,
        &[],
    ));
    assert!(
        masked.is_complete(),
        "an eight-bit codestream states the domain the ranges are in: {:?}",
        masked.unsupported
    );
    assert!(
        cut_out(&rasterise(masked), 20, 20),
        "the sample of 200 is inside 190 to 210 and shall not be painted"
    );

    let painted = interpret(page_with_image(
        "/Width 8 /Height 8 /Filter /JPXDecode /ColorSpace /DeviceGray /Mask [0 100]",
        JPX_ONE_COMPONENT,
        &[],
    ));
    assert!(painted.is_complete(), "{:?}", painted.unsupported);
    assert!(
        about(
            &rasterise(painted),
            20,
            20,
            [JPX_SAMPLE, JPX_SAMPLE, JPX_SAMPLE]
        ),
        "a sample of 200 is outside 0 to 100 and is painted"
    );
}

/// §8.9.6.4 over a JPEG 2000 image the dictionary makes `Indexed`: the ranges name table entries.
///
/// §7.4.9 gives `/ColorSpace` precedence — "the colour space specifications in the JPEG 2000
/// data shall be ignored" — so the codestream's samples are indices into this dictionary's
/// table, and the confined decoder hands them back unscaled because an index stretched to eight
/// bits is a different index. §8.6.6.3 caps `hival` at 255, so the domain is the table's whatever
/// precision the codestream declares, and the range is applied with no depth to check.
///
/// The palette makes entry *i* the colour `(i, 255 − i, 128)`, so the control below is painted in
/// a colour only the lookup can produce: a reader that skipped the table would draw grey.
#[test]
fn a_colour_key_over_an_indexed_jpeg_2000_image_names_table_entries() {
    sandbox_or_panic();

    let palette = (0..=255).fold(String::new(), |mut palette, entry: u16| {
        let entry = u8::try_from(entry).unwrap_or(0);
        let _ = write!(palette, "{entry:02x}{:02x}80", u8::MAX - entry);
        palette
    });
    let space = format!("/ColorSpace [/Indexed /DeviceRGB 255 <{palette}>]");

    let masked = interpret(page_with_image(
        &format!("/Width 8 /Height 8 /Filter /JPXDecode {space} /Mask [190 210]"),
        JPX_ONE_COMPONENT,
        &[],
    ));
    assert!(masked.is_complete(), "{:?}", masked.unsupported);
    assert!(
        cut_out(&rasterise(masked), 20, 20),
        "index 200 is inside 190 to 210 and shall not be painted"
    );

    let painted = interpret(page_with_image(
        &format!("/Width 8 /Height 8 /Filter /JPXDecode {space} /Mask [0 100]"),
        JPX_ONE_COMPONENT,
        &[],
    ));
    assert!(painted.is_complete(), "{:?}", painted.unsupported);
    assert!(
        about(&rasterise(painted), 20, 20, [200, 55, 128]),
        "index 200 is outside 0 to 100 and is painted through the table"
    );
}

/// The one sample value [`JPX_TWELVE_BIT`] carries, in every pixel, in its own twelve bits.
const JPX_TWELVE_BIT_SAMPLE: u32 = 3000;

/// The same sample stretched to eight bits, which is what this tree delivered before ADR 1242.
///
/// `round(3000 ÷ 4095 × 255)`, and a colour key naming it must now leave the image painted.
const JPX_TWELVE_BIT_STRETCHED: u32 = 187;

/// An 8x8 one-component JPEG 2000 codestream whose `SIZ` states **sixteen** unsigned bits.
///
/// Every sample is 65535, the top of that domain and the widest sample this tree carries, which
/// is the one value a `2^n - 1` computed one bit too narrow would answer short of. Generated
/// as [`JPX_TWELVE_BIT`] was:
///
/// ```sh
/// python3 -c "import struct; open('g.raw','wb').write(struct.pack('>64H', *([65535]*64)))"
/// opj_compress -i g.raw -o g.j2k -F 8,8,1,16,u -n 1 -r 1
/// ```
const JPX_SIXTEEN_BIT: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x0f, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x80, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17, 0x00, 0x01, 0xff, 0x93, 0xcf,
    0xfc, 0x30, 0x14, 0x14, 0x00, 0x5c, 0xaf, 0x87, 0xff, 0xd9,
];

/// The widest domain this tree carries, at its own top sample.
///
/// §8.9.6.4 bounds its integers by "0 to 2 `BitsPerComponent` - 1", and at sixteen bits that is
/// 65535 - a number a shift one bit too narrow answers 65534 for, which no other fixture here
/// would catch. The first range is that sample exactly and masks; the second stops one below it
/// and does not. ADR 1242.
#[test]
fn a_sixteen_bit_jpeg_2000_colour_key_reaches_the_top_of_its_domain() {
    sandbox_or_panic();

    let keyed = |range: &str| {
        interpret(page_with_image(
            &format!(
                "/Width 8 /Height 8 /Filter /JPXDecode /ColorSpace /DeviceGray /Mask [{range}]"
            ),
            JPX_SIXTEEN_BIT,
            &[],
        ))
    };

    let top = keyed("65535 65535");
    assert!(top.is_complete(), "{:?}", top.unsupported);
    assert!(
        cut_out(&rasterise(top), 20, 20),
        "the sample is the largest sixteen bits can carry and a range of exactly that holds it"
    );

    let below = keyed("0 65534");
    assert!(below.is_complete(), "{:?}", below.unsupported);
    assert!(
        !cut_out(&rasterise(below), 20, 20),
        "and a range stopping one short of it does not"
    );
}

/// A twelve-bit codestream's colour key is compared in twelve bits, not in eight.
///
/// §8.9.6.4 bounds each integer by "0 to 2 `BitsPerComponent` - 1", and for this filter that
/// number is the codestream's: Table 87 makes the depth "determined by the PDF processor in
/// the process of decoding the JPEG 2000 image", and what this processor determines is what
/// the codestream states, so that the integers a file wrote are compared in the domain it
/// wrote them in. [`JPX_TWELVE_BIT`]'s `SIZ` states twelve unsigned bits and every sample is
/// [`JPX_TWELVE_BIT_SAMPLE`] (ADR 1242).
///
/// Three ranges, each the clause applied to that one number, and each discriminating against
/// the eight-bit narrowing this replaced in a direction of its own:
///
/// - **The sample's own value**, a range of one. It masks, and its integers are above 255, so
///   they were refused outright as outside the domain.
/// - **[`JPX_TWELVE_BIT_STRETCHED`]**, the value the eight-bit hand-off used to deliver. It
///   masked the whole image then and it masks nothing now, which is the narrowing measured at
///   the pixel rather than argued about.
/// - **`[0 255]`**, the bottom sixteenth of a twelve-bit domain, which the sample is outside.
#[test]
fn a_colour_key_over_a_twelve_bit_jpeg_2000_image_is_compared_in_twelve_bits() {
    sandbox_or_panic();

    let keyed = |range: String| {
        interpret(page_with_image(
            &format!(
                "/Width 8 /Height 8 /Filter /JPXDecode /ColorSpace /DeviceGray /Mask [{range}]"
            ),
            JPX_TWELVE_BIT,
            &[],
        ))
    };

    let inside = keyed(format!("{JPX_TWELVE_BIT_SAMPLE} {JPX_TWELVE_BIT_SAMPLE}"));
    assert!(inside.is_complete(), "{:?}", inside.unsupported);
    assert!(
        cut_out(&rasterise(inside), 20, 20),
        "the sample is inside a range of itself, and that range is one this domain has"
    );

    let stretched = keyed(format!(
        "{JPX_TWELVE_BIT_STRETCHED} {JPX_TWELVE_BIT_STRETCHED}"
    ));
    assert!(stretched.is_complete(), "{:?}", stretched.unsupported);
    assert!(
        !cut_out(&rasterise(stretched), 20, 20),
        "the eight-bit stretch of the sample is not the sample, so nothing is masked"
    );

    let below = keyed("0 255".to_owned());
    assert!(below.is_complete(), "{:?}", below.unsupported);
    assert!(
        !cut_out(&rasterise(below), 20, 20),
        "0 to 255 is the bottom sixteenth of twelve bits and the sample is above it"
    );

    let applied = interpret(page_with_image(
        "/Width 8 /Height 8 /Filter /JPXDecode /ColorSpace /DeviceGray /Mask [0 255]",
        JPX_ONE_COMPONENT,
        &[],
    ));
    assert!(
        applied.is_complete(),
        "eight unsigned bits is the domain the samples arrive in: {:?}",
        applied.unsupported
    );
    assert!(
        cut_out(&rasterise(applied), 20, 20),
        "0 to 255 covers every eight-bit sample, so an applied range leaves nothing painted"
    );
}

/// Skips nothing and fails loudly where the confined image decoder cannot start.
fn sandbox_or_panic() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }
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

/// §8.9.6.3 with §8.9.6.2: a stencil finer than the grid combined eagerly for an `/SMask` is
/// still combined, because for a `/Mask` there is no second construction to prefer.
///
/// The mixed-raster shape real scans are written in: a colour layer a few hundred samples
/// across under a full-page bilevel stencil, whose refinement is tens of millions of samples.
/// Until session 615 one number decided both "is combining eagerly worth it" and "can this be
/// built at all", and a stencil can never take the other route — Table 87 forbids an
/// `/ImageMask` a colour space of its own, and the device-scale route needs `DeviceGray`. So the
/// pair was refused, which draws the base image *unmasked*: for this
/// construction, a solid black page. Five documents of the crawl's 7000 were that (ADR 0451).
///
/// The fixture is the smallest pair past the preference: 8192 × 2049 is 16 785 408 samples,
/// 8 192 above it, against a 1 × 1 image. What is asserted is the picture and the silence —
/// `render` fails on any report at all.
#[test]
fn a_stencil_too_fine_to_prefer_combining_is_combined_anyway() {
    let (width, height) = (8192usize, 2049usize);
    assert!(
        width * height > PREFER_DEVICE_SCALE_ABOVE,
        "the fixture must exceed the grid that is combined without asking"
    );
    let raster = render(page_with_image(
        "/Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R",
        &BLUE,
        &[stream_object(
            6,
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} /ImageMask true"
            ),
            &quadrant_mask(width, height),
        )],
    ));

    // §8.9.6.2's default `/Decode [0 1]`: a sample of zero marks. `quadrant_mask` sets the
    // bits of the top-left quadrant, so that quadrant is the one masked out.
    assert!(cut_out(&raster, 5, 30), "top-left quadrant is masked out");
    assert!(
        about(&raster, 35, 30, BLUE),
        "top-right quadrant is painted"
    );
    assert!(about(&raster, 5, 5, BLUE), "the bottom half is painted");
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

/// §11.6.5.2: a soft mask finer than its image keeps its own detail.
///
/// Table 143 makes a mask's `/Width` and `/Height` "independent of" the parent's, with "[b]oth
/// images … mapped to the unit square in user space … regardless of whether the samples
/// coincide individually" — the same sentence §8.9.6.3 writes for an explicit mask, so the
/// same answer: combine on the finer grid. Until the fifteenth session a mask of any other
/// size was refused and reported instead, which drew `smaskdim.pdf`'s two bullets as squares
/// and `issue16263.pdf`'s overlines as black bars.
///
/// The mask's cells are 0 and 255 rather than intermediate values, because what this fixture
/// tests is *which* mask sample reaches which part of the page; the opacity between them is
/// [`a_matte_colour_is_undone_before_the_image_is_drawn`]'s subject.
#[test]
fn a_soft_mask_finer_than_its_image_keeps_its_own_resolution() {
    let raster = render(page_with_image(
        "/Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
        &BLUE,
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 \
             /ColorSpace /DeviceGray /BitsPerComponent 8",
            // The same asymmetric shape as `PATTERN`, one byte a sample and opaque where the
            // stencil marks: `255 0 0 0` over `255 255 255 0`.
            &[255, 0, 0, 0, 255, 255, 255, 0],
        )],
    ));

    assert!(about(&raster, 5, 30, BLUE), "top row, first cell, opaque");
    assert!(cut_out(&raster, 25, 30), "top row, third cell, transparent");
    assert!(about(&raster, 5, 5, BLUE), "bottom row, first cell, opaque");
    assert!(
        cut_out(&raster, 35, 5),
        "bottom row, fourth cell, transparent"
    );
}

/// §11.6.5.2: pre-blended image data is unblended before it is drawn.
///
/// > 𝑐 ′ = 𝑚 + 𝛼 × (𝑐 - 𝑚)
///
/// The fixture states that equation and asks for `c` back: a matte of black, a mask sample of
/// 128, and an image sample of 128 in the red channel — which is what a producer would write
/// for a half-transparent *full* red. Draw it without inverting and the red comes out half
/// strength, which is the dark fringe `issue13931.pdf`'s red seal had against three renderers
/// that undo it.
///
/// The assertion is on the raster's own colour channel rather than through [`about`], because
/// a half-transparent pixel is exactly what this is about and `about` asks for an opaque one.
#[test]
fn a_matte_colour_is_undone_before_the_image_is_drawn() {
    let raster = render(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
        &rgb_image([[128, 0, 0]; 8]),
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Matte [0 0 0]",
            &[128; 8],
        )],
    ));

    let [red, green, blue, alpha] = pixel(&raster, 20, 20);
    assert!(
        (100..=160).contains(&alpha),
        "the mask's own sample is the opacity: {alpha}"
    );
    assert!(
        red > 240 && green < 16 && blue < 16,
        "128 pre-blended with black at α = 128/255 is full red, not {red},{green},{blue}"
    );
}

/// §11.6.5.2: a soft mask's samples are mask values, so §8.6.5.6's defaults cannot remap them.
///
/// Table 143 fixes the mask's space at one word — "Required; shall be `DeviceGray`" — and
/// §8.6.5.6 is what would otherwise redirect that name: "[i]f such an entry is present, its
/// value shall be used as the colour space for the operation currently being performed". The
/// operation here is not a painting of colour. §8.6.5.6 says as much in its own arithmetic —
/// "[c]olour values in the original device colour space shall be passed unchanged to the
/// default colour space" — so a remap changes which colour a value denotes and never the
/// value, and §11.6.5.2 uses the value.
///
/// The fixture states a linear `/DefaultGray`, which is the widest gap the mechanism can open
/// in one number: a sample of 128 read as a *colour* is linear light, and comes back as sRGB's
/// encoding of it, 188 of 255. Read as this clause's mask value it is 128. Before ADR 1054 the
/// mask went through the parent's resources and this page drew at the wrong opacity, which
/// ADR 1008 found and priced without fixing.
#[test]
fn a_soft_masks_samples_are_not_remapped_by_a_default_colour_space() {
    let raster = render(page_with_image_under_defaults(
        "/DefaultGray [/CalGray << /WhitePoint [0.9505 1.0 1.089] /Gamma 1 >>]",
        "/Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
        &[255, 0, 0],
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceGray /BitsPerComponent 8",
            &[128],
        )],
    ));

    let [_, _, _, alpha] = pixel(&raster, 20, 20);
    assert!(
        (120..=136).contains(&alpha),
        "the mask's own sample is the opacity, not a colour a default space remapped: {alpha}"
    );
}

/// An `/SMask` that is an image mask is reported rather than read as an opacity.
///
/// Table 143 says `/ImageMask` "[s]hall be false or absent" in a soft-mask image, and the
/// reason to check rather than trust it is what a stencil decodes to: the current colour where
/// its bits mark and nothing where they do not, with no grey level anywhere. Reading its first
/// component as the opacity would make every such image *fully transparent* — a page silently
/// missing its picture, which is the worst of the outcomes available.
///
/// The fixture's stencil carries a one-component colour space as well, which Table 87 says an
/// image mask has no use for. That is deliberate: without it the mask is caught by Table 143's
/// `DeviceGray` requirement instead — an absent colour space is not one — and the test would
/// pass with the `/ImageMask` rule deleted.
#[test]
fn a_soft_mask_that_is_a_stencil_is_reported() {
    let interpretation = interpret(page_with_image(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
        &rgb_image([GREEN; 8]),
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true \
             /ColorSpace /DeviceGray /BitsPerComponent 1",
            PATTERN,
        )],
    ));

    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("carries no opacity"),
        "an /SMask that is a stencil must say so: {reported}"
    );
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

/// The grid a mask and its image would share, in samples, above which the device combines them.
///
/// `crate::image`'s own `PREFER_DEVICE_SCALE_ABOVE`, restated here because a fixture has to
/// straddle it and a test that only knew "large" would stop testing the boundary the day it
/// moved.
const PREFER_DEVICE_SCALE_ABOVE: usize = 1 << 24;

/// One `DeviceGray` soft mask of `width` × `height` one-bit samples, opaque in its top-left
/// quadrant and transparent everywhere else.
///
/// Asymmetric in both axes, for [`stencil`]'s reason: a quadrant distinguishes a correct read
/// from one mirrored either way, where a half would not.
fn quadrant_mask(width: usize, height: usize) -> Vec<u8> {
    let row_bytes = width.div_ceil(8);
    let mut data = vec![0u8; row_bytes * height];
    for row in data.chunks_exact_mut(row_bytes).take(height / 2) {
        row[..row_bytes / 2].fill(0xFF);
    }
    data
}

/// §11.6.5.2 with §10.7.4: a mask too large to combine on the finer grid is combined by the
/// device instead.
///
/// Table 143 makes a soft mask's grid "independent of" its image's, and
/// [`a_soft_mask_finer_than_its_image_keeps_its_own_resolution`] combines the two on the finer
/// of them — which discards nothing and costs the product of the two larger dimensions. That
/// product is what a document controls: `issue16263.pdf` gives a 2×2 image a 34862×4332 mask,
/// which is 151 million samples and 604 MB of RGBA, and until this test the mask was refused
/// by name and the image drawn opaque — black bars across a page of vector arithmetic.
///
/// §10.7.4 answers the question at *device* resolution, which the interpreter deliberately
/// does not know, so the display list carries the image and the mask separately and the
/// backend puts them together. The fixture is the smallest pair that reaches that route: 8192
/// × 2049 is 16 785 408 samples, 8 192 above the limit, against a 2 × 2 image.
///
/// What is asserted is the picture and the silence — the top-left quadrant of the page keeps
/// the image and the other three are cut out, with nothing reported.
#[test]
fn a_soft_mask_too_large_to_combine_is_placed_by_the_device() {
    let (width, height) = (8192usize, 2049usize);
    assert!(
        width * height > PREFER_DEVICE_SCALE_ABOVE,
        "the fixture must exceed the grid that is built eagerly"
    );
    let raster = render(page_with_image(
        "/Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
        &rgb_image([BLUE; 8])[..12],
        &[stream_object(
            6,
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
                 /ColorSpace /DeviceGray /BitsPerComponent 1"
            ),
            &quadrant_mask(width, height),
        )],
    ));

    assert!(
        about(&raster, 5, 30, BLUE),
        "top left, where the mask marks"
    );
    assert!(cut_out(&raster, 35, 30), "top right");
    assert!(cut_out(&raster, 5, 5), "bottom left");
    assert!(cut_out(&raster, 35, 5), "bottom right");
}

/// One display list, two magnifications, two grids for the same mask.
///
/// This is the property the vocabulary was added for, and it is not visible in any single
/// picture: `zooming_rasterises_again_without_interpreting_again` makes a display list
/// re-rasterisable at any zoom without being interpreted again, so a mask resolved *during*
/// interpretation would be frozen at whatever scale the first frame happened to use. Asking
/// the same `ImageSource` under two placements and getting two grids is what says it was not.
///
/// The grids are the device pixels the unit square covers (§10.7.4), so they are the
/// placements' own extents — 64 and 256 — rather than anything the file states.
#[test]
fn the_same_display_list_asks_for_the_mask_at_the_scale_it_is_drawn() {
    let (width, height) = (8192usize, 2049usize);
    let interpretation = interpret(page_with_image(
        "/Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
        &rgb_image([BLUE; 8])[..12],
        &[stream_object(
            6,
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
                 /ColorSpace /DeviceGray /BitsPerComponent 1"
            ),
            &quadrant_mask(width, height),
        )],
    ));
    assert!(
        interpretation.is_complete(),
        "a mask the device places is not a gap: {:?}",
        interpretation.unsupported
    );

    let source = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Image { image, .. } => Some(image),
            _ => None,
        })
        .expect("the page draws one image");
    assert!(
        matches!(source, pdf_render::ImageSource::AtDeviceScale(_)),
        "the mask travels to the backend rather than into the samples"
    );

    let small = source.at(pdf_render::Transform::scale(64.0, 64.0));
    let large = source.at(pdf_render::Transform::scale(256.0, 256.0));
    assert_eq!((small.width, small.height), (64, 64));
    assert_eq!((large.width, large.height), (256, 256));
    // And the picture is the same one at both: opaque in the top-left quadrant of the image,
    // which is its *first* rows, and transparent in the last column of the first row.
    let alpha = |image: &pdf_render::Image, x: u32, y: u32| {
        image.data[((y * image.width + x) * 4 + 3) as usize]
    };
    assert_eq!((alpha(&small, 1, 1), alpha(&large, 4, 4)), (255, 255));
    assert_eq!(
        (
            alpha(&small, 63, 1),
            alpha(&large, 255, 4),
            alpha(&small, 1, 63)
        ),
        (0, 0, 0)
    );
}

/// A stencil under the *graphics state's* soft mask is drawn through both, and says nothing.
///
/// §11.6.4.3, of an image's own `/SMask` and of the explicit or colour-key mask its `/Mask`
/// entry states:
///
/// > Either form of mask in the image dictionary shall override, for this image object only,
/// > the current soft mask in the graphics state.
///
/// Two forms, both of them entries of the image dictionary, and `/ImageMask` is neither:
/// it says the samples carry no *colour*, not that they carry an opacity. So the mask in force
/// stays in force and the two compose, which is what this test rasterises.
///
/// **It is here because three ledger rows said the opposite.** §8.9.6, §8.9.6.1 and §8.9.6.2
/// each named "a stencil under a graphics-state soft mask" among the family's residue, and
/// the refusal in `content::image` is narrower than that by one condition: it is a stencil
/// whose current colour is a *pattern*, where the recomposition of §8.9.6.2 needs the mask
/// slot the state is already using. An ordinary stencil needs no such slot, and nothing in
/// this tree had ever asked.
///
/// The fixture pairs the stencil with a luminosity group white over the left half of the page
/// and black over the right, so the mask cuts a cell the stencil marks. A reader that dropped
/// the state's mask paints that cell, and one that refused the pair paints nothing at all.
#[test]
fn a_stencil_under_the_graphics_states_soft_mask_is_drawn_through_both() {
    let content = "/GS gs 1 0 0 rg 40 0 0 40 0 0 cm /Im Do";
    let group = "0 g 0 0 40 40 re f 1 g 0 0 20 40 re f";
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> /ExtGState << /GS 6 0 R >> >> \
          /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        stream_object(
            5,
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true",
            PATTERN,
        ),
        b"6 0 obj\n<< /Type /ExtGState /SMask << /S /Luminosity /G 7 0 R >> >>\nendobj\n".to_vec(),
        stream_object(
            7,
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] \
             /Group << /S /Transparency /CS /DeviceGray >>",
            group.as_bytes(),
        ),
    ];
    // `render` asserts the page drew completely, which is half of what this test is for.
    let raster = render(assemble(&objects));

    // The bottom row marks its first three cells. The first two are under the white half of
    // the mask and survive; the third is under the black half and is cut out.
    assert!(marked(&raster, 5, 5), "bottom row, first cell, mask white");
    assert!(
        marked(&raster, 15, 5),
        "bottom row, second cell, mask white"
    );
    assert!(
        cut_out(&raster, 25, 5),
        "bottom row, third cell: the stencil marks it and the state's mask cuts it"
    );
    // And the cell the stencil leaves alone stays unmarked whatever the mask says.
    assert!(!marked(&raster, 35, 5), "bottom row, fourth cell, unmarked");
}

/// A one-page PDF whose only mark is a knockout group drawing one image twice, overlapping.
///
/// `image` is the image `XObject`'s dictionary beyond its size, `data` its samples, and
/// `extra` are the further objects it names, numbered from 7.
fn knockout_group_drawing(image: &str, data: &[u8], extra: &[Vec<u8>]) -> Vec<u8> {
    let form = b"20 0 0 20 5 5 cm /Im Do 1 0 0 1 0.5 0.5 cm /Im Do";
    let mut objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", b"1 0 0 rg /Fm Do"),
        stream_object(
            5,
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] \
             /Group << /S /Transparency /I true /K true >> \
             /Resources << /XObject << /Im 6 0 R >> >>",
            form,
        ),
        stream_object(6, &format!("/Type /XObject /Subtype /Image {image}"), data),
    ];
    objects.extend_from_slice(extra);
    assemble(&objects)
}

/// Every [`pdf_render::Command`] of an interpretation, groups flattened.
fn commands_of(interpretation: &pdf_model::Interpretation) -> Vec<pdf_render::Command> {
    fn walk(commands: &[pdf_render::Command], out: &mut Vec<pdf_render::Command>) {
        for command in commands {
            out.push(command.clone());
            if let pdf_render::Command::Group { commands, .. } = command {
                walk(commands, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(interpretation.display_list.commands(), &mut out);
    out
}

/// A §8.9.6.2 stencil under an `/SMask` of its own keeps the two apart, so §11.4.6's knockout
/// can state its shape.
///
/// The clause names the two quantities separately and says which is which. §11.6.4.2:
///
/// > For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted areas
/// > and 0.0 for masked areas.
///
/// §11.6.4.3 makes the soft mask the other one — "a soft mask ... shall override any explicit
/// or colour key mask" — and §11.3.7.2 keeps shape and opacity as two numbers rather than one.
/// One alpha channel holding their product answers for neither, which is why this element used
/// to be reported instead of drawn. It is kept apart on the way to the display list now, so
/// the knockout element arrives as a `Shaped` command whose *shape* is the stencil alone (ADR
/// 1218): a `Decoded` source on the file's own grid, beside an object that is a producer
/// because the pair is combined where the device scale is known.
///
/// The same fixture with Table 144's `/Matte` on the mask keeps the pair apart too. Table 144
/// counts the matte's numbers in the components of the parent image's `/ColorSpace`, and Table 87
/// does not permit a stencil one, so a stencil carries no pre-blending to undo and nothing makes
/// the pair one raster (ADR 1279). The report prefix below is the one
/// every knockout refusal prints, so the assertion excludes all of them rather than one sentence.
#[test]
fn a_stencil_under_its_own_soft_mask_states_its_shape_to_a_knockout() {
    // Four by two eight-bit grey samples, the opacity of each of the stencil's cells.
    let mask = stream_object(
        7,
        "/Type /XObject /Subtype /Image /Width 4 /Height 2 /BitsPerComponent 8 \
         /ColorSpace /DeviceGray",
        b"\x00\x40\x80\xff\xff\x80\x40\x00",
    );
    let stencil = "/Width 4 /Height 2 /ImageMask true /SMask 7 0 R";

    let kept = interpret(knockout_group_drawing(
        stencil,
        PATTERN,
        std::slice::from_ref(&mask),
    ));
    let reported = format!("{:?}", kept.unsupported);
    assert!(
        !reported.contains(REFUSED),
        "the pair is kept apart, so the shape is statable: {reported}"
    );
    let shaped: Vec<_> = commands_of(&kept)
        .into_iter()
        .filter_map(|command| match command {
            pdf_render::Command::Shaped { object, shape } => Some((*object, *shape)),
            _ => None,
        })
        .collect();
    assert_eq!(shaped.len(), 2, "two elements, each stating its shape");
    for (object, shape) in &shaped {
        let (
            pdf_render::Command::Image { image: object, .. },
            pdf_render::Command::Image { image: shape, .. },
        ) = (object, shape)
        else {
            panic!("both halves draw the image: {object:?}");
        };
        assert_eq!(
            object.sample_alpha(),
            pdf_render::SampleAlpha::Both,
            "the object's alpha is the stencil's shape times the mask's opacity"
        );
        assert_eq!(
            shape.sample_alpha(),
            pdf_render::SampleAlpha::Shape,
            "and the shape beside it is the stencil alone"
        );
        assert!(
            matches!(shape, pdf_render::ImageSource::Decoded(_)),
            "the stencil is on the grid the file states it on"
        );
    }

    // A `/Matte` on a stencil's mask has no colour component to have been blended into, so it
    // is no reason to multiply the pair: the shape is still the stencil alone.
    let matted = stream_object(
        7,
        "/Type /XObject /Subtype /Image /Width 4 /Height 2 /BitsPerComponent 8 \
         /ColorSpace /DeviceGray /Matte [0]",
        b"\x00\x40\x80\xff\xff\x80\x40\x00",
    );
    let apart = interpret(knockout_group_drawing(stencil, PATTERN, &[matted]));
    assert!(
        !format!("{:?}", apart.unsupported).contains(REFUSED),
        "a matte on a stencil's mask leaves the pair apart: {:?}",
        apart.unsupported
    );
    assert_eq!(
        stated_shapes(&apart),
        2,
        "and each element states its shape"
    );
}

/// What every knockout refusal the interpreter raises begins with.
const REFUSED: &str = "knockout, and an element composites over another";

/// How many elements of a knockout group arrive as a `Shaped` command whose shape is an image
/// of `SampleAlpha::Shape` — the stencil alone.
fn stated_shapes(interpretation: &pdf_model::Interpretation) -> usize {
    commands_of(interpretation)
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                pdf_render::Command::Shaped { shape, .. }
                    if matches!(
                        &**shape,
                        pdf_render::Command::Image { image, .. }
                            if image.sample_alpha() == pdf_render::SampleAlpha::Shape
                    )
            )
        })
        .count()
}

/// A §11.6.5.2 soft mask behind an image codec is read at the device's grid like any other.
///
/// The clause's own routing is what this is about rather than the filter. §11.6.4.2 gives a
/// stencil its *shape* — "For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0
/// for painted areas and 0.0 for masked areas" — and §11.6.4.3 makes an `/SMask` its opacity, so
/// the two are kept apart on the way to the display list and combined where the device scale is
/// known (ADR 1218). That route reads the mask's samples at a grid the device chooses, which for
/// a codestream means decoding it: a `JPXDecode` sample has no position until it is.
///
/// So the mask is decoded **once**, into an eight-bit grey plane `MaskCache` holds under the
/// `/SMask`'s own `ObjectId`, under the same bound that sent the pair down this route (ADR 1232).
/// The assertion is the stencil test's above: the shape arrives as a `Decoded` source of its own,
/// which is only possible if the pair was never multiplied together.
///
/// The control is the same page with the mask's filter taken off, so that a reader which had
/// stopped keeping any pair apart would fail both halves rather than one.
#[test]
fn a_stencil_under_a_codec_carrying_soft_mask_still_states_its_shape() {
    sandbox_or_panic();

    let mask = stream_object(
        7,
        "/Type /XObject /Subtype /Image /Width 8 /Height 8 /Filter /JPXDecode \
         /ColorSpace /DeviceGray",
        JPX_ONE_COMPONENT,
    );
    let stencil = "/Width 4 /Height 2 /ImageMask true /SMask 7 0 R";
    let kept = interpret(knockout_group_drawing(
        stencil,
        PATTERN,
        std::slice::from_ref(&mask),
    ));
    let reported = format!("{:?}", kept.unsupported);
    assert!(
        !reported.contains(REFUSED),
        "a codec-carrying mask is decoded once and read at the device's grid: {reported}"
    );
    let shapes: Vec<_> = commands_of(&kept)
        .into_iter()
        .filter_map(|command| match command {
            pdf_render::Command::Shaped { shape, .. } => Some(*shape),
            _ => None,
        })
        .collect();
    assert_eq!(shapes.len(), 2, "two elements, each stating its shape");
    for shape in &shapes {
        let pdf_render::Command::Image { image: shape, .. } = shape else {
            panic!("the shape half draws the image: {shape:?}");
        };
        assert_eq!(
            shape.sample_alpha(),
            pdf_render::SampleAlpha::Shape,
            "the shape beside the object is the stencil alone"
        );
    }

    // The control: the same mask with no codec at all, which took this route before and still
    // does. A reader that had stopped keeping any pair apart fails here too.
    let unfiltered = stream_object(
        7,
        "/Type /XObject /Subtype /Image /Width 4 /Height 2 /BitsPerComponent 8 \
         /ColorSpace /DeviceGray",
        b"\x00\x40\x80\xff\xff\x80\x40\x00",
    );
    let plain = interpret(knockout_group_drawing(stencil, PATTERN, &[unfiltered]));
    assert!(
        !format!("{:?}", plain.unsupported).contains(REFUSED),
        "the control keeps its pair apart: {:?}",
        plain.unsupported
    );
    assert_eq!(
        stated_shapes(&plain),
        2,
        "and each element states its shape"
    );
}

/// A soft mask in a one-component space Table 143 does not permit supplies its samples, and
/// says so.
///
/// Table 143 makes `/ColorSpace` "Required; shall be DeviceGray", so a mask stating `CalGray`
/// is a non-conforming file and the question is what a reader does with it. §11.6.5.2 reads one
/// number per sample as the image's opacity, and a colour space says which colour a value
/// denotes rather than what the value is — the reading ADR 1054 already took for §8.6.5.6's
/// defaults one key along — so the number the file states is the opacity and the departure is
/// reported beside the drawing.
///
/// The fixture's gamma is what discriminates: a `CalGray` of `/Gamma 1` states linear light,
/// which a sample of 128 carries to the device at 188 of 255, so a reader that converted the
/// mask to a colour and took a channel would make this image half again as opaque as the file
/// says. Calibrated by handing the mask on under its own space, which lands the alpha at 188
/// and fails the second assertion.
#[expect(
    clippy::doc_markdown,
    reason = "the sentence quotes Table 143 verbatim, and a quotation may not gain backticks"
)]
#[test]
fn a_soft_mask_in_a_calibrated_grey_states_its_samples_and_is_reported() {
    let fixture = || {
        page_with_image(
            "/Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R",
            &[255, 0, 0],
            &[stream_object(
                6,
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
                 /ColorSpace [/CalGray << /WhitePoint [0.9505 1.0 1.089] /Gamma 1 >>] \
                 /BitsPerComponent 8",
                &[128],
            )],
        )
    };

    let interpretation = interpret(fixture());
    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("Table 143 requires DeviceGray"),
        "the departure from Table 143 is named: {reported}"
    );

    let [_, _, _, alpha] = pixel(&rasterise(interpret(fixture())), 20, 20);
    assert!(
        (120..=136).contains(&alpha),
        "the mask's own sample is the opacity, not the colour its gamma denotes: {alpha}"
    );
}

/// §11.6.5.2: the pre-blending is undone in the image's own colour space, whatever it is.
///
/// > The preblending computation shall be done in the colour space specified by the parent
/// > image's ColorSpace entry. … If a colour conversion is required, inversion of the
/// > pre-blending shall precede the colour conversion.
///
/// The fixture is the smallest one where the two orders disagree: a `DeviceCMYK` image whose
/// samples state a quarter of cyan and a quarter of magenta, pre-blended with white paper at
/// α = 64/255, so the colour the producer meant is one unit of each — the ink cube's blue
/// corner. §10.4.2.4's conversion is multilinear in the four inks, so dividing the *raster*
/// by α instead lands two thirds of the way down a different chord and clamps two channels to
/// zero: 0,0,131 against the corner's 46,49,146. Calibrated by putting the inversion after the
/// conversion, which produces exactly that.
#[test]
fn a_matte_is_undone_in_the_images_own_space_before_it_is_converted() {
    let raster = render(page_with_image(
        "/Width 1 /Height 1 /ColorSpace /DeviceCMYK /BitsPerComponent 8 /SMask 6 0 R",
        &[64, 64, 0, 0],
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Matte [0 0 0 0]",
            &[64],
        )],
    ));

    let [red, green, blue, alpha] = pixel(&raster, 20, 20);
    assert!(
        (56..=72).contains(&alpha),
        "the mask's own sample is the opacity: {alpha}"
    );
    assert!(
        (38..=54).contains(&red) && (41..=57).contains(&green) && (138..=154).contains(&blue),
        "one unit each of cyan and magenta is the cube's blue corner, not {red},{green},{blue}"
    );
}

/// §11.6.5.2 and Table 144: an `Indexed` image's *table entries* carry the matte.
///
/// > If the image colour space is an Indexed space (see 8.6.6.3, "Indexed colour spaces"),
/// > the colour values in the colour table (not the index values themselves) shall be
/// > pre-blended.
///
/// So the `/Matte` states the base space's components — Table 144 counts them there — and the
/// inversion runs on the entry an index selects, never on the index. The fixture's first entry
/// is full red pre-blended with white paper at α = 64/255, which is `FF BF BF`; restored it is
/// red again. A reader that left the entry alone would draw that pink, and one that divided
/// the *index* would run off the end of a two-entry table and draw the paper.
#[test]
fn a_matte_on_an_indexed_image_is_undone_on_its_table_entry() {
    let raster = render(page_with_image(
        "/Width 1 /Height 1 /ColorSpace [/Indexed /DeviceRGB 1 <FFBFBF FFFFFF>] \
         /BitsPerComponent 8 /SMask 6 0 R",
        &[0],
        &[stream_object(
            6,
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Matte [1 1 1]",
            &[64],
        )],
    ));

    let [red, green, blue, alpha] = pixel(&raster, 20, 20);
    assert!(
        (56..=72).contains(&alpha),
        "the mask's own sample is the opacity: {alpha}"
    );
    assert!(
        red > 240 && green < 16 && blue < 16,
        "the entry restored against white paper is full red, not {red},{green},{blue}"
    );
}
