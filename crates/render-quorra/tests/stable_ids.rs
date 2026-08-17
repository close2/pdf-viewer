//! One display list, drawn twice, must hand quorra the same resource identifiers both times.
//!
//! This is the invariant `crate::cache` exists for, and until the five-hundred-and-sixty-seventh
//! session nothing asserted it. What broke it was strokes: §8.4.3.2 makes an *anisotropically*
//! placed stroke's outline something this crate expands with `kurbo::stroke` rather than something
//! quorra widens itself, and computed geometry was uploaded as a transient — released after the
//! frame and re-uploaded, with a new identifier, on the next one. quorra keys every glyph-lane
//! tile on that identifier, so a still page made every key foreign every frame and its atlas
//! repacked at period two, for ever (`render-lib/doc/notes-atlas-budget.md` section 5, ADR 0402).
//!
//! **The observable is [`render_quorra::FrameCost::uploads`], not a duration.** A timing would
//! have to be a threshold, and a threshold on a machine with a graphics driver is a flake; a page
//! that has not changed uploading anything at all is a defect whatever it costs.
//!
//! What this file does *not* pin is the boundary: a **dashed** or **degenerate** stroke is
//! geometry this frame computed from something other than the display list's own path, and it
//! stays transient. That is a limit rather than an invariant, so it is written down here and not
//! asserted — a test that failed when somebody widened the cache would be a ratchet pointing the
//! wrong way.

#![expect(
    clippy::panic,
    reason = "test code: a device that cannot draw two strokes is the failure this file \
              reports, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page every list below is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 200.0,
    height: 150.0,
};

/// A backend, or the reason the whole suite should fail.
///
/// Deliberately does not skip, for `headless_quorra.rs`'s reason (ADR 0004): a skipped suite
/// reports success while verifying nothing.
fn backend() -> QuorraRasterizer {
    match QuorraRasterizer::new_headless() {
        Ok(rasterizer) => rasterizer,
        Err(error) => panic!(
            "no adapter available for quorra: {error}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a software one)."
        ),
    }
}

/// An open zig-zag: enough segments for joins and caps to matter, and not a closed shape, so the
/// expansion is a genuine outline rather than a rectangle.
fn zigzag() -> Arc<Path> {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 20.0)));
    for step in 1..12 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "eleven steps; every one is exact in f32"
        )]
        let x = 10.0 + (step as f32) * 5.0;
        let y = if step % 2 == 0 { 20.0 } else { 100.0 };
        path.push(PathCommand::LineTo(Point::new(x, y)));
    }
    Arc::new(path)
}

/// **The placement that decides the whole file**: three times as wide as it is tall, so
/// `anisotropy` exceeds `MAX_ISOTROPY_ERROR` and `crate::stroke` outlines the stroke in path
/// space instead of handing quorra a scalar width. An isotropic placement takes the other branch
/// and would pass this test with the defect present.
const SHEARED: Transform = Transform::new(3.0, 0.0, 0.0, 1.0, 0.0, 0.0);

fn stroke_of(width: f32) -> Stroke {
    Stroke {
        width,
        adjust: false,
        cap: LineCap::Round,
        join: LineJoin::Miter,
        miter_limit: 10.0,
        dash_array: Vec::new(),
        dash_phase: 0.0,
    }
}

/// A page that strokes `path` once at each of `widths`, all under [`SHEARED`].
fn page(path: &Arc<Path>, widths: &[f32]) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    for width in widths {
        list.push(Command::Stroke {
            path: Arc::clone(path),
            transform: SHEARED,
            stroke: stroke_of(*width),
            paint: Paint::Solid(Color::rgb(0.1, 0.2, 0.7)),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
    }
    list
}

fn target() -> TargetSpec {
    TargetSpec {
        width: 200,
        height: 150,
        transform: Transform::IDENTITY,
    }
}

/// The same list drawn twice uploads on the first frame and on no frame after it.
///
/// Deleting the stroke cache makes the second frame's count equal the first's, which is what
/// establishes that this test guards it rather than merely accompanying it.
#[test]
fn a_page_that_has_not_changed_uploads_nothing_the_second_time() {
    let mut gpu = backend();
    let list = page(&zigzag(), &[2.0]);
    let spec = target();

    let first = gpu.rasterize(&list, spec).expect("the first frame draws");
    let uploads = gpu.last_frame().uploads;
    assert!(
        uploads > 0,
        "a first frame must upload the geometry it draws; it uploaded {uploads}"
    );

    let second = gpu.rasterize(&list, spec).expect("the second frame draws");
    assert_eq!(
        gpu.last_frame().uploads,
        0,
        "an unchanged display list re-uploaded a resource, so the identifiers quorra keys its \
         atlas on are not stable between renders (ADR 0402)"
    );

    assert_eq!(
        first.data, second.data,
        "reusing an uploaded outline changed the picture"
    );
}

/// Two widths of one path are two outlines, and neither answers for the other.
///
/// The discriminator for the *key* rather than for the cache: the entry is keyed by the source
/// path's address plus the arguments the expansion was made with, and a key that dropped the
/// width would serve the first stroke's outline for the second — a page drawn with the wrong
/// line thickness, and no report anywhere, because both are valid uploads.
#[test]
fn an_expanded_stroke_is_keyed_by_the_width_it_was_expanded_at() {
    let mut gpu = backend();
    let path = zigzag();
    let spec = target();

    let thin = gpu
        .rasterize(&page(&path, &[1.0]), spec)
        .expect("the thin page draws");
    let thick = gpu
        .rasterize(&page(&path, &[9.0]), spec)
        .expect("the thick page draws");

    assert_ne!(
        thin.data, thick.data,
        "the same path stroked at two widths drew the same pixels, so the cached outline was \
         served for a width it was not expanded at"
    );
}
