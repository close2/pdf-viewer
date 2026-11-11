//! Consumer checks for §11.3.6's alpha-weighted blend and its zero-alpha limits.
//!
//! `Medium::NONE` makes the page's isolated-group alpha
//! observable in the returned raster. The Hue case uses distinct, nonopaque backdrop and
//! source alphas so neither a Normal-only answer nor an unnormalised premultiplied result
//! matches the independently derived color.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "the complete tiny fixture PDF and fixed raster extent are test preconditions"
)]

use std::fmt::Write as _;

use pdf_render::{Medium, Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn fixture(alpha_backdrop: f32, alpha_source: f32, mode: &str) -> Vec<u8> {
    let content = "/GB gs 0 0 1 rg 10 10 80 80 re f\n/GS gs 1 0 0 rg 10 10 80 80 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /I true /CS /DeviceRGB >> \
         /Resources << /ExtGState << \
         /GB << /Type /ExtGState /ca {alpha_backdrop} /BM /Normal >> \
         /GS << /Type /ExtGState /ca {alpha_source} /BM /{mode} >> \
         >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    assemble(&body)
}

fn assemble(body: &str) -> Vec<u8> {
    let mut output = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(output.len());
        output.push_str(object);
    }
    let xref_at = output.len();
    let size = offsets.len() + 1;
    let _ = writeln!(output, "xref\n0 {size}");
    output.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    output.into_bytes()
}

fn pixel(bytes: Vec<u8>) -> [u8; 4] {
    let document = Document::open(bytes).expect("complete fixture PDF");
    assert!(!document.was_recovered(), "exact source xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one fixture page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture is fully supported: {:?}",
        interpretation.unsupported
    );
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100x100 page target");
    let mut renderer = render_cpu::CpuRasterizer::new().with_medium(Medium::NONE);
    let raster = renderer
        .rasterize(list, target)
        .expect("fixture rasterizes");
    let offset = ((50_u32 * raster.width + 50) * 4) as usize;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

fn assert_near(actual: [u8; 4], expected: [u8; 4], reason: &str) {
    for (got, want) in actual.iter().zip(expected) {
        assert!(
            got.abs_diff(want) <= 3,
            "{reason}: got {actual:?}, expected {expected:?} within raster rounding"
        );
    }
}

#[test]
fn nonopaque_hue_uses_both_alphas_and_returns_normalized_result_colour() {
    // Hue(blue backdrop, red source) is (11/30, 0, 0) by Table 135. With
    // αb=1/2 and αs=1/4, αr=5/8 and the premultiplied result is
    // (41/240, 0, 3/8); normalized Cr is (41/150, 0, 3/5), about (70,0,153).
    // A Normal control gives (2/5,0,3/5), which separates the blend function
    // from the alpha-weighting step. Input/premultiplication byte rounding gets
    // three levels of tolerance.
    let hue = pixel(fixture(0.5, 0.25, "Hue"));
    assert_near(
        hue,
        [70, 0, 153, 160],
        "Hue's independently computed weighted result",
    );

    let normal = pixel(fixture(0.5, 0.25, "Normal"));
    assert_near(normal, [102, 0, 153, 160], "Normal source-over control");
    assert_ne!(
        hue[..3],
        normal[..3],
        "partial backdrop and source alphas retain Hue's distinct blend result"
    );
}

#[test]
fn either_zero_alpha_removes_the_blend_mode_effect() {
    // αb=0 leaves the source color; αs=0 leaves the backdrop unchanged. Both use Hue,
    // whose nonopaque-alpha control above has a different result from Normal.
    let no_backdrop = pixel(fixture(0.0, 0.25, "Hue"));
    assert_near(
        no_backdrop,
        [255, 0, 0, 64],
        "zero backdrop alpha yields source color",
    );

    let no_source = pixel(fixture(0.5, 0.0, "Hue"));
    assert_near(
        no_source,
        [0, 0, 255, 128],
        "zero source alpha preserves backdrop",
    );
}
