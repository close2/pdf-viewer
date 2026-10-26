//! An opaque paint drawn under the backdrop by §11.7.4.3's destination-over is written as its
//! level, and the half level that costs over a translucent backdrop is bounded (ADR 2350).
//!
//! # What is owed
//!
//! Destination-over keeps every channel of the backdrop: §11.3.3's formula with `B(Cb, Cs) = Cb`,
//! premultiplied `cr = cb + (1 − αb)·cs` (ADR 1295). Where the backdrop is empty, `αb = 0`, the
//! source is the pixel, and the solid lanes hand an opaque source over as the level nearest it,
//! as they do under source-over (ADR 2324). Where `0 < αb < 1` the blend scales that level by
//! `1 − αb` before the pixel is final, so the level is rounded before a product: off the clause's
//! value rounded once by at most half a level times `1 − αb`. ADR 2350 weighs handing the source
//! over unrounded instead and keeps the level; this file holds both halves of that decision.
//!
//! # The fixture
//!
//! Bands of three rows, one per backdrop alpha: a black backdrop of that alpha, so `cb = 0` and
//! the owed pixel is `(255 − A) · s` for the stored alpha `A`, read back from the backdrop drawn
//! alone. Each row holds one one-pixel opaque grey per level below 255, the greys of
//! `an_opaque_paint_is_written_as_its_level.rs`: the `f32` nearest each half level and the `f32`
//! either side of it, which is where a source rounded early lands furthest from the pixel.

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

/// The backdrop alphas, in levels: empty, the faintest, and on to the most nearly opaque.
const ALPHAS: [u32; 9] = [0, 1, 32, 64, 128, 192, 224, 250, 254];

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
    match row % 3 {
        0 => f32::from_bits(half.to_bits() - 1),
        1 => half,
        _ => f32::from_bits(half.to_bits() + 1),
    }
}

/// Fills `rect` with `colour` under `compose`.
fn fill(
    device: &mut Device,
    builder: &mut SceneBuilder,
    rect: Rect,
    colour: Color,
    compose: Compose,
) {
    let outline = device.upload_outline(&rect_outline(rect)).expect("upload");
    builder
        .fill(
            outline,
            Affine::IDENTITY,
            FillRule::NonZero,
            Paint::Solid(colour),
            None,
            BlendMode::Normal,
            compose,
            None,
        )
        .expect("a valid solid fill");
}

/// Each band's black backdrop at its alpha; the empty band has none.
fn backdrops(device: &mut Device, builder: &mut SceneBuilder) {
    for (band, &alpha) in ALPHAS.iter().enumerate() {
        if alpha == 0 {
            continue;
        }
        let top = (band * 3) as f32;
        let rect = Rect::new(Point::new(0.0, top), Point::new(WIDTH as f32, top + 3.0));
        let black = Color::new(0.0, 0.0, 0.0, alpha as f32 / 255.0);
        fill(device, builder, rect, black, Compose::SrcOver);
    }
}

#[test]
fn an_opaque_paint_under_the_backdrop_is_its_level_where_the_backdrop_is_empty() {
    let rows = (ALPHAS.len() * 3) as u32;
    for (name, mut device) in devices() {
        let mut builder = SceneBuilder::new();
        backdrops(&mut device, &mut builder);
        let stored = render(&mut device, &builder.finish(), WIDTH, rows);

        let mut builder = SceneBuilder::new();
        backdrops(&mut device, &mut builder);
        for row in 0..rows {
            for k in 0..255 {
                let cell = Rect::new(
                    Point::new(k as f32, row as f32),
                    Point::new(k as f32 + 1.0, row as f32 + 1.0),
                );
                let g = grey(row, k);
                let opaque = Color::new(g, g, g, 1.0);
                fill(&mut device, &mut builder, cell, opaque, Compose::DestOver);
            }
        }
        let pixels = render(&mut device, &builder.finish(), WIDTH, rows);

        let mut empty_wrong = Vec::new();
        let mut beyond_a_level = Vec::new();
        for (band, &alpha) in ALPHAS.iter().enumerate() {
            for row in band as u32 * 3..band as u32 * 3 + 3 {
                for k in 0..255 {
                    let stored_alpha = f64::from(pixel(&stored, WIDTH, k, row)[3]);
                    let exact = (255.0 - stored_alpha) * f64::from(grey(row, k));
                    let owed = exact.round() as i32;
                    let drawn = pixel(&pixels, WIDTH, k, row);
                    assert_eq!(
                        drawn[3], 255,
                        "{name}: the union of an opaque source is opaque"
                    );
                    let drawn = i32::from(drawn[0]);
                    if alpha == 0 && drawn != owed {
                        empty_wrong.push((row, k, owed, drawn));
                    }
                    if (drawn - owed).abs() > 1 {
                        beyond_a_level.push((alpha, row, k, owed, drawn));
                    }
                }
            }
        }
        assert!(
            empty_wrong.is_empty(),
            "{name}: {} of 765 greys under destination-over onto nothing off their level, \
             (row, k, owed, drawn) first: {:?}",
            empty_wrong.len(),
            &empty_wrong[..empty_wrong.len().min(8)]
        );
        assert!(
            beyond_a_level.is_empty(),
            "{name}: {} pixels more than a level from the clause's value rounded once, \
             (alpha, row, k, owed, drawn) first: {:?}",
            beyond_a_level.len(),
            &beyond_a_level[..beyond_a_level.len().min(8)]
        );
    }
}
