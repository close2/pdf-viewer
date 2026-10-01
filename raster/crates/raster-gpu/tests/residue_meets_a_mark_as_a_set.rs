//! A clip that is not a rectangle — a **residue**, rasterised to coverage bytes — meets a
//! mark's coverage as a set, by `min`, on the path lane and on the image lane (the caller's
//! ADR 1444).
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
//! and which side an error left by a scan conversion may fall on:
//!
//! > The area covered by painted pixels shall always be at least as large as the area of
//! > the original shape.
//!
//! In one pixel, with the mark covering `s` of it and the clip `c`, the intersection's area
//! lies in `[max(0, s + c − 1), min(s, c)]`, and is `min(s, c)` wherever one set holds the
//! other there. Two bytes say how much and not where, so no function of them is the area; of
//! those never below it, `min` is the least. A product is below the area wherever a mark
//! stands on its clip's own edge (`0.6 × 0.6` where the area is `0.6`) and above it where the
//! two sets miss each other inside a pixel (`0.3 × 0.3` where the area is `0`).
//!
//! # The tiling, and what it measures
//!
//! [`a_polygon_clip_over_a_tiling_of_rectangles_meets_each_pixel_as_a_set`] clips a lattice
//! of small rectangles with a regular 64-gon — nothing curved, so every expected value is
//! a polygon's area in a pixel, computed here in `f64` by clipping each polygon to the pixel.
//! Over the pixels where both coverages are fractional, against that area, a product misses
//! by 15 levels of 255 on average and by up to 44 on either side; `min` is 22 above on
//! average and never below. The test holds every pixel to `min` of the two closed-form
//! coverages, and to at least the intersection.

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
    Affine, BlendMode, ClipId, Compose, FillRule, ImageFilter, ImageSpec, Point, Scene,
    SceneBuilder, Segment,
};

mod common;

use common::probe::alpha;
use common::scene::black;

/// The edge fixtures' target, the device column both edges fall inside, and the row read.
const WIDTH: u32 = 16;
const HEIGHT: u32 = 8;
const EDGE_COLUMN: u32 = 2;
const ROW: u32 = 3;

/// A right edge at 0.6 of column 2: `0.6 = 153/255`, exact in eight bits.
const SIX_TENTHS: f32 = 2.6;

/// A pentagon whose right side is the vertical line `x = right` over the whole target: not
/// four axis-aligned edges, so as a clip it is a residue and as a mark it is rasterised.
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

/// A closed polygon as segments.
fn polygon_path(points: &[(f64, f64)]) -> Vec<Segment> {
    let mut path = Vec::with_capacity(points.len() + 1);
    for (i, &(x, y)) in points.iter().enumerate() {
        let p = Point::new(x as f32, y as f32);
        path.push(if i == 0 {
            Segment::MoveTo(p)
        } else {
            Segment::LineTo(p)
        });
    }
    path.push(Segment::Close);
    path
}

fn residue_clip(device: &mut Device, builder: &mut SceneBuilder, path: &[Segment]) -> ClipId {
    let outline = device.upload_outline(path).unwrap();
    builder
        .clip(outline, Affine::IDENTITY, FillRule::NonZero, None)
        .unwrap()
}

fn path_scene(device: &mut Device, mark: &[Segment], clip: &[Segment]) -> Scene {
    let mut builder = SceneBuilder::new();
    let clip = residue_clip(device, &mut builder, clip);
    let outline = device.upload_outline(mark).unwrap();
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

fn rendered(device: &mut Device, scene: &Scene, width: u32, height: u32) -> Vec<u8> {
    device
        .render(
            scene,
            &Viewport::full(width, height, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders")
        .into_raster()
        .unwrap()
        .into_pixels()
}

/// A mark standing on its residue clip's own edge keeps that edge's coverage: both regions
/// are the half-plane `x ≤ 2.6` in column 2, their intersection is that half-plane, and the
/// pixel keeps 0.6 — **153**. The product drew 92.
#[test]
fn a_path_whose_edge_is_its_residue_clips_edge_keeps_that_edges_coverage() {
    let mut device = common::headless::device();
    let scene = path_scene(&mut device, &pentagon(SIX_TENTHS), &pentagon(SIX_TENTHS));
    let pixels = rendered(&mut device, &scene, WIDTH, HEIGHT);
    assert_eq!(
        alpha(&pixels, WIDTH, EDGE_COLUMN, ROW),
        153,
        "0.6 of a pixel intersected with the same 0.6 is 0.6, which is 153 of 255"
    );
}

/// The image lane's residue: a one-texel opaque image stretched over `[0, 2.6] × [0, 8]`
/// under the same pentagon clip keeps its 0.6 in column 2 — **153**. The product drew 92.
#[test]
fn an_image_whose_edge_is_its_residue_clips_edge_keeps_that_edges_coverage() {
    let mut device = common::headless::device();
    let image = device
        .upload_image(&ImageSpec {
            width: 1,
            height: 1,
            data: Arc::from(vec![0_u8, 0, 0, 255]),
        })
        .unwrap();
    let mut builder = SceneBuilder::new();
    let clip = residue_clip(&mut device, &mut builder, &pentagon(SIX_TENTHS));
    builder
        .image(
            image,
            Affine {
                a: SIX_TENTHS,
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
    let scene = builder.finish();
    let pixels = rendered(&mut device, &scene, WIDTH, HEIGHT);
    assert_eq!(
        alpha(&pixels, WIDTH, EDGE_COLUMN, ROW),
        153,
        "0.6 of a pixel intersected with the same 0.6 is 0.6, which is 153 of 255"
    );
}

/// The tiling's target.
const TILING: (u32, u32) = (20, 22);

/// The clip: a regular 64-gon of circumradius 6.1 about `(8.3, 8.6)`, wound
/// counter-clockwise in the `y`-down device space the areas below are taken in.
fn clip_polygon() -> Vec<(f64, f64)> {
    (0..64)
        .map(|i| {
            let angle = std::f64::consts::TAU * f64::from(i) / 64.0;
            (8.3 + 6.1 * angle.cos(), 8.6 + 6.1 * angle.sin())
        })
        .collect()
}

/// The marks: rectangles 1.3 × 0.9 on a 2.0 × 1.7 lattice from `(0.35, 0.2)`, nine by
/// eleven, disjoint, wound as the clip is.
fn rectangles() -> Vec<[(f64, f64); 4]> {
    let mut out = Vec::new();
    for i in 0..9 {
        for j in 0..11 {
            let (x0, y0) = (0.35 + 2.0 * f64::from(i), 0.2 + 1.7 * f64::from(j));
            out.push([
                (x0, y0),
                (x0 + 1.3, y0),
                (x0 + 1.3, y0 + 0.9),
                (x0, y0 + 0.9),
            ]);
        }
    }
    out
}

/// A convex `subject` less everything outside the convex, counter-clockwise `window`
/// (Sutherland–Hodgman), in `f64`.
fn clip_convex(subject: &[(f64, f64)], window: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = subject.to_vec();
    for k in 0..window.len() {
        let (from, to) = (window[k], window[(k + 1) % window.len()]);
        let side = |point: (f64, f64)| {
            (to.0 - from.0) * (point.1 - from.1) - (to.1 - from.1) * (point.0 - from.0)
        };
        let input = std::mem::take(&mut out);
        for m in 0..input.len() {
            let (here, next) = (input[m], input[(m + 1) % input.len()]);
            let (sh, sn) = (side(here), side(next));
            if sh >= 0.0 {
                out.push(here);
            }
            if (sh >= 0.0) != (sn >= 0.0) {
                let t = sh / (sh - sn);
                out.push((
                    here.0 + t * (next.0 - here.0),
                    here.1 + t * (next.1 - here.1),
                ));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// A polygon's area, the shoelace sum halved.
fn area(points: &[(f64, f64)]) -> f64 {
    (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The area of a convex polygon inside pixel `(x, y)`, §10.7.4's `[x, x+1) × [y, y+1)`.
fn in_pixel(polygon: &[(f64, f64)], x: u32, y: u32) -> f64 {
    let (x, y) = (f64::from(x), f64::from(y));
    area(&clip_convex(
        polygon,
        &[(x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0)],
    ))
}

/// **A residue clip over a tiling of small rectangles meets every pixel as a set.**
///
/// Per pixel, from the geometry alone: the marks' coverage `s` (the rectangles are disjoint,
/// so their areas add), the clip's `c`, and the intersection's `i` (each rectangle clipped to
/// the 64-gon, then to the pixel). Every pixel is drawn at `min(s, c)` to within the one
/// level two roundings leave, which is never below `i`; and where one set holds the other in
/// the pixel, `min(s, c)` *is* `i`. The fixture discriminates: the product of the same two
/// values is more than a level below `i` in some pixel and more than a level above it in
/// another, and both counts are asserted from the closed form before the device is asked.
#[test]
fn a_polygon_clip_over_a_tiling_of_rectangles_meets_each_pixel_as_a_set() {
    let clip = clip_polygon();
    let marks = rectangles();
    let flat: Vec<(f64, f64)> = marks.iter().flatten().copied().collect();
    let mark_path: Vec<Segment> = marks
        .iter()
        .flat_map(|rectangle| polygon_path(rectangle))
        .collect();
    assert_eq!(flat.len(), 4 * 99);
    let met: Vec<Vec<(f64, f64)>> = marks
        .iter()
        .map(|rectangle| clip_convex(rectangle, &clip))
        .filter(|piece| piece.len() >= 3)
        .collect();

    let mut device = common::headless::device();
    let scene = path_scene(&mut device, &mark_path, &polygon_path(&clip));
    let pixels = rendered(&mut device, &scene, TILING.0, TILING.1);

    let (mut product_below, mut product_above, mut both_fractional) = (0, 0, 0);
    for y in 0..TILING.1 {
        for x in 0..TILING.0 {
            let s: f64 = marks.iter().map(|r| in_pixel(r, x, y)).sum();
            let c = in_pixel(&clip, x, y);
            let i: f64 = met.iter().map(|piece| in_pixel(piece, x, y)).sum();
            if s > 0.0 && s < 1.0 && c > 0.0 && c < 1.0 {
                both_fractional += 1;
                product_below += usize::from(255.0 * s * c < 255.0 * i - 1.0);
                product_above += usize::from(255.0 * s * c > 255.0 * i + 1.0);
            }
            let drawn = f64::from(alpha(&pixels, TILING.0, x, y));
            let bound = 255.0 * s.min(c);
            assert!(
                (drawn - bound).abs() <= 1.0,
                "pixel ({x}, {y}): {drawn} against min(s, c) = {bound:.2} (s {s:.4}, c {c:.4})"
            );
            assert!(
                drawn >= 255.0 * i - 1.0,
                "pixel ({x}, {y}): {drawn} is below the intersection's {:.2}",
                255.0 * i
            );
            if s.min(c) - i < 1e-9 {
                assert!(
                    (drawn - 255.0 * i).abs() <= 1.0,
                    "pixel ({x}, {y}): one set holds the other, so {drawn} should be the \
                     intersection's {:.2}",
                    255.0 * i
                );
            }
        }
    }
    assert!(
        both_fractional > 0 && product_below > 0 && product_above > 0,
        "the fixture must hold pixels a product gets wrong on both sides: {both_fractional} \
         fractional, {product_below} below, {product_above} above"
    );
}
