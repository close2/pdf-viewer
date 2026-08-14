//! ISO 32000-2 §12.4.4's transition frame, drawn by both backends and by neither differently.
//!
//! # Why this gate exists
//!
//! A transition frame is the one thing a window draws that is *not* a page: two page rasters,
//! placed and clipped by [`viewer_core::transition`]. `CLAUDE.md` keeps the CPU backend as the
//! correctness oracle and as the frame the graphics device refuses, so an animation only the
//! device could draw would quietly cost that — a slide show that plays on one machine and cuts
//! on the next. This draws the same frame through both and compares them.
//!
//! # What it checks, and why it needs no reference
//!
//! The two pages are flat colours, so what a frame *should* be at a fraction of the way through
//! is arithmetic rather than a rendering: Table 164's `Wipe` at `/Di 0` is "[l]eft to right", so
//! the left `progress` of the window is the page moved to and the rest is the page being left.
//! Both backends are asked, and both are checked against that closed form as well as against
//! each other.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot be set up must fail loudly rather than pass by \
              doing nothing"
)]

use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_render::{
    Image, Point, Raster, RasterFormat, Rasterizer as _, Rect, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use render_quorra::{PresentFrame, QuorraRasterizer};

/// The window this pretends to be.
const WINDOW: (u32, u32) = (240, 120);

/// How far through the transition the frame is.
const PROGRESS: f32 = 0.25;

/// Where the sweeping line therefore is, in device pixels: a quarter of the window's width.
const LINE: u32 = 60;

/// How far from the line a sample is taken, so that no assertion rests on the pixel the line
/// itself lands in — which is a rasteriser's business and not this clause's.
const CLEAR: u32 = 3;

/// A page of two flat colours, top half then bottom half, as a host's rasteriser hands it over.
///
/// **Two halves rather than one colour, and that is the assertion that found a defect.** A page
/// raster's first row is its *top* one, and a `pdf_render::Command::Image` draws "the unit
/// square in user space, with the image's top row at y = 1" — PDF's y grows upward. A frame
/// drawn without that flip stands every page on its head, which a page of one colour cannot see
/// and which the window showed at once.
fn page(top: [u8; 4], bottom: [u8; 4]) -> Image {
    let half = (WINDOW.0 as usize).saturating_mul(WINDOW.1 as usize / 2);
    let mut data = top.repeat(half);
    data.extend_from_slice(&bottom.repeat(half));
    let raster = Raster {
        width: WINDOW.0,
        height: WINDOW.1,
        format: RasterFormat::Rgba8,
        data,
    };
    viewer_core::transition::drawable(&raster).expect("an Rgba8 page is drawable")
}

/// Table 164's `Wipe`, left to right, with the table's defaults everywhere else.
fn wipe() -> Transition {
    Transition {
        style: Style::Wipe,
        duration: 1.0,
        dimension: Dimension::Horizontal,
        motion: Motion::Inward,
        direction: Direction::Degrees(0.0),
        scale: 1.0,
        opaque: false,
    }
}

/// The pixel `x` across and `y` down.
fn at(raster: &Raster, x: u32, y: u32) -> [u8; 3] {
    let row = y as usize;
    let index = row
        .saturating_mul(WINDOW.0 as usize)
        .saturating_add(x as usize)
        .saturating_mul(4);
    let pixel = raster
        .data
        .get(index..index.saturating_add(3))
        .unwrap_or_else(|| panic!("a {WINDOW:?} raster has a pixel at ({x}, {y})"));
    [pixel[0], pixel[1], pixel[2]]
}

/// What a `Wipe` a quarter of the way through has to look like, whoever drew it.
///
/// Three statements: the swept side is the page moved to, the rest is the page being left, and
/// the page being left is the way up it was rasterised.
fn check(drawn: &Raster, who: &str) {
    let quarter = WINDOW.1 / 4;
    let three_quarters = quarter.saturating_mul(3);
    assert_eq!(
        at(drawn, LINE.saturating_sub(CLEAR), quarter),
        [0, 0, 255],
        "{who}: the swept side is the page moved to"
    );
    assert_eq!(
        at(drawn, LINE.saturating_add(CLEAR), quarter),
        [255, 0, 0],
        "{who}: the rest is the page being left, top half first"
    );
    assert_eq!(
        at(drawn, LINE.saturating_add(CLEAR), three_quarters),
        [255, 255, 0],
        "{who}: and the page being left is not upside down"
    );
}

#[test]
fn both_backends_draw_the_same_transition_frame() {
    let viewport = Rect::from_corners(
        Point::new(0.0, 0.0),
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's extent in pixels, which is hundreds"
        )]
        Point::new(WINDOW.0 as f32, WINDOW.1 as f32),
    );
    // The page being left is red over yellow; the page moved to is blue throughout.
    let (outgoing, incoming) = (
        page([255, 0, 0, 255], [255, 255, 0, 255]),
        page([0, 0, 255, 255], [0, 0, 255, 255]),
    );
    let frame =
        viewer_core::transition::frame(&wipe(), viewport, PROGRESS).expect("a /Wipe is shaped");
    // In an `Arc` because that is how a window's frame is handed to the presenter: the page's
    // identity is what a reused scene is keyed on, so it is an identity that can be pinned.
    let list = std::sync::Arc::new(
        frame
            .draw(viewport, &outgoing, &incoming)
            .expect("two images and one clip"),
    );
    let target = TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: Transform::IDENTITY,
    };

    let processor = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the correctness oracle draws a transition frame");
    check(&processor, "the correctness oracle");

    let mut gpu = match QuorraRasterizer::new_headless_software() {
        Ok(gpu) => gpu,
        Err(error) => {
            println!("skipped: no software adapter on this machine: {error}");
            return;
        }
    };
    let device = gpu
        .rasterize_frame(&PresentFrame {
            width: WINDOW.0,
            height: WINDOW.1,
            page: Some((&list, target)),
            raster: None,
            overlays: &[],
        })
        .unwrap_or_else(|error| panic!("the graphics device refused a transition frame: {error}"));
    check(&device, "the graphics device");

    // And the two agree everywhere, not only at the two samples: a frame of flat colours has no
    // antialiased edge for two rasterisers to distribute differently except at the line itself,
    // which is one column of 240. **Measured at 0 in the three-hundred-and-ninety-third
    // session** — the line falls at exactly 60.0 at this progress, so there is no partial pixel
    // even there — and the bound is a column rather than zero because a fraction of a pixel is a
    // rasteriser's business and not this clause's.
    let differing = processor
        .data
        .chunks_exact(4)
        .zip(device.data.chunks_exact(4))
        .filter(|(ours, theirs)| ours[0..3] != theirs[0..3])
        .count();
    let column = WINDOW.1 as usize;
    assert!(
        differing <= column,
        "{differing} pixels differ, which is more than the one column the line lands in"
    );
}
