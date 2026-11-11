//! Regression for §8.9.3's per-pixel component interleaving and §8.9.5.2's
//! independent per-component `Decode` pairs and byte-aligned rows.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "small test fixture and exact sample indexing; failures should be loud"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

fn stream(number: usize, dict: &str, bytes: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        bytes.len()
    )
    .into_bytes();
    object.extend_from_slice(bytes);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn fixture(parameters: &str, samples: &[u8]) -> Vec<u8> {
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream(4, "", b"40 0 0 40 0 0 cm /Im Do"),
        stream(
            5,
            &format!("/Type /XObject /Subtype /Image /Width 2 /Height 2 {parameters}"),
            samples,
        ),
    ];

    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(&object);
    }
    let xref = bytes.len();
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {}", offsets.len());
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * raster.width + x) * 4) as usize;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

#[test]
fn low_bit_rgb_samples_are_interleaved_per_pixel() {
    let raster = render(
        "/ColorSpace /DeviceRGB /BitsPerComponent 2 /Decode [1 0 0.2 0.8 0.25 0.75]",
        // Rows: (0,1,2),(3,2,1),pad1111; (1,3,0),(2,0,3),pad1110.
        &[0x1b, 0x9f, 0x72, 0x3e],
    );

    // Each component uses a different Decode pair. A shared pair, planar order,
    // or consuming row 0's nonzero padding as row 1's samples changes these pixels.
    assert_eq!(pixel(&raster, 10, 10), [255, 102, 149]);
    assert_eq!(pixel(&raster, 30, 10), [0, 153, 106]);
    assert_eq!(pixel(&raster, 10, 30), [170, 204, 64]);
    assert_eq!(pixel(&raster, 30, 30), [85, 51, 191]);
}

fn render(parameters: &str, samples: &[u8]) -> pdf_render::Raster {
    let document = Document::open(fixture(parameters, samples)).expect("complete PDF opens");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one page in the page tree");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the RGB image is supported: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("40x40 page fits the budget");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported display list rasterizes")
}

#[test]
fn sixteen_bit_image_samples_use_the_high_byte_first() {
    // Every sample has different high/low bytes. The old 0000/5555/AAAA/FFFF ramp
    // cannot detect byte reversal. Expected levels are round(sample * 255 / 65535).
    let raster = render(
        "/ColorSpace /DeviceGray /BitsPerComponent 16",
        &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
    );
    for (x, y, expected) in [(10, 10, 1), (30, 10, 69), (10, 30, 137), (30, 30, 205)] {
        assert_eq!(pixel(&raster, x, y), [expected; 3]);
    }
}
