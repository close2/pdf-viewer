//! What each backend paints at the *edge* of a shape wider than a pixel — ISO 32000-2 §10.7.4.
//!
//! `sub_pixel_marks` asks what happens to a mark thinner than the rasteriser's coverage quantum;
//! this asks the question one step along, about a mark far thicker than one, whose **boundary**
//! falls part way across a device pixel. That is the case `doc/todo/_scan-conversion.md`'s
//! departure (1) describes — "an anti-aliasing rasteriser paints a partly covered pixel partly" —
//! and the departure says nothing about how finely "partly" is measured.
//!
//! It is measured here, against the geometry rather than against another renderer: the rectangle's
//! right edge is placed at a known fraction of a pixel and the coverage of the boundary column is
//! read off the raster, so the third column of each row is the answer the shape's own area states.
//!
//! Both axes, because the two are different constructions in a scan converter that walks rows: a
//! vertical edge is a partial run within a row and a horizontal one is a row that is partly
//! inside the shape.
//!
//! ```sh
//! cargo run --release -p render-quorra --example edge_coverage_ladder
//! ```

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a diagnostic binary: its output is the point, an adapter it cannot open should stop \
              it loudly, and the arithmetic is an index into a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, TargetSpec, Transform,
};

/// The page every rung is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 80.0,
    height: 40.0,
};

/// Where the moving edge sits, to the whole pixel: the boundary column and row that are read.
///
/// The same number on both axes, so that one constant names the column the vertical ladder reads
/// and, with the page's height, the row the horizontal one does.
const EDGE: f32 = 20.0;

/// A closed rectangle.
fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// One black rectangle on the page, nothing else.
fn scene(path: Path) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The coverage a pixel carries, on a black-on-white page where darkness is coverage.
fn coverage(raster: &pdf_render::Raster, x: usize, y: usize) -> f32 {
    let at = (y * raster.width as usize + x) * 4;
    f32::from(255 - raster.data[at]) / 255.0
}

fn main() {
    let mut quorra = render_quorra::QuorraRasterizer::new_headless().expect("an adapter");
    let mut cpu = render_cpu::CpuRasterizer::new();
    println!("  the coverage of the boundary pixel, against the fraction of it the shape covers");
    println!();
    println!("           vertical edge            horizontal edge");
    println!("  shape     cpu     quorra          cpu     quorra");
    for rung in 0_i16..=20 {
        let fraction = f32::from(rung) / 20.0;
        // The page's own y axis runs up, so the horizontal edge is read from the row the
        // rectangle's *top* falls in, which is `PAGE.height - EDGE - fraction`.
        let vertical = scene(rect(4.0, 4.0, EDGE + fraction, PAGE.height - 4.0));
        let horizontal = scene(rect(4.0, 4.0, PAGE.width - 4.0, EDGE + fraction));
        let row = (PAGE.height - EDGE - 1.0) as usize;
        let mut read = |list: &DisplayList, x: usize, y: usize| {
            let target = TargetSpec::for_page(list, 1.0, 1 << 30).expect("a target");
            let ours = cpu.rasterize(list, target).expect("drawn");
            let theirs = quorra.rasterize(list, target).expect("drawn");
            (coverage(&ours, x, y), coverage(&theirs, x, y))
        };
        let across = read(&vertical, EDGE as usize, 20);
        let down = read(&horizontal, 20, row);
        println!(
            "   {fraction:.2}    {:.4}   {:.4}          {:.4}   {:.4}",
            across.0, across.1, down.0, down.1
        );
    }
}
