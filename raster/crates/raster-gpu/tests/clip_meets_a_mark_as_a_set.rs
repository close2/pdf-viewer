//! A rectangular clip meets a mark's coverage as a **set**: the path lane by `min`, the
//! image lane by intersecting the two rectangles (the caller's ADR 1435).
//!
//! # Where the expected values come from
//!
//! ISO 32000-2 §10.7.4:
//!
//! > For clipping, the clipping region consists of the set of pixels that would be
//! > included by a fill operation. Subsequent painting operations shall affect a region
//! > that is the intersection of the set of pixels defined by the clipping region with
//! > the set of pixels for the region to be painted.
//!
//! and, three sentences before it, which side any remaining error may fall on:
//!
//! > The area covered by painted pixels shall always be at least as large as the area of
//! > the original shape.
//!
//! Inside one pixel the area of an intersection is at most the smaller of the two areas,
//! and equal to it wherever one of the two sets contains the other there. A product of the
//! two coverages is below the smaller wherever both are fractional — `0.6 × 0.6 = 0.36`
//! where the mark's edge *is* the clip's edge — so it paints less than the shape the
//! clause intersects. `min` never paints less. Where the lane holds the geometry of both
//! (an axis-preserving image is a rectangle, and so is the clip) the intersection is
//! computed outright.
//!
//! # The arithmetic each test asserts, by hand
//!
//! Each fixture puts a mark's right edge inside device column 2 and reads the alpha of
//! row 3 there. The coverages are chosen exactly representable in eight bits, so no
//! rounding enters: `0.6 = 153/255`, `0.2 = 51/255`; `0.12 × 255 = 30.6`, which rounds
//! to 31.
//!
//! | fixture | mark | clip | area of `S ∩ C` | drawn | product |
//! |---|---|---|---|---|---|
//! | path, edges coincide | 0.6 | 0.6, same half-plane | **0.6** | 153 | 92 |
//! | path, clip contains the mark | 0.2 | 0.6 | **0.2** | 51 | 31 |
//! | path, edges cross at right angles | 0.2 | 0.6 of the row | 0.12 | 51, the bound | 31 |
//! | image, edges coincide | 0.6 | 0.6, same half-plane | **0.6** | 153 | 92 |
//! | image, edges cross at right angles | 0.2 | 0.6 of the row | **0.12** | 31 | 31 |
//!
//! The third row is the one place `min` is not the area: a coverage byte carries no
//! geometry, so the lane states the intersection's upper bound, which is the side the
//! second quotation names. The fifth row is the same crossing on a lane that has both
//! rectangles, where the intersection is computed and the product happened to be right.

// Test-file lint policy as in m1.rs; the arithmetic below is the clause's, over rasters
// this file just drew.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects
)]

use std::sync::Arc;

use raster_gpu::{Device, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, ClipId, Compose, FillRule, ImageFilter, ImageSpec, Point, Rect, Scene,
    SceneBuilder, Segment,
};

mod common;

use common::probe::alpha;
use common::scene::{black, rect_outline};

/// The target: wide enough for column 2 and its neighbours.
const WIDTH: u32 = 16;
const HEIGHT: u32 = 8;

/// The device column both edges fall inside, and the row every assertion reads.
const EDGE_COLUMN: u32 = 2;
const ROW: u32 = 3;

/// Right edges at 0.6 and 0.2 of column 2.
const SIX_TENTHS: f32 = 2.6;
const TWO_TENTHS: f32 = 2.2;

/// A clip rectangle from far left to `right` and from far above to `bottom`, which the
/// device resolves to one clip rectangle with no residue (ADR 0007).
fn rect_clip(device: &mut Device, builder: &mut SceneBuilder, right: f32, bottom: f32) -> ClipId {
    let outline = device
        .upload_outline(&rect_outline(Rect::new(
            Point::new(-4.0, -4.0),
            Point::new(right, bottom),
        )))
        .unwrap();
    builder
        .clip(outline, Affine::IDENTITY, FillRule::NonZero, None)
        .unwrap()
}

/// A pentagon whose right side is the vertical line `x = right` over the whole target:
/// not four axis-aligned edges, so the rectangle recogniser declines it and the fill is
/// rasterised to a coverage byte (`common::scene::rect_outline`'s note).
fn pentagon(right: f32) -> Vec<Segment> {
    let (top, bottom) = (-4.0, HEIGHT as f32 + 4.0);
    vec![
        Segment::MoveTo(Point::new(-4.0, top)),
        Segment::LineTo(Point::new(right, top)),
        Segment::LineTo(Point::new(right, bottom)),
        Segment::LineTo(Point::new(-3.0, bottom)),
        Segment::LineTo(Point::new(-5.0, 4.0)),
        Segment::Close,
    ]
}

/// The path-lane fixture: the pentagon filled black under a clip ending at `clip_right`
/// and `clip_bottom`.
fn path_scene(device: &mut Device, right: f32, clip_right: f32, clip_bottom: f32) -> Scene {
    let mut builder = SceneBuilder::new();
    let clip = rect_clip(device, &mut builder, clip_right, clip_bottom);
    let outline = device.upload_outline(&pentagon(right)).unwrap();
    builder
        .fill(
            outline,
            Affine::IDENTITY,
            FillRule::NonZero,
            black(),
            Some(clip),
            BlendMode::Normal,
            Compose::SrcOver,
            None,
        )
        .unwrap();
    builder.finish()
}

/// The image-lane fixture: a one-texel opaque black image stretched over `[0, right] ×
/// [0, HEIGHT]`, axis-preserving, under the same clip.
fn image_scene(device: &mut Device, right: f32, clip_right: f32, clip_bottom: f32) -> Scene {
    let image = device
        .upload_image(&ImageSpec {
            width: 1,
            height: 1,
            data: Arc::from(vec![0_u8, 0, 0, 255]),
        })
        .unwrap();
    let mut builder = SceneBuilder::new();
    let clip = rect_clip(device, &mut builder, clip_right, clip_bottom);
    builder
        .image(
            image,
            Affine {
                a: right,
                b: 0.0,
                c: 0.0,
                d: HEIGHT as f32,
                e: 0.0,
                f: 0.0,
            },
            1.0,
            ImageFilter::Auto { interpolate: false },
            Some(clip),
            BlendMode::Normal,
            None,
        )
        .unwrap();
    builder.finish()
}

fn edge_alpha(device: &mut Device, scene: &Scene) -> u8 {
    let pixels = device
        .render(
            scene,
            &Viewport::full(WIDTH, HEIGHT, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders")
        .into_raster()
        .unwrap()
        .into_pixels();
    alpha(&pixels, WIDTH, EDGE_COLUMN, ROW)
}

/// Below every row, so a clip given this bottom cuts nothing vertically.
const BELOW: f32 = HEIGHT as f32 + 4.0;

/// A border rule standing on its clip's edge keeps its own coverage there: both regions
/// are the half-plane `x ≤ 2.6`, their intersection is that half-plane, and column 2
/// keeps 0.6 — **153**. The product would draw 92.
#[test]
fn a_path_whose_edge_is_its_clips_edge_keeps_that_edges_coverage() {
    let mut device = common::headless::device();
    let scene = path_scene(&mut device, SIX_TENTHS, SIX_TENTHS, BELOW);
    assert_eq!(
        edge_alpha(&mut device, &scene),
        153,
        "0.6 of a pixel intersected with the same 0.6 is 0.6, which is 153 of 255"
    );
}

/// `S ∩ C = S` where `S ⊆ C`: the mark's half-plane `x ≤ 2.2` lies inside the clip's
/// `x ≤ 2.6`, so column 2 keeps the mark's own 0.2 — **51**. The product would draw 31.
#[test]
fn a_clip_that_contains_the_path_takes_nothing_from_it() {
    let mut device = common::headless::device();
    let scene = path_scene(&mut device, TWO_TENTHS, SIX_TENTHS, BELOW);
    assert_eq!(
        edge_alpha(&mut device, &scene),
        51,
        "0.2 of a pixel inside a clip admitting 0.6 of it is 0.2, which is 51 of 255"
    );
}

/// Where the two edges cross at right angles the area of the intersection is `0.2 × 0.6 =
/// 0.12`, and a coverage byte cannot say so: it holds 0.2 and nothing of where in the pixel
/// that 0.2 lies. So the lane draws `min(0.2, 0.6) = 0.2` — **51** — the intersection's
/// upper bound, on the side §10.7.4's "at least as large as the area of the original
/// shape" states, and at least the area 31 that the product happens to hit here.
#[test]
fn a_path_crossed_by_its_clip_is_drawn_at_the_intersections_upper_bound() {
    let mut device = common::headless::device();
    let scene = path_scene(&mut device, TWO_TENTHS, BELOW, ROW as f32 + 0.6);
    let read = edge_alpha(&mut device, &scene);
    assert_eq!(
        read, 51,
        "min(0.2, 0.6) = 0.2, which is 51 of 255; the area of the intersection is 0.12 (31), \
         and a drawn value below 31 would paint less than the shape"
    );
}

/// An axis-preserving image standing on its clip's edge: the two rectangles intersect to
/// the image's own, and column 2 keeps its 0.6 — **153**. The product would draw 92.
#[test]
fn an_image_whose_edge_is_its_clips_edge_keeps_that_edges_coverage() {
    let mut device = common::headless::device();
    let scene = image_scene(&mut device, SIX_TENTHS, SIX_TENTHS, BELOW);
    assert_eq!(
        edge_alpha(&mut device, &scene),
        153,
        "0.6 of a pixel intersected with the same 0.6 is 0.6, which is 153 of 255"
    );
}

/// The crossing on the lane that holds both rectangles: they intersect to `[0, 2.2] ×
/// [.., 3.6]`, whose overlap with pixel (2, 3) is `0.2 × 0.6 = 0.12` — **31**, the area
/// itself rather than a bound.
#[test]
fn an_image_crossed_by_its_clip_is_drawn_at_the_intersections_area() {
    let mut device = common::headless::device();
    let scene = image_scene(&mut device, TWO_TENTHS, BELOW, ROW as f32 + 0.6);
    assert_eq!(
        edge_alpha(&mut device, &scene),
        31,
        "the rectangles' intersection covers 0.2 × 0.6 = 0.12 of the pixel, 30.6 of 255"
    );
}
