//! The window's present draws a page's own passes as `rasterize` does (ADR 3701): §11.4.7's page
//! group converted out of its blending space, on the route `viewer-device` draws every frame
//! through (`QuorraRasterizer::rasterize_frame` is its read-back form), and a page with no pass
//! still drawn in page space.
//!
//! Review 2248 found the transfer functions and §10.8.3's black ink dropped on this route
//! (`window_frame_10_5_10_8_spec_audit.rs`) and read, without running it, that a four-component
//! page group's black list went the same way. These run it.

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

/// A one-page classic-xref PDF, 40 points square, with the page dictionary's `extra` entries and
/// the content `data`.
fn page(extra: &str, data: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> {extra} \
             /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{data}\nendstream",
            data.len().saturating_add(1)
        ),
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

/// `bytes`' one page as `pdf_model` interprets it for a screen.
fn interpreted(bytes: Vec<u8>) -> Arc<DisplayList> {
    let document = Document::open(bytes).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
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

/// `list`'s picture through `rasterize` and through the window's frame at the same placement.
fn both_routes(list: &Arc<DisplayList>) -> ([u8; 3], [u8; 3], String) {
    let mut gpu = QuorraRasterizer::new_headless().expect("a device for the window's lane");
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40 by 40 target");
    let whole = gpu
        .rasterize(list, target)
        .expect("the device draws the page");
    let frame = gpu
        .rasterize_frame(&PresentFrame {
            width: target.width,
            height: target.height,
            pages: &[(list, target)],
            raster: None,
            overlays: &[],
        })
        .expect("the window's present draws the page");
    (
        centre(&whole),
        centre(&frame),
        gpu.adapter_description().to_owned(),
    )
}

/// ISO 32000-2 §11.4.7: "All page-level compositing shall be done in the default blending colour
/// space of the page, and the entire result shall then be converted to the native colour space of
/// the output device". A `DeviceCMYK` page group makes the composite four components, which the
/// backends receive as two lists — the complements of cyan, magenta and yellow, and black beside
/// them (`DisplayList::black`) — and `0 0 0 1 k` is all black ink, so the page is dark on every
/// route that converts the composite and white on one that drops the second list.
#[test]
fn the_window_frame_draws_a_four_component_page_groups_black_ink() {
    gpu_lock_hook::install();
    let list = interpreted(page(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "0 0 0 1 k 0 0 40 40 re f",
    ));
    assert!(
        list.blending().is_some() && list.black().is_some(),
        "the interpreter carries the page group's black as a second list"
    );
    let (drawn, shown, adapter) = both_routes(&list);
    assert!(
        drawn.iter().all(|&level| level < 100),
        "control: `rasterize` draws full black ink in a CMYK page group dark: {drawn:?}"
    );
    assert!(
        shown.iter().zip(drawn).all(|(&a, b)| a.abs_diff(b) <= 8),
        "the window's frame of a CMYK page group shows what `rasterize` draws, {drawn:?}: \
         {shown:?} on {adapter}"
    );
}

/// The window's route for a page with no pass is unchanged: the same colour as `rasterize`, red
/// for `1 0 0 rg` — the turn path's own case, which ADR 3701 asks only a predicate of.
#[test]
fn a_page_with_no_pass_is_drawn_by_the_window_as_before() {
    gpu_lock_hook::install();
    let list = interpreted(page("", "1 0 0 rg 0 0 40 40 re f"));
    let (drawn, shown, adapter) = both_routes(&list);
    assert!(
        drawn[0] > 239 && drawn[1] < 16 && drawn[2] < 16,
        "control: `rasterize` draws red: {drawn:?}"
    );
    assert_eq!(shown, drawn, "the window's frame on {adapter}");
}
