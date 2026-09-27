//! A blend mode inside a `DeviceCMYK` luminosity mask group — ISO 32000-2 §11.5.3 with §11.3.5.
//!
//! # What the expected values come from
//!
//! §11.3.4 fixes the arithmetic's form for a subtractive space:
//!
//! > When performing blending operations in subtractive colour spaces ( DeviceCMYK , ICCBased
//! > 'CMYK', Separation , and DeviceN ), the colour component values shall be complemented
//! > (subtracted from 1.0) before the blend function is applied and the results of the function
//! > shall then be complemented back before being used.
//!
//! §11.3.5.3 adds, for the non-separable modes in CMYK:
//!
//! > For the K component, the result shall be the K component of Cb for the Hue , Saturation , and
//! > Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.
//!
//! and §11.5.3's device branch turns the composited colour into the mask value with the grey
//! EXAMPLE 2 gives for `DeviceCMYK`, `Y = 1 − min(1, 0.3 C + 0.59 M + 0.11 Y + K)`. Every value
//! below is that arithmetic by hand, in the test that asserts it.
//!
//! The page paints black through the mask onto white, so a pixel's level is `255 × (1 − Y)`; the
//! mask group is opaque over the whole page, so `/BC` never shows. The fixtures are 40-unit pages
//! drawn at one pixel per unit.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, and the \
              fixtures are 40x40 pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// The mask group: four columns across its top half, each a backdrop and then a source under the
/// state its column names, and two `Normal` marks of other spaces across its bottom half.
const GROUP: &str = "\
    0 0 0 0.2 k 0 20 10 20 re f /GA gs 0 0 0 0.8 k 0 20 10 20 re f /GN gs \
    1 0 0 0 k 10 20 10 20 re f /GA gs 0 1 0 0 k 10 20 10 20 re f /GN gs \
    1 0 0 0.3 k 20 20 10 20 re f /GB gs 0 1 0 0.6 k 20 20 10 20 re f /GN gs \
    1 0 0 0.3 k 30 20 10 20 re f /GC gs 0 0.5 0 0.2 k 30 20 10 20 re f /GN gs \
    1 0 0 rg 0 0 20 20 re f 0.5 g 20 0 20 20 re f";

/// A page whose `/SMask` group is [`GROUP`] blending in `DeviceCMYK`, with `/GA`, `/GB` and `/GC`
/// stating `modes` — or all three stating `Normal`.
fn page(modes: [&str; 3]) -> Vec<u8> {
    let [a, b, c] = modes;
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", b"q /GS gs 0 g 0 0 40 40 re f Q"),
        b"5 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >>\n\
          endobj\n"
            .to_vec(),
        stream_object(
            6,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << /ExtGState << \
                 /GA << /BM /{a} >> /GB << /BM /{b} >> /GC << /BM /{c} >> /GN << /BM /Normal >> \
                 >> >> /Group << /Type /Group /S /Transparency /I true /CS /DeviceCMYK >>"
            ),
            GROUP.as_bytes(),
        ),
    ];
    assemble(&objects)
}

/// One numbered stream object, with the `/Length` its data actually has.
fn stream_object(number: usize, dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// Assembles numbered objects into a file with a cross-reference table that points at them.
fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// Renders a fixture at one pixel per unit onto white, and requires it to draw completely — which
/// is also the claim that nothing about the mask was reported.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported")
}

/// The mask value at a point given in PDF coordinates: black through the mask onto white leaves
/// `255 × (1 − Y)`.
fn mask_at(raster: &pdf_render::Raster, x: u32, y: u32) -> f32 {
    let row = raster.height - 1 - y;
    let at = ((row * raster.width + x) as usize) * 4;
    1.0 - f32::from(raster.data[at]) / 255.0
}

/// Two levels of 255: each of the four components is carried in eight bits before `Y` is taken.
const TOLERANCE: f32 = 2.0 / 255.0;

/// Asserts a mask value within [`TOLERANCE`] of the clause's.
fn near(got: f32, want: f32, what: &str) {
    assert!(
        (got - want).abs() <= TOLERANCE,
        "{what}: the mask is {got}, the clause's is {want}"
    );
}

/// §11.3.5.2's Multiply, `B(cb, cs) = cb × cs`, on each component in additive form.
///
/// - Black over black: `k = 0.2` then `0.8` is `0.8 × 0.2 = 0.16` complemented, `K = 0.84`, so
///   `Y = 0.16`. Painted in one weighted channel `1 − ink ÷ 2` instead, the product is
///   `0.9 × 0.6 = 0.54` and the mask `1 − 2 × 0.46 = 0.08`, half the clause's.
/// - Magenta over cyan: in additive form `(0, 1, 1) × (1, 0, 1) = (0, 0, 1)`, which is cyan and
///   magenta both at 1.0 with no yellow or black, so `Y = 1 − (0.3 + 0.59) = 0.11`; one weighted
///   channel gives `0.85 × 0.705` and a mask of `0.1985`.
#[test]
fn multiply_in_a_cmyk_mask_group_multiplies_each_component_in_additive_form() {
    let raster = render(page(["Multiply", "Normal", "Normal"]));
    near(
        mask_at(&raster, 5, 30),
        0.16,
        "black under Multiply over black",
    );
    near(
        mask_at(&raster, 15, 30),
        0.11,
        "magenta under Multiply over cyan",
    );
}

/// The non-separable modes, whose `K` §11.3.5.3 states apart from the three it blends.
///
/// Hue, Saturation and Color keep the backdrop's luminosity in the blended three, and take its
/// `K`; Luminosity keeps the source's, and takes its `K`. EXAMPLE 2's weights are `Lum`'s, so the
/// mask comes out as the backdrop's `Y` under Hue — `(1, 0, 0, 0.3)`, `1 − (0.3 + 0.3) = 0.4` —
/// and as the source's under Luminosity — `(0, 0.5, 0, 0.2)`, `1 − (0.295 + 0.2) = 0.505`. Taking
/// the source's `K` under Hue would read `0.1`.
#[test]
fn hue_keeps_the_backdrops_black_and_luminosity_the_sources() {
    let raster = render(page(["Normal", "Hue", "Luminosity"]));
    near(mask_at(&raster, 25, 30), 0.4, "Hue over (1, 0, 0, 0.3)");
    near(
        mask_at(&raster, 35, 30),
        0.505,
        "Luminosity over (1, 0, 0, 0.3)",
    );
}

/// The four components the group is carried in where it blends change nothing a `Normal` mark
/// shows: red's `Y` is its grey, 0.3, and a grey's is itself, with and without a blend mode in the
/// group's resources.
#[test]
fn a_normal_mark_masks_the_same_whether_or_not_the_group_blends() {
    let blending = render(page(["Multiply", "Hue", "Luminosity"]));
    let plain = render(page(["Normal", "Normal", "Normal"]));
    for (x, want, what) in [(10, 0.3, "red"), (30, 0.5, "a half grey")] {
        near(mask_at(&blending, x, 10), want, what);
        near(mask_at(&plain, x, 10), want, what);
    }
}
