//! §7.3.8.2's inferred extent, applied to the object the clause's own EXAMPLE is about.
//!
//! > Finally, streams are used to represent many objects from whose attributes a length can be
//! > inferred. All of these constraints shall be consistent.
//!
//! and, of an image:
//!
//! > An image with 10 rows and 20 columns, using a single colour component and 8 bits per
//! > component, requires exactly 200 bytes of image data. If the stream uses a filter, there
//! > needs to be enough bytes of encoded data in the PDF file to produce those 200 bytes. An
//! > error occurs if Length is too small, if an explicit EOD marker occurs too soon, or if the
//! > decoded data does not contain 200 bytes.
//!
//! Until the five-hundred-and-twenty-first session the unpacker read a sample past the end of the
//! data as **zero**, so an image whose stream stopped short was completed with samples nobody
//! wrote: a `DeviceGray` picture gained black rows, and `178360.pdf`'s `/ImageMask` — 359 bytes
//! of the 50 048 its grid needs — marked 99.3% of its area in the fill colour, which no reference
//! renderer draws. What the file carries is drawn where it belongs; the rest of the grid is left
//! unpainted and `image::short_of_its_grid` says so beside the drawing.
//!
//! The fixtures come in pairs differing in exactly one thing, the number of bytes after
//! `stream`, so that nothing else can explain what changes.

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

/// A one-page PDF drawing one 2×2-sample image over the whole 40×40 page.
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

/// Interprets a fixture and hands back what it drew and what it said about it.
fn interpret(bytes: Vec<u8>) -> (pdf_render::Raster, Vec<String>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let said = interpretation
        .unsupported
        .iter()
        .map(|report| format!("{report:?}"))
        .collect();
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("a 40x40 page is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the display list holds nothing the CPU backend refuses");
    (raster, said)
}

/// The grey at the centre of the cell in column `column`, row `row` of a 2×2 image.
fn quadrant(raster: &pdf_render::Raster, column: u32, row: u32) -> u8 {
    let x = column * 20 + 10;
    let y = row * 20 + 10;
    raster.data[((y * raster.width + x) * 4) as usize]
}

/// A 2×2 8-bit `DeviceGray` image: black on the top row, white on the bottom.
const DICT: &str = "/Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8";

/// The whole grid present draws the picture and says nothing.
#[test]
fn an_image_whose_stream_holds_its_grid_draws_it_in_silence() {
    let (raster, said) = interpret(page_with_image(DICT, &[0x00, 0x00, 0xFF, 0xFF]));
    assert!(
        said.is_empty(),
        "a whole image is not worth a report: {said:?}"
    );
    assert_eq!(quadrant(&raster, 0, 0), 0, "top left is the first sample");
    assert_eq!(quadrant(&raster, 1, 0), 0, "top right is the second");
    assert_eq!(quadrant(&raster, 0, 1), 255, "bottom left is the third");
    assert_eq!(quadrant(&raster, 1, 1), 255, "bottom right is the fourth");
}

/// A row short leaves that row unpainted rather than black, and reports the shortfall.
///
/// The discriminator is the *colour* of the missing row: read as zero samples it is black, which
/// is what a `DeviceGray` image's absent bytes used to draw, and left unpainted it is the page.
#[test]
fn an_image_a_row_short_leaves_that_row_unpainted_and_reports_it() {
    let (raster, said) = interpret(page_with_image(DICT, &[0x00, 0x00]));
    assert_eq!(
        quadrant(&raster, 0, 0),
        0,
        "the row the file carries is drawn"
    );
    assert_eq!(quadrant(&raster, 1, 0), 0, "both of its samples");
    assert_eq!(
        quadrant(&raster, 0, 1),
        255,
        "the row the file does not carry is not painted, so the page shows"
    );
    assert_eq!(
        quadrant(&raster, 1, 1),
        255,
        "and neither of its samples is"
    );
    assert!(
        said.iter().any(|report| report.contains("7.3.8.2")
            && report.contains("stop at 2 bytes")
            && report.contains("needs 4")),
        "the shortfall should be named with both numbers, and said {said:?}"
    );
}

/// Half a row is half a row: the samples that arrived are drawn and the rest are not.
///
/// The bound is on *whole* samples — a sample whose last bits are past the end of the data was
/// never written — which is what keeps a partial byte from becoming a colour.
#[test]
fn an_image_that_stops_inside_a_row_draws_the_samples_that_arrived() {
    let (raster, said) = interpret(page_with_image(DICT, &[0x00, 0x00, 0x40]));
    assert_eq!(quadrant(&raster, 0, 0), 0, "the whole first row is drawn");
    assert_eq!(quadrant(&raster, 1, 0), 0, "both of its samples");
    assert_eq!(
        quadrant(&raster, 0, 1),
        0x40,
        "the one sample of the second row that arrived is drawn"
    );
    assert_eq!(
        quadrant(&raster, 1, 1),
        255,
        "and the one that did not is not painted"
    );
    assert!(
        said.iter().any(|report| report.contains("7.3.8.2")),
        "and it is still short of its grid: {said:?}"
    );
}

/// A stencil mask marks the page only where the file says so.
///
/// §8.9.6.2 makes a sample of 0 mark the page under the default `/Decode`, which is exactly why
/// an absent sample must not be read as one: `178360.pdf`'s truncated `/ImageMask` painted a
/// solid rectangle over its page for that reason. One byte holds the first row's two bits.
#[test]
fn a_stencil_marks_the_page_only_where_its_samples_reach() {
    let (raster, said) = interpret(page_with_image(
        "/Width 2 /Height 2 /ImageMask true",
        // Row one: two clear bits, which paint; the second row's byte is absent.
        &[0b0000_0000],
    ));
    assert_eq!(quadrant(&raster, 0, 0), 0, "the stated row marks the page");
    assert_eq!(
        quadrant(&raster, 1, 0),
        0,
        "in the fill colour, which is black"
    );
    assert_eq!(
        quadrant(&raster, 0, 1),
        255,
        "the absent row marks nothing at all"
    );
    assert_eq!(quadrant(&raster, 1, 1), 255, "neither sample of it");
    assert!(
        said.iter().any(|report| report.contains("7.3.8.2")),
        "and the shortfall is reported: {said:?}"
    );
}
