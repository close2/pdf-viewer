//! The idle build of a window opened zoomed is the frame it repeats, and the step from it keeps
//! the page's lines (ADR 2134).
//!
//! A page's first frame in a window that shows part of it culls the lines the window does not
//! show, so its encode keeps no line runs and every step from it walks the page's lines again (ADR
//! 2056). `QuorraRasterizer::build_whole` is the host's answer, made while nobody waits: the same
//! frame walked again culling none. This file holds the two things that make it worth having and
//! safe to take — it draws the pixels of the frame it repeats, and the step after it sends no line
//! byte and draws what a fresh backend draws at that view — and that it is not built twice.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a device that cannot draw a page of lines is the failure this file \
              reports, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path,
    PathCommand, Point, Size, Stroke, TargetSpec, Transform,
};
use render_raster::{PresentFrame, QuorraRasterizer};

/// The window every frame is drawn into.
const WINDOW: (u32, u32) = (200, 150);

/// The window's middle, in its own pixels.
const WINDOW_MIDDLE: (f32, f32) = (100.0, 75.0);

/// The page, at one device pixel per user unit at scale one.
const PAGE: Size = Size {
    width: 400.0,
    height: 300.0,
};

/// A backend, or the reason the suite fails: a skipped suite verifies nothing (ADR 0004).
fn backend() -> QuorraRasterizer {
    let mut backend = QuorraRasterizer::new_headless().unwrap_or_else(|error| {
        panic!(
            "no adapter available for raster: {error}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a software one)."
        )
    });
    // The lane the window takes on a moved view, which is the step the idle build is for.
    backend.set_coverage(raster_gpu::Coverage::Compute);
    backend
}

/// A page of solid segments spread over the whole page, every one a raster line at every view —
/// and, where `walked`, one fill under a triangle's clip, a residue no record replays, so that a
/// step walks the page rather than its records (ADR 0087).
fn lines(walked: bool) -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    if walked {
        let mut triangle = Path::new();
        triangle.push(PathCommand::MoveTo(Point::new(150.0, 100.0)));
        triangle.push(PathCommand::LineTo(Point::new(260.0, 120.0)));
        triangle.push(PathCommand::LineTo(Point::new(190.0, 210.0)));
        triangle.push(PathCommand::Close);
        let clip = list
            .add_clip(Clip {
                path: triangle,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("one clip fits in a display list");
        let mut square = Path::new();
        square.push(PathCommand::MoveTo(Point::new(140.0, 90.0)));
        square.push(PathCommand::LineTo(Point::new(270.0, 90.0)));
        square.push(PathCommand::LineTo(Point::new(270.0, 220.0)));
        square.push(PathCommand::LineTo(Point::new(140.0, 220.0)));
        square.push(PathCommand::Close);
        list.push(Command::Fill {
            path: Arc::new(square),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::rgb(0.9, 0.6, 0.2)),
            clip: Some(clip),
            mask: None,
            blend: BlendMode::Normal,
        });
    }
    for row in 0..12_u8 {
        for column in 0..4_u8 {
            let (x, y) = (
                10.0 + 98.0 * f32::from(column),
                12.0 + 24.0 * f32::from(row),
            );
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(x, y)));
            path.push(PathCommand::LineTo(Point::new(
                x + 80.0,
                y + f32::from(row % 4),
            )));
            list.push(Command::Stroke {
                path: Arc::new(path),
                transform: Transform::IDENTITY,
                stroke: Stroke {
                    width: 0.4 + 0.3 * f32::from(column),
                    adjust: false,
                    cap: LineCap::Butt,
                    join: LineJoin::Miter,
                    miter_limit: 10.0,
                    dash_array: Vec::new(),
                    dash_phase: 0.0,
                },
                paint: Paint::Solid(Color::rgb(0.1, 0.2 * f32::from(row % 5), 0.3)),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
    }
    Arc::new(list)
}

/// The window at `zoom` over the page's own scale, its middle on the page's middle.
fn placed(list: &DisplayList, zoom: f32) -> TargetSpec {
    let page = TargetSpec::for_page(list, zoom, 1 << 30).expect("a target");
    let (width, height) = WINDOW;
    TargetSpec {
        width,
        height,
        transform: page.transform.then(Transform::translate(
            WINDOW_MIDDLE.0 - PAGE.width * zoom * 0.5,
            WINDOW_MIDDLE.1 - PAGE.height * zoom * 0.5,
        )),
    }
}

fn frame<'a>(pages: &'a [(&'a Arc<DisplayList>, TargetSpec)]) -> PresentFrame<'a> {
    PresentFrame {
        width: WINDOW.0,
        height: WINDOW.1,
        pages,
        raster: None,
        overlays: &[],
    }
}

/// **The idle build draws the frame it repeats, and the step after it sends no line byte and draws
/// a fresh backend's frame**; a second idle build builds nothing.
#[test]
fn the_idle_build_is_the_frame_it_repeats_and_the_step_after_it_keeps_the_lines() {
    let list = lines(true);
    let opened = [(&list, placed(&list, 2.4))];
    let mut window = backend();
    let first = window
        .rasterize_frame(&frame(&opened))
        .expect("the first frame draws");
    assert!(
        window.last_frame().commands_culled > 0,
        "the window opened zoomed culls the page's lines it does not show"
    );
    let whole = window
        .build_whole(&frame(&opened))
        .expect("the idle build draws")
        .expect("a first frame that culled is built whole");
    assert_eq!(
        whole.data, first.data,
        "the idle build is the frame it repeats"
    );
    assert!(
        window
            .build_whole(&frame(&opened))
            .expect("asked again")
            .is_none(),
        "an encode that keeps its lines is not built again"
    );
    let stepped_to = [(&list, placed(&list, 3.0))];
    let stepped = window
        .rasterize_frame(&frame(&stepped_to))
        .expect("the step draws");
    assert_eq!(
        window.last_frame().streams.line_instances,
        0,
        "the step sends none of the line stream"
    );
    assert!(
        window.last_counters().lines_kept > 0,
        "the step keeps the lines in runs"
    );
    let fresh = backend()
        .rasterize_frame(&frame(&stepped_to))
        .expect("a fresh backend draws the step's view");
    assert_eq!(
        stepped.data, fresh.data,
        "the step draws a fresh backend's frame"
    );
}

/// **A window that shows the whole page keeps its lines already**, so the idle build builds
/// nothing there.
#[test]
fn a_window_that_shows_the_whole_page_is_not_built_again() {
    let list = lines(true);
    let fit = [(&list, placed(&list, 0.45))];
    let mut window = backend();
    window
        .rasterize_frame(&frame(&fit))
        .expect("the first frame draws");
    assert_eq!(
        window.last_frame().commands_culled,
        0,
        "nothing is culled at the fit"
    );
    assert!(
        window
            .build_whole(&frame(&fit))
            .expect("the idle build is asked")
            .is_none()
    );
}

/// **A page whose step replays its records is not built whole**: the record replay walks every
/// line again and keeps no runs (ADR 0087), so the build would be paid for nothing, and it is
/// declined once.
#[test]
fn a_page_the_step_replays_from_its_records_is_not_built_whole() {
    let list = lines(false);
    let opened = [(&list, placed(&list, 2.4))];
    let mut window = backend();
    window
        .rasterize_frame(&frame(&opened))
        .expect("the first frame draws");
    assert!(window.last_frame().commands_culled > 0);
    assert!(
        window
            .build_whole(&frame(&opened))
            .expect("the idle build is asked")
            .is_none()
    );
}
