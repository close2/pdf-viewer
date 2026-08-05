//! What each backend does with a mark thinner than a pixel — the two numbers `doc/todo/11`
//! called *unmeasured*.
//!
//! §10.7.4 says "a shape that is smaller than a device pixel is nevertheless rendered", and
//! `doc/todo/11` records two places where the CPU backend loses one anyway:
//!
//! 1. **A fill under an eighth of a device pixel thick vanishes.** `tiny-skia` samples four times
//!    per row and rounds, so a sliver *with* an area disappears — 0.05 and 0.1 user units of an
//!    80-unit rule give zero ink at scale 1. That is the device's coverage quantum rather than the
//!    shape's geometry, which is why it is a rule of its own rather than ADR 0154's.
//! 2. **A sub-pixel stroke within half a pixel of the raster's top edge loses half its ink.**
//!    `tiny-skia` draws a stroke under a pixel wide as a hairline smeared symmetrically about the
//!    path rather than as an exact area, and the half of the smear above row zero has nowhere to
//!    go. Only the top and left edges lose, because `TargetSpec::for_page` rounds the raster *up*
//!    to contain the page (ADR 0064).
//!
//! Both entries end "**Unmeasured**: whether the GPU backend does the same", and the comparison
//! that would say so — `render-quorra/tests/corpus.rs` — only ever draws page one, while both
//! witnesses are pages 2 and 3 of `vertical.pdf`. So this asks the question directly, with a
//! synthetic page rather than a document: five identical sub-pixel rules at known distances from
//! the raster's edges, and a ladder of filled slivers of decreasing thickness.
//!
//! ```sh
//! cargo run --release -p render-quorra --example sub_pixel_marks
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, Stroke, TargetSpec, Transform,
};

/// The page every scene is drawn on: tall enough for five rules with room between them.
const PAGE: Size = Size {
    width: 100.0,
    height: 320.0,
};

/// How thick each mark is, in user units — a tenth of a pixel at scale 1.
const THIN: f32 = 0.1;

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

/// A horizontal line at `y`.
fn line(y: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, y)));
    path.push(PathCommand::LineTo(Point::new(90.0, y)));
    path
}

fn fill(path: Path) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn stroke(path: Path, width: f32) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// Ink carried by one raster row, as a fraction of a fully covered row over the marked columns.
///
/// Measured over columns 10 to 90 at scale 1, which is where every mark here is, so a row's value
/// is directly comparable with the mark's own thickness in pixels.
fn row_ink(raster: &pdf_render::Raster, row: usize, scale: f32) -> f32 {
    let w = raster.width as usize;
    let (from, to) = ((10.0 * scale) as usize, (90.0 * scale) as usize);
    let mut total = 0.0_f32;
    for x in from..to {
        let at = (row * w + x) * 4;
        // The scenes are black on white, so darkness is coverage.
        total += f32::from(255 - raster.data[at]) / 255.0;
    }
    total / (to - from) as f32
}

/// Every row with ink in it, and how much.
fn inked(raster: &pdf_render::Raster, scale: f32) -> Vec<(usize, f32)> {
    (0..raster.height as usize)
        .map(|row| (row, row_ink(raster, row, scale)))
        .filter(|(_, ink)| *ink > 0.0005)
        .collect()
}

fn draw(list: &DisplayList, scale: f32) -> Vec<(&'static str, pdf_render::Raster)> {
    let target = TargetSpec::for_page(list, scale, 1 << 30).unwrap();
    vec![
        (
            "cpu",
            render_cpu::CpuRasterizer::new()
                .rasterize(list, target)
                .unwrap(),
        ),
        (
            "quorra",
            render_quorra::QuorraRasterizer::new_headless()
                .unwrap()
                .rasterize(list, target)
                .unwrap(),
        ),
    ]
}

fn main() {
    let scale = 1.0_f32;

    // 1. The quantum: filled slivers of decreasing thickness, each on its own page so that one
    //    row's ink is one sliver's whole answer.
    println!("a filled sliver, {}-unit wide, at scale {scale}", 80);
    println!("  thickness   backend   rows with ink   total ink   expected");
    for thickness in [0.05_f32, 0.1, 0.2, 0.5, 1.0] {
        let mut list = DisplayList::new(PAGE);
        list.push(fill(rect(10.0, 160.0, 90.0, 160.0 + thickness)));
        for (name, raster) in draw(&list, scale) {
            let rows = inked(&raster, scale);
            let total: f32 = rows.iter().map(|(_, ink)| ink).sum();
            println!(
                "  {thickness:>9.2}   {name:<8}  {:>13}   {total:>9.4}   {thickness:>8.2}",
                rows.len()
            );
        }
    }

    // 2. The edge: five identical sub-pixel strokes, two of them within half a pixel of the
    //    raster's edges. The page is 320 tall, so at scale 1 the raster is 320 rows.
    println!();
    println!("a {THIN}-unit stroke, at five distances from the raster's edges, at scale {scale}");
    println!("  where              backend   rows      ink     of an expected {THIN}");
    for (label, y) in [
        ("the top edge", PAGE.height - 0.05),
        ("y 300", 300.0),
        ("y 160", 160.0),
        ("y 20", 20.0),
        ("the bottom edge", 0.05),
    ] {
        let mut list = DisplayList::new(PAGE);
        list.push(stroke(line(y), THIN));
        for (name, raster) in draw(&list, scale) {
            let rows = inked(&raster, scale);
            let total: f32 = rows.iter().map(|(_, ink)| ink).sum();
            let spread: Vec<String> = rows
                .iter()
                .map(|(row, ink)| format!("{row}:{ink:.3}"))
                .collect();
            println!(
                "  {label:<17}  {name:<8}  {:>4}   {total:>6.4}   {}",
                rows.len(),
                spread.join(" ")
            );
        }
    }
}
