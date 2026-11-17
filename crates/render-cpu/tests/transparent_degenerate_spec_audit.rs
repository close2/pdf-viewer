//! EC3 #103 / §11.6.2 control: parts of one stroked path must not composite twice.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::expect_used,
    reason = "small bounded scene and target; a rejected ordinary stroke is a test failure"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

const PAGE: f32 = 80.0;
const SCALE: f32 = 2.0;

fn stroke(with_degenerate_subpath: bool) -> Raster {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(20.0, 50.0)));
    path.push(PathCommand::LineTo(Point::new(40.0, 50.0)));
    if with_degenerate_subpath {
        path.push(PathCommand::MoveTo(Point::new(30.0, 50.0)));
        path.push(PathCommand::Close);
    }

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
    let target = TargetSpec::for_page(&list, SCALE, 1 << 20).expect("valid small target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a single stroked path is supported")
}

fn red_at(raster: &Raster, x: f32, y: f32) -> u8 {
    // Raster rows begin at the page's top; PDF user-space y increases from the bottom.
    let column = (x * SCALE) as u32;
    let row = ((PAGE - y) * SCALE) as u32;
    let index = ((row * raster.width + column) * 4) as usize;
    raster
        .data
        .get(index)
        .copied()
        .expect("sample is inside page")
}

#[test]
fn a_degenerate_subpath_does_not_composite_twice_with_the_same_stroke_object() {
    // Positive control: the pixel is wholly inside the width-8 line. One alpha-.5 stroke
    // over white leaves the red channel at 127–129 (8-bit rounding).
    let line = stroke(false);
    let baseline = red_at(&line, 30.25, 50.25);
    assert!(
        (127..=129).contains(&baseline),
        "ordinary line red {baseline}"
    );

    // Section 11.6.2 forbids compositing parts of one graphics object with each other.
    // EC3 #103 qualifies the subpath-versus-dash rule, and gives no exception to that
    // one-object requirement. Here the width-8 ordinary stroke already covers the zero-length
    // round-capped circle completely, so whether transparent processing contributes that
    // extra geometry or not, the object alpha in this pixel is still exactly one application.
    let combined = stroke(true);
    let got = red_at(&combined, 30.25, 50.25);
    assert!(
        (127..=129).contains(&got),
        "one source path is one object: expected red {baseline}, got {got} (two SourceOver draws would be about 64)"
    );
}
