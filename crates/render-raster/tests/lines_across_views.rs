//! A scene of lines is kept across views, and draws at each view what a scene built there draws
//! (ADR 1877).
//!
//! A one-segment stroke is raster's own primitive at every size (raster's ADR 0097), and the width
//! is resolved per placement (raster's ADR 0085), so a page whose marks are lines is true at every
//! view and a zoom step keeps its scene (ADR 0702). The claim has two halves and this file holds
//! both, byte for byte:
//!
//! - **a kept scene draws the frame a fresh build draws**, at every view of a sequence, on one
//!   device against a device that has drawn nothing else — the only fidelity argument a kept scene
//!   can have;
//! - **the scene is kept where it may be and rebuilt where it may not**: §8.5.3.2's dot is the
//!   one line whose road is this view's question (§10.7.4's substitution below a level), so a dot
//!   a step out takes off its level rebuilds, and a frame that rebuilt for the right reason is
//!   told apart from one that kept for the wrong one by [`render_raster::FrameCost::scene_kept`].

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a device that cannot draw a page of lines is the failure this file \
              reports, and it reports it by name"
)]

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
    QuorraRasterizer::new_headless().unwrap_or_else(|error| {
        panic!(
            "no adapter available for raster: {error}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a software one)."
        )
    })
}

/// One stroke of the segment `from`–`to`, solid black.
fn line(list: &mut DisplayList, (from, to): ((f32, f32), (f32, f32)), width: f32, cap: LineCap) {
    stroke(list, (from, to), width, cap, false);
}

/// One stroke of the segment `from`–`to`, solid black, with §10.7.5's adjustment as asked.
fn stroke(
    list: &mut DisplayList,
    (from, to): ((f32, f32), (f32, f32)),
    width: f32,
    cap: LineCap,
    adjust: bool,
) {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(from.0, from.1)));
    path.push(PathCommand::LineTo(Point::new(to.0, to.1)));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            adjust,
            cap,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
        },
        paint: Paint::Solid(Color::rgb(0.1, 0.1, 0.1)),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
}

/// A page of lines a fit view draws as a fifth of a pixel and a zoom as several: every cap,
/// four directions, widths from §8.4.3.2's zero to six units, and round dots a pixel across or
/// more, which no view in [`VIEWS`] takes under a level.
fn lines() -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    let caps = [LineCap::Butt, LineCap::Round, LineCap::Square];
    let widths = [0.0, 0.15, 0.4, 1.3, 6.0];
    for (row, width) in widths.into_iter().enumerate() {
        for (column, cap) in caps.into_iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "indices under ten")]
            let (x, y) = (12.0 + 70.0 * column as f32, 14.0 + 30.0 * row as f32);
            line(&mut list, ((x, y), (x + 50.0, y)), width, cap);
            line(
                &mut list,
                ((x + 3.3, y + 2.0), (x + 3.3, y + 20.0)),
                width,
                cap,
            );
            line(
                &mut list,
                ((x + 8.0, y + 3.0), (x + 40.0, y + 21.0)),
                width,
                cap,
            );
            // A segment a fifth of a unit long, as the owner's drawing is made of.
            line(
                &mut list,
                ((x + 45.1, y + 8.3), (x + 45.3, y + 8.35)),
                width,
                cap,
            );
        }
    }
    stroke(
        &mut list,
        ((10.0, 170.0), (230.0, 171.5)),
        0.2,
        LineCap::Butt,
        true,
    );
    for (i, width) in [2.0_f32, 3.5, 7.0].into_iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "an index under ten")]
        let x = 30.0 + 60.0 * i as f32;
        line(
            &mut list,
            ((x + 0.37, 160.2), (x + 0.37, 160.2)),
            width,
            LineCap::Round,
        );
    }
    Arc::new(list)
}

/// A page holding one round dot a tenth of a unit across, centred in its pixel at scale one —
/// a level of that pixel there, and under a level of every pixel at half that scale, where
/// §10.7.4 substitutes for it — beside a line, so that the page is otherwise a page of lines.
fn a_dot_a_step_out_takes_off_its_level() -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    line(
        &mut list,
        ((20.0, 20.0), (200.0, 60.0)),
        0.5,
        LineCap::Round,
    );
    line(
        &mut list,
        ((100.5, 75.5), (100.5, 75.5)),
        0.1,
        LineCap::Round,
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
        .expect("the device draws a page of lines");
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

/// Every view of the sequence keeps the first view's scene and draws, byte for byte, what a
/// device that builds the scene at that view draws.
#[test]
fn a_scene_of_lines_is_kept_across_views_and_draws_what_a_fresh_build_draws() {
    let list = lines();
    let mut kept = backend();
    for (step, view) in VIEWS.into_iter().enumerate() {
        let (drawn, scene_kept) = frame(&mut kept, &list, placed(view));
        assert_eq!(
            scene_kept,
            step > 0,
            "view {step} ({view:?}): a page of lines keeps the scene it built first"
        );
        let (fresh, _) = frame(&mut backend(), &list, placed(view));
        identical(&format!("view {step} ({view:?})"), &drawn, &fresh);
        assert!(
            drawn.data.chunks_exact(4).any(|p| p[0] < 128),
            "view {step}: the lines are on the page"
        );
    }
}

/// A dot a step out takes off its level rebuilds the scene there, and the frame is still the
/// fresh build's; the scale it came from is kept again only by a build that read no view.
#[test]
fn a_dot_that_changes_its_road_rebuilds_the_scene() {
    let list = a_dot_a_step_out_takes_off_its_level();
    let mut kept = backend();
    let fit = (1.0, (0.0, 0.0));
    let (_, scene_kept) = frame(&mut kept, &list, placed(fit));
    assert!(!scene_kept, "the first frame builds");
    let (_, scene_kept) = frame(&mut kept, &list, placed((1.4, (-30.0, -20.0))));
    assert!(scene_kept, "a step in keeps the dot a line, and the scene");
    let out = (0.5, (10.0, 5.0));
    let (drawn, scene_kept) = frame(&mut kept, &list, placed(out));
    assert!(
        !scene_kept,
        "a step out takes the dot under a level, so the scene is rebuilt"
    );
    let (fresh, _) = frame(&mut backend(), &list, placed(out));
    identical("half the scale", &drawn, &fresh);
}
