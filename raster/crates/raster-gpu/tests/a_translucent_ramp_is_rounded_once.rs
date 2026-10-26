//! A ramp whose alpha is below 1 reaches the blend as it is, so its pixel is rounded once, on
//! every adapter present (ADR 2350).
//!
//! # What is owed
//!
//! §8.7.4.5.3 gives a pixel the ramp's colour at its centre's `t`, and §11.3.3's compositing with
//! an opaque backdrop makes the pixel `(1 − α)·cb + α·cs`. The byte is this tree's choice under
//! §10.7, the level nearest that value (ADR 2311), taken once where the pixel is final (trap 257).
//! A lane that rounded the ramp's colour first and let the blend multiply the level by `α` would
//! round it twice, off by up to half a level times `α` before the blend's own rounding.
//!
//! # The fixture
//!
//! Over opaque white, one column per level `L`: a constant ramp of grey `(L − 0.45) / 255` at alpha
//! 0.8, whose nearest level is `L`. Its pixel is `51 + 0.8 (L − 0.45)` levels, and wherever
//! `0.8 L` has a fraction of 0.6 or 0.8 a level taken early lands on the other side of a half.

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

use raster_gpu::{Device, Options};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, Paint, Point, Rect, SceneBuilder, ShadingKind,
    Stop,
};

mod common;

use common::headless::render;
use common::probe::pixel;
use common::scene::rect_outline;

/// Columns in the target: one per level, and one readback row of 1 024 bytes.
const WIDTH: u32 = 256;

/// Rows of the target.
const HEIGHT: u32 = 4;

/// The ramp's alpha, and the share of the white backdrop that shows through, `255 (1 − α)` = 51.
const ALPHA: f32 = 0.8;

/// Every adapter this machine offers a headless device on, llvmpipe among them.
fn devices() -> Vec<(String, Device)> {
    let devices: Vec<(String, Device)> = Device::adapter_names()
        .into_iter()
        .filter_map(|name| {
            let options = Options {
                adapter: Some(name.clone()),
                ..Options::default()
            };
            match Device::headless(&options) {
                Ok(device) => Some((name, device)),
                Err(refused) => {
                    eprintln!("{name}: no headless device ({refused}); not drawn on");
                    None
                }
            }
        })
        .collect();
    assert!(
        devices.iter().any(|(name, _)| name.contains("llvmpipe")),
        "llvmpipe is present wherever this suite runs"
    );
    devices
}

/// The grey of column `level`: 0.45 of a level below it, so `level` is the level nearest it.
fn grey(level: u32) -> f32 {
    ((f64::from(level) - 0.45) / 255.0) as f32
}

/// Fills `rect` with `paint` under the over operator.
fn fill(device: &mut Device, builder: &mut SceneBuilder, rect: Rect, paint: Paint) {
    let outline = device.upload_outline(&rect_outline(rect)).expect("upload");
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
        .expect("a valid fill");
}

#[test]
fn a_translucent_ramp_over_an_opaque_backdrop_is_its_pixel_rounded_once() {
    for (name, mut device) in devices() {
        let mut builder = SceneBuilder::new();
        let whole = Rect::new(
            Point::new(0.0, 0.0),
            Point::new(WIDTH as f32, HEIGHT as f32),
        );
        fill(
            &mut device,
            &mut builder,
            whole,
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 1.0)),
        );
        for level in 1..WIDTH {
            let g = grey(level);
            let stops = [0.0_f32, 1.0].map(|offset| Stop {
                offset,
                color: Color::new(g, g, g, ALPHA),
            });
            let ramp = device
                .upload_ramp(&stops)
                .expect("ascending stops in 0..=1");
            let x = level as f32;
            let paint = Paint::Shading {
                ramp,
                kind: ShadingKind::Axial {
                    start: Point::new(x, 0.0),
                    end: Point::new(x, HEIGHT as f32),
                    extend: (true, true),
                },
                transform: Affine::IDENTITY,
            };
            let column = Rect::new(Point::new(x, 0.0), Point::new(x + 1.0, HEIGHT as f32));
            fill(&mut device, &mut builder, column, paint);
        }
        let pixels = render(&mut device, &builder.finish(), WIDTH, HEIGHT);
        let mut wrong = Vec::new();
        for level in 1..WIDTH {
            let alpha = f64::from(ALPHA);
            let exact = 255.0 * (1.0 - alpha) + alpha * 255.0 * f64::from(grey(level));
            let owed = exact.round() as u8;
            for row in 0..HEIGHT {
                let drawn = pixel(&pixels, WIDTH, level, row)[0];
                if drawn != owed {
                    wrong.push((level, row, owed, drawn));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{name}: {} of {} pixels off the value rounded once, (level, row, owed, drawn) \
             first: {:?}",
            wrong.len(),
            (WIDTH - 1) * HEIGHT,
            &wrong[..wrong.len().min(8)]
        );
    }
}
