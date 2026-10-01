//! A frame draws what its scene says, whatever frames came before it on that device.
//!
//! Everything a device keeps between frames — the glyph atlas, the winding texture, the
//! pipeline store — is a cache, and a cache is only sound while it is invisible. So the
//! shape of every test here is the same: render a scene on a device that has already
//! drawn something *else*, and require the pixels to equal what a device that had drawn
//! nothing produces. A difference is not a tolerance, it is a defect: the same scene
//! reached both.
//!
//! The order that matters is **large then small**, because that is the direction a cache
//! grows in and the one a viewer walks when a person zooms in and back out. It is also
//! the direction the caller's `QUORRA_FEEDBACK.md` section 11 reported from the window: a page
//! magnified past 1000% and brought back drew one letter as another, and kept doing it.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Coverage, Device, Options, Target, Viewport};
use raster_scene::{Affine, BlendMode, Compose, FillRule, Point, Scene, SceneBuilder, Segment};

mod common;

use common::scene::black;

/// The side of the target the compared frame is drawn at.
const SMALL: u32 = 48;
/// The side of the target the frame *before* it is drawn at — large enough that its
/// coverage tiles pack a taller scratch sheet than the small frame's do.
const LARGE: u32 = 240;

fn device(coverage: Coverage) -> Device {
    let device = Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        coverage,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs");
    device.wait_until_warm();
    device
}

/// A curve scaled to `side`, past `MAX_GLYPH_DIM` at either size, so no atlas stands in
/// front of it and its coverage lands on the frame's scratch sheet.
fn blob(device: &mut Device, side: u32) -> Scene {
    let s = side as f32 / 240.0;
    let at = |x: f32, y: f32| Point::new(x * s, y * s);
    let outline = device
        .upload_outline(&[
            Segment::MoveTo(at(20.0, 20.0)),
            Segment::CubicTo {
                c1: at(220.0, 4.0),
                c2: at(236.0, 160.0),
                to: at(120.0, 220.0),
            },
            Segment::CubicTo {
                c1: at(40.0, 236.0),
                c2: at(4.0, 120.0),
                to: at(20.0, 20.0),
            },
            Segment::Close,
        ])
        .unwrap();
    let mut builder = SceneBuilder::new();
    builder
        .fill(
            outline,
            Affine::IDENTITY,
            FillRule::NonZero,
            black(),
            None,
            BlendMode::Normal,
            Compose::SrcOver,
            None,
        )
        .unwrap();
    builder.finish()
}

fn draw(device: &mut Device, side: u32) -> Vec<u8> {
    let scene = blob(device, side);
    device
        .render(
            &scene,
            &Viewport::full(side, side, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("the scene is inside every budget")
        .into_raster()
        .unwrap()
        .into_pixels()
}

/// The GPU coverage lane keeps one winding texture across frames and grows it to the
/// largest sheet any frame has needed (`winding.rs` states the measurement that bought
/// the reuse: allocating and zeroing it per frame cost 10.7 ms of a 15 ms frame). A
/// frame whose sheet is *shorter* than that texture must still draw its own coverage.
///
/// This is section 11 of the caller's feedback, made small: their ladder went wrong at 6400%
/// and stayed wrong at 1600% on the way down, while 3200% — whose sheet was the tallest
/// the device had seen — stayed right. Two frames on one device is the whole recipe.
#[test]
fn a_short_gpu_frame_after_a_tall_one_draws_its_own_coverage() {
    let alone = draw(&mut device(Coverage::Gpu), SMALL);

    let mut device = device(Coverage::Gpu);
    let large = draw(&mut device, LARGE);
    let after = draw(&mut device, SMALL);

    assert!(
        large.iter().skip(3).step_by(4).any(|&a| a > 0),
        "the frame that goes first draws something, or it warms nothing"
    );
    assert_eq!(
        after, alone,
        "the same scene, drawn on a device that had drawn a larger frame"
    );
}

/// The same ordering under the CPU lane, which shares the scratch sheet but makes no
/// winding texture — the control that says the defect above is the lane's and not the
/// sheet's.
#[test]
fn a_short_cpu_frame_after_a_tall_one_draws_its_own_coverage() {
    let alone = draw(&mut device(Coverage::Cpu), SMALL);

    let mut device = device(Coverage::Cpu);
    let _ = draw(&mut device, LARGE);
    let after = draw(&mut device, SMALL);

    assert_eq!(after, alone);
}

/// The atlas side this section's devices are given: room for a few dozen small glyph tiles.
const SMALL_ATLAS: u64 = 160 * 160;

fn atlas_device(max_frame_bytes: u64) -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        atlas_budget: SMALL_ATLAS,
        max_frame_bytes,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

/// A scene of `count` distinct small outlines, each a lobed curve about 20 pixels across,
/// placed twice on a lattice — glyph tiles the atlas admits and would keep — over one large
/// square where `backdrop` asks, whose tile no atlas this size admits and which therefore
/// sets the scratch sheet. `seed` makes the outlines differ from another scene's, so two
/// scenes share no atlas key.
fn glyph_page(device: &mut Device, count: u32, seed: f32, backdrop: bool) -> Scene {
    let mut builder = SceneBuilder::new();
    if backdrop {
        let square = [
            (-100.0, -100.0),
            (100.0, -100.0),
            (100.0, 100.0),
            (-100.0, 100.0),
        ];
        let mut path: Vec<Segment> = square
            .iter()
            .enumerate()
            .map(|(k, &(x, y))| {
                let p = Point::new(x, y);
                if k == 0 {
                    Segment::MoveTo(p)
                } else {
                    Segment::LineTo(p)
                }
            })
            .collect();
        path.push(Segment::Close);
        let outline = device.upload_outline(&path).unwrap();
        builder
            .fill(
                outline,
                Affine {
                    a: 0.9,
                    b: 0.3,
                    c: -0.3,
                    d: 0.9,
                    e: 120.5,
                    f: 120.5,
                },
                FillRule::NonZero,
                black(),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .unwrap();
    }
    for i in 0..count {
        let r = 8.0 + seed + (i % 7) as f32 * 0.13;
        let lobes = 5 + i % 4;
        let point =
            |angle: f32, radius: f32| Point::new(radius * angle.cos(), radius * angle.sin());
        let mut path = vec![Segment::MoveTo(point(0.0, r))];
        for step in 0..lobes {
            let to = (step + 1) as f32 / lobes as f32 * std::f32::consts::TAU;
            let mid = (step as f32 + 0.5) / lobes as f32 * std::f32::consts::TAU;
            path.push(Segment::CubicTo {
                c1: point(mid, r * 1.2),
                c2: point(mid, r * 1.2),
                to: point(to, r),
            });
        }
        path.push(Segment::Close);
        let outline = device.upload_outline(&path).unwrap();
        for copy in 0..2 {
            let x = 12.0 + ((i * 2 + copy) % 11) as f32 * 21.0;
            let y = 12.0 + ((i * 2 + copy) / 11) as f32 * 21.0;
            builder
                .fill(
                    outline,
                    Affine::translate(x, y),
                    FillRule::NonZero,
                    black(),
                    None,
                    BlendMode::Normal,
                    Compose::SrcOver,
                    None,
                )
                .unwrap();
        }
    }
    builder.finish()
}

/// Draw `scene`'s builder on `device` at the target this section uses, or say why not.
fn try_draw(device: &mut Device, scene: &Scene) -> Result<Vec<u8>, String> {
    device
        .render(
            scene,
            &Viewport::full(LARGE, LARGE, Affine::IDENTITY),
            Target::Readback,
        )
        .map(|frame| frame.into_raster().unwrap().into_pixels())
        .map_err(|error| error.to_string())
}

/// **A frame's admission against the frame budget is its own** (ADR 1467 section 4).
///
/// A page of glyphs that fills a small atlas, then a page of other glyphs at the least
/// frame budget it is drawn under alone. Answered by the full atlas, the second page's
/// glyphs fell through to the scratch sheet, the sheet grew, and the page was refused —
/// a verdict decided by the page before it. Its verdict and its bytes must be the ones a
/// fresh device gives it.
#[test]
fn a_page_after_one_that_filled_the_atlas_is_admitted_as_it_is_alone() {
    let fits_alone = |budget: u64| {
        let mut device = atlas_device(budget);
        let page = glyph_page(&mut device, 40, 0.0, true);
        try_draw(&mut device, &page).is_ok()
    };
    // The least budget the page is drawn under alone, by bisection.
    let (mut refused, mut drawn) = (0_u64, 64 << 20);
    assert!(fits_alone(drawn), "the page draws at a generous budget");
    while drawn - refused > 1 {
        let mid = refused + (drawn - refused) / 2;
        if fits_alone(mid) {
            drawn = mid;
        } else {
            refused = mid;
        }
    }
    let mut fresh = atlas_device(drawn);
    let page = glyph_page(&mut fresh, 40, 0.0, true);
    let alone = try_draw(&mut fresh, &page).expect("drawn alone at its own least budget");

    let mut used = atlas_device(drawn);
    let filler = glyph_page(&mut used, 52, 0.5, false);
    let page = glyph_page(&mut used, 40, 0.0, true);
    assert!(
        try_draw(&mut used, &filler).is_ok(),
        "the filling page is drawn at this budget and leaves the atlas nearly full"
    );
    let after = try_draw(&mut used, &page);
    assert_eq!(
        after.as_ref().map(Vec::len),
        Ok(alone.len()),
        "the page after the filling one was refused at the budget it is drawn under alone: {after:?}"
    );
    assert!(
        after.unwrap() == alone,
        "and its pixels are the ones a fresh device draws"
    );
}
