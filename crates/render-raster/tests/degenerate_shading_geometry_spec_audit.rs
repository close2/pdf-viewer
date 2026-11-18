//! Public-pixel checks for the Table 79 coincident-axis and Table 80 both-zero-radius
//! rules. Positive controls use the same direct `sh` path, valid functions, `Extend`, `Background`,
//! and exact-xref fixture construction.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and literal pixel controls"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_raster::QuorraRasterizer;
#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

fn backend(coverage: raster_gpu::Coverage) -> QuorraRasterizer {
    gpu_lock_hook::install();
    let rasterizer = QuorraRasterizer::with_options(&raster_gpu::Options {
        coverage,
        ..render_raster::options()
    })
    .expect("requested GPU coverage is available")
    .with_medium(pdf_render::Medium::NONE);
    eprintln!(
        "coverage {coverage:?} adapter {}",
        rasterizer.adapter_description()
    );
    rasterizer
}

const CONTENT: &str = "q /Sh0 sh Q";
const GENEROUS: u64 = 1 << 20;

fn fixture(shading: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
          /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{CONTENT}\nendstream",
            CONTENT.len() + 1
        ),
        shading.to_owned(),
    ];
    let mut body = String::new();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(body.len());
        let _ = writeln!(body, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref = "%PDF-1.7\n".len() + body.len();
    let mut out = format!("%PDF-1.7\n{body}");
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{:010} 00000 n ", "%PDF-1.7\n".len() + offset);
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(backend: &mut QuorraRasterizer, shading: &str) -> Result<pdf_render::Raster, String> {
    let document = Document::open(fixture(shading)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "the single xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the valid shading is fully interpreted: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("small page target");
    backend
        .rasterize(&interpretation.display_list, target)
        .map_err(|error| format!("{error:?}"))
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let i = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel index");
    [
        raster.data[i],
        raster.data[i + 1],
        raster.data[i + 2],
        raster.data[i + 3],
    ]
}

fn check_axial(coverage: raster_gpu::Coverage) {
    let mut backend = backend(coverage);
    let function = "/Function << /FunctionType 2 /Domain [0 1] \
        /C0 [1 0 0] /C1 [1 0 0] /N 1 >> /Extend [true true] /Background [0 1 0]";
    // Table 77 says direct `sh` ignores Background; this nondegenerate axis proves the same
    // reader path and color function paint the expected red instead of the green Background.
    let visible = render(
        &mut backend,
        &format!("<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [10 50 90 50] {function} >>"),
    )
    .expect("nondegenerate axial control renders");
    assert_eq!(pixel(&visible, 50, 50), [255, 0, 0, 255]);

    let blank = render(
        &mut backend,
        &format!("<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [50 50 50 50] {function} >>"),
    );
    let painted = blank.map(|raster| {
        raster
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|rgba| rgba[3] != 0)
            .count()
    });
    assert_eq!(
        painted,
        Ok(0),
        "coincident axis paints nothing despite Extend=true"
    );
}

fn check_radial(coverage: raster_gpu::Coverage) {
    let mut backend = backend(coverage);
    let function = "/Function << /FunctionType 2 /Domain [0 1] \
        /C0 [1 0 0] /C1 [1 0 0] /N 1 >> /Extend [true true] /Background [0 1 0]";
    // A single zero radius is a point, not the both-zero no-paint special case. With
    // coincident centres, the family grows from that point to r=40. This positive control uses a constant red function to avoid making a
    // claim about interpolation at the singular center. Direct `sh` ignores the green
    // Background.
    let point_to_circle = render(
        &mut backend,
        &format!(
            "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [50 50 0 50 50 40] {function} >>"
        ),
    )
    .expect("one-zero-radius control renders");
    assert_eq!(pixel(&point_to_circle, 50, 50), [255, 0, 0, 255]);

    let empty_coincident = render(
        &mut backend,
        &format!(
            "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [50 50 0 50 50 0] {function} >>"
        ),
    );
    let empty_distinct = render(
        &mut backend,
        &format!(
            "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [20 30 0 80 70 0] {function} >>"
        ),
    );
    let observations = [empty_coincident, empty_distinct].map(|result| {
        result.map(|raster| {
            raster
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|rgba| rgba[3] != 0)
                .count()
        })
    });
    assert_eq!(
        observations,
        [Ok(0), Ok(0)],
        "coincident and distinct centers both paint nothing"
    );
}

#[test]
fn cpu_coverage_degenerate_axial_paints_nothing() {
    check_axial(raster_gpu::Coverage::Cpu);
}

#[test]
fn cpu_coverage_degenerate_radial_paints_nothing() {
    check_radial(raster_gpu::Coverage::Cpu);
}

#[test]
fn gpu_coverage_degenerate_axial_paints_nothing() {
    check_axial(raster_gpu::Coverage::Gpu);
}

#[test]
fn gpu_coverage_degenerate_radial_paints_nothing() {
    check_radial(raster_gpu::Coverage::Gpu);
}

#[test]
fn compute_coverage_degenerate_axial_paints_nothing() {
    check_axial(raster_gpu::Coverage::Compute);
}

#[test]
fn compute_coverage_degenerate_radial_paints_nothing() {
    check_radial(raster_gpu::Coverage::Compute);
}
