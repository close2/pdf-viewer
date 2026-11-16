//! Regression: a base image's /Decode array must contain numeric pairs.
//!
//! ISO 32000-2 §8.9.5.1 says inconsistent entries in a base image dictionary shall cause an
//! error. Table 87 types /Decode as an array of numbers and requires one pair per color component.
//! The current check enforces only the array length; `Decode::read` filters nonnumeric elements,
//! then silently shifts later entries into earlier component pairs. This fixture keeps the length
//! exact and changes one number to null, isolating element typing from pair-count behavior.

#![expect(
    clippy::expect_used,
    reason = "small exact fixture; invalid input and out-of-bounds accesses should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(decode: &str) -> Vec<u8> {
    let content = b"q 10 0 0 10 0 0 cm /Im Do Q";
    let image_data = [255, 0, 0];
    let image_object = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Decode {decode} /Length {} >>\nstream\n",
        image_data.len()
    )
    .into_bytes()
    .into_iter()
    .chain(image_data)
    .chain(b"\nendstream\nendobj\n".iter().copied())
    .collect();
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len())
            .into_bytes()
            .into_iter()
            .chain(content.iter().copied())
            .chain(b"\nendstream\nendobj\n".iter().copied())
            .collect(),
        image_object,
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(&object);
    }
    let xref = bytes.len();
    let mut table = String::new();
    let _ = writeln!(table, "xref\n0 {}", offsets.len());
    table.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    bytes.extend_from_slice(table.as_bytes());
    bytes
}

fn interpret(decode: &str) -> pdf_model::Interpretation {
    let document = Document::open(fixture(decode)).expect("syntactically complete PDF");
    assert!(!document.was_recovered(), "the fixture's xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    pdf_model::interpret(&document, &page)
}

#[test]
fn decode_pairs_must_each_be_numbers() {
    let valid = interpret("[0 1 0 1 0 1]");
    assert!(
        valid.is_complete(),
        "valid positive control: {:?}",
        valid.unsupported
    );
    let target = TargetSpec::for_page(&valid.display_list, 1.0, 1 << 20).expect("small target");
    let raster = CpuRasterizer::new()
        .rasterize(&valid.display_list, target)
        .expect("supported");
    let at = ((5 * raster.width + 5) as usize) * 4;
    assert_eq!(&raster.data[at..at + 4], &[255, 0, 0, 255]);

    // Same six slots / three pairs, but the second low endpoint is null. Filtering that value
    // out shifts the following values; silently drawing is not the stated six-number mapping.
    let invalid = interpret("[0 1 null 1 0 1]");
    assert!(
        !invalid.is_complete(),
        "Table 87's numeric Decode elements must not be silently filtered: {:?}",
        invalid.unsupported
    );
}
