//! A clip that is not a rectangle — a **residue**, rasterised to coverage bytes — meets a
//! mark's coverage as a set: by the intersection's own area where both sets cut a pixel and
//! by `min` where one holds the other — on the path lane (ADR 1467) and on the image lane's
//! axis-preserving placements (ADR 1480, `an_image_meets_its_residue_as_a_set.rs`).
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
//! other there. Two bytes say how much and not where, so no function of them is the area: a
//! product is below it wherever a mark stands on its clip's own edge (`0.6 × 0.6` where the
//! area is `0.6`) and above it where the two sets miss each other inside a pixel (`0.3 × 0.3`
//! where the area is `0`), and `min` is above it wherever both edges cross the pixel apart.
//! The path lane has both sets' edges and computes the area, and so does the image lane
//! wherever its image is a rectangle.
//!
//! # The tiling, and what it measures
//!
//! [`a_polygon_clip_over_a_tiling_of_rectangles_meets_each_pixel_as_a_set`] clips a lattice
//! of small rectangles with a regular 64-gon — nothing curved, so every expected value is
//! a polygon's area in a pixel, computed here in `f64` by clipping each polygon to the pixel.
//! Over the pixels where both coverages are fractional, against that area, a product misses
//! by 15 levels of 255 on average and by up to 44 on either side, and `min` is 22 above on
//! average. The test holds every pixel to the intersection's area to within the half level
//! one rounding leaves, and a stroke's pieces to the same.

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

use raster_gpu::Device;
use raster_scene::{
    Affine, BlendMode, Compose, FillRule, ImageFilter, ImageSpec, LineCap, LineJoin, Point, Scene,
    SceneBuilder, Segment, Stroke,
};

mod common;

use common::meet::{
    ARMS, clip_convex, clip_polygon, device_with, held_to_the_intersection, in_pixel, polygon_path,
    rendered, residue_clip,
};
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

/// **A residue clip over a tiling of small rectangles meets every pixel as a set** (ADR 1467).
///
/// Per pixel, from the geometry alone: the marks' coverage `s` (the rectangles are disjoint,
/// so their areas add), the clip's `c`, and the intersection's `i` (each rectangle clipped to
/// the 64-gon, then to the pixel). Every pixel is drawn at `i` to within half a level, on the
/// walk's lane and the fan-out's at one thread and four, and the three draw the same bytes.
/// Under `min` it failed on the pixels both sets cut, up to 43 levels (`(6, 2)`: 55 against
/// 12.06).
/// The fixture discriminates: a product of `s` and `c` is more than a level below `i` in some
/// pixel and above it in another, and `min(s, c)` — ADR 1444's bound — is more than a level
/// above it in some; all three counts are asserted from the closed form.
#[test]
fn a_polygon_clip_over_a_tiling_of_rectangles_meets_each_pixel_as_a_set() {
    let clip = clip_polygon();
    let marks = rectangles();
    let mark_path: Vec<Segment> = marks
        .iter()
        .flat_map(|rectangle| polygon_path(rectangle))
        .collect();
    let met: Vec<Vec<(f64, f64)>> = marks
        .iter()
        .map(|rectangle| clip_convex(rectangle, &clip))
        .filter(|piece| piece.len() >= 3)
        .collect();

    let mut drawn = Vec::new();
    for (coverage, threads) in ARMS {
        let mut device = device_with(coverage, threads);
        let scene = path_scene(&mut device, &mark_path, &polygon_path(&clip));
        let pixels = rendered(&mut device, &scene, TILING.0, TILING.1);
        let (both, below, above, min_above) = held_to_the_intersection(
            &pixels,
            TILING,
            |x, y| marks.iter().map(|r| in_pixel(r, x, y)).sum(),
            |x, y| in_pixel(&clip, x, y),
            |x, y| met.iter().map(|piece| in_pixel(piece, x, y)).sum(),
        );
        assert!(
            both > 0 && below > 0 && above > 0 && min_above > 0,
            "the fixture must hold pixels a product gets wrong on both sides and `min` above: \
             {both} cut by both, {below} below, {above} above, {min_above} above under `min`"
        );
        drawn.push(pixels);
    }
    assert!(
        drawn.windows(2).all(|pair| pair[0] == pair[1]),
        "the walk's lane and the fan-out's at one thread and four draw the same bytes"
    );
}

/// The stroked lines' half-width, and the direction they run in.
const HALF_WIDTH: f64 = 0.65;
const RUN: (f64, f64) = (0.8, 0.6);

/// Five parallel lines at a slant, three units apart, as one outline of five open subpaths.
fn slanted_lines() -> Vec<((f64, f64), (f64, f64))> {
    (0..5)
        .map(|k| {
            let offset = 3.0 * f64::from(k);
            let from = (1.3 + offset * RUN.1, 2.1 - offset * RUN.0 + 9.0);
            let to = (from.0 + 14.0 * RUN.0, from.1 + 14.0 * RUN.1 - 6.0);
            (from, to)
        })
        .collect()
}

/// A line stroked with butt caps, as §8.4.3.3 squares it off at its ends: the rectangle of
/// the half-width either side of it, wound as the clip is.
fn stroked_rectangle(((x0, y0), (x1, y1)): ((f64, f64), (f64, f64))) -> [(f64, f64); 4] {
    let length = (x1 - x0).hypot(y1 - y0);
    let (nx, ny) = (
        -(y1 - y0) / length * HALF_WIDTH,
        (x1 - x0) / length * HALF_WIDTH,
    );
    let corners = [
        (x0 + nx, y0 + ny),
        (x1 + nx, y1 + ny),
        (x1 - nx, y1 - ny),
        (x0 - nx, y0 - ny),
    ];
    // Wound counter-clockwise in device space, as `clip_convex` wants its subject.
    let signed: f64 = (0..4)
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    if signed < 0.0 {
        [corners[3], corners[2], corners[1], corners[0]]
    } else {
        corners
    }
}

/// **A stroke meets its residue clip as a set too** (ADR 1467): the stroke's own pieces are
/// what the meet reads, on the fan-out's commit (a solid stroke) and on the walk's tile.
///
/// §8.4.3.2 paints "all points whose perpendicular distance from the path in user space is
/// less than or equal to half the line width", and with butt caps a straight line's set is
/// the rectangle of that half-width along it; five such lines three units apart do not
/// overlap, so their areas add, and each meets the 64-gon as a convex polygon does.
#[test]
fn a_stroke_under_a_polygon_clip_meets_each_pixel_as_a_set() {
    let clip = clip_polygon();
    let lines = slanted_lines();
    let bodies: Vec<[(f64, f64); 4]> = lines.iter().map(|&l| stroked_rectangle(l)).collect();
    let met: Vec<Vec<(f64, f64)>> = bodies
        .iter()
        .map(|body| clip_convex(body, &clip))
        .filter(|piece| piece.len() >= 3)
        .collect();
    let path: Vec<Segment> = lines
        .iter()
        .flat_map(|&((x0, y0), (x1, y1))| {
            [
                Segment::MoveTo(Point::new(x0 as f32, y0 as f32)),
                Segment::LineTo(Point::new(x1 as f32, y1 as f32)),
            ]
        })
        .collect();
    let mut drawn = Vec::new();
    for (coverage, threads) in ARMS {
        let mut device = device_with(coverage, threads);
        let mut builder = SceneBuilder::new();
        let clip_id = residue_clip(&mut device, &mut builder, &polygon_path(&clip));
        let outline = device.upload_outline(&path).unwrap();
        builder
            .stroke(
                outline,
                Affine::IDENTITY,
                Stroke {
                    width: (2.0 * HALF_WIDTH) as f32,
                    adjust: false,
                    cap: LineCap::Butt,
                    join: LineJoin::Miter,
                    miter_limit: 10.0,
                },
                black(),
                Some(clip_id),
                BlendMode::Normal,
                None,
            )
            .unwrap();
        let scene = builder.finish();
        let pixels = rendered(&mut device, &scene, TILING.0, TILING.1);
        let (both, ..) = held_to_the_intersection(
            &pixels,
            TILING,
            |x, y| bodies.iter().map(|b| in_pixel(b, x, y)).sum(),
            |x, y| in_pixel(&clip, x, y),
            |x, y| met.iter().map(|piece| in_pixel(piece, x, y)).sum(),
        );
        assert!(both > 0, "the strokes must cross the clip's rim");
        drawn.push(pixels);
    }
    assert!(
        drawn.windows(2).all(|pair| pair[0] == pair[1]),
        "the walk's lane and the fan-out's at one thread and four draw the same bytes"
    );
}
