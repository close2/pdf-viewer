//! A frame's exact meets are made once the walk has placed every tile, beside each other, and
//! written where the walk packed each tile (ADR 1541) — and they draw the intersection's area
//! exactly as a meet made in place does, at every thread count and on every lane.
//!
//! # Where the expected values come from
//!
//! ISO 32000-2 §10.7.4:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the
//! > set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! Each mark is a diamond inside a three-pixel cell of its own, so no two marks reach one
//! pixel and a pixel's alpha is one mark's coverage: the area of that diamond met with the
//! clip, a convex polygon's area in the pixel computed here in `f64`
//! (`common::meet::held_to_the_intersection`). The clip is a 64-gon, a residue, whose edge
//! crosses 41 diamonds, so the frame holds over the settle's floor of pixels both sets cut —
//! enough to divide across threads.
//!
//! # The two budgets
//!
//! At the default frame budget every meet is settled once, when the frame finishes. At 64 KiB
//! the meets the walk holds pass the queue's limit, a sixty-fourth of that, every eighteen
//! marks, so the walk settles them in batches as it goes. Both draw the same bytes.

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

use raster_gpu::{Coverage, Device, Options};
use raster_scene::{Affine, BlendMode, Compose, FillRule, Scene, SceneBuilder};

mod common;

use common::meet::{
    clip_convex, held_to_the_intersection, in_pixel, polygon_path, rendered, residue_clip,
};
use common::scene::black;

/// The target, square.
const SIDE: u32 = 48;

/// The clip: a regular 64-gon of circumradius 19.4 about `(24.3, 24.6)`, wound
/// counter-clockwise in the `y`-down device space the areas are taken in.
fn clip() -> Vec<(f64, f64)> {
    (0..64)
        .map(|i| {
            let angle = std::f64::consts::TAU * f64::from(i) / 64.0;
            (24.3 + 19.4 * angle.cos(), 24.6 + 19.4 * angle.sin())
        })
        .collect()
}

/// One diamond per three-pixel cell, its half-diagonal 1.0 about the point 1.67 into the
/// cell both ways, so it spans `0.67 .. 2.67` of its cell and shares no pixel with another.
/// No pixel holds more than 0.942 of it: a pixel a set covers within a rounding of whole is
/// met by `min` (ADR 1467), which the half-level tolerance below does not admit twice.
fn diamonds() -> Vec<Vec<(f64, f64)>> {
    let mut marks = Vec::new();
    for row in 0..SIDE / 3 {
        for column in 0..SIDE / 3 {
            let (cx, cy) = (3.0 * f64::from(column) + 1.67, 3.0 * f64::from(row) + 1.67);
            marks.push(vec![
                (cx, cy - 1.0),
                (cx + 1.0, cy),
                (cx, cy + 1.0),
                (cx - 1.0, cy),
            ]);
        }
    }
    marks
}

/// Every diamond as a fill of its own under the 64-gon's residue.
fn scene(device: &mut Device, marks: &[Vec<(f64, f64)>]) -> Scene {
    let mut builder = SceneBuilder::new();
    let clip = residue_clip(device, &mut builder, &polygon_path(&clip()));
    for mark in marks {
        let outline = device.upload_outline(&polygon_path(mark)).unwrap();
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

fn device(coverage: Coverage, threads: usize, max_frame_bytes: Option<u64>) -> Device {
    let defaults = Options::default();
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        coverage,
        encode_threads: threads,
        max_frame_bytes: max_frame_bytes.unwrap_or(defaults.max_frame_bytes),
        ..defaults
    })
    .expect("llvmpipe is present wherever this suite runs")
}

#[test]
fn meets_settled_after_the_walk_draw_the_intersection_at_every_thread_count() {
    let clip = clip();
    let marks = diamonds();
    let met: Vec<Vec<(f64, f64)>> = marks
        .iter()
        .map(|mark| clip_convex(mark, &clip))
        .filter(|piece| piece.len() >= 3)
        .collect();

    let mut drawn = Vec::new();
    for budget in [None, Some(1 << 16)] {
        for (coverage, threads) in [
            (Coverage::Gpu, 1),
            (Coverage::Gpu, 4),
            (Coverage::Cpu, 1),
            (Coverage::Cpu, 4),
        ] {
            let mut device = device(coverage, threads, budget);
            let scene = scene(&mut device, &marks);
            let pixels = rendered(&mut device, &scene, SIDE, SIDE);
            let (both, _, _, min_above) = held_to_the_intersection(
                &pixels,
                (SIDE, SIDE),
                |x, y| marks.iter().map(|mark| in_pixel(mark, x, y)).sum(),
                |x, y| in_pixel(&clip, x, y),
                |x, y| met.iter().map(|piece| in_pixel(piece, x, y)).sum(),
            );
            assert!(
                both >= 64 && min_above > 0,
                "the fixture must reach the settle's floor and hold pixels `min` draws above \
                 the intersection: {both} cut by both, {min_above} above under `min`"
            );
            drawn.push(pixels);
        }
    }
    assert!(
        drawn.windows(2).all(|pair| pair[0] == pair[1]),
        "every lane, thread count and budget draws the same bytes"
    );
}
