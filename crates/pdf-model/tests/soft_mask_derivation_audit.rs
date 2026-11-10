//! Integration regression for ISO 32000-2 §11.5.2.
//!
//! Existing PDF-level `/TR` coverage is luminosity-only and existing alpha transfer
//! coverage constructs `SoftMask` directly. This complete PDF joins those two routes.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture construction and known in-bounds raster samples"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;

#[test]
fn a_pdf_alpha_soft_mask_applies_transfer_to_group_alpha_and_to_zero_outside() {
    // The group has the same black colour in both cells and alpha .25/.75. Inversion
    // therefore produces mask values .75/.25; painting black over white leaves .25/.75,
    // approximately 64/191. Outside the form BBox alpha is zero, so transfer(0)=1 and the
    // masked black page mark is fully painted (0). /BC [1] must not turn this Alpha mask into
    // a luminosity mask: alpha derivation has no backdrop colour input.
    let bytes = pdf();
    let document = Document::open(bytes).expect("complete PDF fixture opens");
    assert!(!document.was_recovered(), "the fixture has a correct xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one exists");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture must draw without recovery or unsupported content: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("40-unit page at one pixel per unit is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported complete fixture");

    near_gray(&raster, 5, 20, 64); // TR(0.25)=0.75 mask.
    near_gray(&raster, 15, 20, 191); // TR(0.75)=0.25 mask.
    near_gray(&raster, 30, 20, 0); // TR(0)=1 outside group BBox.
}

fn pdf() -> Vec<u8> {
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream(4, "", b"q /GS gs 0 g 0 0 40 40 re f Q"),
        b"5 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Alpha /G 6 0 R \
          /BC [1] /TR << /FunctionType 2 /Domain [0 1] /Range [0 1] \
          /C0 [1] /C1 [0] /N 1 >> >> >>\nendobj\n"
            .to_vec(),
        stream(
            6,
            "/Type /XObject /Subtype /Form /BBox [0 0 20 40] \
             /Resources << /ExtGState << /Quarter 7 0 R /ThreeQuarter 8 0 R >> >> \
             /Group << /Type /Group /S /Transparency /CS /DeviceGray >>",
            b"/Quarter gs 0 g 0 0 10 40 re f /ThreeQuarter gs 0 g 10 0 10 40 re f",
        ),
        b"7 0 obj\n<< /Type /ExtGState /ca 0.25 >>\nendobj\n".to_vec(),
        b"8 0 obj\n<< /Type /ExtGState /ca 0.75 >>\nendobj\n".to_vec(),
    ];
    assemble(&objects)
}

fn stream(number: usize, dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(output.len());
        output.extend_from_slice(object);
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    output.extend_from_slice(tail.as_bytes());
    output
}

fn near_gray(raster: &pdf_render::Raster, x: u32, y: u32, expected: u8) {
    assert!(x < raster.width && y < raster.height);
    let row = raster.height - 1 - y;
    let offset = usize::try_from(row * raster.width + x).expect("small fixture") * 4;
    for channel in &raster.data[offset..offset + 3] {
        assert!(
            channel.abs_diff(expected) <= 1,
            "({x}, {y}): {channel} versus {expected}"
        );
    }
    assert_eq!(raster.data[offset + 3], 255, "opaque white page backdrop");
}
