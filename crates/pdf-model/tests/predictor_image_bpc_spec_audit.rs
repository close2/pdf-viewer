//! Independent public-render regression for EC3 issue #366.
//!
//! The Flate predictor sees the `ASCIIHex` text `B0A580>` as seven 8-bit samples. `ASCIIHexDecode`
//! then produces the final three bytes `B0 A5 80`, which are exactly 24 `DeviceGray` samples at
//! one bit per sample. Thus predictor `DecodeParms` /BitsPerComponent=8 and image
//! /BitsPerComponent=1 are both meaningful and intentionally different. Changing the image
//! BPC to the predictor BPC would change the final sample unpacking; changing predictor BPC to
//! the image BPC would change the predictor row geometry (7 bytes versus 1 byte).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "bounded exact-xref fixture and fixed raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 10;
const PIXEL_BUDGET: u64 = 1 << 20;

// zlib.compress(b"\x00B0A580>") from Python 3's zlib module. The uncompressed input is one
// PNG Predictor 15 None row: leading row-type 0, followed by seven ASCII bytes.
const FLATE_PREDICTED_ASCIIHEX: &[u8] = &[
    0x78, 0x9c, 0x63, 0x70, 0x32, 0x70, 0x34, 0xb5, 0x30, 0xb0, 0x03, 0x00, 0x06, 0x55, 0x01, 0x8f,
];

const RAW_IMAGE_BYTES: &[u8] = &[0xb0, 0xa5, 0x80];

// The packed one-bit samples in RAW_IMAGE_BYTES, with 1 -> white and 0 -> black under the
// default DeviceGray image Decode array. This is the independent expected pixel classification.
const EXPECTED_WHITE: [bool; 24] = [
    true, false, true, true, false, false, false, false, // B0
    true, false, true, false, false, true, false, true, // A5
    true, false, false, false, false, false, false, false, // 80
];

fn stream_object(number: usize, entries: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {entries} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn fixture(image_entries: &str, image_data: &[u8]) -> Vec<u8> {
    let content = b"240 0 0 10 0 0 cm /Im Do";
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {WIDTH} {HEIGHT}] \
             /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
        )
        .into_bytes(),
        stream_object(4, "", content),
        stream_object(
            5,
            &format!(
                "/Type /XObject /Subtype /Image /Width 24 /Height 1 \
                 /ColorSpace /DeviceGray /BitsPerComponent 1 {image_entries}"
            ),
            image_data,
        ),
    ];

    let mut pdf = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(object);
    }
    let xref_at = pdf.len();
    let size = offsets.len() + 1;
    let mut xref = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.extend_from_slice(xref.as_bytes());
    pdf
}

fn render(pdf: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(pdf).expect("exact-xref fixture opens");
    assert!(!document.was_recovered(), "fixture has exact offsets");
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid supported fixture is fully interpreted: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("240-by-10 target fits the budget");
    assert_eq!((target.width, target.height), (WIDTH, HEIGHT));
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU renderer supports this image")
}

fn sampled_white_bits(raster: &pdf_render::Raster) -> [bool; 24] {
    std::array::from_fn(|sample| {
        // Ten raster pixels per source sample; the center avoids interpolation boundaries.
        let x = u32::try_from(sample).expect("twenty-four samples fit u32") * 10 + 5;
        let raster_y = raster.height - 1 - 5;
        let at = ((raster_y * raster.width + x) * 4) as usize;
        match &raster.data[at..at + 4] {
            [255, 255, 255, 255] => true,
            [0, 0, 0, 255] => false,
            pixel => panic!("sample {sample} must be exactly opaque black or white, got {pixel:?}"),
        }
    })
}

#[test]
fn predictor_parameter_bpc_is_independent_of_final_image_sample_bpc() {
    // The Flate stage reverses a PNG predictor over seven ASCII characters. The subsequent
    // ASCIIHex stage turns those characters into the 3 packed bytes required by the 1-bpc image.
    let filtered = render(fixture(
        "/Filter [/FlateDecode /ASCIIHexDecode] \
         /DecodeParms [<< /Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns 7 >> null]",
        FLATE_PREDICTED_ASCIIHEX,
    ));

    // Independent positive control: same image dictionary and expected final sample bytes,
    // with filters absent. This isolates sample unpacking from filter/predictor processing.
    let unfiltered = render(fixture("", RAW_IMAGE_BYTES));
    let expected_filtered = sampled_white_bits(&filtered);
    let expected_unfiltered = sampled_white_bits(&unfiltered);
    assert_eq!(expected_filtered, EXPECTED_WHITE);
    assert_eq!(expected_unfiltered, EXPECTED_WHITE);
    assert_eq!(expected_filtered, expected_unfiltered);
}
