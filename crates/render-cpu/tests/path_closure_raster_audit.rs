//! ISO 32000-2 §8.5.3.2: an explicit line to a subpath start ends in caps;
//! `h` closes with a join.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "the scene is bounded to a small raster and a supported stroke must render"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

fn raster(commands: &[PathCommand]) -> Raster {
    let mut path = Path::new();
    for command in commands {
        path.push(*command);
    }
    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 2.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("stroke is supported")
}

fn vertical_leg_pixels(raster: &Raster) -> Vec<u8> {
    let mut pixels = Vec::new();
    for row in 29_u32..70 {
        for column in 18_u32..23 {
            let start = ((row * raster.width + column) * 4) as usize;
            pixels.extend_from_slice(
                raster
                    .data
                    .get(start..start + 4)
                    .expect("bounded leg pixel"),
            );
        }
    }
    pixels
}

#[test]
fn explicit_line_to_start_uses_caps_where_h_uses_a_join() {
    let common = [
        PathCommand::MoveTo(Point::new(20.0, 20.0)),
        PathCommand::LineTo(Point::new(20.0, 80.0)),
        PathCommand::LineTo(Point::new(80.0, 80.0)),
        PathCommand::LineTo(Point::new(80.0, 20.0)),
    ];
    let mut explicit_path = common.to_vec();
    explicit_path.push(PathCommand::LineTo(Point::new(20.0, 20.0)));
    let mut closed_path = common.to_vec();
    closed_path.push(PathCommand::Close);
    let explicit = raster(&explicit_path);
    let closed = raster(&closed_path);

    assert_eq!(
        vertical_leg_pixels(&explicit),
        vertical_leg_pixels(&closed),
        "the shared straight leg is the positive control"
    );
    assert!(
        vertical_leg_pixels(&explicit)
            .chunks_exact(4)
            .any(|pixel| pixel[0] < 255),
        "the common leg contains visible stroke ink"
    );
    // At page-space (19.5, 19.5), neither butt-capped segment reaches the southwest
    // quadrant. A closing miter join covers its whole one-unit square.
    let corner = (80 * explicit.width as usize + 19) * 4;
    assert_eq!(
        explicit.data.get(corner..corner + 4),
        Some(&[255, 255, 255, 255][..])
    );
    assert_eq!(
        closed.data.get(corner..corner + 4),
        Some(&[0, 0, 0, 255][..])
    );
}
