//! A scene holding §8.5.3.2's marks is kept across views wherever a build there states the same
//! marks, and draws at each view what a scene built there draws (ADR 2884).
//!
//! ISO 32000-2 §8.5.3.2 makes two kinds of mark out of a stroke with no length: a degenerate
//! subpath is "a filled circle centred at the single point" under round caps, and of a zero-length
//! dash "the line caps shall always be painted, since their orientation is determined by the
//! direction of the underlying path". Both are shapes of the line's width, and §10.7.4 — "no
//! shape ever disappears as a result of unfavourable placement relative to the device pixel grid"
//! — is what restates one too small for the raster to hold (`pdf_render::stated_point_mark`).
//! Those two answers, the width and the restatement, are all of the view the marks read, so:
//!
//! - **a page of marks at least a device pixel across at every view of a sequence keeps its first
//!   scene**, because §10.7.4 restates nothing at or above the device's quantum, and each kept
//!   frame is byte for byte the frame of a device that built the scene at that view;
//! - **a step that takes the marks under a device pixel rebuilds**, because the restated width is
//!   one device pixel there and the build's was the document's; and a scroll at that scale keeps
//!   the restated marks, whose width is the same pixel at every translation;
//! - **a mark restated as the device pixel its centre lies in** — §10.7.4's "let i = floor( x )
//!   and j = floor( y )" — names this grid's pixel, so even a scroll rebuilds it.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a device that cannot draw a page of marks is the failure this file \
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

/// One stroke of `points` as one path — each pair a subpath `m l` — solid, under `cap` and the
/// dash `array`.
fn stroke(
    list: &mut DisplayList,
    subpaths: &[[(f32, f32); 2]],
    width: f32,
    cap: LineCap,
    array: &[f32],
) {
    let mut path = Path::new();
    for [from, to] in subpaths {
        path.push(PathCommand::MoveTo(Point::new(from.0, from.1)));
        path.push(PathCommand::LineTo(Point::new(to.0, to.1)));
    }
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            adjust: false,
            cap,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: array.to_vec(),
            dash_phase: 0.0,
        },
        paint: Paint::Solid(Color::rgb(0.1, 0.1, 0.1)),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
}

/// A page of every mark §8.5.3.2 makes, all `width` wide: dotted rules (`[0 w·3] 0 d` under round
/// caps) along both axes and a diagonal, the same under projecting square caps — each mark a
/// square facing along its rule — a dashed rule whose dashes alternate with zero-length ones, and
/// a path whose degenerate subpaths sit between two segments of length.
fn marks(width: f32) -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    let gap = width * 3.0;
    for (row, cap) in [LineCap::Round, LineCap::Square].into_iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "an index under ten")]
        let y = 20.0 + 40.0 * row as f32;
        stroke(
            &mut list,
            &[[(12.3, y), (200.3, y)]],
            width,
            cap,
            &[0.0, gap],
        );
        stroke(
            &mut list,
            &[[(210.6, y), (210.6, y + 30.0)]],
            width,
            cap,
            &[0.0, gap],
        );
        stroke(
            &mut list,
            &[[(14.0, y + 6.0), (180.0, y + 30.0)]],
            width,
            cap,
            &[0.0, gap],
        );
    }
    stroke(
        &mut list,
        &[[(12.0, 110.2), (220.0, 110.2)]],
        width,
        LineCap::Round,
        &[gap, gap, 0.0, gap],
    );
    stroke(
        &mut list,
        &[
            [(20.0, 140.0), (60.0, 140.0)],
            [(80.4, 140.4), (80.4, 140.4)],
            [(100.7, 141.1), (100.7, 141.1)],
            [(120.0, 150.0), (200.0, 160.0)],
        ],
        width,
        LineCap::Round,
        &[],
    );
    Arc::new(list)
}

/// The views of the sequence: scale, and the window offset of the page's origin.
const VIEWS: [(f32, (f32, f32)); 6] = [
    (1.0, (0.0, 0.0)),
    (1.25, (-11.0, -7.5)),
    (1.7, (-40.25, -31.0)),
    (3.0, (-120.0, -90.0)),
    (0.6, (20.0, 15.0)),
    (1.0, (0.5, 0.25)),
];

/// The placement of the page at one view, the y axis turned down as a page placement turns it.
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
        .expect("the device draws a page of marks");
    (drawn, gpu.last_frame().scene_kept)
}

/// Asserts two frames are the same bytes, and says how many differ where they are not.
fn identical(what: &str, kept: &Raster, fresh: &Raster) {
    let differing = kept
        .data
        .iter()
        .zip(&fresh.data)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing,
        0,
        "{what}: {differing} of {} bytes differ from a scene built at this view",
        kept.data.len()
    );
}

/// The frame at `view` keeps (or rebuilds, as `kept` says) and is the bytes of a fresh build.
fn step(gpu: &mut QuorraRasterizer, list: &Arc<DisplayList>, view: (f32, (f32, f32)), kept: bool) {
    let (drawn, scene_kept) = frame(gpu, list, placed(view));
    assert_eq!(scene_kept, kept, "view {view:?}: whether the scene is kept");
    let (fresh, _) = frame(&mut backend(), list, placed(view));
    identical(&format!("view {view:?}"), &drawn, &fresh);
    assert!(
        drawn.data.chunks_exact(4).any(|p| p[0] < 255),
        "view {view:?}: the marks are on the page"
    );
}

/// Marks 2.5 units across are at least 1.5 device pixels at every view of the sequence, where
/// §10.7.4 restates nothing: every view keeps the first view's scene and draws a fresh build's
/// bytes.
#[test]
fn a_page_of_marks_a_pixel_across_is_kept_across_views() {
    let list = marks(2.5);
    let mut kept = backend();
    for (index, view) in VIEWS.into_iter().enumerate() {
        step(&mut kept, &list, view, index > 0);
    }
}

/// Marks 0.6 units across are 1.2 device pixels at scale 2 and are stated there; at scale 1 they
/// are 0.6, where §10.7.4 restates each one a device pixel across, so the step rebuilds — and a
/// scroll at scale 1 keeps the restated marks, whose width is that pixel at every translation.
#[test]
fn a_step_that_restates_the_marks_rebuilds_and_a_scroll_keeps_them() {
    let list = marks(0.6);
    let mut kept = backend();
    step(&mut kept, &list, (2.0, (-100.0, -80.0)), false);
    step(&mut kept, &list, (1.0, (0.0, 0.0)), false);
    step(&mut kept, &list, (1.0, (-3.25, 2.5)), true);
}

/// Marks 0.02 units across are restated as the device pixel each centre lies in at scale 1, which
/// is this grid's pixel: a scroll of a quarter pixel rebuilds them.
#[test]
fn marks_restated_as_their_pixels_rebuild_on_a_scroll() {
    let list = marks(0.02);
    let mut kept = backend();
    step(&mut kept, &list, (1.0, (0.0, 0.0)), false);
    step(&mut kept, &list, (1.0, (0.25, -0.25)), false);
}
