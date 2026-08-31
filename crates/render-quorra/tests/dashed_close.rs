//! What a dashed closed subpath paints at its own closing vertex, on **all three** backends,
//! against the two sentences that decide it.
//!
//! ISO 32000-2 §8.4.3.4, below Table 54:
//!
//! > In a closed subpath that is dashed, if the first segment starts with an on-dash and the last
//! > segment ends within an on-dash, then they shall be joined.
//!
//! ISO 32000-2 §8.4.3.6:
//!
//! > If the end of a dashed segment coincides exactly with a join point, then the end cap is
//! > painted before the corner.
//!
//! So one scene is joined and the other is capped, and which of the two a rectangle is depends on
//! nothing but its perimeter against the dash pattern. Both scenes below are a rectangle stroked
//! with `[ 10 10 ] 0 d` under **butt** caps and a **round** join — `DegenerateDashing.pdf`'s own
//! pattern and join, with the cap chosen so that the two answers differ by the whole of a quarter
//! disc rather than by a shape a projecting square cap would have covered anyway, and the width
//! chosen at 4 rather than that file's 5 so that the quadrant measured below is whole device
//! pixels at both of the scales here:
//!
//! - **200 × 45**, perimeter 490, which is 24 whole periods and one on-dash. The last on-dash
//!   *finishes* at the closing vertex, so §8.4.3.6 paints its end cap there and the first dash's
//!   cap beside it. Butt caps state no mark beyond their own end, so the quadrant outside the
//!   corner carries **nothing at all**.
//! - **200 × 44**, perimeter 488, which stops 8 units into the last on-dash. That dash *ends
//!   within* an on-dash, so §8.4.3.4 joins it to the first, and the round join the scene sets
//!   fills the quadrant with a quarter of a disc of the line's own width: `π (w/2)² / 4`.
//!
//! Nothing here is derived from another renderer. `doc/corpora/pdf-differences`'s
//! `DegenerateDashing.pdf` states both cases in one file and its README says which is which, but
//! the numbers asserted below are the clause's arithmetic and the areas that follow from it.
//!
//! # Why all three backends and why this file exists
//!
//! Because a dasher decides this vertex and the three of them are three libraries. Every *other*
//! corner is right by construction — a dash that stops short of a corner becomes its own open
//! contour and is capped, one that spans a corner keeps it and is joined — and only at the close
//! does a dasher have to decide whether the last dash and the first are one mark wrapping round.
//! Skia's merges them whenever both are on, which reads §8.4.3.4 without its "within" and joins
//! the case §8.4.3.6 caps. **All three did**, measured with the rule turned off: the processor put
//! 3.133 square units in the quadrant the clause leaves empty, quorra 3.086 and vello 2.753 — one
//! wrong answer from three libraries, which is trap 2's shape rather than one library's bug.
//! `pdf_render::opened_where_a_dash_ends_at_the_close` is the rule, in the crate all three consume,
//! and this file is what holds each of them to it (`pdf_render::degenerate`'s comment has the same
//! argument one clause over).

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: the page's own coordinates are the small numbers below, and an index over \
              a raster this file sized itself cannot overflow"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one dashed rectangle onto a 300 x 150 page is \
              the failure this file exists to report, and it reports it by name"
)]
#![expect(
    clippy::print_stdout,
    reason = "test code: the ink beside the closed form is the measurement, and `--nocapture` is \
              how it is read"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page both scenes are drawn on.
const PAGE: Size = Size {
    width: 300.0,
    height: 150.0,
};

/// The rectangle's lower-left corner in page space, which is the vertex under test.
const CORNER: Point = Point { x: 20.0, y: 20.0 };

/// The rectangle's width, shared by both scenes so that only the perimeter differs.
const WIDTH: f32 = 200.0;

/// The line width.
///
/// 4 rather than the witness document's 5 so that the quadrant `ink_outside_the_corner` reads is
/// an exact number of device pixels at scale 1 and at scale 2; a quadrant cut across a pixel would
/// make the closed form below depend on the scale rather than on the clause.
const LINE: f32 = 4.0;

/// The dash pattern, which is `DegenerateDashing.pdf`'s own.
const DASH: [f32; 2] = [10.0, 10.0];

/// A rectangle of the given height, stroked as the two clauses' witness is.
///
/// Table 58's `re`: a `MoveTo` at the corner, three straight segments and a close.
fn witness(height: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(CORNER));
    path.push(PathCommand::LineTo(Point::new(CORNER.x + WIDTH, CORNER.y)));
    path.push(PathCommand::LineTo(Point::new(
        CORNER.x + WIDTH,
        CORNER.y + height,
    )));
    path.push(PathCommand::LineTo(Point::new(CORNER.x, CORNER.y + height)));
    path.push(PathCommand::Close);

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: LINE,
            cap: LineCap::Butt,
            join: LineJoin::Round,
            dash_array: DASH.to_vec(),
            ..Stroke::default()
        },
        paint: Paint::Solid(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The ink, in page-space square units, in the quadrant outside the corner under test.
///
/// The quadrant is `[x0 − w/2, x0] × [y0 − w/2, y0]` in page space, which is where the join would
/// reach and where two butt caps reach not at all. Whole device pixels only: the region's own
/// edges are where the two segments' bodies begin, so a pixel straddling one of them carries their
/// coverage rather than the corner's and reading it would measure the wrong mark.
fn ink_outside_the_corner(raster: &Raster, scale: f32) -> f64 {
    let half = f64::from(LINE) / 2.0;
    let left = ((f64::from(CORNER.x) - half) * f64::from(scale)).ceil() as u32;
    let right = (f64::from(CORNER.x) * f64::from(scale)).floor() as u32;
    // Device rows count down from the top of the page, so the quadrant *below* the corner in page
    // space is the rows just after it.
    let top = ((f64::from(PAGE.height) - f64::from(CORNER.y)) * f64::from(scale)).ceil() as u32;
    let bottom =
        ((f64::from(PAGE.height) - f64::from(CORNER.y) + half) * f64::from(scale)).floor() as u32;
    let mut total = 0.0;
    for row in top..bottom.min(raster.height) {
        for column in left..right.min(raster.width) {
            let at = ((row * raster.width + column) * 4) as usize;
            total += 1.0 - f64::from(raster.data[at]) / 255.0;
        }
    }
    total / f64::from(scale * scale)
}

/// One backend: its name and a closure that draws a scene with it.
///
/// A closure rather than a `dyn Rasterizer`, because the three rasterisers have three error types
/// and what this file wants from each is one raster or a loud failure.
type Backend = (
    &'static str,
    Box<dyn FnMut(&DisplayList, TargetSpec) -> Raster>,
);

/// The three backends, each built once per scene so that a device is not held across the run.
fn backends() -> Vec<Backend> {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut quorra = QuorraRasterizer::new_headless().expect("a quorra device");
    let mut gpu = render_gpu::GpuRasterizer::new_headless().expect("a vello device");
    vec![
        (
            "processor",
            Box::new(move |list, target| cpu.rasterize(list, target).expect("a cpu raster")),
        ),
        (
            "quorra",
            Box::new(move |list, target| quorra.rasterize(list, target).expect("a quorra raster")),
        ),
        (
            "vello",
            Box::new(move |list, target| gpu.rasterize(list, target).expect("a vello raster")),
        ),
    ]
}

/// A perimeter that finishes an on-dash at the close is capped there, so the quadrant is empty.
///
/// The bound is a fraction of a single page-space unit against a quarter disc of **π**, which is
/// what the wrong answer puts there; it is not zero only because the quadrant's own edges are
/// device pixels and a rasteriser may carry a level of eight-bit rounding into the one beside them.
#[test]
fn every_backend_caps_a_dash_that_finishes_at_the_close() {
    let empty = 0.0;
    let disc = std::f64::consts::PI * f64::from(LINE / 2.0).powi(2) / 4.0;
    println!("§8.4.3.6: nothing outside the corner, against the {disc:.2} a round join would add");
    for scale in [1.0_f32, 2.0] {
        // 2 (200 + 45) = 490 = 24 periods of 20 and one on-dash of 10.
        let list = witness(45.0);
        let target = TargetSpec::for_page(&list, scale, 1 << 30).expect("a valid target");
        for (name, draw) in &mut backends() {
            let ink = ink_outside_the_corner(&draw(&list, target), scale);
            println!("  scale {scale} {name:>9}: ink {ink:6.3} of {empty:.3}");
            assert!(
                ink < 0.1,
                "{name} at scale {scale} painted {ink:.3} square units outside a corner where \
                 §8.4.3.6 asks for the two dashes' end caps and butt caps reach past neither"
            );
        }
    }
}

/// How far a backend's ink may sit from the quarter disc, as a fraction of it.
///
/// **Measured rather than chosen.** The processor lands within 0.3% of `π` and quorra within 1.8%,
/// while vello is 12.4% under it at scale 1 — its own flattening of an arc two device pixels
/// across, which is a backend's tolerance rather than anything this construction states. 20% is a
/// little over half again the worst of the three and still fails by the whole quantity on the
/// capped answer, which puts nothing in the quadrant at all.
const TOLERANCE: f64 = 0.2;

/// A perimeter that stops inside an on-dash is joined there, so the round join's quadrant is full.
///
/// The closed form is a quarter of a disc of the line's width, `π (w/2)² / 4` = π square units.
#[test]
fn every_backend_joins_a_dash_the_close_cuts_short() {
    let disc = std::f64::consts::PI * f64::from(LINE / 2.0).powi(2) / 4.0;
    println!("§8.4.3.4: a quarter disc of {disc:.3} square units outside the corner");
    for scale in [1.0_f32, 2.0] {
        // 2 (200 + 44) = 488, which is 8 units into the twenty-fifth period's on-dash.
        let list = witness(44.0);
        let target = TargetSpec::for_page(&list, scale, 1 << 30).expect("a valid target");
        for (name, draw) in &mut backends() {
            let ink = ink_outside_the_corner(&draw(&list, target), scale);
            println!(
                "  scale {scale} {name:>9}: ink {ink:6.3} of {disc:.3} ({:+.1}%)",
                (ink / disc - 1.0) * 100.0
            );
            assert!(
                (ink - disc).abs() < disc * TOLERANCE,
                "{name} at scale {scale} painted {ink:.3} square units where §8.4.3.4 joins the \
                 last dash to the first and the round join covers {disc:.3}"
            );
        }
    }
}
