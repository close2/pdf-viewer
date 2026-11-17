//! Section11.6.2: one stroke composites once through the direct GPU backend.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "small bounded target and explicit setup assertions"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_gpu::GpuRasterizer;

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

#[test]
fn direct_gpu_backend_applies_one_alpha_for_one_stroke_object() {
    // Matches the direct headless-GPU integration suite's device-lock boundary.
    let _gpu = gpu_lock::Hold::unless_software("vello-device");
    let mut backend = GpuRasterizer::new_headless().unwrap_or_else(|error| {
        panic!("the headless GPU adapter is required for this backend assertion: {error}")
    });
    eprintln!(
        "direct render-gpu adapter: {}",
        backend.context().adapter_description()
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
            "direct render-gpu {case}: one alpha-.5 application gives red 127–128, got {red}; a second application is about 64"
        );
    }
}
