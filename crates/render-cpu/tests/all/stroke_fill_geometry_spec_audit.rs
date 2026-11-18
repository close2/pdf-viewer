//! Two source-shaped geometry regressions for §8.4.3.4 and §8.5.3.3.2–.3.
//! Both use pixel centers well inside/outside the stipulated shapes, away from raster edges.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::expect_used,
    reason = "small bounded display lists and rasters; refusal to rasterize these simple fills is a test failure"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path, PathCommand,
    Point, Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

const PAGE: f32 = 100.0;
const SCALE: f32 = 2.0;
const BUDGET: u64 = 1 << 20;

fn draw_path(commands: &[PathCommand], join: LineJoin) -> Raster {
    let mut path = Path::new();
    for command in commands {
        path.push(*command);
    }
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 4.0,
            cap: LineCap::Butt,
            join,
            miter_limit: 10.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, SCALE, BUDGET).expect("bounded valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a simple joined path is supported")
}

fn red_at(raster: &Raster, x: f32, y: f32) -> u8 {
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
fn only_consecutive_connected_segments_receive_the_miter_join() {
    // Positive control: east then north are consecutive segments at a right-angle corner.
    // The miter triangle fills the literal interior pixel centered at (51.25, 48.25).
    let connected = draw_path(
        &[
            PathCommand::MoveTo(Point::new(40.0, 50.0)),
            PathCommand::LineTo(Point::new(50.0, 50.0)),
            PathCommand::LineTo(Point::new(50.0, 60.0)),
        ],
        LineJoin::Miter,
    );
    assert!(
        red_at(&connected, 51.25, 48.25) < 16,
        "miter wedge is filled"
    );

    // Same segments and meeting point, but a MoveTo starts a second subpath. The paths only
    // meet fortuitously; the first's butt cap ends at x=50 and the second starts at y=50.
    // The sample is outside both rectangular stroke bodies and must stay white.
    let separate = draw_path(
        &[
            PathCommand::MoveTo(Point::new(40.0, 50.0)),
            PathCommand::LineTo(Point::new(50.0, 50.0)),
            PathCommand::MoveTo(Point::new(50.0, 50.0)),
            PathCommand::LineTo(Point::new(50.0, 60.0)),
        ],
        LineJoin::Miter,
    );
    assert!(
        red_at(&separate, 51.25, 48.25) > 240,
        "no join at a fortuitous meeting"
    );
    assert!(
        red_at(&separate, 49.25, 51.25) < 16,
        "ordinary segment bodies remain painted"
    );
}

fn pentagram(rule: FillRule) -> Raster {
    // A regular five-point star as one self-intersecting closed subpath. Coordinates are rounded
    // to millipoints; all sampled pixels are several units from its edges.
    let mut path = Path::new();
    for command in [
        PathCommand::MoveTo(Point::new(50.0, 80.0)),
        PathCommand::LineTo(Point::new(67.634, 25.729)),
        PathCommand::LineTo(Point::new(21.468, 59.271)),
        PathCommand::LineTo(Point::new(78.532, 59.271)),
        PathCommand::LineTo(Point::new(32.366, 25.729)),
        PathCommand::Close,
    ] {
        path.push(command);
    }
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: rule,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, SCALE, BUDGET).expect("bounded valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a self-intersecting fill is supported")
}

#[test]
fn the_pentagram_center_distinguishes_winding_from_parity() {
    let winding = pentagram(FillRule::NonZero);
    let parity = pentagram(FillRule::EvenOdd);

    // The standard's §8.5.3.3.2 example says the star's central pentagon is inside under
    // nonzero winding; §8.5.3.3.3 says it is outside under even-odd parity.
    assert!(red_at(&winding, 50.25, 50.25) < 16, "nonzero fills center");
    assert!(
        red_at(&parity, 50.25, 50.25) > 240,
        "even-odd leaves center open"
    );

    // Positive control: both rules fill a point well inside a star arm. Negative control: both
    // leave the page above the top tip untouched.
    for raster in [&winding, &parity] {
        assert!(red_at(raster, 50.25, 75.25) < 16, "star arm is inside");
        assert!(red_at(raster, 50.25, 85.25) > 240, "exterior remains white");
    }
}
