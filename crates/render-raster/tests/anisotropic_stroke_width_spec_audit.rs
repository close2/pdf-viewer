//! Positive stroke widths under unequal horizontal and vertical scaling.
//! The transformed distance-set oracle comes directly from §8.4.3.2, not another renderer.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "bounded raster indexing and explicit backend/setup failure"
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

const WIDTH: u32 = 200;
const HEIGHT: u32 = 80;

fn stroke(path: Path, transform: Transform) -> DisplayList {
    let mut list = DisplayList::new(Size {
        width: WIDTH as f32,
        height: HEIGHT as f32,
    });
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform,
        stroke: Stroke {
            width: 2.0,
            adjust: false,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

fn horizontal(transform: Transform, joined: bool) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 40.0)));
    path.push(PathCommand::LineTo(Point::new(30.0, 40.0)));
    if joined {
        path.push(PathCommand::LineTo(Point::new(30.0, 50.0)));
    }
    stroke(path, transform)
}

fn vertical(transform: Transform, joined: bool) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(20.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(20.0, 30.0)));
    if joined {
        path.push(PathCommand::LineTo(Point::new(30.0, 30.0)));
    }
    stroke(path, transform)
}

fn draw(list: &DisplayList, rasterizer: &mut QuorraRasterizer) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("bounded page target");
    rasterizer
        .rasterize(list, target)
        .expect("requested lane renders stroke")
}

fn count_covered_rows(raster: &Raster, x: usize) -> usize {
    (0..raster.height as usize)
        .filter(|&y| {
            let alpha = raster.data[(y * raster.width as usize + x) * 4 + 3];
            alpha > 127 && raster.data[(y * raster.width as usize + x) * 4] < 128
        })
        .count()
}

fn count_covered_columns(raster: &Raster, y: usize) -> usize {
    (0..raster.width as usize)
        .filter(|&x| {
            let alpha = raster.data[(y * raster.width as usize + x) * 4 + 3];
            alpha > 127 && raster.data[(y * raster.width as usize + x) * 4] < 128
        })
        .count()
}

fn check(coverage: raster_gpu::Coverage) {
    gpu_lock_hook::install();
    let mut renderer = QuorraRasterizer::with_options(&raster_gpu::Options {
        coverage,
        ..render_raster::options()
    })
    .expect("requested coverage lane is available");

    eprintln!("adapter: {}", renderer.adapter_description());

    // A single segment can use the line path; the L-shaped path also checks path-space expansion.
    // Samples stay far from the join, so this oracle claims band width, not join geometry.
    for joined in [false, true] {
        eprintln!("coverage={coverage:?}, joined={joined}");
        // At identity, a width-2 horizontal stroke has two full interior rows. This positive
        // control also checks that the sampling column is well inside the butt-capped segment.
        let identity = draw(&horizontal(Transform::IDENTITY, joined), &mut renderer);
        let identity_rows = count_covered_rows(&identity, 20);
        assert_eq!(identity_rows, 2, "identity width-2 control");

        // For CTM diag(4,1), §8.4.3.2's user-space distance band is transformed with the path.
        // A horizontal segment's normal is y, so width 2 becomes width 2 in device space; a
        // vertical segment's normal is x, so width 2 becomes width 8. Pixel-grid placement permits
        // up to two device pixels of achieved-width difference, so the bounds stay conservative.
        let anisotropic = Transform::scale(4.0, 1.0);
        let horizontal_raster = draw(&horizontal(anisotropic, joined), &mut renderer);
        let vertical_raster = draw(&vertical(anisotropic, joined), &mut renderer);
        let horizontal_rows = count_covered_rows(&horizontal_raster, 80);
        let vertical_columns = count_covered_columns(&vertical_raster, 60);
        eprintln!(
            "identity={identity_rows}, horizontal={horizontal_rows}, vertical={vertical_columns}"
        );

        assert!(
            (6..=10).contains(&vertical_columns),
            "vertical transverse scale is 4: expected 8 columns ±2, got {vertical_columns}"
        );
        assert!(
            (1..=4).contains(&horizontal_rows),
            "horizontal transverse scale is 1: expected 2 rows ±2 and visible ink, got {horizontal_rows}"
        );
    }
}

#[test]
fn cpu_stroke_width_follows_the_transformed_normal() {
    check(raster_gpu::Coverage::Cpu);
}
#[test]
fn gpu_stroke_width_follows_the_transformed_normal() {
    check(raster_gpu::Coverage::Gpu);
}
#[test]
fn compute_stroke_width_follows_the_transformed_normal() {
    check(raster_gpu::Coverage::Compute);
}
