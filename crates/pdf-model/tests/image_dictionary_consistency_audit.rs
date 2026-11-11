//! Regression for §8.9.5.1's requirement to error on mutually inconsistent image entries.
//! The malformed RGB `Decode` array has one pair rather than three; the positive control differs
//! only by having the required three pairs.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "small exact fixture; failures should be loud"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(decode: &str) -> Vec<u8> {
    let image = [255, 0, 0];
    let content = "10 0 0 10 0 0 cm /Im Do";
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        format!("4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n", content.len()).into_bytes(),
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Decode {decode} /Length {} >>\nstream\n",
            image.len()
        )
        .into_bytes()
        .into_iter()
        .chain(image)
        .chain(b"\nendstream\nendobj\n".iter().copied())
        .collect(),
    ];
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

fn interpretation(decode: &str) -> pdf_model::Interpretation {
    let document = Document::open(fixture(decode)).expect("complete PDF opens");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one page in the page tree");
    pdf_model::interpret(&document, &page)
}

fn valid_control() {
    let result = interpretation("[0 1 0 1 0 1]");
    assert!(
        result.is_complete(),
        "valid RGB control: {:?}",
        result.unsupported
    );
    let target = TargetSpec::for_page(&result.display_list, 1.0, 1 << 20).expect("small target");
    let raster = CpuRasterizer::new()
        .rasterize(&result.display_list, target)
        .expect("RGB image draws");
    let at = ((5 * raster.width + 5) * 4) as usize;
    assert_eq!(
        &raster.data[at..at + 4],
        &[255, 0, 0, 255],
        "valid control really paints the RGB sample"
    );
}

#[test]
fn an_image_decode_array_with_fewer_pairs_than_its_colour_space_errors() {
    valid_control();
    let result = interpretation("[0 1]");
    assert!(
        !result.is_complete(),
        "§8.9.5.1 requires an error for inconsistent image entries; reported: {:?}",
        result.unsupported
    );
}

#[test]
fn an_image_decode_array_with_extra_pairs_for_its_colour_space_errors() {
    valid_control();
    let result = interpretation("[0 1 0 1 0 1 0 1]");
    assert!(
        !result.is_complete(),
        "RGB requires exactly three pairs; reported: {:?}",
        result.unsupported
    );
}
