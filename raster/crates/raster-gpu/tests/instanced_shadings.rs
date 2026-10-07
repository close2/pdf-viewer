//! A run of shading quads drawn as one instanced draw deposits what the same quads drawn one
//! by one deposit, and a frame collected after another was submitted is the frame `render`
//! draws (ADRs 1594, 1595).
//!
//! **Why each comparison is between two routes of this crate's own.** Both claims are about
//! the route the same numbers take, not about what a shading is: §8.7.4.5's colour at a pixel
//! is computed by the shading lane from the 176 bytes of its quad either way, so the reference
//! is the route the lane took before — a quad per draw, and a frame waited for before the next
//! is walked — and the claim is byte equality, not closeness.
//!
//! - **Instanced against single.** One scene draws its shadings consecutively, so each run of a
//!   window's quads under one paint is one draw; the second interleaves a solid mark after each
//!   shading, which ends every run at one quad; the third wraps each solid mark in a group of its
//!   own, which ends the pass, so every shading is the first and only quad of its window and reads
//!   its numbers where a quad with a binding of its own read them. The solid marks sit in a band
//!   no shading reaches, and the shadings' band is compared, so painter's order puts the same
//!   paint on every compared pixel in all three.
//! - **Pending against rendered.** Two frames of different geometry, the second walked and
//!   submitted — with outlines uploaded and atlas tiles rasterised for it — while the first is
//!   still on the device, then both collected; against the same two frames rendered in turn on a
//!   device of their own.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Device, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, GroupSpec, OutlineId, Paint, Point, RampId, Rect,
    Scene, SceneBuilder, ShadingKind, Stop,
};

mod common;

use common::headless::{device, render};
use common::scene::rect_outline;

/// The target: one readback row is 256 bytes, the copy alignment.
const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

/// More shadings than two of the lane's windows of 93 hold, so a run crosses two window
/// boundaries.
const SHADINGS: u32 = 220;

/// A ramp from a translucent orange to a translucent blue, so that overlapping quads blend and
/// the order of their deposits shows in the bytes.
fn ramp(device: &mut Device) -> RampId {
    device
        .upload_ramp(&[
            Stop {
                offset: 0.0,
                color: Color::new(0.9, 0.5, 0.1, 0.35),
            },
            Stop {
                offset: 1.0,
                color: Color::new(0.1, 0.3, 0.9, 0.6),
            },
        ])
        .expect("ascending stops in 0..=1")
}

/// Shading `k`: an axial sweep over a rectangle in the target's top 48 rows, each placed and
/// angled differently, overlapping its neighbours.
fn shading(k: u32, ramp: RampId) -> (Rect, Paint) {
    let x = (k % 20) as f32 * 2.5;
    let y = (k / 20) as f32 * 3.0;
    let rect = Rect::new(Point::new(x, y), Point::new(x + 14.0, y + 14.0));
    let paint = Paint::Shading {
        ramp,
        transform: Affine::IDENTITY,
        kind: ShadingKind::Axial {
            start: Point::new(x, y + (k % 7) as f32),
            end: Point::new(x + 14.0, y + 14.0 - (k % 5) as f32),
            extend: (true, !k.is_multiple_of(3)),
        },
    };
    (rect, paint)
}

/// Solid mark `k`, in the bottom rows no shading reaches.
fn solid(k: u32) -> (Rect, Paint) {
    let x = (k % 60) as f32;
    let y = 50.0 + (k / 60) as f32 * 3.0;
    let rect = Rect::new(Point::new(x, y), Point::new(x + 1.0, y + 2.0));
    let grey = (k % 9) as f32 / 9.0;
    (rect, Paint::Solid(Color::new(grey, grey, grey, 1.0)))
}

fn fill(builder: &mut SceneBuilder, device: &mut Device, (rect, paint): (Rect, Paint)) {
    let outline = device.upload_outline(&rect_outline(rect)).unwrap();
    builder
        .fill(
            outline,
            Affine::IDENTITY,
            FillRule::NonZero,
            paint,
            None,
            BlendMode::Normal,
            Compose::SrcOver,
            None,
        )
        .unwrap();
}

/// **Every shading run drawn as instances deposits what one draw per shading deposits**, across
/// two window boundaries, with overlapping translucent quads whose order shows in the bytes.
#[test]
fn a_run_of_shadings_drawn_as_instances_is_the_run_drawn_one_by_one() {
    let mut device = device();
    let ramp = ramp(&mut device);

    let mut runs = SceneBuilder::new();
    for k in 0..SHADINGS {
        fill(&mut runs, &mut device, shading(k, ramp));
    }
    for k in 0..SHADINGS {
        fill(&mut runs, &mut device, solid(k));
    }
    let runs = render(&mut device, &runs.finish(), WIDTH, HEIGHT);

    let mut singles = SceneBuilder::new();
    for k in 0..SHADINGS {
        fill(&mut singles, &mut device, shading(k, ramp));
        fill(&mut singles, &mut device, solid(k));
    }
    let singles = render(&mut device, &singles.finish(), WIDTH, HEIGHT);

    let mut alone = SceneBuilder::new();
    let group = GroupSpec {
        alpha: 0.5,
        blend: BlendMode::Normal,
        clip: None,
        knockout: false,
        mask: None,
        isolated: true,
        compose: Compose::SrcOver,
    };
    for k in 0..SHADINGS {
        fill(&mut alone, &mut device, shading(k, ramp));
        let (rect, paint) = solid(k);
        let outline = device.upload_outline(&rect_outline(rect)).unwrap();
        alone
            .group(group, |body| {
                body.fill(
                    outline,
                    Affine::IDENTITY,
                    FillRule::NonZero,
                    paint,
                    None,
                    BlendMode::Normal,
                    Compose::SrcOver,
                    None,
                )
            })
            .unwrap();
    }
    let alone = render(&mut device, &alone.finish(), WIDTH, HEIGHT);

    // The rows the shadings reach, and no solid mark does.
    let band = |pixels: &[u8]| pixels[..(48 * WIDTH * 4) as usize].to_vec();
    assert_eq!(
        band(&runs),
        band(&singles),
        "instanced and single draws deposit the same bytes"
    );
    assert_eq!(
        band(&runs),
        band(&alone),
        "and the same bytes as each quad read at the start of a window of its own"
    );
    assert!(
        band(&runs).chunks_exact(4).filter(|p| p[3] != 0).count() > 2_000,
        "the shadings mark the target"
    );
}

/// One frame of `count` letterform-sized squares at a pitch of `pitch`, each its own outline,
/// in `ink`.
fn squares(device: &mut Device, count: u32, pitch: f32, ink: Color) -> (Scene, Vec<OutlineId>) {
    let mut builder = SceneBuilder::new();
    let mut outlines = Vec::new();
    for k in 0..count {
        let x = (k % 12) as f32 * pitch + 1.0;
        let y = (k / 12) as f32 * pitch + 1.0;
        let side = 2.0 + (k % 4) as f32 * 0.75;
        let outline = device
            .upload_outline(&rect_outline(Rect::new(
                Point::new(0.0, 0.0),
                Point::new(side, side * 1.3),
            )))
            .unwrap();
        outlines.push(outline);
        builder
            .fill(
                outline,
                Affine::translate(x + 0.37, y + 0.61),
                FillRule::NonZero,
                Paint::Solid(ink),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .unwrap();
    }
    (builder.finish(), outlines)
}

/// **A frame walked and submitted while another is on the device, then both collected, is the
/// pair `render` draws in turn** — the second frame's uploads and atlas tiles made after the
/// first was submitted, and each frame's pixels its own.
#[test]
fn a_frame_collected_after_the_next_was_submitted_is_the_frame_render_draws() {
    let viewport = Viewport::full(WIDTH, HEIGHT, Affine::IDENTITY);
    let first_ink = Color::new(0.2, 0.6, 0.9, 0.8);
    let second_ink = Color::new(0.8, 0.1, 0.3, 0.7);

    let mut rendered = device();
    let (first, _) = squares(&mut rendered, 60, 5.0, first_ink);
    let first_rendered = rendered
        .render(&first, &viewport, Target::Readback)
        .unwrap()
        .into_raster()
        .unwrap()
        .into_pixels();
    let (second, _) = squares(&mut rendered, 90, 5.25, second_ink);
    let second_rendered = rendered
        .render(&second, &viewport, Target::Readback)
        .unwrap()
        .into_raster()
        .unwrap()
        .into_pixels();

    let mut overlapped = device();
    let (first, _) = squares(&mut overlapped, 60, 5.0, first_ink);
    let first_pending = overlapped.submit(&first, &viewport).unwrap();
    let (second, _) = squares(&mut overlapped, 90, 5.25, second_ink);
    let second_pending = overlapped.submit(&second, &viewport).unwrap();
    let first_collected = overlapped
        .collect(first_pending)
        .unwrap()
        .into_raster()
        .unwrap()
        .into_pixels();
    let second_collected = overlapped
        .collect(second_pending)
        .unwrap()
        .into_raster()
        .unwrap()
        .into_pixels();

    assert_ne!(first_rendered, second_rendered, "two different pictures");
    assert_eq!(
        first_collected, first_rendered,
        "the first frame's own pixels"
    );
    assert_eq!(
        second_collected, second_rendered,
        "the second frame's own pixels"
    );
}
