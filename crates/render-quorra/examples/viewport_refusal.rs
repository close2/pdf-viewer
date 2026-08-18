//! Whether a page the 4× gate refuses is also refused by the *product*, whose target is a window.
//!
//! `tests/corpus.rs` draws every corpus page twice — at the page's own scale and at four times it
//! — and both runs ask for **one target the size of the whole magnified page**. A viewer does not:
//! `viewer-ui`'s `surface.rs` keeps the magnification in the transform and replaces the extent
//! with the window's, so a page four times too big for the screen is drawn into a window-sized
//! frame with most of it off the edge. Those are two different requests of the same device, and
//! `REFUSED_BY_THE_DEVICE_AT_FOUR` is a statement about the first of them only.
//!
//! The quorra developers asked which of the two a named refusal belongs to — "it's a whole page at
//! 4× in one target, and a viewer's viewport is its window" — and the answer is a measurement
//! rather than a reading of either code path. This takes it: the same display list, the same
//! magnification, the same coverage lane, twice, differing only in the frame's extent and the
//! translation that places the page inside it.
//!
//! ```sh
//! cargo run --release -p render-quorra --example viewport_refusal -- <file.pdf> [page] [scale] [w] [h]
//! ```
//!
//! Three window frames are drawn rather than one, because where the page sits in the window is
//! part of the request: a device-space bound is a *coordinate* and not only an extent, so the
//! corner of the page a person has scrolled to could decide the answer. Top-left, centre and
//! bottom-right are the three, from `viewer_core`'s own centring-and-scrolling rule.
//!
//! # What `zoom_ladder.rs` already does, and what this adds
//!
//! [`zoom_ladder`](../zoom_ladder.rs) walks the same window-shaped transform up a ladder of
//! magnifications and compares the two backends on each rung; `doc/QUORRA_FEEDBACK.md` section
//! 28.7 answered this question from its table once already. Three things are here that are not
//! there. **The gate's own target is drawn in the same run**, so the contrast is a line of output
//! rather than an inference across two tools and two invocations. **The scroll position is
//! swept**, which is the confound that answer had to disclaim in prose. And **the marked share is
//! counted**, so
//! that "the device took the frame" is not silently read as "the page is on it" — a ladder rung
//! whose window holds a blank corner of a magnified page draws nothing and refuses nothing, and
//! those two are only distinguishable if somebody counts the pixels.
//!
//! The lane is [`quorra_gpu::Coverage::Cpu`], which is what `viewer-ui` draws with below its
//! `GPU_COVERAGE_MAGNIFICATION` of ten and what the corpus ratchet is measured on.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::sync::Arc;

use pdf_render::{DisplayList, TargetSpec, Transform};
use pdf_syntax::Document;
use render_quorra::{PresentFrame, QuorraRasterizer};

/// Draws one frame and answers what share of it was marked, or what the device said instead.
///
/// **The share is the point rather than decoration.** Trap 12b is a vello scene that overflowed a
/// device buffer, set a flag, stopped filling and returned `Ok(())` over a blank target — so
/// "the device took the frame" is not the same claim as "the page is on it", and a question about
/// a refusal answered with the first would be answered with the wrong one. A pixel counts as
/// marked when it differs from the medium the backend imposed, which is white here.
fn attempt(
    backend: &mut QuorraRasterizer,
    list: &Arc<DisplayList>,
    target: TargetSpec,
) -> Result<f64, String> {
    let frame = PresentFrame {
        width: target.width,
        height: target.height,
        page: Some((list, target)),
        raster: None,
        overlays: &[],
    };
    let raster = backend
        .rasterize_frame(&frame)
        .map_err(|error| error.to_string())?;
    let pixels = raster.data.chunks_exact(4);
    let total = pixels.len();
    let marked = raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[..3] != [u8::MAX; 3])
        .count();
    Ok(if total == 0 {
        0.0
    } else {
        marked as f64 / total as f64
    })
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to draw");
    let index: usize = arguments
        .next()
        .map_or(1, |n| n.parse().expect("a page number"));
    let scale: f32 = arguments
        .next()
        .map_or(4.0, |s| s.parse().expect("a scale"));
    let window: (u32, u32) = (
        arguments
            .next()
            .map_or(1600, |w| w.parse().expect("a window width")),
        arguments
            .next()
            .map_or(1000, |h| h.parse().expect("a window height")),
    );

    let document =
        Document::open(std::fs::read(&path).expect("the document is readable")).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(index.saturating_sub(1))
        .expect("that page exists");
    let list = Arc::new(pdf_model::content::interpret(&document, &page).display_list);

    // The gate's own request: `TargetSpec::for_page`, whole page, no viewport at all. The budget
    // is this example's rather than the gate's, so that the refusal measured is the device's and
    // never `TargetSpec`'s arithmetic — which would answer a different question.
    let whole = TargetSpec::for_page(&list, scale, u64::MAX).expect("a target");

    let mut backend = QuorraRasterizer::new_headless().expect("an adapter");
    backend.set_coverage(quorra_gpu::Coverage::Cpu);
    println!(
        "{path} page {index} at {scale}×, {}",
        backend.adapter_description()
    );
    println!(
        "page {:.2} × {:.2} units → {} × {} px magnified; window {} × {}",
        list.page_size.width, list.page_size.height, whole.width, whole.height, window.0, window.1
    );

    let verdict = |result: &Result<f64, String>| match result {
        Ok(marked) => format!("drawn, {:.1}% of the frame marked", marked * 100.0),
        Err(problem) => format!("REFUSED — {problem}"),
    };

    let gate = attempt(&mut backend, &list, whole);
    println!("  whole page in one target   {}", verdict(&gate));

    // Where `viewer_core::Open::origin` puts the raster's top-left corner in the viewport: zero
    // while unscrolled, and down to `viewport − raster` at the far end. Negative, because the
    // page is larger than the window in both axes at any magnification worth asking about.
    let far = |viewport: u32, raster: u32| -(raster.saturating_sub(viewport) as f32);
    for (name, origin) in [
        ("top-left", (0.0, 0.0)),
        (
            "centre",
            (
                far(window.0, whole.width) / 2.0,
                far(window.1, whole.height) / 2.0,
            ),
        ),
        (
            "bottom-right",
            (far(window.0, whole.width), far(window.1, whole.height)),
        ),
    ] {
        let target = TargetSpec {
            width: window.0,
            height: window.1,
            transform: whole
                .transform
                .then(Transform::translate(origin.0, origin.1)),
        };
        let result = attempt(&mut backend, &list, target);
        println!(
            "  window, scrolled {name:<13}{} (origin {:.0}, {:.0})",
            verdict(&result),
            origin.0,
            origin.1
        );
    }
}
