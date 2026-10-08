//! A frame whose exact meets are all recorded in one drain's commit starts one set of threads,
//! not two.
//!
//! The commit of a drain meets each tile under a residue clip with the clip (ISO 32000-2
//! §10.7.4, ADR 1467) and records the meets whose exact pixels are still to be made; the
//! frame's helpers make them beside the walk (ADR 1541). The drain's fan-out threads are lent
//! to those meets for as long as the drain commits (ADR 1692), so a frame like the stroked Type
//! 3 page — one drain past the fan-out's floor, every meet in its commit — starts the fan-out's
//! threads and no others: `threads - 1` a frame, where it started twice that.
//!
//! **One test in its own binary, on purpose.** `raster_gpu::threads::started` is the process's
//! count, read as a difference around one frame; a second test of the same binary drawing on
//! another thread would put its threads into this one's difference.
//!
//! `llvmpipe` by name, as most of this suite does, so CI on a software rasteriser reads the same
//! numbers.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Device, Options, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, FillRule, LineCap, LineJoin, Paint, Point, Scene, SceneBuilder,
    Segment, Stroke,
};

const SIDE: u32 = 220;

/// The threads the host allows, which is the most the fan-out starts in one drain, the calling
/// thread among them.
const THREADS: usize = 8;

fn device(threads: usize) -> Device {
    let device = Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        encode_threads: threads,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs");
    device.wait_until_warm();
    device
}

/// A closed curve of `lobes` cubic pieces, so that a clip of it leaves a residue and a mark of
/// it cuts the residue's pixels.
fn blob(lobes: u32, radius: f32) -> Vec<Segment> {
    let point = |angle: f32, r: f32| Point::new(r * angle.cos(), r * angle.sin());
    let mut path = vec![Segment::MoveTo(point(0.0, radius))];
    for step in 0..lobes {
        let from = f32::from(step as u16) / lobes as f32 * std::f32::consts::TAU;
        let to = f32::from(step as u16 + 1) / lobes as f32 * std::f32::consts::TAU;
        let (a, b) = (point(from, radius), point(to, radius * 0.8));
        path.push(Segment::CubicTo {
            c1: Point::new(a.x + (b.x - a.x) * 0.3, a.y + (b.y - a.y) * 0.1),
            c2: Point::new(a.x + (b.x - a.x) * 0.7, a.y + (b.y - a.y) * 0.9),
            to: b,
        });
    }
    path.push(Segment::Close);
    path
}

/// A hundred and twenty strokes of one small curve, each at its own fractional placement along
/// the edge of a curved clip, so that every tile is met with the residue in the one drain at the
/// frame's end and the cut pixels pass the helpers' floor many times over.
fn stroked_under_a_curve(device: &mut Device) -> Scene {
    let mark = device.upload_outline(&blob(12, 9.0)).unwrap();
    let region = device.upload_outline(&blob(24, 80.0)).unwrap();
    let mut builder = SceneBuilder::new();
    let clip = builder
        .clip(
            region,
            Affine::translate(110.0, 110.0),
            FillRule::NonZero,
            None,
        )
        .unwrap();
    for index in 0..120_u32 {
        let angle = index as f32 / 120.0 * std::f32::consts::TAU;
        let at = Affine::translate(
            110.0 + 72.0 * angle.cos() + 0.37,
            110.0 + 72.0 * angle.sin() + 0.61,
        );
        builder
            .stroke(
                mark,
                at,
                Stroke {
                    width: 2.5,
                    adjust: false,
                    cap: LineCap::Butt,
                    join: LineJoin::Round,
                    miter_limit: 4.0,
                },
                Paint::Solid(Color::new(0.1, 0.3, 0.8, 0.9)),
                Some(clip),
                BlendMode::Normal,
                None,
            )
            .unwrap();
    }
    builder.finish()
}

/// The frame's pixels, and the threads raster started while drawing it.
fn draw(threads: usize) -> (Vec<u8>, u64) {
    let mut device = device(threads);
    let scene = stroked_under_a_curve(&mut device);
    let before = raster_gpu::threads::started();
    let frame = device
        .render(
            &scene,
            &Viewport::full(SIDE, SIDE, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("the fixture is inside every budget");
    let started = raster_gpu::threads::started() - before;
    (frame.into_raster().unwrap().into_pixels(), started)
}

#[test]
fn a_drain_whose_commit_records_every_meet_starts_one_set_of_threads() {
    let (alone, none) = draw(1);
    assert_eq!(none, 0, "a host that allowed one thread is given none");
    let (divided, started) = draw(THREADS);
    assert!(
        divided == alone,
        "the frame drawn on {THREADS} threads is not the frame drawn on one"
    );
    assert_eq!(
        started,
        THREADS as u64 - 1,
        "the fan-out's threads, lent to the meets its commit records, are the frame's only set"
    );
}
