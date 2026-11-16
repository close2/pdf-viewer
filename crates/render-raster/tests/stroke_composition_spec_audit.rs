//! Section11.6.2: one stroke composites once in each hardware coverage lane.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "small bounded target and explicit setup assertions"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

const PAGE: f32 = 80.0;
const SCALE: f32 = 2.0;
const RED_ONCE: std::ops::RangeInclusive<u8> = 127..=128;

fn scene(with_degenerate_subpath: bool) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut backdrop = Path::new();
    backdrop.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    backdrop.push(PathCommand::LineTo(Point::new(PAGE, 0.0)));
    backdrop.push(PathCommand::LineTo(Point::new(PAGE, PAGE)));
    backdrop.push(PathCommand::LineTo(Point::new(0.0, PAGE)));
    backdrop.push(PathCommand::Close);
    list.push(Command::Fill {
        path: Arc::new(backdrop),
        transform: Transform::IDENTITY,
        fill_rule: pdf_render::FillRule::NonZero,
        paint: Paint::Solid(Color::WHITE),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(20.0, 50.0)));
    path.push(PathCommand::LineTo(Point::new(40.0, 50.0)));
    if with_degenerate_subpath {
        path.push(PathCommand::MoveTo(Point::new(30.0, 50.0)));
        path.push(PathCommand::Close);
    }
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 8.0,
            cap: LineCap::Round,
            join: LineJoin::Miter,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::rgba(0.0, 0.0, 0.0, 0.5)),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

fn interior(raster: &Raster) -> (u8, u8) {
    let column = (30.25 * SCALE) as usize;
    let row = ((PAGE - 50.25) * SCALE) as usize;
    let start = (row * raster.width as usize + column) * 4;
    (raster.data[start], raster.data[start + 3])
}

fn check(coverage: raster_gpu::Coverage, lane_name: &str) {
    gpu_lock_hook::install();
    let mut backend = QuorraRasterizer::with_options(&raster_gpu::Options {
        coverage,
        ..render_raster::options()
    })
    .expect("the requested coverage lane is available");
    eprintln!(
        "{lane_name} coverage lane on {}",
        backend.adapter_description()
    );
    for (has_degenerate, case) in [(false, "line control"), (true, "same-path case")] {
        let list = scene(has_degenerate);
        let target = TargetSpec::for_page(&list, SCALE, 1 << 20).expect("bounded target fits");
        let raster = backend
            .rasterize(&list, target)
            .expect("scene is supported");
        let (red, alpha) = interior(&raster);
        assert_eq!(alpha, 255, "opaque white page for {case}");
        assert!(
            RED_ONCE.contains(&red),
            "{lane_name} {case}: one alpha-.5 application gives red127–128, got{red}; two give about64"
        );
    }
}

#[test]
fn gpu_coverage_applies_one_alpha_for_one_stroke_object() {
    check(raster_gpu::Coverage::Gpu, "gpu");
}

#[test]
fn compute_coverage_applies_one_alpha_for_one_stroke_object() {
    check(raster_gpu::Coverage::Compute, "compute");
}
