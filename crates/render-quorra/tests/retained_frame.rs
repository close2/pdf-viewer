//! The frame that is not built again, and every way it must refuse to be reused (ADR 0351).
//!
//! A retained scene is a machine for drawing a *plausible* wrong page: nothing about a stale
//! frame looks like a failure, so the only defence is to enumerate the inputs and assert each one
//! separately. That is what this file is. Two claims, and the second is the one with teeth:
//!
//! - **a replayed frame is byte-identical to the encode it replaces**, so reuse is never a
//!   fidelity decision — if this held only approximately, every gate in this tree would be
//!   measuring a different renderer from the one a person uses;
//! - **every input the scene was built from misses when it moves**, one test per input, each
//!   also checking that what is drawn after the change is the *new* page rather than the old
//!   one. A miss that drew the right page for the wrong reason would pass the first half alone.
//!
//! quorra owns the other side of this — its own `tests/retained_frame.rs` enumerates what an
//! *encode* reads, viewport and coverage lane and resource generation among them — and this file
//! deliberately does not restate it. What it covers is the key this crate invented: which frames
//! of a *window* are the same frame.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a device that cannot draw four rectangles is the failure this file \
              reports, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Size, TargetSpec, Transform,
};
use quorra_gpu::EncodeSource;
use render_quorra::{PresentFrame, QuorraRasterizer};

/// The window every frame below is drawn into.
const WINDOW: (u32, u32) = (200, 150);

/// The page, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 200.0,
    height: 150.0,
};

/// A backend, or the reason the whole suite should fail.
///
/// Deliberately does not skip, for `headless_quorra.rs`'s reason (ADR 0004): a skipped suite
/// reports success while verifying nothing, and what this file verifies is that a viewer does not
/// show the wrong page.
fn backend() -> QuorraRasterizer {
    match QuorraRasterizer::new_headless() {
        Ok(rasterizer) => rasterizer,
        Err(error) => panic!(
            "no adapter available for quorra: {error}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a software one)."
        ),
    }
}

/// A closed rectangle.
fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// One opaque fill.
fn fill(list: &mut DisplayList, path: Path, colour: Color) {
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
}

/// A page carrying one square of `colour` — a different colour is a visibly different page, which
/// is what the staleness half of each test needs.
fn page(colour: Color) -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    fill(&mut list, rectangle(20.0, 20.0, 120.0, 100.0), colour);
    Arc::new(list)
}

/// A strip of chrome across the top of the window, in the window's own pixels.
fn chrome(height: f32) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    fill(
        &mut list,
        rectangle(0.0, 0.0, 200.0, height),
        Color::rgb(0.2, 0.4, 0.9),
    );
    list
}

/// The page's placement at `scale`.
fn placed(scale: f32) -> TargetSpec {
    TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: Transform::scale(scale, scale),
    }
}

/// Draws one frame and says what it cost to: the pixels, and whether the scene was encoded here
/// or replayed from an earlier frame.
fn draw(
    gpu: &mut QuorraRasterizer,
    pages: &[(&Arc<DisplayList>, TargetSpec)],
    raster: Option<&Raster>,
    overlays: &[&DisplayList],
    size: (u32, u32),
) -> (Raster, EncodeSource) {
    let drawn = gpu
        .rasterize_frame(&PresentFrame {
            width: size.0,
            height: size.1,
            pages,
            raster,
            overlays,
        })
        .expect("the device draws a window of rectangles");
    let source = gpu
        .last_frame()
        .encode_source
        .expect("a frame that drew reached the device, so it has an encode source");
    (drawn, source)
}

/// The common case: one page, at page scale, with no chrome and no stand-in.
fn plain(gpu: &mut QuorraRasterizer, list: &Arc<DisplayList>) -> (Raster, EncodeSource) {
    draw(gpu, &[(list, placed(1.0))], None, &[], WINDOW)
}

/// Asserts two rasters are the same bytes, and says how many differ when they are not.
fn identical(what: &str, left: &Raster, right: &Raster) {
    assert_eq!(
        (left.width, left.height),
        (right.width, right.height),
        "{what}: different dimensions"
    );
    let differing = left
        .data
        .iter()
        .zip(&right.data)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing,
        0,
        "{what}: {differing} of {} bytes differ",
        left.data.len()
    );
}

/// A replayed frame draws the encode it replaced, byte for byte.
///
/// **The claim the whole change rests on.** quorra measured it on its own archetypes (0 of
/// 8 022 576 bytes, on two adapters); this asserts it through *this* crate's frame — a medium, a
/// placed page and chrome over it, which is the scene a window actually presents — because what
/// is retained here is a scene this crate built and quorra's fixtures are not that scene.
#[test]
fn a_replayed_frame_is_byte_identical_to_the_encode_it_replaces() {
    let mut gpu = backend();
    let list = page(Color::rgb(0.9, 0.1, 0.1));
    let overlay = chrome(24.0);

    let (encoded, first) = draw(&mut gpu, &[(&list, placed(1.0))], None, &[&overlay], WINDOW);
    assert_eq!(
        first,
        EncodeSource::Encoded,
        "the first frame has nothing to replay"
    );
    assert!(
        gpu.last_frame().retained_bytes > 0,
        "an encode was retained, so the handle holds something"
    );

    // The same frame again, built the way a host builds it: a fresh chrome list carrying the
    // same picture, and the page's own `Arc` unmoved.
    let again = chrome(24.0);
    let (replayed, second) = draw(&mut gpu, &[(&list, placed(1.0))], None, &[&again], WINDOW);
    assert_eq!(
        second,
        EncodeSource::Replayed,
        "nothing about the frame moved, so nothing had to be encoded again"
    );
    identical("an encoded frame against its replay", &encoded, &replayed);

    // And against a device that retained nothing at all, which is what this tree drew before
    // ADR 0351 and what every other gate still compares against.
    let (fresh, source) = draw(
        &mut backend(),
        &[(&list, placed(1.0))],
        None,
        &[&overlay],
        WINDOW,
    );
    assert_eq!(source, EncodeSource::Encoded);
    identical(
        "a replayed frame against a cold device's encode",
        &replayed,
        &fresh,
    );
}

/// A different page misses, and draws the page it was handed rather than the one before it.
#[test]
fn a_new_page_misses_and_draws_the_new_page() {
    let mut gpu = backend();
    let first = page(Color::rgb(0.9, 0.1, 0.1));
    let second = page(Color::rgb(0.1, 0.2, 0.9));

    let (_, source) = plain(&mut gpu, &first);
    assert_eq!(source, EncodeSource::Encoded);
    let (_, source) = plain(&mut gpu, &first);
    assert_eq!(source, EncodeSource::Replayed, "the frame settled");

    let (drawn, source) = plain(&mut gpu, &second);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "a different display list is a different scene"
    );
    let (expected, _) = plain(&mut backend(), &second);
    identical("the page after a page turn", &drawn, &expected);
}

/// The *same picture* at a different address misses, because the key is identity and not content.
///
/// A page turn to a page that happens to look the same must not replay the previous one's encode:
/// the two are one clause apart from being different pages, and a key that could not tell them
/// apart would be a key that had stopped being about identity at all. This is also the case an
/// address-only key gets wrong in the other direction — see `PresentFrame::page` on why the
/// address is *pinned* rather than merely remembered.
#[test]
fn the_same_picture_at_a_new_identity_misses() {
    let mut gpu = backend();
    let first = page(Color::rgb(0.9, 0.1, 0.1));
    let (before, _) = plain(&mut gpu, &first);
    let (_, source) = plain(&mut gpu, &first);
    assert_eq!(source, EncodeSource::Replayed);

    let twin = Arc::new((*first).clone());
    let (after, source) = plain(&mut gpu, &twin);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "a second display list is a second scene, whatever it draws"
    );
    identical("a page redrawn from a twin list", &before, &after);
}

/// A window that changed size misses.
#[test]
fn a_resized_window_misses() {
    let mut gpu = backend();
    let list = page(Color::rgb(0.9, 0.1, 0.1));
    let (_, source) = plain(&mut gpu, &list);
    assert_eq!(source, EncodeSource::Encoded);
    let (_, source) = plain(&mut gpu, &list);
    assert_eq!(source, EncodeSource::Replayed);

    let smaller = (WINDOW.0 - 40, WINDOW.1 - 30);
    let (drawn, source) = draw(&mut gpu, &[(&list, placed(1.0))], None, &[], smaller);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "every clip and cull in an encode is against the target rectangle"
    );
    let (expected, _) = draw(&mut backend(), &[(&list, placed(1.0))], None, &[], smaller);
    identical("the frame after a resize", &drawn, &expected);
}

/// A zoom step misses, which is the case quorra says no design can make survive.
#[test]
fn a_zoom_step_misses() {
    let mut gpu = backend();
    let list = page(Color::rgb(0.9, 0.1, 0.1));
    let (_, source) = plain(&mut gpu, &list);
    assert_eq!(source, EncodeSource::Encoded);
    let (_, source) = plain(&mut gpu, &list);
    assert_eq!(source, EncodeSource::Replayed);

    let (drawn, source) = draw(&mut gpu, &[(&list, placed(1.4))], None, &[], WINDOW);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "the placement is baked into every command, so a new scale is a new scene"
    );
    let (expected, _) = draw(&mut backend(), &[(&list, placed(1.4))], None, &[], WINDOW);
    identical("the frame after a zoom step", &drawn, &expected);
}

/// Chrome that changed misses; chrome rebuilt with the same picture does not.
///
/// Both halves, because the overlays are the one part of the key compared *by value*: a key that
/// only ever missed on them would be no key at all — the host rebuilds every overlay on every
/// frame — and one that only ever hit would leave a dragged selection a frame behind.
#[test]
fn chrome_misses_when_it_changes_and_not_when_it_is_merely_rebuilt() {
    let mut gpu = backend();
    let list = page(Color::rgb(0.9, 0.1, 0.1));
    let narrow = chrome(24.0);
    let wide = chrome(48.0);

    let (_, source) = draw(&mut gpu, &[(&list, placed(1.0))], None, &[&narrow], WINDOW);
    assert_eq!(source, EncodeSource::Encoded);

    let rebuilt = chrome(24.0);
    let (_, source) = draw(&mut gpu, &[(&list, placed(1.0))], None, &[&rebuilt], WINDOW);
    assert_eq!(
        source,
        EncodeSource::Replayed,
        "the same chrome at a new address is the same chrome"
    );

    let (drawn, source) = draw(&mut gpu, &[(&list, placed(1.0))], None, &[&wide], WINDOW);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "different chrome, different scene"
    );
    let (expected, _) = draw(
        &mut backend(),
        &[(&list, placed(1.0))],
        None,
        &[&wide],
        WINDOW,
    );
    identical("the frame after the chrome changed", &drawn, &expected);

    // And chrome *removed* is a change as much as chrome altered — the length is part of the
    // comparison, not only the lists that are there.
    let (_, source) = plain(&mut gpu, &list);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "chrome that went away is a change"
    );
}

/// A frame carrying the processor's raster never replays, however unchanged it looks.
///
/// The stand-in is bytes the processor produced for *this* frame, handed over by reference: there
/// is no allocation to pin and no identity to key on, so `SceneKey::raster` denies reuse outright
/// rather than trusting an address the allocator is free to hand out twice.
#[test]
fn a_raster_stand_in_never_replays() {
    let mut gpu = backend();
    let stand_in = Raster {
        width: WINDOW.0,
        height: WINDOW.1,
        format: pdf_render::RasterFormat::Rgba8,
        data: vec![200; (WINDOW.0 as usize) * (WINDOW.1 as usize) * 4],
    };
    for round in 0..3 {
        let (_, source) = draw(&mut gpu, &[], Some(&stand_in), &[], WINDOW);
        assert_eq!(
            source,
            EncodeSource::Encoded,
            "round {round}: a raster frame has no identity to reuse"
        );
    }
}

/// A page drawn again after a stand-in frame replays once the stand-in is gone — the fallback
/// costs one encode and does not poison the slot.
#[test]
fn the_page_settles_again_after_a_fallback_frame() {
    let mut gpu = backend();
    let list = page(Color::rgb(0.9, 0.1, 0.1));
    let stand_in = Raster {
        width: WINDOW.0,
        height: WINDOW.1,
        format: pdf_render::RasterFormat::Rgba8,
        data: vec![200; (WINDOW.0 as usize) * (WINDOW.1 as usize) * 4],
    };

    let (before, _) = plain(&mut gpu, &list);
    let (_, source) = draw(&mut gpu, &[], Some(&stand_in), &[], WINDOW);
    assert_eq!(source, EncodeSource::Encoded);

    let (again, source) = plain(&mut gpu, &list);
    assert_eq!(
        source,
        EncodeSource::Encoded,
        "the slot is holding the stand-in's scene, so the page is a rebuild"
    );
    identical("the page after a fallback frame", &before, &again);

    let (settled, source) = plain(&mut gpu, &list);
    assert_eq!(source, EncodeSource::Replayed, "and then it settles");
    identical("the page settled after a fallback frame", &before, &settled);
}
