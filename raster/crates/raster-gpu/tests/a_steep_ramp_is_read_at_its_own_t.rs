//! A ramp's colour is the function at the pixel's own `t`, however steep the ramp (ISO 32000-2
//! §8.7.4.5.3, §7.10.3; ADR 2310).
//!
//! # The clause
//!
//! §8.7.4.5.3 maps each point of an axial shading to a parameter by its projection onto the
//! axis, and "[t]he variable t becomes the input argument to the colour function(s)" (Table
//! 79's `Domain`). A device pixel's colour is the function at the `t` of its centre — §10.7.4's
//! "point whose coordinate values have fractional parts of one-half". Between two stops a ramp
//! is §7.10.3's type 2 with `N` of 1, `C0 + x × (C1 − C0)`: so the colour at a centre is exact
//! arithmetic, and the byte a pixel holds is the level nearest it.
//!
//! # The fixture
//!
//! `issue14165.pdf`'s construction made steep: a ramp of 33 stops alternating black and white,
//! along an axis of 256 device rows from row 0 down, so the grey rises or falls 255 levels over
//! eight rows. Every row centre is a sixteenth of an interval from a stop, so the colour there
//! is `255 · j / 16` for an odd `j`, never half way between two levels: the nearest level is
//! decided by arithmetic alone. A table of the ramp read at its nearest texel moves `t` by up to
//! half of `1/4095`, a thirty-second of a row, which on this slope is a level.

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

use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, Paint, Point, Rect, SceneBuilder, ShadingKind,
    Stop,
};

mod common;

use common::headless::{device, render};
use common::probe::pixel;
use common::scene::rect_outline;

/// Columns in the target: one readback row is 256 bytes, the copy alignment.
const WIDTH: u32 = 64;

/// Rows of the axis, and of the target.
const HEIGHT: u32 = 256;

/// Intervals of the ramp: stop `k` at `k / 32`, black where `k` is even and white where odd.
const INTERVALS: u32 = 32;

/// The level §8.7.4.5.3 and §7.10.3 give device row `row`, in exact arithmetic: the centre's
/// `t = (row + ½) / 256`, the interval `k` holding it, and the type 2 interpolation across it.
fn clause_level(row: u32) -> u8 {
    let t = (f64::from(row) + 0.5) / f64::from(HEIGHT);
    let along = t * f64::from(INTERVALS);
    let k = along.floor();
    let x = along - k;
    let grey = if (k as u32).is_multiple_of(2) {
        x
    } else {
        1.0 - x
    };
    (grey * 255.0).round() as u8
}

/// The rows whose grey is not the clause's level, with the level drawn and the level owed.
fn rows_off_the_clause() -> Vec<(u32, u8, u8)> {
    let mut device = device();
    let stops: Vec<Stop> = (0..=INTERVALS)
        .map(|k| {
            let grey = if k % 2 == 0 { 0.0 } else { 1.0 };
            Stop {
                offset: k as f32 / INTERVALS as f32,
                color: Color::new(grey, grey, grey, 1.0),
            }
        })
        .collect();
    let ramp = device
        .upload_ramp(&stops)
        .expect("ascending stops in 0..=1");
    let outline = device
        .upload_outline(&rect_outline(Rect::new(
            Point::new(0.0, 0.0),
            Point::new(WIDTH as f32, HEIGHT as f32),
        )))
        .expect("upload");
    let paint = Paint::Shading {
        ramp,
        kind: ShadingKind::Axial {
            start: Point::new(32.0, 0.0),
            end: Point::new(32.0, HEIGHT as f32),
            extend: (false, false),
        },
        transform: Affine::IDENTITY,
    };
    let mut builder = SceneBuilder::new();
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
        .expect("a valid shaded fill");
    let pixels = render(&mut device, &builder.finish(), WIDTH, HEIGHT);
    (0..HEIGHT)
        .filter_map(|row| {
            let drawn = pixel(&pixels, WIDTH, WIDTH / 2, row)[0];
            let owed = clause_level(row);
            (drawn != owed).then_some((row, drawn, owed))
        })
        .collect()
}

/// Every row of the steep ramp is the level nearest the function at its centre's `t`.
#[test]
fn every_row_of_a_steep_ramp_is_the_level_its_own_t_gives() {
    let wrong = rows_off_the_clause();
    assert!(
        wrong.is_empty(),
        "{} of {HEIGHT} rows off the clause's level, as (row, drawn, owed): {wrong:?}",
        wrong.len()
    );
}

/// A ramp's colour a hair below a half level stays below it: `0.7` as an `f32` is 178.499 997
/// levels, which an `f32` product with 255 rounds onto 178.5 and a half-up rounding then carries to
/// 179. The level nearest the colour itself is 178 (ADR 2311), on a constant ramp at every row.
#[test]
fn a_colour_a_hair_below_a_half_level_is_the_level_below() {
    let mut device = device();
    let grey = 0.7_f32;
    let owed = (f64::from(grey) * 255.0).round() as u8;
    let stops = [0.0_f32, 1.0].map(|offset| Stop {
        offset,
        color: Color::new(grey, grey, grey, 1.0),
    });
    let ramp = device
        .upload_ramp(&stops)
        .expect("ascending stops in 0..=1");
    let outline = device
        .upload_outline(&rect_outline(Rect::new(
            Point::new(0.0, 0.0),
            Point::new(WIDTH as f32, 8.0),
        )))
        .expect("upload");
    let paint = Paint::Shading {
        ramp,
        kind: ShadingKind::Axial {
            start: Point::new(32.0, 0.0),
            end: Point::new(32.0, 8.0),
            extend: (false, false),
        },
        transform: Affine::IDENTITY,
    };
    let mut builder = SceneBuilder::new();
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
        .expect("a valid shaded fill");
    let pixels = render(&mut device, &builder.finish(), WIDTH, 8);
    let drawn: Vec<u8> = (0..8)
        .map(|row| pixel(&pixels, WIDTH, WIDTH / 2, row)[0])
        .collect();
    assert_eq!(owed, 178, "the level nearest 0.7 as an f32");
    assert!(
        drawn.iter().all(|&level| level == owed),
        "rows drawn {drawn:?}, the level owed {owed}"
    );
}
