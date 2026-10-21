//! Retained selection preserves painter order where runs share global bounds blocks.
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
use raster_gpu::{Coverage, Device, Options, RetainedScene, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, GroupSpec, LineCap, Paint, Point, Rect, Scene,
    SceneBuilder, Segment,
};

fn page(device: &mut Device) -> Scene {
    let triangle = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(10., 10.)),
            Segment::LineTo(Point::new(55., 14.)),
            Segment::LineTo(Point::new(35., 60.)),
            Segment::Close,
        ])
        .unwrap();
    let mut builder = SceneBuilder::new();
    for patch in 0..4 {
        let x = (patch % 2) as f32 * 140.;
        let y = (patch / 2) as f32 * 90.;
        // Adjacent runs share a global bounds block across the intervening translucent fills.
        for index in 0..[257, 259, 257, 259][patch] {
            let row = (index / 16) as f32 * 3.;
            builder
                .line(
                    (
                        Point::new(x + 10., y + 10. + row),
                        Point::new(x + 55., y + 10.25 + row),
                    ),
                    Affine::IDENTITY,
                    ([0., 0.03, 1.][index % 3], index % 3 == 1),
                    LineCap::Round,
                    Color::new((index % 7) as f32 / 7., 0.3, 0.7, 0.4),
                    None,
                    None,
                )
                .unwrap();
        }
        builder
            .rect(
                Rect::new(Point::new(x + 20., y + 15.), Point::new(x + 45., y + 40.)),
                Affine::IDENTITY,
                Color::new(0.8, 0.2, 0.1, 0.4),
                None,
                None,
            )
            .unwrap();
        builder
            .fill(
                triangle,
                Affine::translate(x, y),
                FillRule::NonZero,
                Paint::Solid(Color::new(0.1, 0.8, 0.3, 0.4)),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .unwrap();
    }
    // A child layer keeps this fixture on the scene-walk path for both coverage modes.
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
                    Rect::new(Point::new(210., 170.), Point::new(214., 174.)),
                    Affine::IDENTITY,
                    Color::new(0.7, 0.2, 0.3, 1.),
                    None,
                    None,
                )
            },
        )
        .unwrap();
    builder.finish()
}

#[test]
fn selection_keeps_unaligned_runs_in_painter_order() {
    for coverage in [Coverage::Cpu, Coverage::Compute] {
        let options = Options {
            coverage,
            ..Options::default()
        };
        let mut device = device_with(&options);
        let mut retained = RetainedScene::new(page(&mut device));
        let opened = Viewport::full(220, 180, Affine::IDENTITY);
        let first = device
            .render_retained(&mut retained, &opened, Target::Readback)
            .unwrap();
        assert_eq!(first.counters().lanes.line, 1032);
        let mut fresh = device_with(&options);
        let scene = page(&mut fresh);
        assert_eq!(
            pixels(first),
            pixels(fresh.render(&scene, &opened, Target::Readback).unwrap())
        );

        let transforms = [
            Affine {
                a: 2.0,
                d: 2.0,
                e: -140.25,
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
            Affine::IDENTITY,
        ];
        for (step, transform) in transforms.into_iter().enumerate() {
            let view = Viewport::full(220, 180, transform);
            let held = device
                .render_retained(&mut retained, &view, Target::Readback)
                .unwrap();
            assert_eq!(
                held.counters().lines_kept,
                1032,
                "{coverage:?}, step {step}"
            );
            assert_eq!(
                held.counters().uploads.line_instances,
                0,
                "{coverage:?}, step {step}"
            );
            if step == 3 {
                assert_eq!(held.counters().line_instances_omitted, 0);
            } else {
                assert!(
                    held.counters().line_instances_omitted > 0,
                    "step {step} must exclude some blocks"
                );
            }
            let mut fresh = device_with(&options);
            let scene = page(&mut fresh);
            let walked = fresh.render(&scene, &view, Target::Readback).unwrap();
            assert_eq!(pixels(held), pixels(walked), "{coverage:?}, step {step}");
        }
    }
}
