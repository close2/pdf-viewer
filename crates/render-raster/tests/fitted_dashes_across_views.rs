//! A dashed stroke under `/SA` whose fit a view declined is kept at another view only where that
//! view declines the fit too, and draws there what a scene built there draws (ADR 2902).
//!
//! ISO 32000-2 §10.7.5:
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness. The thickness
//! > shall be as near as possible to the requested line width -no more than half a pixel
//! > different.
//!
//! Whether a stroke needs adjusting is a question about the device grid — its width in device
//! pixels and where its coordinates fall among them — so `pdf_render::grid_fit` takes a thin rule
//! along the device axes at one view and declines it at another. §8.5.3.2 cuts the dash pattern
//! along the path the stroke follows, and the fit moves a corner along the segments it ends, so a
//! dashed rectangle the fit takes is dashed along other lengths than the same rectangle the fit
//! declines. A scene built where the fit declined is therefore that view's scene and no other's:
//!
//! - **a zoom that brings the width under the fit's bound rebuilds** — 0.6 units are 2.4 device
//!   pixels at scale 4, which the fit leaves alone, and 0.6 at scale 1, which it fits to one pixel
//!   through pixel centres;
//! - **a scroll that takes a rule off the pixel centres it lay on rebuilds** — a one-unit
//!   rectangle on pixel centres at scale 1 is already what the fit makes, and a quarter pixel away
//!   it is not;
//! - **a scroll the fit declines at as well keeps the scene**, and each kept frame is byte for byte
//!   the frame of a device that built the scene at that view.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a device that cannot draw a dashed rectangle is the failure this file \
              reports, and it reports it by name"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Size, Stroke, TargetSpec, Transform,
};
use render_raster::{PresentFrame, QuorraRasterizer};

/// The window every frame is drawn into.
const WINDOW: (u32, u32) = (240, 180);

/// The page, at one device pixel per user unit at scale one.
const PAGE: Size = Size {
    width: 240.0,
    height: 180.0,
};

/// A backend, or the reason the suite fails: a skipped suite verifies nothing (ADR 0004).
fn backend() -> QuorraRasterizer {
    gpu_lock_hook::install();
    QuorraRasterizer::new_headless().unwrap_or_else(|error| {
        panic!(
            "no adapter available for raster: {error}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a software one)."
        )
    })
}

/// A page holding one closed rectangle from `(x0, y0)` to `(x1, y1)`, stroked `width` wide under
/// `/SA` with the dash pattern `[3 2] 0`, butt caps and mitred corners.
fn dashed_rectangle((x0, y0): (f32, f32), (x1, y1): (f32, f32), width: f32) -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            adjust: true,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: vec![3.0, 2.0],
            dash_phase: 0.0,
        },
        paint: Paint::Solid(Color::rgb(0.1, 0.1, 0.1)),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    Arc::new(list)
}

/// The placement of the page at a view of `scale` whose page origin sits at the window offset
/// `(dx, dy)`, the y axis turned down as a page placement turns it.
fn placed((scale, (dx, dy)): (f32, (f32, f32))) -> TargetSpec {
    TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: Transform::scale(scale, -scale)
            .then(Transform::translate(dx, scale.mul_add(PAGE.height, dy))),
    }
}

/// One frame of `list` at `view`, and whether it kept the scene of the frame before.
fn frame(gpu: &mut QuorraRasterizer, list: &Arc<DisplayList>, view: TargetSpec) -> (Raster, bool) {
    let drawn = gpu
        .rasterize_frame(&PresentFrame {
            width: WINDOW.0,
            height: WINDOW.1,
            pages: &[(list, view)],
            raster: None,
            overlays: &[],
        })
        .expect("the device draws a dashed rectangle");
    (drawn, gpu.last_frame().scene_kept)
}

/// The frame at `view` keeps (or rebuilds, as `kept` says) and is the bytes of a fresh build.
fn step(gpu: &mut QuorraRasterizer, list: &Arc<DisplayList>, view: (f32, (f32, f32)), kept: bool) {
    let (drawn, scene_kept) = frame(gpu, list, placed(view));
    let (fresh, _) = frame(&mut backend(), list, placed(view));
    let differing = drawn
        .data
        .iter()
        .zip(&fresh.data)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing,
        0,
        "view {view:?}: {differing} of {} bytes differ from a scene built at this view",
        drawn.data.len()
    );
    assert_eq!(scene_kept, kept, "view {view:?}: whether the scene is kept");
    assert!(
        drawn.data.chunks_exact(4).any(|p| p[0] < 255),
        "view {view:?}: the rectangle is on the page"
    );
}

/// At scale 4 the 0.6-unit width is 2.4 device pixels, at or over `pdf_render::FIT_BELOW`, and
/// the fit declines; a scroll there declines it too and keeps the scene. At scale 1 the width is
/// 0.6 of a pixel and the corners, a fifth of a unit off the pixel centres, move to them along the
/// segments they end — so the dashes after each corner are cut along other lengths, and the step
/// rebuilds.
#[test]
fn a_zoom_that_brings_a_dashed_rule_under_the_fit_rebuilds_it() {
    let list = dashed_rectangle((20.2, 20.2), (60.2, 50.2), 0.6);
    let mut kept = backend();
    step(&mut kept, &list, (4.0, (-60.0, -490.0)), false);
    step(&mut kept, &list, (4.0, (-60.25, -489.5)), true);
    step(&mut kept, &list, (1.0, (0.0, 0.0)), false);
}

/// A one-unit rectangle with its corners on pixel centres at scale 1 is already what the fit makes,
/// and the fit declines it; a scroll of a quarter pixel takes the corners off the centres, where
/// the fit moves them back along their segments, and the scroll rebuilds.
#[test]
fn a_scroll_that_takes_a_dashed_rule_off_the_pixel_centres_rebuilds_it() {
    let list = dashed_rectangle((20.5, 20.5), (60.5, 50.5), 1.0);
    let mut kept = backend();
    step(&mut kept, &list, (1.0, (0.0, 0.0)), false);
    step(&mut kept, &list, (1.0, (0.25, -0.25)), false);
}
