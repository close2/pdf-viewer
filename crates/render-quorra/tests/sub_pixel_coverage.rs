//! §10.7.4's "no shape ever disappears", on **both** backends.
//!
//! > This ensures that no shape ever disappears as a result of unfavourable placement relative
//! > to the device pixel grid, as might happen with other possible scan conversion rules.
//!
//! `pdf_render::collapsed` gives a subpath with *no* area the thinnest mark the device has (ADR
//! 0154), and that is gated in `render-cpu`. What was measured and ungated until the
//! three-hundred-and-eighty-ninth session is the case one step along: a shape that **has** an
//! area and is thinner than the rasteriser's coverage quantum. `tiny-skia` supersamples four
//! times per pixel row and takes each sub-row's sample at its centre, so a sliver under an eighth
//! of a pixel crossed no sample line and vanished — 0.05 and 0.1 user units of an 80-unit rule
//! gave zero ink at scale 1 — and a stroke under a pixel wide was drawn as a hairline smeared
//! about the path, so one within half a pixel of the raster's edge lost the half of its smear
//! that fell outside.
//!
//! **This file asserted nothing about the processor for that whole time, deliberately**: a gate
//! on the behaviour above would have ratcheted a defect rather than a requirement. Since ADR 0226
//! it asserts the same thing of both backends, which is what makes it a gate on the *clause*
//! rather than on one library — and the number a backend is held to is the shape's own area, not
//! the other backend's answer.
//!
//! `render-quorra/examples/sub_pixel_marks` prints both ladders side by side and is what to run
//! when this fails.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: an index over a raster this file sized itself cannot overflow, and the \
              page's own coordinates are the small integers above"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one rectangle onto a 100 x 320 page is the \
              failure this file exists to report, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, Stroke, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page each mark is drawn on.
const PAGE: Size = Size {
    width: 100.0,
    height: 320.0,
};

/// Where a mark starts and stops horizontally, which is where the ink is counted.
const LEFT: f32 = 10.0;
/// See [`LEFT`].
const RIGHT: f32 = 90.0;

/// How far a backend's answer may sit from the shape's own area, as a fraction of it.
///
/// Measured rather than chosen. The five sliver thicknesses below come back 0.0510, 0.1020,
/// 0.2000, 0.5020 and 1.0000 on both backends against 0.05, 0.1, 0.2, 0.5 and 1.0, so the worst
/// is 2% — which is one level of 255 on the thinnest of them, and there is nowhere finer for an
/// eight-bit raster to put it. This is 8%: wide enough that a different adapter's rounding cannot
/// fail it, narrow enough that a mark promoted to a whole pixel (twenty times its area, at the
/// thinnest) or lost to a quantum would.
const TOLERANCE: f32 = 0.08;

/// A rule of the given thickness, filled.
fn sliver(thickness: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(LEFT, 160.0)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, 160.0)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, 160.0 + thickness)));
    path.push(PathCommand::LineTo(Point::new(LEFT, 160.0 + thickness)));
    path.push(PathCommand::Close);

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

/// A rule of the given width, stroked along `y`.
fn rule(y: f32, width: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(LEFT, y)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, y)));

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
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
    });
    list
}

/// Total ink over the mark's own columns, in units of one fully covered row.
fn ink(raster: &pdf_render::Raster) -> f32 {
    let w = raster.width as usize;
    let (from, to) = (LEFT as usize, RIGHT as usize);
    let mut total = 0.0_f32;
    for row in 0..raster.height as usize {
        for x in from..to {
            let at = (row * w + x) * 4;
            total += f32::from(255 - raster.data[at]) / 255.0;
        }
    }
    total / (to - from) as f32
}

/// Both backends' ink for one scene, or nothing where this machine has no adapter.
fn both(list: &DisplayList) -> Option<[(&'static str, f32); 2]> {
    let target = TargetSpec::for_page(list, 1.0, 1 << 30).expect("a 100 x 320 page");
    let mut gpu = QuorraRasterizer::new_headless().ok()?;
    let processor = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a scene of one mark");
    let device = gpu.rasterize(list, target).expect("a scene of one mark");
    Some([("processor", ink(&processor)), ("device", ink(&device))])
}

/// What both backends owe a mark: some ink, and the amount its own area implies.
fn agrees_with_the_area(list: &DisplayList, area: f32, what: &str) {
    let Some(drawn) = both(list) else {
        println!("skipped: no adapter on this machine");
        return;
    };
    for (backend, drawn) in drawn {
        assert!(
            drawn > 0.0,
            "§10.7.4: no shape ever disappears, and {what} did on the {backend}"
        );
        let error = (drawn - area).abs() / area;
        assert!(
            error < TOLERANCE,
            "{what} drew {drawn:.4} of ink on the {backend}, {:.1}% from its own area of {area} \
             — run `cargo run --release -p render-quorra --example sub_pixel_marks` for both \
             backends' ladders",
            error * 100.0
        );
    }
}

/// A filled sliver thinner than either rasteriser's coverage quantum still carries its own area.
#[test]
fn a_sliver_thinner_than_a_quantum_carries_its_area_on_both_backends() {
    for thickness in [0.05_f32, 0.1, 0.2, 0.5, 1.0] {
        agrees_with_the_area(
            &sliver(thickness),
            thickness,
            &format!("a {thickness}-unit sliver"),
        );
    }
}

/// A stroke a tenth of a unit wide carries a tenth of a row wherever it is placed — including
/// within half a pixel of the raster's edges, where the processor's hairline used to lose 45% of
/// it and the device loses 2%.
///
/// The page is 320 units tall and the raster 320 rows, so there is no spare fraction of a row at
/// either end for a mark to spill into: both edges are the hard case. `TargetSpec::for_page`
/// rounds the raster *up* to contain the page (ADR 0064), and on a page whose height is not a
/// whole number only the top edge would be.
#[test]
fn a_sub_pixel_rule_at_the_rasters_edge_keeps_its_ink_on_both_backends() {
    for (where_it_is, y) in [
        ("at the top edge", PAGE.height - 0.05),
        ("at y 300", 300.0),
        ("at y 160", 160.0),
        ("at y 20", 20.0),
        ("at the bottom edge", 0.05),
    ] {
        agrees_with_the_area(
            &rule(y, 0.1),
            0.1,
            &format!("a 0.1-unit rule {where_it_is}"),
        );
    }
}
