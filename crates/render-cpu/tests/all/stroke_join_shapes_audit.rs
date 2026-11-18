//! ISO 32000-2 §8.4.3.4, Table 54: round and bevel joins fill different regions.
//!
//! A right-angle path's outer corner is a quarter disc for a round join and a right triangle
//! for a bevel join. Measuring that region distinguishes both from each other and from a miter
//! join, which fills the whole square.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    reason = "test code: the scene is bounded to a 100-unit page and a small raster, and a
              rasteriser that refuses this supported scene should fail loudly"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

const PAGE: Size = Size {
    width: 100.0,
    height: 100.0,
};
const HALF: f32 = 4.0;

fn corner(join: LineJoin) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(20.0, 20.0)));
    path.push(PathCommand::LineTo(Point::new(40.0, 20.0)));
    path.push(PathCommand::LineTo(Point::new(40.0, 40.0)));

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: HALF * 2.0,
            cap: LineCap::Butt,
            join,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Ink in the outer southeast quarter of the right-angle vertex, in page-space square units.
fn outer_corner_ink(raster: &Raster) -> f64 {
    let mut ink = 0_u64;
    for row in 80_u32..84 {
        for column in 40_u32..44 {
            let at = ((row * raster.width + column) * 4) as usize;
            ink += u64::from(255 - raster.data.get(at).copied().expect("bounded corner pixel"));
        }
    }
    f64::from(ink as u32) / 255.0
}

fn drawn(join: LineJoin) -> f64 {
    let list = corner(join);
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the CPU backend supports joined strokes");
    outer_corner_ink(&raster)
}

#[test]
fn round_and_bevel_joins_paint_their_table_54_shapes() {
    let round = drawn(LineJoin::Round);
    let bevel = drawn(LineJoin::Bevel);
    let quarter_disc = std::f64::consts::PI * f64::from(HALF).powi(2) / 4.0;
    let triangle = f64::from(HALF * HALF) / 2.0;

    assert!(
        (round - quarter_disc).abs() < 0.2,
        "round join should fill the quarter disc {quarter_disc:.3}, got {round:.3}"
    );
    assert!(
        (bevel - triangle).abs() < 0.2,
        "bevel join should fill the triangle {triangle:.3}, got {bevel:.3}"
    );
    assert!(
        round > bevel + 4.0,
        "the two join styles must be discriminated"
    );
}
