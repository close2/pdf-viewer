//! A residue meet one render made is handed to the next render that asks it under the same
//! words, and the bytes are the bytes a fresh device computes (ADR 1517).
//!
//! # Where the expected values come from
//!
//! ISO 32000-2 §10.7.4:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the
//! > set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! The intersection's area in a pixel is a function of the two sets alone, so a meet computed
//! for one render is the meet of any later render of the same sets. The page this exists for
//! draws a four-component group as two frames of the same elements (ADR 1471), each uploading
//! its outlines afresh, so the second scene here uploads its clip and its marks again under new
//! ids and must still be met from the first. Every pixel is also held to the closed-form area,
//! the 64-gon clipped against each rectangle, to within the half level one rounding leaves.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Counters, Device, Target, Viewport};
use raster_scene::{Affine, BlendMode, Compose, FillRule, Scene, SceneBuilder};

mod common;

use common::meet::{
    ARMS, clip_convex, clip_polygon, device_with, held_to_the_intersection, in_pixel, polygon_path,
    residue_clip,
};
use common::scene::black;

/// The target: the 64-gon's box and a margin.
const SIZE: (u32, u32) = (20, 22);

/// Small rectangles, 1.3 × 0.9 on a 2.0 × 1.7 lattice from `(x, y)`, nine by eleven, each a
/// mark of its own so that every one is a meet of its own.
fn rectangles((x, y): (f64, f64)) -> Vec<[(f64, f64); 4]> {
    let mut out = Vec::new();
    for i in 0..9 {
        for j in 0..11 {
            let (x0, y0) = (x + 2.0 * f64::from(i), y + 1.7 * f64::from(j));
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

/// The rectangles under the 64-gon, every outline uploaded for this scene alone.
fn scene(device: &mut Device, marks: &[[(f64, f64); 4]]) -> Scene {
    let mut builder = SceneBuilder::new();
    let clip = residue_clip(device, &mut builder, &polygon_path(&clip_polygon()));
    for rectangle in marks {
        let outline = device.upload_outline(&polygon_path(rectangle)).unwrap();
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
    }
    builder.finish()
}

fn render(device: &mut Device, scene: &Scene) -> (Vec<u8>, Counters) {
    let frame = device
        .render(
            scene,
            &Viewport::full(SIZE.0, SIZE.1, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders");
    let counters = frame.counters();
    (frame.into_raster().unwrap().into_pixels(), counters)
}

/// Every pixel against the intersection's closed form.
fn held(pixels: &[u8], marks: &[[(f64, f64); 4]]) -> usize {
    let clip = clip_polygon();
    let met: Vec<Vec<(f64, f64)>> = marks
        .iter()
        .map(|rectangle| clip_convex(rectangle, &clip))
        .filter(|piece| piece.len() >= 3)
        .collect();
    let (both, ..) = held_to_the_intersection(
        pixels,
        SIZE,
        |x, y| marks.iter().map(|r| in_pixel(r, x, y)).sum(),
        |x, y| in_pixel(&clip, x, y),
        |x, y| met.iter().map(|piece| in_pixel(piece, x, y)).sum(),
    );
    both
}

/// **The second render of the same marks under the same chain draws the first's bytes and
/// rasterises no residue for them**, its outlines uploaded again under new ids, on the walk's
/// lane and the fan-out's at one thread and four.
///
/// Watched failing with the chain named by its outline ids: the second render found nothing
/// and rasterised the residue for every mark again.
#[test]
fn a_render_of_the_same_marks_after_another_is_met_from_it() {
    let marks = rectangles((0.35, 0.2));
    for (coverage, threads) in ARMS {
        let mut fresh = device_with(coverage, threads);
        let first_scene = scene(&mut fresh, &marks);
        let (alone, computed) = render(&mut fresh, &first_scene);
        assert!(
            held(&alone, &marks) > 0,
            "the fixture holds pixels both sets cut"
        );
        assert!(
            computed.clip_residue_regions + computed.clip_residue_tiles > 0,
            "a first render rasterises the residue"
        );

        let second_scene = scene(&mut fresh, &marks);
        let (again, kept) = render(&mut fresh, &second_scene);
        assert_eq!(
            again, alone,
            "{coverage:?} at {threads}: the kept meets are the bytes"
        );
        assert_eq!(
            (kept.clip_residue_regions, kept.clip_residue_tiles),
            (0, 0),
            "{coverage:?} at {threads}: every mark was met from the render before"
        );
    }
}

/// **Marks that differ from the render before are met afresh**: the same lattice a quarter of
/// a pixel across draws what a fresh device draws for it, held to its own closed form.
#[test]
fn a_render_of_other_marks_after_another_is_met_afresh() {
    let marks = rectangles((0.35, 0.2));
    let moved = rectangles((0.6, 0.2));
    for (coverage, threads) in ARMS {
        let mut device = device_with(coverage, threads);
        let first = scene(&mut device, &marks);
        let _ = render(&mut device, &first);
        let second = scene(&mut device, &moved);
        let (after, _) = render(&mut device, &second);

        let mut fresh = device_with(coverage, threads);
        let alone = scene(&mut fresh, &moved);
        let (expected, _) = render(&mut fresh, &alone);
        assert_eq!(after, expected, "{coverage:?} at {threads}");
        assert!(held(&after, &moved) > 0);
    }
}
