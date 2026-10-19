//! The compute lane's segment arena keeps no more holes than live words (viewer ADR 2037).
//!
//! A released outline's words are a hole in the arena, and the resource budget, which charged
//! the outline's segments at upload, gave them back at the release: so the holes are device
//! memory nothing pays for. The release that would leave more of them than live words
//! compacts the arena — the live ranges copied down on the device — and the outlines still
//! resident must then draw exactly what they drew before, from their new offsets.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Coverage, Device, Options, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, OutlineId, Paint, Point, Scene, SceneBuilder,
    Segment,
};

const SIZE: u32 = 128;

/// A small atlas, so the fills reach the compute lane rather than the glyph atlas in front of
/// it (`compute_lane.rs`'s reason).
const TINY_ATLAS: u64 = 4 * 1024;

fn device() -> Device {
    let device = Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        coverage: Coverage::Compute,
        atlas_budget: TINY_ATLAS,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs");
    device.wait_until_warm();
    device
}

/// A polygon of `corners` corners about `centre`, the corners jittered so that no two
/// outlines are the same words.
fn polygon(centre: Point, radius: f32, corners: usize, phase: f32) -> Vec<Segment> {
    let mut path = Vec::with_capacity(corners + 1);
    for k in 0..corners {
        let angle = phase + k as f32 * std::f32::consts::TAU / corners as f32;
        let reach = radius * (1.0 + 0.15 * ((k * 7) % 5) as f32 / 5.0);
        let point = Point::new(
            centre.x + reach * angle.cos(),
            centre.y + reach * angle.sin(),
        );
        path.push(if k == 0 {
            Segment::MoveTo(point)
        } else {
            Segment::LineTo(point)
        });
    }
    path.push(Segment::Close);
    path
}

/// The outlines that stay: a grid of small polygons.
fn kept_paths() -> Vec<Vec<Segment>> {
    (0..16)
        .map(|n| {
            let centre = Point::new(16.0 + (n % 4) as f32 * 32.0, 16.0 + (n / 4) as f32 * 32.0);
            polygon(centre, 11.0, 5 + n % 3, n as f32 * 0.3)
        })
        .collect()
}

/// The outlines that go: many corners each — past the arena's starting size of 1 MiB together,
/// so that the compacted buffer is smaller than the grown one — and releasing them leaves more
/// hole than live word, so the arena must compact.
fn released_paths() -> Vec<Vec<Segment>> {
    (0..16)
        .map(|n| {
            polygon(
                Point::new(64.0, 64.0),
                20.0 + n as f32 * 2.0,
                8_000,
                n as f32,
            )
        })
        .collect()
}

fn scene(outlines: &[OutlineId]) -> Scene {
    let mut builder = SceneBuilder::new();
    for (n, &outline) in outlines.iter().enumerate() {
        builder
            .fill(
                outline,
                Affine::IDENTITY,
                FillRule::NonZero,
                Paint::Solid(Color::new(0.1 + 0.05 * (n % 10) as f32, 0.4, 0.7, 0.8)),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .unwrap();
    }
    builder.finish()
}

fn draw(device: &mut Device, scene: &Scene) -> Vec<u8> {
    device
        .render(
            scene,
            &Viewport::full(SIZE, SIZE, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("the scene is inside every budget")
        .into_raster()
        .unwrap()
        .into_pixels()
}

fn upload(device: &mut Device, paths: &[Vec<Segment>]) -> Vec<OutlineId> {
    paths
        .iter()
        .map(|path| device.upload_outline(path).unwrap())
        .collect()
}

/// The claim: every release leaves no more holes than live words, the arena compacted where
/// it would have, and the outlines that stayed draw a fresh device's bytes from their new
/// offsets.
#[test]
fn a_release_leaves_no_more_holes_than_live_words_and_the_rest_draws_the_same() {
    let mut fresh = device();
    let fresh_kept = upload(&mut fresh, &kept_paths());
    let expected = draw(&mut fresh, &scene(&fresh_kept));
    assert!(
        expected.iter().any(|&b| b != 0),
        "the fixture draws something, or the comparison is vacuous"
    );

    let mut device = device();
    // Interleaved, so that the kept outlines' ranges move when the arena compacts.
    let mut kept = Vec::new();
    let mut released = Vec::new();
    for (keep, go) in kept_paths().iter().zip(released_paths().iter().cycle()) {
        kept.push(device.upload_outline(keep).unwrap());
        released.push(device.upload_outline(go).unwrap());
    }
    let both: Vec<OutlineId> = kept.iter().chain(&released).copied().collect();
    draw(&mut device, &scene(&both));
    let before = device.segment_arena_bytes();
    assert!(
        before.live > 0 && before.holes == 0 && before.compactions == 0,
        "the fills reached the arena: {before:?}"
    );

    for &outline in &released {
        device.release(outline).unwrap();
        let now = device.segment_arena_bytes();
        assert!(
            now.holes <= now.live,
            "a release left more holes than live words: {now:?}"
        );
    }
    let after = device.segment_arena_bytes();
    println!("before the releases {before:?}, after them {after:?}");
    assert!(
        after.compactions > 0,
        "releasing most of the arena compacted it: {after:?}"
    );
    assert!(
        after.device < before.device && after.device <= (4 * after.live).max(1 << 20),
        "and the buffer shrank to its bound, four times the live bytes or the starting size: \
         {after:?} against {before:?}"
    );
    assert_eq!(
        draw(&mut device, &scene(&kept)),
        expected,
        "the outlines that stayed draw a fresh device's bytes from their new offsets"
    );

    for &outline in &kept {
        device.release(outline).unwrap();
    }
    let empty = device.segment_arena_bytes();
    assert_eq!(
        (empty.live, empty.holes, empty.device),
        (0, 0, 0),
        "an arena with nothing live holds no buffer: {empty:?}"
    );
}
