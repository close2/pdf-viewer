//! ISO 32000-2 §10.7.4's clipping region can be the union of two fills, and this backend says so
//! rather than admitting a smaller set.
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by
//! > a fill operation.
//!
//! §8.5.4 states the same from the operator's side — "For a given path definition, the same area
//! that would be filled by the f operator is the area that would be used for a clip" — and a path
//! that encloses an area *and* rules a line is filled as both, the second by this subclause's own
//! EXAMPLE: "A zero-width or zero-height rectangle paints a line 1 pixel wide". The two are filled
//! under two different rules, because two marks that cross would cancel to a hole under the
//! even-odd rule, so the region is a union this scene's `clip` — one outline, one rule — cannot
//! state. `render-cpu` composes it into its own mask; here the list is refused before anything is
//! drawn and the caller falls back to the backend `CLAUDE.md` keeps for exactly this (ADR 1231).

#![expect(
    clippy::panic,
    reason = "test code: a missing adapter is a failure rather than a skip (ADR 0004)"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// The page every fixture here is drawn on.
const PAGE: Size = Size {
    width: 64.0,
    height: 64.0,
};

/// Builds a path from its commands.
fn path(commands: &[PathCommand]) -> Path {
    let mut built = Path::new();
    for command in commands {
        built.push(*command);
    }
    built
}

/// A 30-unit square, optionally with a rule of no height drawn across it.
///
/// With the rule, the fill of this path is a square and a line of pixels that crosses it, and
/// §10.7.4's region for it is their union; without it, the path is an ordinary clip.
fn clip_path(with_a_rule: bool) -> Path {
    let mut commands = vec![
        PathCommand::MoveTo(Point::new(10.0, 10.0)),
        PathCommand::LineTo(Point::new(40.0, 10.0)),
        PathCommand::LineTo(Point::new(40.0, 40.0)),
        PathCommand::LineTo(Point::new(10.0, 40.0)),
        PathCommand::Close,
    ];
    if with_a_rule {
        commands.extend_from_slice(&[
            PathCommand::MoveTo(Point::new(5.0, 20.5)),
            PathCommand::LineTo(Point::new(60.0, 20.5)),
            PathCommand::LineTo(Point::new(60.0, 20.5)),
            PathCommand::LineTo(Point::new(5.0, 20.5)),
            PathCommand::Close,
        ]);
    }
    path(&commands)
}

/// A page-wide fill under that clip.
fn scene(with_a_rule: bool) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    let clip = list
        .add_clip(Clip {
            path: clip_path(with_a_rule),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .unwrap_or_else(|e| panic!("a clip of one path is buildable: {e}"));
    let page = path(&[
        PathCommand::MoveTo(Point::new(0.0, 0.0)),
        PathCommand::LineTo(Point::new(PAGE.width, 0.0)),
        PathCommand::LineTo(Point::new(PAGE.width, PAGE.height)),
        PathCommand::LineTo(Point::new(0.0, PAGE.height)),
        PathCommand::Close,
    ]);
    list.push(Command::Fill {
        path: Arc::new(page),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: Some(clip),
        mask: None,
        blend: BlendMode::Normal,
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

/// The refusal, and the control that must still draw.
///
/// The control is the same square without the rule across it: a clip this scene states directly.
/// Without it a backend that had stopped drawing clips at all would pass the first half.
#[test]
fn a_clip_whose_region_is_a_union_of_two_fills_is_refused_and_the_square_alone_is_drawn() {
    let mut raster = raster();

    let refused = scene(true);
    let target = TargetSpec::for_page(&refused, 1.0, GENEROUS).expect("valid target");
    let error = raster
        .rasterize(&refused, target)
        .expect_err("a clipping region that is a union of two fills is refused");
    assert!(
        format!("{error}").contains("union of two fills"),
        "the refusal names what it refused, was: {error}"
    );

    let drawn = scene(false);
    let target = TargetSpec::for_page(&drawn, 1.0, GENEROUS).expect("valid target");
    raster
        .rasterize(&drawn, target)
        .expect("an ordinary clip is drawn");
}
