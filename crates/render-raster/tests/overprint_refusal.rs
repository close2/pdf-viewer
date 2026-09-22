//! ISO 32000-2 §11.7.4.3's special overprinting blend mode is refused here, by name.
//!
//! `render-cpu` computes the mode (ADR 1157); this backend's scene vocabulary has only Table
//! 135's sixteen, and the one value the clause's first bullet gives that is not the source
//! colour has no arm among them. A page drawn as though the producer had not asked for
//! overprinting would differ from the oracle's with nothing saying so, which is the divergence
//! the cross-backend comparison exists to prevent — so the list is refused before anything is
//! drawn, and the caller falls back to the backend `CLAUDE.md` keeps for exactly this.

#![expect(
    clippy::panic,
    reason = "test code: a missing adapter is a failure rather than a skip (ADR 0004)"
)]

use std::sync::Arc;

use pdf_render::display_list::Command;
use pdf_render::{
    BlendMode, Color, DisplayList, FillRule, Overprint, Paint, Path, PathCommand, Point,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// The page every fixture here is drawn on.
const PAGE: Size = Size {
    width: 16.0,
    height: 16.0,
};

/// One opaque fill, under `blend`, on a page that says whether it overprints.
fn scene(blend: BlendMode, overprinting: bool) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(2.0, 2.0)));
    path.push(PathCommand::LineTo(Point::new(12.0, 2.0)));
    path.push(PathCommand::LineTo(Point::new(12.0, 12.0)));
    path.push(PathCommand::LineTo(Point::new(2.0, 12.0)));
    path.push(PathCommand::Close);

    let mut list = DisplayList::new(PAGE);
    if overprinting {
        list.note_overprinting();
    }
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, PAGE.height),
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend,
    });
    list
}

/// The raster backend, or a panic naming why the whole suite should fail rather than skip.
fn raster() -> QuorraRasterizer {
    QuorraRasterizer::new_headless().unwrap_or_else(|e| {
        panic!(
            "no adapter available for raster: {e}\n\
             Install a Vulkan driver (mesa-vulkan-drivers for a software one). These tests do \
             not skip, because a skipped suite reports success while verifying nothing."
        )
    })
}

#[test]
fn a_list_that_overprints_is_refused_and_the_same_list_without_it_is_drawn() {
    let mut raster = raster();
    let kept = Overprint::new([false, true, true]);

    let refused = scene(BlendMode::Overprint(kept), true);
    let target = TargetSpec::for_page(&refused, 1.0, GENEROUS).expect("valid target");
    let error = raster
        .rasterize(&refused, target)
        .expect_err("the special overprinting blend mode is refused");
    assert!(
        format!("{error}").contains("overprinting"),
        "the refusal names what it refused, was: {error}"
    );

    // And the refusal is the mode's rather than the page's: the same geometry with Table 135's
    // Normal draws, so nothing here is a backend that stopped working.
    let drawn = scene(BlendMode::Normal, false);
    let target = TargetSpec::for_page(&drawn, 1.0, GENEROUS).expect("valid target");
    raster
        .rasterize(&drawn, target)
        .expect("an ordinary fill is drawn");
}
