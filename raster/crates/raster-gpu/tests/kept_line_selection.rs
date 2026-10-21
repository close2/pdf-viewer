//! Retained line selection must preserve fresh-frame bytes through changing views and budgets.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

mod common;

use common::headless::pixels;
use common::retained::device_with;
use raster_gpu::{Coverage, Options, RetainedScene, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, GroupSpec, LineCap, Point, Rect, Scene, SceneBuilder,
};

/// Four spatial patches exceed one bound each. Translucent overlapping lines make ordering visible.
fn page(group: bool) -> Scene {
    let mut builder = SceneBuilder::new();
    for patch in 0..4 {
        for i in 0..256 {
            let x = 10.0 + (patch % 2) as f32 * 140.0 + (i % 16) as f32;
            let y = 10.0 + (patch / 2) as f32 * 90.0 + (i / 16) as f32 * 3.0;
            builder
                .line(
                    (Point::new(x, y), Point::new(x + 32.0, y + 0.125)),
                    Affine::IDENTITY,
                    ([0.0, 0.03, 1.0][i % 3], i % 3 == 1),
                    [LineCap::Butt, LineCap::Round, LineCap::Square][i % 3],
                    Color::new((i % 7) as f32 / 7.0, 0.3, 0.7, 0.4),
                    None,
                    None,
                )
                .unwrap();
        }
    }
    if group {
        // A child layer disables record replay, so Compute exercises kept selection too.
        builder
            .group(
                GroupSpec {
                    alpha: 0.5,
                    blend: BlendMode::Normal,
                    clip: None,
                    knockout: false,
                    mask: None,
                    isolated: true,
                    compose: Compose::SrcOver,
                },
                |inside| {
                    inside.rect(
                        Rect::new(Point::new(210.0, 170.0), Point::new(214.0, 174.0)),
                        Affine::IDENTITY,
                        Color::new(0.7, 0.2, 0.3, 1.0),
                        None,
                        None,
                    )
                },
            )
            .unwrap();
    }
    builder.finish()
}

#[test]
fn skipped_blocks_preserve_fresh_pixels_across_view_history() {
    for coverage in [Coverage::Cpu, Coverage::Compute] {
        let options = Options {
            coverage,
            ..Options::default()
        };
        let mut device = device_with(&options);
        let mut retained = RetainedScene::new(page(true));
        let fit = Viewport::full(220, 180, Affine::IDENTITY);
        device
            .render_retained(&mut retained, &fit, Target::Readback)
            .unwrap();
        let steps = [
            Affine {
                a: 2.0,
                d: 2.0,
                e: -20.25,
                f: -20.25,
                ..Affine::IDENTITY
            },
            Affine {
                a: -2.0,
                d: 2.0,
                e: 420.25,
                f: -20.25,
                ..Affine::IDENTITY
            },
            Affine {
                a: 0.0,
                b: 2.0,
                c: -2.0,
                d: 0.0,
                e: 220.25,
                f: -20.25,
            },
            Affine::scale(0.1, 0.1),
            Affine::IDENTITY,
        ];
        for (step, transform) in steps.into_iter().enumerate() {
            let view = Viewport::full(220, 180, transform);
            let held = device
                .render_retained(&mut retained, &view, Target::Readback)
                .unwrap();
            assert_eq!(
                held.counters().lines_kept,
                1024,
                "{coverage:?}, step {step}"
            );
            assert_eq!(held.counters().uploads.line_instances, 0);
            if step < 3 {
                assert!(
                    held.counters().line_instances_omitted >= 256,
                    "{coverage:?}, step {step}"
                );
            }
            if step == 4 {
                assert_eq!(held.counters().line_instances_omitted, 0);
            }
            let mut fresh = device_with(&options);
            let walked = fresh.render(&page(true), &view, Target::Readback).unwrap();
            assert_eq!(pixels(held), pixels(walked), "{coverage:?}, step {step}");
        }
    }
}

#[test]
fn an_optional_bounds_refusal_preserves_the_original_frame() {
    let mut constrained = device_with(&Options {
        coverage: Coverage::Cpu,
        max_frame_bytes: 1024 * 64,
        ..Options::default()
    });
    let mut retained = RetainedScene::new(page(false));
    let fit = Viewport::full(220, 180, Affine::IDENTITY);
    let first = constrained
        .render_retained(&mut retained, &fit, Target::Readback)
        .unwrap();
    assert_eq!(first.counters().line_bounds_refused, 1);
    let view = Viewport::full(220, 180, Affine::scale(2.0, 2.0));
    let held = constrained
        .render_retained(&mut retained, &view, Target::Readback)
        .unwrap();
    assert_eq!(held.counters().line_bounds_refused, 1);
    assert_eq!(held.counters().line_instances_omitted, 0);
    assert_eq!(held.counters().lines_kept, 1024);
    let mut fresh = device_with(&Options {
        coverage: Coverage::Cpu,
        ..Options::default()
    });
    let walked = fresh.render(&page(false), &view, Target::Readback).unwrap();
    assert_eq!(pixels(held), pixels(walked));
}
