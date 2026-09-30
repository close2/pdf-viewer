//! The compute coverage lane, held to the CPU lane's bytes (ADR 0080).
//!
//! `Coverage::Compute` is not another answer to the coverage question the way
//! [`Coverage::Gpu`] is — it is the *same* answer computed by the device: the shader is
//! a statement-for-statement port of `fill_mask`, whose determinism
//! `tests/compute_coverage_determinism.rs` measures in isolation. What that file cannot
//! see is the lane around the shader — the routing, the seats, the edge extraction, the
//! image round-trip, the mixed sheet — so this file renders whole scenes both ways and
//! compares the frames.
//!
//! **Where byte equality is claimed and where it is not.** A fill the CPU lane sends
//! through the path lane is rasterised under its full device transform, exactly as the
//! compute lane rasterises everything — same flattening inputs, same arithmetic, same
//! bytes. A fill the CPU lane caches rasterises at the *quantised phase* and draws at
//! the integer origin (ADR 0009), so its flattening sees different float values and the
//! last unorm step can differ. So the exact comparisons below keep the atlas out of the
//! way (a tiny budget, `coverage_lanes.rs`'s trick), and the one scene that exercises
//! the atlas asserts a one-step bound instead and says why.
//!
//! **Same arithmetic is not yet the same bits under every transform.** The mosaic's fills
//! are all placed by the identity; glyphs scaled, rotated and set at a fractional phase put
//! a pixel on a rounding tie now and then, and there the lanes are one level apart
//! ([`the_compute_lane_is_within_one_level_of_the_cpu_lane_on_glyphs`], ADR 1407).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    clippy::unreadable_literal,
    reason = "test-file policy as in coverage_lanes.rs, plus one scene builder that is               one fixture and an LCG seed quoted as-is"
)]

use raster_gpu::{Coverage, Device, Options, RetainedScene, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, LineCap, LineJoin, Paint, Point, Scene,
    SceneBuilder, Segment, Stroke,
};

const SIZE: u32 = 256;

/// A small atlas, so the CPU lane's fills reach the path lane rather than being
/// rasterised at a quantised phase in front of it — the condition under which byte
/// equality is the honest claim (module comment).
const TINY_ATLAS: u64 = 4 * 1024;

/// Every adapter on this machine, by name — llvmpipe is always among them where this
/// suite runs, and a real adapter joins on developer machines (`m45.rs`'s discipline).
fn adapter_names() -> Vec<String> {
    Device::adapter_names()
        .into_iter()
        .filter(|name| {
            // One name per driver: the same GPU can appear through Vulkan and GL, and
            // rendering on both doubles the suite for a comparison m6.rs already makes.
            !name.contains("radeonsi")
        })
        .collect()
}

fn device_with(adapter: &str, coverage: Coverage) -> Device {
    Device::headless(&Options {
        adapter: Some(adapter.into()),
        coverage,
        atlas_budget: TINY_ATLAS,
        ..Options::default()
    })
    .expect("an adapter that enumerated can be opened")
}

fn render(adapter: &str, coverage: Coverage, scene: &dyn Fn(&mut Device) -> Scene) -> Vec<u8> {
    let mut device = device_with(adapter, coverage);
    device.wait_until_warm();
    let built = scene(&mut device);
    device
        .render(
            &built,
            &Viewport::full(SIZE, SIZE, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("the scene is inside every budget")
        .into_raster()
        .unwrap()
        .into_pixels()
}

fn diff(a: &[u8], b: &[u8]) -> (usize, u8) {
    let mut pixels = 0;
    let mut max = 0_u8;
    for (a, b) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = a
            .iter()
            .zip(b)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        if worst > 0 {
            pixels += 1;
            max = max.max(worst);
        }
    }
    (pixels, max)
}

/// The Entwurf shape: a mosaic of abutting quads over a jittered lattice, every corner
/// shared, every coordinate fractional — plus a seven-point star for self-crossings
/// under both rules, and one stroke so the sheet carries a CPU tile beside the compute
/// tiles (the mixed-sheet seeding is exactly what it exercises).
fn mosaic(device: &mut Device) -> Scene {
    let mut lcg = 0x5DEECE66D_u64;
    let mut random = move |low: f32, high: f32| {
        lcg = lcg.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let unit = ((lcg >> 33) as f32) / (u32::MAX >> 1) as f32;
        low + unit * (high - low)
    };
    let mut builder = SceneBuilder::new();
    let cell = 25.0_f32;
    let mut corners = Vec::new();
    for gy in 0..10 {
        let mut row = Vec::new();
        for gx in 0..10 {
            row.push(Point::new(
                gx as f32 * cell + random(-4.0, 4.0),
                gy as f32 * cell + random(-4.0, 4.0),
            ));
        }
        corners.push(row);
    }
    for gy in 0..9 {
        for gx in 0..9 {
            let quad = [
                corners[gy][gx],
                corners[gy][gx + 1],
                corners[gy + 1][gx + 1],
                corners[gy + 1][gx],
            ];
            let outline = device
                .upload_outline(&[
                    Segment::MoveTo(quad[0]),
                    Segment::LineTo(quad[1]),
                    Segment::LineTo(quad[2]),
                    Segment::LineTo(quad[3]),
                    Segment::Close,
                ])
                .expect("a quad");
            builder
                .fill(
                    outline,
                    Affine::IDENTITY,
                    FillRule::NonZero,
                    Paint::Solid(Color::new(
                        random(0.2, 1.0),
                        random(0.1, 0.7),
                        random(0.0, 0.4),
                        1.0,
                    )),
                    None,
                    BlendMode::Normal,
                    Compose::SrcOver,
                    None,
                )
                .expect("a quad fill the builder admits");
        }
    }
    for (rule, offset) in [(FillRule::NonZero, 60.0), (FillRule::EvenOdd, 150.0)] {
        let mut star = Vec::new();
        for k in 0..7 {
            let angle = (k * 3) as f32 * (std::f32::consts::TAU / 7.0);
            let point = Point::new(
                offset + 40.0 * angle.cos() + random(-0.4, 0.4),
                offset + 40.0 * angle.sin() + random(-0.4, 0.4),
            );
            star.push(if k == 0 {
                Segment::MoveTo(point)
            } else {
                Segment::LineTo(point)
            });
        }
        star.push(Segment::Close);
        let outline = device.upload_outline(&star).expect("a star");
        builder
            .fill(
                outline,
                Affine::IDENTITY,
                rule,
                Paint::Solid(Color::new(0.1, 0.3, 0.8, 0.8)),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .expect("a star fill the builder admits");
    }
    // Cubics, so the flattening arithmetic itself is under test and not only the
    // scanline pass: wavy blobs whose curves cross the flatness test at many scales,
    // a sub-pixel loop that exercises the relative-tolerance branch, and a closed
    // curve whose chord is degenerate (p0 = p3), which is the epsilon fallback.
    for blob in 0..5 {
        let cx = 40.0 + blob as f32 * 42.0 + random(-3.0, 3.0);
        let cy = 210.0 + random(-6.0, 6.0);
        let r = 12.0 + random(0.0, 10.0);
        let outline = device
            .upload_outline(&[
                Segment::MoveTo(Point::new(cx - r, cy)),
                Segment::CubicTo {
                    c1: Point::new(cx - r, cy - r * random(1.0, 1.8)),
                    c2: Point::new(cx + r, cy - r * random(1.0, 1.8)),
                    to: Point::new(cx + r, cy),
                },
                Segment::CubicTo {
                    c1: Point::new(cx + r, cy + r * random(1.0, 1.8)),
                    c2: Point::new(cx - r, cy + r * random(1.0, 1.8)),
                    to: Point::new(cx - r, cy),
                },
                Segment::Close,
            ])
            .expect("a blob");
        builder
            .fill(
                outline,
                Affine::IDENTITY,
                FillRule::NonZero,
                Paint::Solid(Color::new(0.9, 0.5, 0.1, 0.9)),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .expect("a blob fill the builder admits");
    }
    let degenerate = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(120.0, 120.0)),
            Segment::CubicTo {
                c1: Point::new(160.0, 80.0),
                c2: Point::new(160.0, 160.0),
                to: Point::new(120.0, 120.0),
            },
            Segment::Close,
        ])
        .expect("a loop whose chord is a point");
    let speck = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(200.25, 200.25)),
            Segment::CubicTo {
                c1: Point::new(200.55, 199.95),
                c2: Point::new(200.85, 200.55),
                to: Point::new(200.35, 200.65),
            },
            Segment::Close,
        ])
        .expect("a sub-pixel curve");
    for outline in [degenerate, speck] {
        builder
            .fill(
                outline,
                Affine::IDENTITY,
                FillRule::NonZero,
                Paint::Solid(Color::new(0.2, 0.2, 0.2, 1.0)),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .expect("a curve fill the builder admits");
    }
    let diagonal = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(10.0, 240.0)),
            Segment::LineTo(Point::new(240.0, 10.0)),
        ])
        .expect("a line");
    builder
        .stroke(
            diagonal,
            Affine::IDENTITY,
            outline_stroke(),
            Paint::Solid(Color::new(0.0, 0.0, 0.0, 1.0)),
            None,
            BlendMode::Normal,
            None,
        )
        .expect("a stroke the builder admits");
    builder.finish()
}

fn outline_stroke() -> Stroke {
    Stroke {
        width: 3.0,
        adjust: false,
        cap: LineCap::Butt,
        join: LineJoin::Miter,
        miter_limit: 10.0,
    }
}

/// **The lane's whole claim, held frame for frame**: a scene of abutting fills,
/// self-crossing stars under both rules, and a stroked line sharing the sheet renders
/// the same bytes whether the scanline pass ran on the host or on the device — on every
/// adapter this machine has.
#[test]
fn the_compute_lane_draws_the_cpu_lanes_bytes() {
    let adapters = adapter_names();
    assert!(!adapters.is_empty(), "no adapter at all on this machine");
    for adapter in &adapters {
        let cpu = render(adapter, Coverage::Cpu, &mosaic);
        let compute = render(adapter, Coverage::Compute, &mosaic);
        let (pixels, max) = diff(&cpu, &compute);
        println!("{adapter}: {pixels} pixel(s) differ, max {max}");
        assert_eq!(
            (pixels, max),
            (0, 0),
            "{adapter}: the compute lane diverged from the CPU lane"
        );
    }
}

/// A second frame on one device replays the retained encode; the dispatch re-runs per
/// frame from the same record, so the bytes must not move between the first frame and
/// the replay (ADR 0048's contract, extended to this lane).
#[test]
fn a_replayed_compute_frame_draws_the_same_bytes() {
    let mut device = device_with("llvmpipe", Coverage::Compute);
    device.wait_until_warm();
    let scene = mosaic(&mut device);
    let viewport = Viewport::full(SIZE, SIZE, Affine::IDENTITY);
    let mut retained = RetainedScene::new(scene);
    let first = device
        .render_retained(&mut retained, &viewport, Target::Readback)
        .expect("draws")
        .into_raster()
        .unwrap()
        .into_pixels();
    let second = device
        .render_retained(&mut retained, &viewport, Target::Readback)
        .expect("replays")
        .into_raster()
        .unwrap()
        .into_pixels();
    assert_eq!(first, second, "a replay moved pixels");
}

/// A letter `v` rotated 60° and a letter `w` on a half-pixel baseline, as `issue1905.pdf`'s first
/// page draws them — outline and composed transform read off the page, one placement each — and
/// 150 more placements of each at random rotations and phases.
fn glyphs(device: &mut Device) -> Scene {
    let point = Point::new;
    let cubic = |c1: (f32, f32), c2: (f32, f32), to: (f32, f32)| Segment::CubicTo {
        c1: point(c1.0, c1.1),
        c2: point(c2.0, c2.1),
        to: point(to.0, to.1),
    };
    let lines = |points: &[(f32, f32)]| {
        let mut path = vec![Segment::MoveTo(point(points[0].0, points[0].1))];
        path.extend(
            points[1..]
                .iter()
                .map(|&(x, y)| Segment::LineTo(point(x, y))),
        );
        path
    };
    let mut letter_v = lines(&[
        (0.645, 0.678),
        (0.4, 0.066),
        (0.397, 0.066),
        (0.157, 0.678),
        (0.107, 0.678),
        (0.083, 0.0),
        (0.12, 0.0),
        (0.137, 0.624),
        (0.14, 0.624),
        (0.378, 0.02),
        (0.416, 0.02),
        (0.658, 0.624),
        (0.661, 0.624),
        (0.678, 0.0),
        (0.718, 0.0),
        (0.694, 0.678),
    ]);
    letter_v.push(Segment::Close);
    let mut letter_w = lines(&[(0.645, 0.488), (0.581, 0.258)]);
    letter_w.extend([
        cubic((0.563, 0.195), (0.554, 0.155), (0.552, 0.107)),
        Segment::LineTo(point(0.549, 0.107)),
        cubic((0.546, 0.155), (0.538, 0.195), (0.522, 0.258)),
        Segment::LineTo(point(0.464, 0.488)),
        Segment::LineTo(point(0.328, 0.488)),
        Segment::LineTo(point(0.27, 0.257)),
        cubic((0.254, 0.195), (0.246, 0.155), (0.244, 0.107)),
        Segment::LineTo(point(0.241, 0.107)),
        cubic((0.238, 0.155), (0.229, 0.195), (0.211, 0.259)),
        Segment::LineTo(point(0.147, 0.488)),
        Segment::LineTo(point(0.003, 0.488)),
        Segment::LineTo(point(0.17, 0.0)),
        Segment::LineTo(point(0.309, 0.0)),
        Segment::LineTo(point(0.367, 0.227)),
        cubic((0.376, 0.263), (0.387, 0.321), (0.39, 0.372)),
        Segment::LineTo(point(0.393, 0.372)),
        cubic((0.396, 0.321), (0.408, 0.263), (0.417, 0.226)),
        Segment::LineTo(point(0.475, 0.0)),
        Segment::LineTo(point(0.613, 0.0)),
        Segment::LineTo(point(0.78, 0.488)),
        Segment::Close,
    ]);
    let (v, w) = (
        device.upload_outline(&letter_v).expect("a glyph"),
        device.upload_outline(&letter_w).expect("a glyph"),
    );
    let mut lcg = 0x5DEECE66D_u64;
    let mut random = move || {
        lcg = lcg.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        ((lcg >> 33) as f32) / (u32::MAX >> 1) as f32
    };
    let mut placements = vec![
        (
            v,
            Affine {
                a: 4.0,
                b: -6.92816,
                c: -6.92816,
                d: -4.0,
                e: 493.8,
                f: 608.19995,
            },
        ),
        (
            w,
            Affine {
                a: 18.0,
                b: 0.0,
                c: 0.0,
                d: -18.0,
                e: 248.37997,
                f: 295.5,
            },
        ),
    ];
    for k in 0..150 {
        let (x, y) = ((k % 15) as f32 * 16.0 + 2.0, (k / 15) as f32 * 16.0 + 2.0);
        let angle = random() * std::f32::consts::TAU;
        let (sin, cos) = (8.0 * angle.sin(), 8.0 * angle.cos());
        placements.push((
            v,
            Affine {
                a: cos,
                b: -sin,
                c: -sin,
                d: -cos,
                e: x + 6.0 + random(),
                f: y + 6.0 + random(),
            },
        ));
        placements.push((
            w,
            Affine {
                a: 18.0,
                b: 0.0,
                c: 0.0,
                d: -18.0,
                e: x + 330.0 + random(),
                f: y + 9.0 + random(),
            },
        ));
    }
    let mut builder = SceneBuilder::new();
    for (outline, transform) in placements {
        builder
            .fill(
                outline,
                transform,
                FillRule::NonZero,
                Paint::Solid(Color::new(0.0, 0.0, 0.0, 1.0)),
                None,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
            .expect("a glyph fill the builder admits");
    }
    builder.finish()
}

/// **The compute lane and the CPU lane are one level apart at most, on glyphs as a page draws
/// them** — the bound ADR 0082 allows between a device and the processor, and the one the room
/// probe's lane choice relied on being zero (ADR 1407).
///
/// Both lanes compute §10.7.4's coverage, the area of the pixel's half-open square inside the
/// set, by the same flattening and trapezoid deposits in `f32`, the WGSL a statement-for-statement
/// port. [`mosaic`]'s transforms are all the identity, and there the two agree to the byte; under
/// a scale, a rotation and a fractional phase they do not, on RADV and on llvmpipe both, and every
/// pixel that differs is one level apart — on `issue1905.pdf` the first three read 127 against 128
/// under a baseline on a half-pixel row, a coverage of exactly one half. Which operation rounds
/// differently is not isolated (a device compiler may evaluate `a · b + c` with one rounding
/// where the processor takes two). The atlas and ADR 0090's hybrid are both switched off, so
/// that the CPU lane's own arithmetic is what is compared.
#[test]
fn the_compute_lane_is_within_one_level_of_the_cpu_lane_on_glyphs() {
    const PAGE: u32 = 640;
    let render = |adapter: &str, coverage: Coverage| {
        let mut device = Device::headless(&Options {
            adapter: Some(adapter.into()),
            coverage,
            atlas_budget: 1,
            compute_assist: Some(false),
            ..Options::default()
        })
        .expect("an adapter that enumerated can be opened");
        device.wait_until_warm();
        let scene = glyphs(&mut device);
        device
            .render(
                &scene,
                &Viewport::full(PAGE, PAGE, Affine::IDENTITY),
                Target::Readback,
            )
            .expect("the scene is inside every budget")
            .into_raster()
            .unwrap()
            .into_pixels()
    };
    for adapter in &adapter_names() {
        let (pixels, max) = diff(
            &render(adapter, Coverage::Cpu),
            &render(adapter, Coverage::Compute),
        );
        println!("{adapter}: {pixels} pixel(s) differ, max {max}");
        assert!(
            max <= 1,
            "{adapter}: the lanes differ by {max} levels at a pixel"
        );
    }
}
