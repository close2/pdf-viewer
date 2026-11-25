//! Specification audit of ISO 32000-2 §10.5 to §10.8 (review 2248): §10.5's transfer function and
//! §10.8.3's simulated press on the window's own route to the device.
//!
//! `pdf_model` carries a page's transfer functions beside its display list as §11.7.5.2's channel
//! (`DisplayList::transfers`), and `pdf_render::resolve_transfers` applies it over a finished
//! raster. `QuorraRasterizer::rasterize` runs that pass; `QuorraRasterizer::rasterize_frame`, the
//! read-back form of the window's present (`QuorraWindowRenderer::render`, the frame
//! `viewer-device` draws), translates the same list and is the finding's subject. Each test is
//! one behaviour finding of `doc/reviews/2248-specification-audit-10-5-10-8.md`, with a control
//! the same fixture passes on main beside the failing half.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "exact-xref test fixtures, their assertions and a small raster's pixel index"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_render::{DisplayList, Raster, Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_raster::{PresentFrame, QuorraRasterizer};

/// A one-page classic-xref PDF, 40 points square, with `resources` and the content `data`.
fn page(resources: &str, data: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << {resources} >> \
             /Contents 4 0 R >>"
        ),
        {
            format!(
                "<< /Length {} >>\nstream\n{data}\nendstream",
                data.len().saturating_add(1)
            )
        },
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let count = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {count}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A page whose `/ExtGState` `/GS` states `/TR` as an inversion, filled red under it.
fn inverted_red_page() -> Vec<u8> {
    page(
        "/ExtGState << /GS << /TR << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> \
         >> >>",
        "/GS gs 1 0 0 rg 0 0 40 40 re f",
    )
}

/// `bytes` interpreted by `pdf_model`, as every host interprets it, with §10.8.3's simulation
/// asked for when `simulate`.
fn interpreted_from(bytes: Vec<u8>, simulate: bool) -> Arc<DisplayList> {
    let document = Document::open(bytes).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_separation_simulation(simulate);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "the fixture draws completely: {:?}",
        interpretation.unsupported
    );
    Arc::new(interpretation.display_list)
}

/// The colour at the raster's centre.
fn centre(raster: &Raster) -> [u8; 3] {
    let at = usize::try_from((raster.height / 2) * raster.width + raster.width / 2)
        .expect("a pixel index")
        * 4;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// ISO 32000-2 §10.5: "The output shall be the transformed component value to be transmitted to
/// the device (after halftoning, if necessary)." and §10.6.1: "Halftoning is not required for
/// such devices; after gamma correction by the transfer functions, the colour components shall
/// be transmitted directly to the device." `/TR` maps every component `x` to `1 − x`, so red
/// `(1, 0, 0)` reaches the device as cyan `(0, 1, 1)`. The control draws the list through
/// `rasterize`, which applies the channel; the window's frame of the same list at the same
/// placement must show the same cyan.
#[test]
fn the_window_frame_applies_a_pages_transfer_function_spec_audit() {
    gpu_lock_hook::install();
    let list = interpreted_from(inverted_red_page(), false);
    assert!(
        list.transfers().is_some(),
        "the interpreter carries the page's transfer function as §11.7.5.2's channel"
    );
    let mut gpu = QuorraRasterizer::new_headless().expect("a device for the window's lane");
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a 40 by 40 target");

    let whole = gpu
        .rasterize(&list, target)
        .expect("the device draws the page");
    let drawn = centre(&whole);
    assert!(
        drawn[0] < 16 && drawn[1] > 239 && drawn[2] > 239,
        "control: `rasterize` maps red through the inversion to cyan: {drawn:?}"
    );

    let frame = gpu
        .rasterize_frame(&PresentFrame {
            width: target.width,
            height: target.height,
            pages: &[(&list, target)],
            raster: None,
            overlays: &[],
        })
        .expect("the window's present draws the page");
    let shown = centre(&frame);
    assert!(
        shown[0] < 16 && shown[1] > 239 && shown[2] > 239,
        "the window's frame shows the colour the device is to receive after §10.5's function, \
         cyan, not the file's red before it: {shown:?} on {}",
        gpu.adapter_description()
    );
}

/// ISO 32000-2 §10.8.3, step c): "Blend the resulting separations into a single result using a
/// multiply blend", and d): "Convert the result to the actual device colour space and output it."
/// Under the reader's simulation the page composites on the simulated press, and `pdf_model`
/// hands the backends the process inks as two lists on one geometry — the complements of cyan,
/// magenta and yellow on the list itself and black beside it (`DisplayList::black`), which
/// `rasterize` draws and multiplies back (`pdf_render::resolve_blending`). The page fills with
/// `0 0 0 1 k`, full black ink: the control is `rasterize`, which shows it dark; the window's
/// frame of the same list must show the same.
#[test]
fn the_window_frame_draws_the_simulated_press_s_black_ink_spec_audit() {
    gpu_lock_hook::install();
    let list = interpreted_from(page("", "0 0 0 1 k 0 0 40 40 re f"), true);
    let mut gpu = QuorraRasterizer::new_headless().expect("a device for the window's lane");
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a 40 by 40 target");

    let whole = gpu
        .rasterize(&list, target)
        .expect("the device draws the page");
    let drawn = centre(&whole);
    assert!(
        drawn.iter().all(|&level| level < 100),
        "control: `rasterize` draws full black ink on the simulated press dark: {drawn:?}"
    );

    let frame = gpu
        .rasterize_frame(&PresentFrame {
            width: target.width,
            height: target.height,
            pages: &[(&list, target)],
            raster: None,
            overlays: &[],
        })
        .expect("the window's present draws the page");
    let shown = centre(&frame);
    assert!(
        shown.iter().zip(drawn).all(|(&a, b)| a.abs_diff(b) <= 8),
        "the window's frame of the simulated press shows what `rasterize` draws, {drawn:?}: \
         {shown:?} on {}",
        gpu.adapter_description()
    );
}
