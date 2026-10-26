//! An opaque solid paint reaches an eight-bit target as the level nearest it, on every adapter
//! present, and an antialiased edge is left to the blend (ADR 2324).
//!
//! # What is owed
//!
//! ISO 32000-2 §10.7 leaves the conversion of a colour to the device's bits to the device, so the
//! byte is this tree's documented choice, not a clause's: the level nearest the colour's own value,
//! half away from zero (ADR 2311). Where the solid lanes' source is opaque the blend keeps none of
//! the backdrop and the fragment's colour is the pixel, so the shader hands it over as that level.
//! Where the source is not opaque the pixel is the blend's, from the backdrop and the source as it
//! is, so the pixel is rounded once — the second test pins that a source there is not rounded
//! first.
//!
//! # Why every adapter
//!
//! RADV exports a fragment's colour to an eight-bit target as a half float rounded toward zero,
//! up to an eighth of a level low, where llvmpipe rounds to nearest (trap 251); and an `fma`
//! residual that states the rounding exactly on llvmpipe did not survive RADV's compiler (ADR
//! 2324 §2). So the fixture runs on llvmpipe, which every run of this suite has, and on each
//! other adapter the machine offers.
//!
//! # The fixture
//!
//! For each level `k` below 255, three opaque greys: the `f32` nearest the half level
//! `(k + ½) / 255`, and the `f32` either side of it. The middle one's exact product with 255 lies
//! within an `f32` step of `k + ½`, the case an `f32` product with 255 rounds onto the half and a
//! half-float export takes below it; the two either side are decided by arithmetic alone.

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
use raster_scene::{Affine, BlendMode, Color, Compose, FillRule, Paint, Point, Rect, SceneBuilder};

mod common;

use common::headless::render;
use common::probe::pixel;
use common::scene::rect_outline;

/// Columns in the target: one per level below 255, and one readback row of 1 024 bytes.
const WIDTH: u32 = 256;

/// Rows of the target: the three greys of each level, one row each.
const ROWS: u32 = 3;

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

/// The grey of row `row` at level `k`: one `f32` step below the half level, the `f32` nearest
/// it, and one step above.
fn grey(row: u32, k: u32) -> f32 {
    let half = ((f64::from(k) + 0.5) / 255.0) as f32;
    match row {
        0 => f32::from_bits(half.to_bits() - 1),
        1 => half,
        _ => f32::from_bits(half.to_bits() + 1),
    }
}

/// Fills `rect` with an opaque grey under the over operator.
fn fill(device: &mut Device, builder: &mut SceneBuilder, rect: Rect, grey: f32) {
    let outline = device.upload_outline(&rect_outline(rect)).expect("upload");
    builder
        .fill(
            outline,
            Affine::IDENTITY,
            FillRule::NonZero,
            Paint::Solid(Color::new(grey, grey, grey, 1.0)),
            None,
            BlendMode::Normal,
            Compose::SrcOver,
            None,
        )
        .expect("a valid solid fill");
}

#[test]
fn an_opaque_grey_is_written_as_the_level_nearest_it_on_every_adapter() {
    for (name, mut device) in devices() {
        let mut builder = SceneBuilder::new();
        for row in 0..ROWS {
            for k in 0..255 {
                let cell = Rect::new(
                    Point::new(k as f32, row as f32),
                    Point::new(k as f32 + 1.0, row as f32 + 1.0),
                );
                fill(&mut device, &mut builder, cell, grey(row, k));
            }
        }
        let pixels = render(&mut device, &builder.finish(), WIDTH, ROWS);
        let mut wrong = Vec::new();
        for row in 0..ROWS {
            for k in 0..255 {
                let owed = (f64::from(grey(row, k)) * 255.0).round() as u8;
                let drawn = pixel(&pixels, WIDTH, k, row)[0];
                if drawn != owed {
                    wrong.push((row, k, owed, drawn));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{name}: {} of {} greys off their level, (row, k, owed, drawn) first: {:?}",
            wrong.len(),
            ROWS * 255,
            &wrong[..wrong.len().min(8)]
        );
    }
}

/// A grey of 0.5 at a coverage of 76.6 levels over opaque white: the blend makes it
/// `255 − 38.3 = 216.7` and the pixel 217, where a source rounded before the blend — 38 for the
/// colour and 77 for its alpha — would make it 216. The edge is the column the rectangle covers
/// in part.
///
/// llvmpipe's blend takes the source at eight bits itself, so it draws 216 whatever the shader
/// hands it — the adapter's own rounding, inside ADR 0006's bound — and the test holds it to that;
/// RADV's blend carries the source's fraction, and there a source the shader had rounded would be
/// the one level off this test names.
#[test]
fn an_antialiased_edge_is_rounded_once_by_the_blend() {
    let coverage = 76.6_f32 / 255.0;
    for (name, mut device) in devices() {
        let mut builder = SceneBuilder::new();
        let whole = Rect::new(Point::new(0.0, 0.0), Point::new(WIDTH as f32, 1.0));
        fill(&mut device, &mut builder, whole, 1.0);
        let edge = Rect::new(Point::new(0.0, 0.0), Point::new(coverage, 1.0));
        fill(&mut device, &mut builder, edge, 0.5);
        let pixels = render(&mut device, &builder.finish(), WIDTH, 1);
        let blended =
            (255.0 * (0.5 * f64::from(coverage) + 1.0 - f64::from(coverage))).round() as u8;
        assert_eq!(blended, 217, "the blend's single rounding");
        let owed = if name.contains("llvmpipe") { 216 } else { 217 };
        assert_eq!(
            pixel(&pixels, WIDTH, 0, 0)[0],
            owed,
            "{name}: the edge column"
        );
    }
}
