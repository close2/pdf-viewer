//! ISO 32000-2 §8.4.3.6: each subpath restarts the dash pattern and reapplies its phase.
//!
//! The first rule below is 12 units long, and phase 3 leaves `[10 10]` five units into its gap.
//! The second rule is only 5 units long. A dasher that carries the first rule's progress
//! into the second paints none of it; the required restart at phase 3 paints it all. Two control
//! points on the first rule catch a renderer that ignores dashes or ignores the nonzero phase.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    reason = "test code: the scene is bounded to a 100-unit page and a small raster, and a
              rasteriser that refuses this supported scene should fail loudly"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

const PAGE: Size = Size {
    width: 100.0,
    height: 100.0,
};

fn two_subpaths() -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 40.5)));
    path.push(PathCommand::LineTo(Point::new(22.0, 40.5)));
    path.push(PathCommand::MoveTo(Point::new(30.0, 40.5)));
    path.push(PathCommand::LineTo(Point::new(35.0, 40.5)));

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 2.0,
            cap: LineCap::Butt,
            dash_array: vec![10.0, 10.0],
            dash_phase: 3.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

fn ink_in_second_subpath(raster: &Raster) -> f64 {
    let mut ink = 0_u64;
    for row in 58_u32..62 {
        for column in 30_u32..35 {
            let at = ((row * raster.width + column) * 4) as usize;
            ink += u64::from(255 - raster.data.get(at).copied().expect("bounded scene pixel"));
        }
    }
    f64::from(ink as u32) / 255.0
}

fn red_at(raster: &Raster, column: u32, row: u32) -> u8 {
    let at = ((row * raster.width + column) * 4) as usize;
    raster.data.get(at).copied().expect("bounded scene pixel")
}

#[test]
fn every_subpath_restarts_with_the_declared_dash_phase() {
    let list = two_subpaths();
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the CPU backend supports dashed strokes");

    assert_eq!(
        red_at(&raster, 15, 59),
        0,
        "phase 3 starts the first dash at x=10"
    );
    assert_eq!(red_at(&raster, 19, 59), 255, "phase 3 puts x=19 in the gap");
    assert_eq!(
        red_at(&raster, 21, 59),
        255,
        "the gap continues past the first path"
    );

    let ink = ink_in_second_subpath(&raster);
    assert!(
        (ink - 10.0).abs() < 0.1,
        "the 5-unit second subpath restarts at phase 3 and should deposit 10 pixel-units;
         it deposited {ink}"
    );
}
