//! Regression for ISO 32000-2:2020 Table 88's `DeviceN` default Decode row.
//!
//! A two-colourant `DeviceN` image omits Decode. The Type 4 calculator averages its tint inputs
//! into equal `DeviceRGB` channels. The explicit [0 1 0 1] control must match the implicit default;
//! reversing the second pair must produce a distinctly lighter result.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the standalone fixture has fixed bounds and must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const FUNCTION: &str = "{ add 2 div dup dup }";
const PIXEL_BUDGET: u64 = 1 << 20;

fn pdf(decode: &str) -> Vec<u8> {
    let content = b"40 0 0 40 0 0 cm /Im Do";
    let image_data = [0x80, 0x40];
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n",
            content.len()
        )
        .into_bytes()
        .into_iter()
        .chain(content.iter().copied())
        .chain(b"\nendstream\nendobj\n".iter().copied())
        .collect(),
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace [/DeviceN [/SpotA /SpotB] /DeviceRGB 6 0 R] /BitsPerComponent 8 {decode} /Length 2 >>\nstream\n"
        )
        .into_bytes()
        .into_iter()
        .chain(image_data)
        .chain(b"\nendstream\nendobj\n".iter().copied())
        .collect(),
        format!(
            "6 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            FUNCTION.len(),
            FUNCTION
        )
        .into_bytes(),
    ];

    // Each entry starts at its object-number digit; xref offsets point to the exact header byte.
    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(&object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut xref = String::new();
    let _ = writeln!(xref, "xref\n0 {size}");
    xref.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());
    out
}

fn center(decode: &str) -> [u8; 4] {
    let document = Document::open(pdf(decode)).expect("complete PDF 1.7 fixture");
    assert!(!document.was_recovered(), "fixture xref is exact");
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("40x40 page fits target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("DeviceN image rasterizes");
    let row = raster.height.saturating_sub(1).saturating_sub(20);
    let at = ((row * raster.width + 20) as usize) * 4;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

#[test]
fn table88_devicen_default_supplies_a_pair_for_each_image_component() {
    // Table 88 gives every DeviceN component [0 1]. The raw samples therefore reach the
    // calculator as 128/255 and 64/255. Their mean is 96/255, giving literal output channel 96.
    let implicit_default = center("");
    assert_eq!(implicit_default, [96, 96, 96, 255]);

    // Independent positive control: explicitly spell the two Table 88 pairs.
    assert_eq!(center("/Decode [0 1 0 1]"), implicit_default);

    // Reverse SpotB only: its decoded sample is 1 - 64/255 = 191/255. The mean is 159.5/255;
    // permit either adjacent raster byte rather than making a claim about the tie-rounding rule.
    let reversed = center("/Decode [0 1 1 0]");
    assert!((159..=160).contains(&reversed[0]));
    assert_eq!(reversed[0], reversed[1]);
    assert_eq!(reversed[1], reversed[2]);
    assert_eq!(reversed[3], 255);
    assert!(reversed[0].abs_diff(implicit_default[0]) > 50);
}
