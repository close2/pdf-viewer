//! §7.3.10 indirection inside extended graphics-state values.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "small handwritten PDFs and fixed raster coordinates are test preconditions"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn assemble(objects: &[String]) -> Vec<u8> {
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = write!(pdf, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref = pdf.len();
    let size = objects.len() + 1;
    let _ = write!(pdf, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn stream(content: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len() + 1
    )
}

fn render(objects: &[String]) -> (u32, u32, Vec<u8>) {
    render_from_bytes(assemble(objects))
}

fn line_fixture(indirect: bool) -> Vec<String> {
    let content = "1 1 1 rg 0 0 100 100 re f /GS gs 4 w 10 50 m 90 50 l S";
    let state = if indirect {
        "<< /Type /ExtGState /D [6 0 R 9 0 R] >>"
    } else {
        "<< /Type /ExtGState /D [[12 8] 5] >>"
    };
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        stream(content),
        state.to_owned(),
    ];
    if indirect {
        objects.extend([
            "[7 0 R 8 0 R]".to_owned(),
            "12".to_owned(),
            "8".to_owned(),
            "5".to_owned(),
        ]);
    }
    objects
}

fn rgb_at(raster: &(u32, u32, Vec<u8>), x: u32, y: u32) -> [u8; 3] {
    let index = ((y * raster.0 + x) * 4) as usize;
    [raster.2[index], raster.2[index + 1], raster.2[index + 2]]
}

#[test]
fn indirect_dash_array_and_numeric_members_match_the_direct_extgstate_control() {
    let direct = render(&line_fixture(false));
    let indirect = render(&line_fixture(true));
    for (label, raster) in [("direct", direct), ("indirect", indirect)] {
        // [12 8] with phase 5 starts with only seven units of dash. A defaulted phase
        // would paint x=20, and a defaulted solid line would also paint that gap.
        assert_eq!(rgb_at(&raster, 14, 50), [0, 0, 0], "{label}: first dash");
        assert_eq!(rgb_at(&raster, 20, 50), [255, 255, 255], "{label}: gap");
        assert_eq!(rgb_at(&raster, 30, 50), [0, 0, 0], "{label}: second dash");
        assert_eq!(
            rgb_at(&raster, 20, 20),
            [255, 255, 255],
            "{label}: outside stroke"
        );
    }
}

fn blend_fixture(indirect: bool) -> Vec<u8> {
    let content = "0 0 1 rg 0 0 100 100 re f\n/GS gs 1 0 0 rg 0 0 100 100 re f";
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        stream(content),
    ];
    if indirect {
        objects.extend([
            "<< /Type /ExtGState /BM [6 0 R 7 0 R] /ca 8 0 R >>".to_owned(),
            "/Screen".to_owned(),
            "/Multiply".to_owned(),
            "0.5".to_owned(),
        ]);
    } else {
        objects.push("<< /Type /ExtGState /BM [/Screen /Multiply] /ca 0.5 >>".to_owned());
    }
    assemble(&objects)
}

#[test]
fn indirect_blend_names_and_alpha_match_the_pdf_17_direct_control() {
    // Screen(red, blue) = magenta. At alpha 1/2 over blue, literal source-over gives
    // (1/2, 0, 1). A normal blend instead gives (1/2, 0, 1/2), so this also isolates
    // the selected blend-array member from alpha handling.
    let expected = [128_u8, 0, 255];
    for (label, bytes) in [
        ("direct control", blend_fixture(false)),
        ("indirect target", blend_fixture(true)),
    ] {
        let raster = render_from_bytes(bytes);
        let actual = rgb_at(&raster, 50, 50);
        for channel in 0..3 {
            assert!(
                actual[channel].abs_diff(expected[channel]) <= 3,
                "{label}: got {actual:?}, expected Screen/half-alpha {expected:?} within raster rounding"
            );
        }
    }
}

fn render_from_bytes(bytes: Vec<u8>) -> (u32, u32, Vec<u8>) {
    let document = Document::open(bytes).expect("complete fixture PDF");
    assert!(
        !document.was_recovered(),
        "fixture uses its exact xref table"
    );
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("fixture page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture is fully interpreted: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("100x100 target");
    let mut rasterizer = render_cpu::CpuRasterizer::new();
    let raster = rasterizer
        .rasterize(&interpretation.display_list, target)
        .expect("fixture rasterizes");
    (raster.width, raster.height, raster.data)
}
