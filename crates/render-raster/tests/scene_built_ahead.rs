//! The device's scene built while nobody waits is the scene the next frame keeps (ADR 1898).
//!
//! `QuorraWindowRenderer::prepare` builds a frame's page scene and draws nothing, so that a host
//! can pay the build while the processor's frame is on the window and the next view of the page
//! re-encodes rather than builds (ADR 1877). Two things make that true, and this file holds both:
//!
//! - **a prepared scene is the held scene**: a frame at another view of the same page keeps it, as
//!   it would keep the scene of a frame drawn there, and a second prepare builds nothing;
//! - **a page the processor drew, uploaded, does not take it away**: a raster frame keeps no scene,
//!   so through the page's own lane every upload replaced what the device had built of the page.
//!
//! What a kept scene draws against a fresh build is `lines_across_views.rs`'s claim, held there
//! byte for byte; here the observable is [`render_raster::FrameCost::scene_kept`].

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a device that cannot build a page of lines is the failure this file \
              reports, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Size, Stroke, TargetSpec, Transform,
};
use render_raster::{PresentFrame, QuorraWindowRenderer, WindowTextures};

/// The window every frame is drawn into.
const WINDOW: (u32, u32) = (240, 180);

/// A window's worth of RGBA bytes.
const RASTER_BYTES: usize = 240 * 180 * 4;

/// The page, at one device pixel per user unit at scale one.
const PAGE: Size = Size {
    width: 240.0,
    height: 180.0,
};

/// A window renderer, or the reason the suite fails: a skipped suite verifies nothing (ADR 0004).
fn renderer() -> QuorraWindowRenderer {
    QuorraWindowRenderer::new_headless().unwrap_or_else(|error| {
        panic!(
            "no adapter available for raster: {error}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a software one)."
        )
    })
}

/// A page of solid segments, every one a raster line at every view (ADR 1877).
fn lines() -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    for row in 0..6_u8 {
        let y = 15.0 + 25.0 * f32::from(row);
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(10.0, y)));
        path.push(PathCommand::LineTo(Point::new(230.0, y + 4.0)));
        list.push(Command::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: Stroke {
                width: 0.3 * f32::from(row),
                adjust: false,
                cap: LineCap::Butt,
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
    Arc::new(list)
}

/// The page's placement at `scale`.
fn placed(scale: f32) -> TargetSpec {
    TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: Transform::scale(scale, scale),
    }
}

/// A frame of `list` at `scale`, or of `raster` alone where one is given.
fn frame<'a>(
    pages: &'a [(&'a Arc<DisplayList>, TargetSpec)],
    raster: Option<&'a Raster>,
) -> PresentFrame<'a> {
    PresentFrame {
        width: WINDOW.0,
        height: WINDOW.1,
        pages,
        raster,
        overlays: &[],
    }
}

/// Draws one frame into a pair of the renderer's own textures, and says whether it kept its scene.
fn drawn_keeping(renderer: &mut QuorraWindowRenderer, frame: PresentFrame<'_>) -> bool {
    let page = renderer.layer_texture("the page", WINDOW.0, WINDOW.1);
    let chrome = renderer.layer_texture("the chrome", WINDOW.0, WINDOW.1);
    renderer
        .render(
            frame,
            WindowTextures {
                page: &page,
                chrome: &chrome,
            },
        )
        .expect("the device draws a window of lines");
    renderer.last_frame().scene_kept
}

/// What the processor's page would be: any raster the size of the window.
fn processor_page() -> Raster {
    Raster {
        width: WINDOW.0,
        height: WINDOW.1,
        format: pdf_render::RasterFormat::Rgba8,
        data: vec![200; RASTER_BYTES],
    }
}

/// A scene built ahead at the view on hand is kept by the next view's frame, and a second build
/// ahead of the same frame builds nothing — against a renderer that was not asked, which builds.
#[test]
fn a_scene_built_ahead_is_kept_by_the_next_view() {
    let list = lines();
    let fit = [(&list, placed(1.0))];
    let zoomed = [(&list, placed(1.25))];

    let mut asked = renderer();
    let built = asked
        .prepare(frame(&fit, None))
        .expect("the device builds a page of lines");
    assert!(!built.scene_kept, "the first build ahead builds");
    let again = asked
        .prepare(frame(&fit, None))
        .expect("the device builds a page of lines");
    assert!(again.scene_kept, "a scene held already is not built again");
    assert!(
        drawn_keeping(&mut asked, frame(&zoomed, None)),
        "the next view keeps the scene built ahead of it"
    );

    let mut unasked = renderer();
    assert!(
        !drawn_keeping(&mut unasked, frame(&zoomed, None)),
        "with nothing built ahead the same frame builds its scene"
    );
}

/// A page the processor drew, uploaded between the build and the next view, leaves the device's
/// scene where it was: an upload goes through a lane of its own (ADR 1898).
#[test]
fn an_upload_does_not_take_the_scene_built_ahead_away() {
    let list = lines();
    let fit = [(&list, placed(1.0))];
    let zoomed = [(&list, placed(1.25))];
    let raster = processor_page();

    let mut renderer = renderer();
    renderer
        .prepare(frame(&fit, None))
        .expect("the device builds a page of lines");
    assert!(
        !drawn_keeping(&mut renderer, frame(&[], Some(&raster))),
        "a raster frame keeps no scene"
    );
    assert!(
        drawn_keeping(&mut renderer, frame(&zoomed, None)),
        "the page's scene outlived the processor's frame uploaded over it"
    );
}
