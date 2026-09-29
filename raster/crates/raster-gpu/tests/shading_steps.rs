//! A shading function's hard step is placed at its own `t`, not at the nearest texel of a
//! table (ISO 32000-2 §8.7.4.5.3, §7.10.4; ADR 1389).
//!
//! # The clause
//!
//! §8.7.4.5.3 maps each point of an axial shading to a parameter by its projection onto the
//! axis, and "[t]he variable t becomes the input argument to the colour function(s)" (Table
//! 79's `Domain`). A device pixel's colour is therefore the function at the `t` of the
//! pixel's centre — §10.7.4 names the centre as "the point whose coordinate values have
//! fractional parts of one-half". Where the function is a §7.10.4 stitching of constant
//! pieces, its value jumps at each bound, and a pixel whose centre lies below the bound
//! takes the lower piece's colour whatever texel grid a renderer samples the function on.
//!
//! # The fixture
//!
//! `issue10572.pdf`'s stripe shape, stated in device space: an axis of `1800 · s` device
//! rows from `(32, 1616 s)` to `(32, −184 s)`, twenty-four constant stripes of `1/24` of the
//! parameter each, a rectangle over rows `0 .. 450 s`. Every stripe bound falls on a whole
//! device row at `s` = 1, 2, 4 and 8, so every pixel lies wholly in one stripe and its colour
//! is decided by arithmetic alone: nothing is left to antialiasing or to rounding.

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

use raster_gpu::Device;
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, Paint, Point, RampId, Rect, SceneBuilder,
    ShadingKind, Stop,
};

mod common;

use common::headless::{device, render};
use common::probe::pixel;
use common::scene::rect_outline;

/// Columns in the target: one readback row is 256 bytes, the copy alignment.
const WIDTH: u32 = 64;

/// Stripes in the fixture's stitching function, as in the page's twelve pairs.
const STRIPES: u32 = 24;

/// Stripe `k`'s grey level, distinct for every stripe so that a row in its neighbour's
/// stripe is a different byte.
fn stripe_level(k: u32) -> u8 {
    (k * 10 + 5) as u8
}

/// Twenty-four constant pieces, each a coincident pair of offsets at its bounds — the
/// ramp a §7.10.4 stitching of constant type 2 functions arrives as.
fn stripe_ramp(device: &mut Device) -> RampId {
    let mut stops = Vec::new();
    for k in 0..STRIPES {
        let grey = f32::from(stripe_level(k)) / 255.0;
        let color = Color::new(grey, grey, grey, 1.0);
        stops.push(Stop {
            offset: k as f32 / STRIPES as f32,
            color,
        });
        stops.push(Stop {
            offset: (k + 1) as f32 / STRIPES as f32,
            color,
        });
    }
    device
        .upload_ramp(&stops)
        .expect("ascending stops in 0..=1")
}

/// The stripe §8.7.4.5.3 puts at device row `row` under scale `s`, in exact arithmetic:
/// the row's centre projected onto the axis, `x′ = (1616 s − (row + ½)) / (1800 s)`, and
/// the stripe the half-open interval `[k/24, (k+1)/24)` holding it (§7.10.4).
fn clause_stripe(row: u32, s: u32) -> u32 {
    let s = f64::from(s);
    let x = (1616.0 * s - (f64::from(row) + 0.5)) / (1800.0 * s);
    ((x * f64::from(STRIPES)).floor() as u32).min(STRIPES - 1)
}

/// The rows at scale `s` whose colour is not their stripe's.
fn rows_wrong_at(s: u32) -> Vec<u32> {
    let mut device = device();
    let ramp = stripe_ramp(&mut device);
    let height = 450 * s;
    let sf = s as f32;
    let outline = device
        .upload_outline(&rect_outline(Rect::new(
            Point::new(0.0, 0.0),
            Point::new(WIDTH as f32, height as f32),
        )))
        .expect("upload");
    let paint = Paint::Shading {
        ramp,
        kind: ShadingKind::Axial {
            start: Point::new(32.0, 1616.0 * sf),
            end: Point::new(32.0, -184.0 * sf),
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
    let pixels = render(&mut device, &builder.finish(), WIDTH, height);
    (0..height)
        .filter(|&row| {
            let expected = stripe_level(clause_stripe(row, s));
            pixel(&pixels, WIDTH, WIDTH / 2, row)[0] != expected
        })
        .collect()
}

/// Every row of the stripe page is its own stripe's colour at 1×, 2×, 4× and 8×.
#[test]
fn a_hard_step_falls_between_the_two_rows_its_bound_separates() {
    let wrong: Vec<(u32, Vec<u32>)> = [1, 2, 4, 8]
        .into_iter()
        .map(|s| (s, rows_wrong_at(s)))
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    assert!(
        wrong.is_empty(),
        "rows drawn in a neighbouring stripe's colour, per scale: {wrong:?}"
    );
}
