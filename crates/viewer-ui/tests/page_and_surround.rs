//! ISO 32000-2 §11.4.7's 𝑊 stops where the page does, and both backends stop it in the same place.
//!
//! # Why this gate exists
//!
//! `doc/traps/pixels-and-rasterisers.md` trap 2: *a decision either backend can make alone is a
//! decision neither has made*. Where the page's own colour ends and the window's surround begins
//! is exactly such a decision — `render-cpu` composites it per pixel after the page is drawn,
//! `render-quorra` draws it as two rectangles at the bottom of a scene — and until the
//! six-hundred-and-eleventh session there was no boundary at all, because one colour served as
//! both. The gap between two pages of Table 29's `OneColumn` was page white on page white, so a
//! reader could not see where one page ended.
//!
//! # What it checks, and why it needs no reference
//!
//! Two blank pages side by side in one window, with a gap between them. What each pixel should be
//! is stated rather than rendered: §11.4.7 makes an unmarked page 𝑊, which is white, and
//! `pdf_render::medium::SURROUND` is this program's own documented choice for everywhere else. So
//! the three assertions are inside the first page, inside the second, and in the gap — and then
//! the two backends are compared to each other everywhere, which is the trap-2 half.
//!
//! Deleting the separation — `Medium::WINDOW` for `Medium::PAGE_ONLY` at either call site — makes
//! `the_gap_between_two_pages_is_not_the_page` fail on that backend, which is what says this test
//! guards what it claims to.

#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot be set up must fail loudly rather than pass by \
              doing nothing"
)]

use std::sync::Arc;

use pdf_render::{DisplayList, Medium, Raster, Size, TargetSpec, Transform};
use render_quorra::{PresentFrame, QuorraRasterizer};

/// The window this pretends to be, in device pixels.
const WINDOW: (u32, u32) = (240, 120);

/// Each page's extent in user space, drawn at 1:1 — so also its extent in device pixels.
///
/// **Whole numbers throughout this file, and that is a choice rather than laziness.** Every edge
/// then lands on a pixel boundary, so what each sample should be is stated exactly and no
/// assertion needs a tolerance; the sub-pixel case is `pdf_render::medium`'s own
/// `an_edge_inside_a_pixel_mixes_the_two`, which is where it can be checked against arithmetic
/// rather than against a rasteriser.
const PAGE: (u32, u32) = (100, 100);

/// The gap `viewer_core::layout` leaves between two pages of a row, in device pixels.
const GAP: u32 = 8;

/// Where the left page's top-left corner sits in the window.
const LEFT: (u32, u32) = (10, 10);

/// Whole pixels as the floats a placement is stated in.
fn px(pixels: u32) -> f32 {
    f32::from(u16::try_from(pixels).unwrap_or(u16::MAX))
}

/// One blank page, placed with its top-left corner at `origin` in the window.
///
/// The transform is `TargetSpec::for_page`'s — scale then flip about the page's own top edge —
/// with the placement composed onto it, which is exactly what `viewer-ui`'s `arrangement` builds
/// for the frame it hands the render thread.
fn placed(origin: (u32, u32)) -> (Arc<DisplayList>, TargetSpec) {
    let list = Arc::new(DisplayList::new(Size::new(px(PAGE.0), px(PAGE.1))));
    let target = TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: Transform::scale(1.0, -1.0)
            .then(Transform::translate(0.0, px(PAGE.1)))
            .then(Transform::translate(px(origin.0), px(origin.1))),
    };
    (list, target)
}

/// Where the right page's top-left corner sits: one page and one gap to the right of the left one.
fn right_origin() -> (u32, u32) {
    (LEFT.0.saturating_add(PAGE.0).saturating_add(GAP), LEFT.1)
}

/// The pixel `x` across and `y` down, as three channels.
fn at(raster: &Raster, x: u32, y: u32) -> [u8; 3] {
    let index = (y as usize)
        .saturating_mul(WINDOW.0 as usize)
        .saturating_add(x as usize)
        .saturating_mul(4);
    let pixel = raster
        .data
        .get(index..index.saturating_add(3))
        .unwrap_or_else(|| panic!("a {WINDOW:?} raster has a pixel at ({x}, {y})"));
    [pixel[0], pixel[1], pixel[2]]
}

/// `pdf_render::SURROUND` as this composite quantises it, so the expectation is the constant
/// rather than a number copied out of a run.
fn surround_bytes() -> [u8; 3] {
    let level = |component: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a component in 0..=1 scaled by 255"
        )]
        {
            (component * 255.0 + 0.5) as u8
        }
    };
    [
        level(pdf_render::SURROUND.r),
        level(pdf_render::SURROUND.g),
        level(pdf_render::SURROUND.b),
    ]
}

/// The three statements every backend's frame has to satisfy.
fn check(drawn: &Raster, who: &str) {
    let middle = WINDOW.1 / 2;
    let inside_left = LEFT.0.saturating_add(20);
    let inside_right = right_origin().0.saturating_add(20);
    // The gap's own middle column, four pixels from either page's edge.
    let in_the_gap = LEFT.0.saturating_add(PAGE.0).saturating_add(GAP / 2);
    assert_eq!(
        at(drawn, inside_left, middle),
        [255, 255, 255],
        "{who}: an unmarked page is §11.4.7's 𝑊"
    );
    assert_eq!(
        at(drawn, inside_right, middle),
        [255, 255, 255],
        "{who}: the second page has its own 𝑊, not the first page's"
    );
    assert_eq!(
        at(drawn, in_the_gap, middle),
        surround_bytes(),
        "{who}: the gap between two pages is the surround"
    );
    assert_eq!(
        at(drawn, 1, 1),
        surround_bytes(),
        "{who}: so is the window's corner, which no page covers"
    );
}

#[test]
fn the_gap_between_two_pages_is_not_the_page() {
    let (left, left_target) = placed(LEFT);
    let (right, right_target) = placed(right_origin());

    let processor = viewer_ui::software::compose_pages(&[
        (left.as_ref(), left_target),
        (right.as_ref(), right_target),
    ])
    .expect("the correctness oracle draws a two-page arrangement");
    check(&processor, "the correctness oracle");

    let mut gpu = match QuorraRasterizer::new_headless_software() {
        Ok(gpu) => gpu.with_medium(Medium::WINDOW),
        Err(error) => {
            println!("skipped: no software adapter on this machine: {error}");
            return;
        }
    };
    let device = gpu
        .rasterize_frame(&PresentFrame {
            width: WINDOW.0,
            height: WINDOW.1,
            pages: &[(&left, left_target), (&right, right_target)],
            raster: None,
            overlays: &[],
        })
        .unwrap_or_else(|error| panic!("the graphics device refused the arrangement: {error}"));
    check(&device, "the graphics device");

    // And the two agree everywhere rather than only at the four samples. The frame is flat
    // colours with four upright edges in it, so the only pixels two rasterisers may distribute
    // differently are the ones an edge lands in — and every edge here is on a whole pixel, so
    // the bound is the page perimeters rather than a tolerance nobody measured.
    let perimeter = 4 * (PAGE.0 as usize).saturating_add(PAGE.1 as usize);
    let differing = processor
        .data
        .chunks_exact(4)
        .zip(device.data.chunks_exact(4))
        .filter(|(ours, theirs)| ours[0..3] != theirs[0..3])
        .count();
    assert!(
        differing <= perimeter,
        "{differing} pixels differ between the backends, which is more than the {perimeter} \
         either could disagree about at a page's edge"
    );
}
