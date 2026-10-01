//! ISO 32000-2 §11.3.2's undefined quantities and §11.4.2's recurrence, at the pixel.
//!
//! §11.3.2 is notation, and it carries two `shall`s that no formula of §11.3 states again:
//!
//! > It is significant that although any arbitrary value may be chosen for such an undefined
//! > quantity, the computation shall not malfunction because of exceptions caused by overflow or
//! > division by zero. In addition, the convention that 0 ÷ 0 = 0 shall also be adopted.
//!
//! The place both bite is an isolated group: its initial backdrop is transparent, so the first
//! element is composited over a backdrop colour the clause calls undefined, and recovering a
//! straight colour from a premultiplied one divides zero by zero there. A blend mode other than
//! `Normal` is what makes the backdrop colour enter the arithmetic at all.
//!
//! §11.4.2 then restates the basic formulas as a recurrence over a stack, each object's backdrop
//! the result of the one before it — so the second element below is composited over what the
//! first left, which is what an overlap of two blended marks inside one group measures.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and this page is 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// A one-page fixture: a white page, and on it an isolated group drawing `group`.
fn fixture(group: &str) -> Vec<u8> {
    let page = "1 1 1 rg 0 0 100 100 re f /Fm Do";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /Hue << /BM /Hue >> /Multiply << /BM /Multiply >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I true >> /Length {} >>\n\
         stream\n{group}\nendstream\nendobj\n",
        page.len() + 1,
        group.len() + 1,
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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

/// The fixture rendered at one pixel per unit by the CPU backend, which is the oracle.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises")
}

/// The RGBA pixel at device `(x, y)`; PDF's y-up origin is flipped by the target transform.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let at = usize::try_from((y * raster.width + x) * 4).expect("a small raster");
    let bytes = &raster.data[at..at + 4];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// §11.3.2: a non-separable blend over a group's transparent backdrop is the source colour.
///
/// With αb = 0 §11.3.3's result is the source whatever B(Cb, Cs) is, because B is multiplied by
/// αb — but only if Cb, the backdrop's straight colour, was recovered as 0 ÷ 0 = 0 rather than
/// as a NaN that the multiplication by zero then carries. `Hue` is one of §11.3.5.3's
/// non-separable modes, so B reads every component of Cb.
///
/// §11.4.2: the second mark's backdrop is the first mark's result. Red multiplied by blue is
/// black, which is the overlap; where the second lies over the transparent backdrop alone it is
/// blue, by the same convention.
#[test]
fn a_blend_over_an_undefined_backdrop_is_the_source_and_the_next_blends_over_the_result() {
    let raster = render(fixture(
        "/Hue gs 1 0 0 rg 10 10 50 50 re f /Multiply gs 0 0 1 rg 30 30 50 50 re f",
    ));
    // Device y = 75 is page y = 25, inside the first square only; device (70, 25) is page
    // (70, 75), inside the second only; device (45, 55) is page (45, 45), inside both.
    assert_eq!(
        pixel(&raster, 20, 75),
        [255, 0, 0, 255],
        "the first mark blended over nothing is its own colour"
    );
    assert_eq!(
        pixel(&raster, 70, 25),
        [0, 0, 255, 255],
        "the second mark over nothing is its own colour"
    );
    assert_eq!(
        pixel(&raster, 45, 55),
        [0, 0, 0, 255],
        "the second mark multiplies the first mark's result, not the page behind the group"
    );
    assert_eq!(
        pixel(&raster, 95, 5),
        [255, 255, 255, 255],
        "outside the group's marks the page is what it painted"
    );
}
