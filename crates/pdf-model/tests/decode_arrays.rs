//! What an image sample *means*: ISO 32000-2 §8.9.5.2's `/Decode` array.
//!
//! The clause is a linear map — `D min + x × (D max − D min) ÷ (2^n − 1)` — plus Table 88,
//! which says what the pair is when the dictionary states none. Until the twenty-fifth
//! session this tree implemented one point of that map: a first element above 0.5 reversed
//! the samples, and every other array was silently ignored inside a `partial` ledger row.
//!
//! # Why the fixtures are synthetic, when the corpus has 974 documents
//!
//! Because the corpus cannot reach the rule. Every `/Decode` array any of the 974 documents
//! writes is either Table 88's own default or its exact reversal — measured, by walking every
//! image dictionary in the corpus. The 140 image objects that state one: 115 `[1 0]` on a
//! stencil, 15 on `DeviceGray` either way round, 5 on an `Indexed` space either way round, 3
//! on `DeviceRGB` either way round, 2 `[1 0]` on a `Separation`. Not one states a *general*
//! pair. That is trap 8 in `doc/HANDOVER.md` exactly: a corpus finds what documents contain,
//! and the standard describes what a valid file may contain.
//!
//! So the general map, the clamp its closing sentence requires, and Table 88's two
//! non-obvious rows have nothing but this file defending them. The two rows are worth naming,
//! because both were wrong before it: a `Lab` image's default decode is `[0 100 a min a max
//! b min b max]`, and this tree scaled its lightness onto 0.0..=1.0 like every other space —
//! a page of Lab imagery would have rendered essentially black. An `Indexed` image's default
//! is `[0 2^n − 1]`, which the clause's NOTE 2 says exists so that "component values that
//! index a colour table are passed through unchanged", and computing it as `x ÷ 255 × 255`
//! rather than `x × 255 ÷ 255` sends sample 254 to 253.99998.
//!
//! The one non-default array the corpus *does* state and this tree ignored is the last test
//! here: `issue7406.pdf` inverts a JPEG with `[1 0 1 0 1 0]`, and the `DCTDecode` route never
//! looked at the entry at all, so its page one rendered in complementary colours against all
//! four reference renderers.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the fixtures are 40x40 \
              pages where no index can overflow"
)]

use std::fmt::Write as _;
use std::path::PathBuf;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF drawing one image over the whole 40×40 page.
fn page_with_image(dict: &str, data: &[u8]) -> Vec<u8> {
    let content = "40 0 0 40 0 0 cm /Im Do";
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        stream_object(5, &format!("/Type /XObject /Subtype /Image {dict}"), data),
    ];
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
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("a 40x40 page is a valid target");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the display list holds nothing the CPU backend refuses")
}

/// The pixel at the centre of cell `cell` of a 4×1 image drawn across the page.
fn cell(raster: &pdf_render::Raster, cell: u32) -> [u8; 4] {
    let x = cell * 10 + 5;
    let at = ((20 * raster.width + x) * 4) as usize;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// Asserts a channel is within one level of `expected`, which is what eight-bit rounding and
/// `tiny-skia`'s filtering between two equal neighbours can move it by.
#[track_caller]
fn about(actual: [u8; 4], expected: [u8; 3], what: &str) {
    for (index, want) in expected.iter().enumerate() {
        let got = i32::from(actual[index]);
        assert!(
            (got - i32::from(*want)).abs() <= 1,
            "{what}: channel {index} is {got}, expected about {want} (whole pixel {actual:?})"
        );
    }
}

/// A 4×1 eight-bit image, one component per cell.
fn grey_ramp(dict_extra: &str) -> pdf_render::Raster {
    render(page_with_image(
        &format!("/Width 4 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 {dict_extra}"),
        &[0, 64, 128, 255],
    ))
}

/// §8.9.5.2's formula, at a pair that is neither the default nor its reversal.
///
/// > Samples with a value of 0 shall be mapped to D min … those with intermediate values
/// > shall be mapped linearly between D min and D max
///
/// `[0.2 0.8]` compresses the ramp into the middle three fifths of the range: 0 → 0.2,
/// 64 → 0.35, 128 → 0.50, 255 → 0.8. Before the twenty-fifth session this array was read
/// only for the sign of its first element, so every one of these four cells came out as the
/// raw sample and nothing said so.
#[test]
fn a_general_decode_array_maps_the_samples_linearly_between_its_two_values() {
    let raster = grey_ramp("/Decode [0.2 0.8]");
    about(cell(&raster, 0), [51, 51, 51], "sample 0 is D min");
    about(
        cell(&raster, 1),
        [90, 90, 90],
        "sample 64 is 0.2 + 0.6 x 64/255",
    );
    about(
        cell(&raster, 2),
        [128, 128, 128],
        "sample 128 is the midpoint",
    );
    about(cell(&raster, 3), [204, 204, 204], "sample 255 is D max");
}

/// The default is Table 88's, which for `DeviceGray` is the identity on eight-bit channels.
#[test]
fn no_decode_array_leaves_an_eight_bit_grey_sample_alone() {
    let raster = grey_ramp("");
    about(cell(&raster, 0), [0, 0, 0], "sample 0");
    about(cell(&raster, 1), [64, 64, 64], "sample 64");
    about(cell(&raster, 2), [128, 128, 128], "sample 128");
    about(cell(&raster, 3), [255, 255, 255], "sample 255");
}

/// NOTE 3's inversion, which is the one form of the array the corpus states.
#[test]
fn a_decode_array_of_one_zero_inverts_a_grey_ramp() {
    let raster = grey_ramp("/Decode [1 0]");
    about(cell(&raster, 0), [255, 255, 255], "sample 0 is D min = 1");
    about(cell(&raster, 1), [191, 191, 191], "sample 64");
    about(cell(&raster, 2), [127, 127, 127], "sample 128");
    about(cell(&raster, 3), [0, 0, 0], "sample 255 is D max = 0");
}

/// §8.9.5.2's closing sentence, on a pair that leaves the component's range.
///
/// > If an output value is not permitted for a component, it shall be adjusted to the
/// > nearest allowed value.
///
/// `[-0.5 1.5]` sends sample 0 to −0.5 and sample 255 to 1.5, neither of which is a grey.
/// The clause says what to do with them, and it is not to wrap or to rescale: 0.0 and 1.0.
/// The two interior samples are unaffected by the clamp and are the check that it is a clamp
/// rather than a rescale — 64 maps to 0.0, on the boundary, and 128 to 0.5039.
#[test]
fn a_decoded_value_outside_the_components_range_is_clamped_to_it() {
    let raster = grey_ramp("/Decode [-0.5 1.5]");
    about(cell(&raster, 0), [0, 0, 0], "sample 0 decodes to -0.5");
    about(cell(&raster, 1), [0, 0, 0], "sample 64 decodes to 0.0");
    about(cell(&raster, 2), [128, 128, 128], "sample 128");
    about(
        cell(&raster, 3),
        [255, 255, 255],
        "sample 255 decodes to 1.5",
    );
}

/// §8.9.5.2 Table 88's `Lab` row, the only one whose default is not the unit interval.
///
/// > Lab | \[0 100 a min a max b min b max \] where a min , a max , b min , and b max
/// > correspond to the values in the Range array of the image's colour space
///
/// So a lightness sample of 255 is L = 100 — white — and one of 0 is L = 0. Scaling it onto
/// 0.0..=1.0 as every device space's default does makes L = 1 the brightest colour the image
/// can hold, which is black to within a level. That is what this tree did until Table 88 was
/// read: `to_rgb` has taken real `Lab` values since ADR 0012, and the unpacker was handing it
/// fractions.
#[test]
fn a_lab_images_default_decode_is_table_88s_rather_than_the_unit_interval() {
    // Four pixels, each three components: L then a then b. 128 in the chromatic axes is the
    // centre of `/Range`, so each cell is a neutral grey of its own lightness.
    let raster = render(page_with_image(
        "/Width 4 /Height 1 /BitsPerComponent 8 /ColorSpace \
         [/Lab << /WhitePoint [0.9642 1 0.8249] /Range [-100 100 -100 100] >>]",
        &[0, 128, 128, 128, 128, 128, 191, 128, 128, 255, 128, 128],
    ));
    about(cell(&raster, 0), [0, 0, 0], "L = 0 is black");
    about(cell(&raster, 3), [255, 255, 255], "L = 100 is white");
    // L = 50.2 is middle grey in CIE terms, which is around 0.46 of the way up sRGB's
    // non-linear ramp rather than half way — the number is the conversion's, not this
    // clause's, and it is here so that the test fails if the lightness stops being a
    // percentage at all.
    let middle = cell(&raster, 1)[0];
    assert!(
        (100..=140).contains(&middle),
        "L = 50.2 should be a middle grey, got {middle}"
    );
    assert!(
        cell(&raster, 2)[0] > middle,
        "L = 74.9 should be lighter than L = 50.2"
    );
}

/// Table 88's `Indexed` row and its NOTE 2, at the sample most likely to be misrounded.
///
/// The default pair is `[0 2^n − 1]`, so the map is the identity on indices — the clause says
/// it exists precisely so that "component values that index a colour table are passed through
/// unchanged". Written as `x ÷ (2^n − 1) × (2^n − 1)` in floating point it is *not* the
/// identity: sample 254 comes back as 253.99998, and every entry of the table would be
/// reachable only by the sample below it if the result were truncated rather than rounded.
///
/// The table here makes each index a distinct red, so an index off by one is a channel off
/// by one and the assertion is exact. Unlike the four tests above it, this one *passes*
/// against the code it replaced — that code reached the identity by never leaving the
/// integers. It is here because the general formula could lose it silently, and an index is
/// the one component where being off by one is not a rounding difference but a different
/// colour.
#[test]
fn an_indexed_images_default_decode_passes_every_index_through_unchanged() {
    let mut lookup = Vec::with_capacity(256 * 3);
    for index in 0..=255u8 {
        lookup.extend_from_slice(&[index, 0, 0]);
    }
    let table = stream_object(6, "", &lookup);
    let mut objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", b"40 0 0 40 0 0 cm /Im Do"),
        stream_object(
            5,
            "/Type /XObject /Subtype /Image /Width 4 /Height 1 /BitsPerComponent 8 \
             /ColorSpace [/Indexed /DeviceRGB 255 6 0 R]",
            &[1, 127, 254, 255],
        ),
    ];
    objects.push(table);
    let raster = render(assemble(&objects));
    about(cell(&raster, 0), [1, 0, 0], "index 1");
    about(cell(&raster, 1), [127, 0, 0], "index 127");
    about(cell(&raster, 2), [254, 0, 0], "index 254");
    about(cell(&raster, 3), [255, 0, 0], "index 255");
}

/// The corpus's own witness that a `DCTDecode` image's `/Decode` array is not decorative.
///
/// `issue7406.pdf` writes `/Decode [1 0 1 0 1 0]` on a JPEG whose samples are stored
/// inverted, so the array is what makes the page come out in its intended colours. The route
/// through `zune-jpeg` never consulted the entry, so the pdf.js logo on its first page was
/// drawn cyan on black against all four reference renderers' red on white. The oracle could
/// not fail on it: the page is text-heavy, the references disagree among themselves about the
/// text, and the verdict is `ambiguous` either way — our distance from them fell from a mean
/// of 17.36 to 5.02 and no ratchet watches a page in that class.
///
/// `None` means one thing: the corpus submodule is not checked out.
#[test]
fn a_decode_array_on_a_jpeg_is_applied_to_its_channels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    if !root.is_dir() {
        println!("skipped: the pdf.js corpus submodule is not checked out");
        return;
    }
    let path = root.join("issue7406.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        panic!("the corpus is present but {} is missing", path.display());
    };
    let document = Document::open(bytes).expect("issue7406.pdf opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "issue7406.pdf page one draws completely: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("a letter page is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported");

    // The logo sits at the top left of the page. Under the array the surrounding shield is
    // light and the wordmark's background is a strong red; with the array dropped both are
    // their complements, so the mean red channel over the badge separates the two readings by
    // more than any rounding can.
    let mut red = 0u64;
    let mut blue = 0u64;
    let mut counted = 0u64;
    for y in 20..40u32 {
        for x in 20..40u32 {
            let at = ((y * raster.width + x) * 4) as usize;
            red += u64::from(raster.data[at]);
            blue += u64::from(raster.data[at + 2]);
            counted += 1;
        }
    }
    assert!(counted > 0, "the badge region is inside the page");
    let red = red / counted;
    let blue = blue / counted;
    assert!(
        red > blue,
        "the logo should be warm, not its complement: mean red {red}, mean blue {blue}"
    );
}
