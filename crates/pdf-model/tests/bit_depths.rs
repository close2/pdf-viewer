//! The five component widths ISO 32000-2 §8.9.5.1's Table 87 permits.
//!
//! > The value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16 .
//!
//! Two of the five were read and the other three were refused and reported, which was honest
//! and cost three corpus documents their images. The rule the unpacker has to get right is
//! not the arithmetic — `/Decode`'s table already covers every sample a depth can carry — but
//! the *packing*: samples run continuously across a row, most significant bits first, and
//! each row restarts on a byte boundary.
//!
//! # Why these fixtures are synthetic
//!
//! Trap 8's argument, and it is sharper here than usual. The corpus holds three documents
//! with a depth other than 1 or 8, and between them they exercise 4-bit `DeviceRGB` and
//! nothing else — no 2-bit image, no 16-bit image, and no row whose width leaves a partial
//! byte at its end, which is exactly the case the padding rule exists for. A test written
//! against those three documents would pass with the padding rule missing.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the fixtures are tiny \
              pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the pages these tests build.
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

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
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

fn stream_object(number: u32, dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// Renders a fixture at one pixel per unit.
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

/// The pixel at the centre of cell `column` of a `columns`-wide image, in row `row` of `rows`.
fn cell(raster: &pdf_render::Raster, columns: u32, column: u32, rows: u32, row: u32) -> u8 {
    let x = column * (40 / columns) + (40 / columns) / 2;
    let y = row * (40 / rows) + (40 / rows) / 2;
    raster.data[((y * raster.width + x) * 4) as usize]
}

/// Asserts a grey is within one level of `expected`.
#[track_caller]
fn about(actual: u8, expected: u8, what: &str) {
    assert!(
        i32::from(actual).abs_diff(i32::from(expected)) <= 1,
        "{what}: got {actual}, expected about {expected}"
    );
}

/// A four-sample grey ramp at the given depth must decode to the same four greys.
///
/// §8.9.5.2's map is `D min + x × (D max − D min) ÷ (2^n − 1)`, so a sample at a given
/// *fraction* of its depth's range means the same colour at every depth. Choosing the four
/// samples as 0, ⅓, ⅔ and 1 of each range makes the expectation one line for all five, which
/// is the point: a depth is a packing, not a meaning.
fn ramp(bits: u32, data: &[u8]) {
    let raster = render(page_with_image(
        &format!("/Width 4 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent {bits}"),
        data,
    ));
    for (column, expected) in [0u8, 85, 170, 255].into_iter().enumerate() {
        about(
            cell(&raster, 4, u32::try_from(column).unwrap_or(0), 1, 0),
            expected,
            &format!("{bits} bits, sample {column}"),
        );
    }
}

/// One bit per component: four samples in the top half of one byte.
#[test]
fn one_bit_samples_are_the_two_ends_of_the_range() {
    let raster = render(page_with_image(
        "/Width 4 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 1",
        &[0b0101_0000],
    ));
    about(cell(&raster, 4, 0, 1, 0), 0, "1 bit, sample 0");
    about(cell(&raster, 4, 1, 1, 0), 255, "1 bit, sample 1");
    about(cell(&raster, 4, 2, 1, 0), 0, "1 bit, sample 0");
    about(cell(&raster, 4, 3, 1, 0), 255, "1 bit, sample 1");
}

/// Two bits: four samples in one byte, most significant pair first.
#[test]
fn two_bit_samples_are_four_to_a_byte() {
    ramp(2, &[0b00_01_10_11]);
}

/// Four bits: two samples per byte, high nibble first.
#[test]
fn four_bit_samples_are_two_to_a_byte() {
    ramp(4, &[0x05, 0xAF]);
}

/// Eight bits, which is the case that always worked, stated so the five are one family.
#[test]
fn eight_bit_samples_are_one_to_a_byte() {
    ramp(8, &[0x00, 0x55, 0xAA, 0xFF]);
}

/// Sixteen bits: two bytes per sample, most significant first.
#[test]
fn sixteen_bit_samples_are_two_bytes_most_significant_first() {
    ramp(16, &[0x00, 0x00, 0x55, 0x55, 0xAA, 0xAA, 0xFF, 0xFF]);
}

/// Each row starts on a byte boundary, whatever the previous one left over.
///
/// The rule no corpus document exercises. Three 4-bit samples are twelve bits, so each row
/// of this image ends four bits into its second byte and the next row begins in a *third*.
/// Packed continuously instead, row two would read the low nibble of byte 1 as its first
/// sample and every row after it would shift further.
#[test]
fn a_row_restarts_on_a_byte_boundary() {
    let raster = render(page_with_image(
        "/Width 3 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 4",
        // Row 1: 0, 8, 15 then four bits of padding. Row 2: 15, 8, 0 then padding.
        &[0x08, 0xF0, 0xF8, 0x00],
    ));

    about(cell(&raster, 3, 0, 2, 0), 0, "row 1 sample 0");
    about(cell(&raster, 3, 1, 2, 0), 136, "row 1 sample 8");
    about(cell(&raster, 3, 2, 2, 0), 255, "row 1 sample 15");
    about(cell(&raster, 3, 0, 2, 1), 255, "row 2 sample 15");
    about(cell(&raster, 3, 1, 2, 1), 136, "row 2 sample 8");
    about(cell(&raster, 3, 2, 2, 1), 0, "row 2 sample 0");
}

/// A depth Table 87 does not name is refused rather than rounded to one that is.
#[test]
fn an_unnamed_depth_is_reported() {
    let bytes = page_with_image(
        "/Width 4 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 12",
        &[0; 6],
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    assert!(
        !interpretation.is_complete(),
        "12 bits per component is not one of Table 87's five and must be reported"
    );
}
