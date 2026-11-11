//! Consumer regression for §11.4.4 result/backdrop removal under a non-Normal Do mode.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "complete tiny fixtures and bounded raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;

fn fixture(isolated: bool) -> Vec<u8> {
    let page = "0.5 g 0 0 100 100 re f /Outer gs /Fm Do";
    // A half-opacity mid-grey element, composited with Multiply against the group's initial
    // mid-grey backdrop. Its shape covers only the middle 80×80 square.
    let form = "/Inner gs 0.5 g 10 10 80 80 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /Outer << /BM /Multiply >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I {} >> \
         /Resources << /ExtGState << /Inner << /BM /Multiply /ca 0.5 >> >> >> \
         /Length {} >>\nstream\n{form}\nendstream\nendobj\n",
        page.len().saturating_add(1),
        if isolated { "true" } else { "false" },
        form.len().saturating_add(1),
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
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(isolated: bool) -> pdf_render::Raster {
    let document = Document::open(fixture(isolated)).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "exact source xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid transparency group should interpret: {:?}",
        interpretation.unsupported
    );
    assert!(
        interpretation
            .display_list
            .commands()
            .iter()
            .any(|command| matches!(
                command,
                Command::Group {
                    isolated: actual_isolated,
                    knockout: false,
                    blend: pdf_render::BlendMode::Multiply,
                    ..
                } if *actual_isolated == isolated
            )),
        "the consumer must receive a nonknockout group with the stated isolation and Do Multiply"
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("valid raster target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU supports the stated transparency group")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let start = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel index");
    [
        raster.data[start],
        raster.data[start + 1],
        raster.data[start + 2],
        raster.data[start + 3],
    ]
}

fn near(got: [u8; 4], want: [u8; 4], context: &str) {
    assert!(
        got.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 2),
        "{context}: got {got:?}, expected {want:?}"
    );
}

/// Independent §11.4.4 / §11.3.6 calculation, with a nonzero removal factor.
///
/// Backdrop C0=.5, α0=1; internal source Cs=.5, αs=.5, and its Multiply blend is .25.
/// Therefore Cn=.5*.5 + .5*.25=.375 and αgn=.5. §11.4.4 removes the backdrop:
/// C=(.375 - .5*.5)/.5=.25, with group alpha .5. At the Do, Multiply against the same
/// .5 page gives blend=.125; the final color is .5*.5 + .5*.125=.3125, byte 80.
///
/// If the result step is omitted and the backdrop-included .375 is used as the group source,
/// while alpha is reset to the group alpha .5, the result is about byte 88; if the group is incorrectly treated as isolated, its inner
/// Multiply has no backdrop and the Do result is byte 96. Thus this is a discriminating oracle
/// for both nonisolated initial-backdrop handling and the nonzero backdrop-removal arithmetic.
#[test]
fn nonisolated_group_removes_backdrop_before_non_normal_do_blend() {
    let raster = render(false);
    near(
        pixel(&raster, 50, 50),
        [80, 80, 80, 255],
        "inside the group element",
    );
    near(
        pixel(&raster, 5, 5),
        [128, 128, 128, 255],
        "outside the group element",
    );
}

/// A valid isolated version is the positive control for the isolation distinction: the internal
/// Multiply sees transparency, so it has no effect; source grey .5 at alpha .5 then multiplies
/// the .5 page at the Do, producing .5*.5 + .5*.25=.375, byte 96.
#[test]
fn isolated_group_control_does_not_read_the_page_as_its_initial_backdrop() {
    let raster = render(true);
    near(
        pixel(&raster, 50, 50),
        [96, 96, 96, 255],
        "isolated control pixel",
    );
    near(
        pixel(&raster, 5, 5),
        [128, 128, 128, 255],
        "outside the group element",
    );
}
