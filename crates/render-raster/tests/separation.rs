//! ISO 32000-2 §10.8.3's simulated press, drawn by this backend and by the CPU oracle (ADR 1317).
//!
//! A separated page is the process pair and a plane per three spot colourants, each a whole list
//! drawn against the same device, and `pdf_render::resolve_separation` multiplies them together
//! over the readback exactly as the CPU backend does over its own rasters. The fixture is
//! `pdf-model`'s `tests/spot_press.rs`'s `LogoGreen` overprinting process yellow, whose centre is
//! worked by hand there: sRGB (69, 139, 0) — and the same mark over an RGB yellow on a page whose
//! group is `/DeviceRGB`, which is its own process separation (ADR 1329), (64, 146, 0). What is
//! asserted here is that this backend draws the same page — the centre within a level of the
//! worked value, and every pixel within two of the oracle's.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: the arithmetic indexes a raster this file just drew"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// `LogoGreen` overprinting yellow on a 40-unit page on the device's components.
fn logo_green_over_yellow() -> Vec<u8> {
    logo_green_over("", "0 0 1 0 k")
}

/// `LogoGreen` overprinting `yellow`, a fill operator's operands and operator, on a 40-unit page
/// stating `group`.
fn logo_green_over(group: &str, yellow: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] {group} \
             /Resources << /ColorSpace << /LG [/Separation /LogoGreen /DeviceCMYK 5 0 R] >> \
             /ExtGState << /OP << /OP true /op true >> >> >> /Contents 4 0 R >>"
        ),
        {
            let content = format!("{yellow} 0 0 40 40 re f /OP gs /LG cs 1 scn 5 5 30 30 re f");
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len() + 1
            )
        },
        "<< /FunctionType 4 /Domain [0.0 1.0] /Range [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] \
         /Length 62 >>\nstream\n{dup 0.84 mul exch 0.00 exch dup 0.44 mul exch 0.21 mul}\n\
         endstream"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

#[test]
fn a_separated_page_is_the_oracles_page() {
    same_page_as_the_oracle(logo_green_over_yellow(), [69, 139, 0]);
}

/// A page whose group is `/DeviceRGB`: the list is its own process separation and the spot plane
/// is multiplied in after it (`pdf_render::separation::resolve_over_device`).
#[test]
fn a_separated_rgb_page_is_the_oracles_page() {
    same_page_as_the_oracle(
        logo_green_over("/Group << /S /Transparency /CS /DeviceRGB >>", "1 1 0 rg"),
        [64, 146, 0],
    );
}

/// Draws `bytes` separated on this backend and on the CPU oracle, and holds the centre to
/// `expected` and every pixel to the oracle's.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test helper: a fixture that will not draw is a failed test"
)]
fn same_page_as_the_oracle(bytes: Vec<u8>, expected: [u8; 3]) {
    let document = pdf_syntax::Document::open(bytes).expect("a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_separation_simulation(true);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    let list = &interpretation.display_list;
    assert!(list.separation().is_some(), "the page is separated");
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40x40 target");

    let oracle = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws a separated page");
    let mut raster = QuorraRasterizer::new_headless().unwrap_or_else(|error| {
        panic!(
            "no adapter available for raster: {error}\n\
             Install a Vulkan driver (mesa-vulkan-drivers for a software one). These tests do \
             not skip, because a skipped suite reports success while verifying nothing."
        )
    });
    let drawn = raster
        .rasterize(list, target)
        .expect("this backend draws a separated page");

    let at = ((20 * drawn.width) + 20) as usize * 4;
    let centre = [drawn.data[at], drawn.data[at + 1], drawn.data[at + 2]];
    assert!(
        centre
            .iter()
            .zip(expected)
            .all(|(found, expected)| found.abs_diff(expected) <= 1),
        "the centre is yellow times LogoGreen: {centre:?}, worked as {expected:?}"
    );
    let worst = drawn
        .data
        .iter()
        .zip(&oracle.data)
        .map(|(ours, theirs)| ours.abs_diff(*theirs))
        .max()
        .unwrap_or(0);
    assert!(worst <= 2, "the two backends differ by {worst} levels");
}
