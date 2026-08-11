//! What each backend does with a mark thinner than a pixel — the two numbers `doc/todo/11`
//! called *unmeasured*, and then the instrument that closed them.
//!
//! §10.7.4 says "[t]his ensures that no shape ever disappears", and `doc/todo/11` recorded two
//! places where the CPU backend lost one anyway. Both were **that backend's alone** — the
//! graphics device drew every one of them — and both were closed in the
//! three-hundred-and-eighty-ninth session by ADR 0226:
//!
//! 1. **A fill under an eighth of a device pixel thick vanished.** `tiny-skia` supersamples four
//!    times per pixel row and takes each sub-row's sample at its centre, so a sliver *with* an
//!    area crossed no sample line — 0.05 and 0.1 user units of an 80-unit rule gave zero ink at
//!    scale 1. That is the rasteriser's coverage quantum rather than the shape's geometry, which
//!    is why it was a rule of its own rather than ADR 0154's.
//! 2. **A sub-pixel stroke within half a pixel of the raster's edge lost half its ink.**
//!    `tiny-skia` drew a stroke under a pixel wide as a hairline smeared symmetrically about the
//!    path rather than as an exact area, and the half of the smear outside the raster had nowhere
//!    to go. On a page whose height is a whole number of units **both** edges lose, which the
//!    100 × 320 page below shows; where the page's extent is fractional only the top and left do,
//!    because `TargetSpec::for_page` rounds the raster *up* to contain the page (ADR 0064).
//!
//! `render-quorra/tests/corpus.rs` could not answer whether the device did the same, because it
//! only ever draws page one and both witnesses are pages 2 and 3 of `vertical.pdf`. So this asks
//! the question directly, with synthetic pages rather than documents, and it is now what to run
//! when `tests/sub_pixel_coverage.rs` fails. Three sections:
//!
//! - a ladder of filled slivers of decreasing thickness — the quantum;
//! - five identical sub-pixel rules at known distances from the raster's edges — the smear;
//! - a ladder **across** the one-pixel boundary, which is where the substitution stops. That one
//!   is the constraint rather than the symptom: a rule that promoted every sub-quantum mark to a
//!   full mark would fight the anti-aliasing departure on ordinary thin shapes, and this section
//!   is what says whether it does. It reads a step at the boundary if it ever starts to.
//!
//! **A fourth since the four-hundred-and-thirty-second, and it is the one ADR 0226 left open**: the
//! same sliver *turned*, as a fill and as a stroke, at seven angles and six thicknesses. A
//! diagonal lies in no single row, so its answer is the ink over the whole raster against the
//! band's own area — and that comparison is what found the defect ADR 0268 answers: `tiny-skia`'s
//! hairline lays down one pixel per step along the line's *longer* device axis, so it carried
//! `cos θ` of a turned rule's area, 29.3% short at 45° and at every thickness rather than only
//! near the quantum.
//!
//! The last two thicknesses are the boundary again, one axis over, and **the 1.00 row is a defect
//! this instrument found and this tree has not paid**: at exactly one device pixel `tiny-skia`
//! still chooses the hairline, so a 45° rule reads 141.42 of its own 200 where the fill of the
//! same outline reads 177.44. `doc/todo/11` carries it.
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

/// The page the turned slivers are drawn on: square, so that a band of one length fits at every
/// angle without the page's own aspect deciding which angles are measurable.
const TURNED: Size = Size {
    width: 320.0,
    height: 320.0,
};

/// Half the length of every turned band, in user units.
const REACH: f32 = 100.0;

/// Ink carried by the whole raster, in units of one fully covered device pixel.
///
/// The measure a mark that is not axis-aligned needs: a diagonal band lies in no single row, so
/// its answer is the sum over every pixel and is directly comparable with the band's own area in
/// device pixels.
fn total_ink(raster: &pdf_render::Raster) -> f32 {
    raster
        .data
        .chunks_exact(4)
        // The scenes are black on white, so darkness is coverage.
        .map(|pixel| f32::from(255 - pixel[0]) / 255.0)
        .sum()
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

    // 3. The shapes that are *not* degenerate, which is the constraint on any rule written for
    //    the two above: the substitution stops at one device pixel, so the ladder must cross that
    //    boundary without a step. A rule that promoted a sub-quantum mark to a whole pixel would
    //    print 1.0000 on the left of the boundary and the thickness on the right.
    println!();
    println!("a filled sliver either side of the one-pixel boundary, at scale {scale}");
    println!("  thickness   backend   total ink   expected   error");
    for thickness in [
        0.60_f32, 0.80, 0.90, 0.95, 0.99, 1.00, 1.01, 1.05, 1.20, 1.50, 2.00,
    ] {
        let mut list = DisplayList::new(PAGE);
        list.push(fill(rect(10.0, 160.0, 90.0, 160.0 + thickness)));
        for (name, raster) in draw(&list, scale) {
            let total: f32 = inked(&raster, scale).iter().map(|(_, ink)| ink).sum();
            println!(
                "  {thickness:>9.2}   {name:<8}  {total:>9.4}   {thickness:>8.2}   {:>6.2}%",
                100.0 * (total - thickness) / thickness
            );
        }
    }

    // 4. The sliver *turned*, which is what ADR 0226 declined and `doc/todo/11` carried. A band of
    //    fixed length and thickness at seven angles: at 0 and 90 degrees it is the axis-aligned
    //    case the substitution takes, and every angle between is one it does not. The comparison
    //    is total ink against the band's own area, because a diagonal lies in no single row.
    println!();
    println!(
        "a turned sliver, {} units long, at scale {scale}",
        2.0 * REACH
    );
    println!("  drawn as   angle   thickness   backend   total ink   its own area   error");
    for turned in [Turn::Fill, Turn::Stroke] {
        for degrees in [0.0_f32, 5.0, 15.0, 30.0, 45.0, 60.0, 90.0] {
            for thickness in [0.05_f32, 0.1, 0.2, 0.5, 1.0, 2.0] {
                let mut list = DisplayList::new(TURNED);
                list.push(turned.command(degrees, thickness));
                let area = 2.0 * REACH * thickness * scale * scale;
                for (name, raster) in draw(&list, scale) {
                    let total = total_ink(&raster);
                    println!(
                        "  {:<8}   {degrees:>5.0}   {thickness:>9.2}   {name:<8}  {total:>9.4}   \
                         {area:>12.4}   {:>6.1}%",
                        turned.label(),
                        100.0 * (total - area) / area
                    );
                }
            }
        }
    }
}

/// Which of the two operators states the turned band.
///
/// Both are drawn because they reach the rasteriser by different routes: a fill hands it the
/// parallelogram outright, while a stroke under a pixel wide is `tiny-skia`'s hairline case.
#[derive(Clone, Copy)]
enum Turn {
    Fill,
    Stroke,
}

impl Turn {
    fn label(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Stroke => "stroke",
        }
    }

    /// The band of half-length [`REACH`] centred on the page at `degrees` from the x axis.
    ///
    /// Butt caps and a mitre join are the defaults, so the stroke's outline is exactly the
    /// parallelogram the fill states and the two have the same area — `2 * REACH * thickness`.
    fn command(self, degrees: f32, thickness: f32) -> Command {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let (cx, cy) = (TURNED.width / 2.0, TURNED.height / 2.0);
        let along = Point::new(REACH * cos, REACH * sin);
        let ends = [
            Point::new(cx - along.x, cy - along.y),
            Point::new(cx + along.x, cy + along.y),
        ];
        match self {
            Self::Stroke => {
                let mut path = Path::new();
                path.push(PathCommand::MoveTo(ends[0]));
                path.push(PathCommand::LineTo(ends[1]));
                stroke(path, thickness)
            }
            Self::Fill => {
                let across = Point::new(-thickness / 2.0 * sin, thickness / 2.0 * cos);
                let mut path = Path::new();
                path.push(PathCommand::MoveTo(Point::new(
                    ends[0].x + across.x,
                    ends[0].y + across.y,
                )));
                path.push(PathCommand::LineTo(Point::new(
                    ends[1].x + across.x,
                    ends[1].y + across.y,
                )));
                path.push(PathCommand::LineTo(Point::new(
                    ends[1].x - across.x,
                    ends[1].y - across.y,
                )));
                path.push(PathCommand::LineTo(Point::new(
                    ends[0].x - across.x,
                    ends[0].y - across.y,
                )));
                path.push(PathCommand::Close);
                fill(path)
            }
        }
    }
}
