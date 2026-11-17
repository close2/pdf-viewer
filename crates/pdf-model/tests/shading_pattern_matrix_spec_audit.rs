//! Public raster regression for Table 75's shading-pattern `/Matrix`.
//!
//! This targets the pattern dictionary's matrix, not the Type 1 shading object's matrix. The
//! omitted and explicit identity cases are positive controls; a translation then changes the
//! gradient coordinate at the same painted point. Expected values follow the Type 2 function
//! equation and the pattern-space inverse translation, not another render path.
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "small controlled exact-xref PDF fixture; failures should be explicit"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;
const CONTENT: &str = "/Pattern cs /P0 scn 0 0 100 100 re f";

fn fixture(pattern_matrix: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Pattern << /P0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Pattern /PatternType 2 /Shading 6 0 R {pattern_matrix} >>\nendobj\n\
         6 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
         /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] \
         /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >>\nendobj\n",
        CONTENT.len().saturating_add(1),
        CONTENT,
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn center(bytes: Vec<u8>) -> [u8; 3] {
    let document = Document::open(bytes).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    assert!(
        !document.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::content::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "supported dictionary shading pattern: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU supports the gradient pattern");
    assert_eq!((raster.width, raster.height), (100, 100));
    let at = ((50u32.saturating_mul(raster.width)).saturating_add(50) as usize).saturating_mul(4);
    assert_eq!(raster.data[at + 3], 255, "the interior fill is opaque");
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

#[test]
fn shading_pattern_matrix_maps_pattern_space_before_sampling_its_gradient() {
    let omitted = center(fixture(""));
    let explicit_identity = center(fixture("/Matrix [1 0 0 1 0 0]"));
    let translated = center(fixture("/Matrix [1 0 0 1 20 0]"));

    // At the center pixel (x≈50.5), an identity matrix samples t≈0.505, giving
    // approximately [126, 0, 129]. With +20 x translation, the inverse-mapped shading point
    // is x≈30.5, t≈0.305, giving approximately [177, 0, 78]. Allow rounding only.
    assert!(
        omitted[0].abs_diff(126) <= 3 && omitted[1] == 0 && omitted[2].abs_diff(129) <= 3,
        "omitted Table 75 Matrix defaults to identity; got {omitted:?}"
    );
    assert_eq!(
        explicit_identity, omitted,
        "the specified identity equals Table 75's default"
    );
    assert!(
        translated[0].abs_diff(177) <= 3 && translated[1] == 0 && translated[2].abs_diff(78) <= 3,
        "the pattern matrix translates pattern space before evaluating the ramp; got {translated:?}"
    );
}
