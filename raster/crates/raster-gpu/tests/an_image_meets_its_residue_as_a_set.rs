//! An image under a clip that is not a rectangle meets that residue as a set (ADR 1480): where
//! the image's rectangle and the residue both cut a pixel, the pixel holds the area of their
//! intersection, as a path's mark does (ADR 1467).
//!
//! # Where the expected values come from
//!
//! ISO 32000-2 §10.7.4:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the
//! > set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! and §11.6.4.2 says what an image's region is: "[f]or images … the shape shall be 1.0
//! inside the image rectangle and 0.0 outside it". An image whose placement keeps the axes —
//! a scale, a flip, or a quarter turn — maps the unit square onto an axis-aligned rectangle,
//! so its set is a rectangle; this one is a one-texel opaque image turned a quarter turn
//! (§8.3.3's rotation by 90°) onto `[2.35, 12.65] × [4.2, 13.9]`. The clip is the 64-gon the
//! path lane's fixture uses. Per pixel, from the geometry alone: the image's coverage `s`
//! (the rectangle's area in the pixel), the clip's `c`, and the intersection's `i` (the
//! rectangle clipped to the 64-gon, then to the pixel). The texel is opaque, so the drawn
//! alpha is the shape: every pixel is held to `i` to within the half level one rounding
//! leaves. Under `min` the pixels both sets cut drew above it.

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

use raster_scene::{Affine, BlendMode, ImageFilter, ImageSpec, SceneBuilder};

mod common;

use common::meet::{
    ARMS, clip_convex, clip_polygon, device_with, held_to_the_intersection, in_pixel, polygon_path,
    rendered, residue_clip,
};

/// The target, wide and tall enough for the 64-gon.
const TARGET: (u32, u32) = (20, 22);

/// The image's device rectangle: left, top, right, bottom.
const PLACED: (f64, f64, f64, f64) = (2.35, 4.2, 12.65, 13.9);

/// The quarter turn that maps the unit square onto [`PLACED`]: `(0, 0)` to the top right
/// corner, `(1, 0)` down the right side, `(0, 1)` along the top — `b` and `c` carry the
/// extents and `a` and `d` are zero, so the placement keeps the axes.
fn quarter_turn() -> Affine {
    let (left, top, right, bottom) = PLACED;
    Affine {
        a: 0.0,
        b: (bottom - top) as f32,
        c: -(right - left) as f32,
        d: 0.0,
        e: right as f32,
        f: top as f32,
    }
}

/// **A quarter-turned image under a 64-gon clip meets every pixel as a set** (ADR 1480), on
/// the walk's lane and the fan-out's at one thread and four, to the same bytes. The fixture
/// discriminates: `min(s, c)` is more than a level above `i` in some pixel both sets cut.
#[test]
fn a_quarter_turned_image_under_a_polygon_clip_meets_each_pixel_as_a_set() {
    let clip = clip_polygon();
    let (left, top, right, bottom) = PLACED;
    let rectangle = [(left, top), (right, top), (right, bottom), (left, bottom)];
    let met = clip_convex(&rectangle, &clip);
    let mut drawn = Vec::new();
    for (coverage, threads) in ARMS {
        let mut device = device_with(coverage, threads);
        let image = device
            .upload_image(&ImageSpec {
                width: 1,
                height: 1,
                data: Arc::from(vec![0_u8, 0, 0, 255]),
            })
            .unwrap();
        let mut builder = SceneBuilder::new();
        let clip_id = residue_clip(&mut device, &mut builder, &polygon_path(&clip));
        builder
            .image(
                image,
                quarter_turn(),
                1.0,
                ImageFilter::Auto { interpolate: false },
                Some(clip_id),
                BlendMode::Normal,
                None,
            )
            .unwrap();
        let scene = builder.finish();
        let pixels = rendered(&mut device, &scene, TARGET.0, TARGET.1);
        let (both, _, _, min_above) = held_to_the_intersection(
            &pixels,
            TARGET,
            |x, y| in_pixel(&rectangle, x, y),
            |x, y| in_pixel(&clip, x, y),
            |x, y| in_pixel(&met, x, y),
        );
        assert!(
            both > 0 && min_above > 0,
            "the image's edges must cross the clip's rim where `min` is above the area: \
             {both} pixels cut by both, {min_above} above under `min`"
        );
        drawn.push(pixels);
    }
    assert!(
        drawn.windows(2).all(|pair| pair[0] == pair[1]),
        "the walk's lane and the fan-out's at one thread and four draw the same bytes"
    );
}
