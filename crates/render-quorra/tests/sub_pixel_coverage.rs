//! §10.7.4's "no shape ever disappears", on the backend a page actually goes to.
//!
//! > This ensures that no shape ever disappears as a result of unfavourable placement relative
//! > to the device pixel grid, as might happen with other possible scan conversion rules.
//!
//! `pdf_render::collapsed` gives a subpath with *no* area the thinnest mark the device has (ADR
//! 0154), and that is gated in `render-cpu`. What was measured and ungated is the case one step
//! along: a fill that **has** an area and is thinner than the rasteriser's coverage quantum.
//! `tiny-skia` samples four times per row and rounds, so on the processor a sliver under about an
//! eighth of a pixel vanishes — 0.05 and 0.1 user units of an 80-unit rule give zero ink at scale
//! 1 (`render-cpu/tests/zero_area_fill.rs` records that ladder).
//!
//! **The graphics device has no such quantum**, which the three-hundred-and-forty-fourth session
//! measured and this holds it to. That matters because of the project owner's decision in the
//! two-hundred-and-seventy-third that page one goes to the device: the sentence above is obeyed on
//! the path a page takes, and the departure is the *oracle*'s. `examples/sub_pixel_marks` prints
//! both backends side by side and is what to run when this fails.
//!
//! Nothing here asserts what the processor does. A gate that did would be ratcheting a defect
//! rather than a requirement, and `doc/todo/11` is where the defect is carried.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: an index over a raster this file sized itself cannot overflow, and the \
              page's own coordinates are the small integers above"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page each sliver is drawn on.
const PAGE: Size = Size {
    width: 100.0,
    height: 320.0,
};

/// Where the sliver starts and stops horizontally, which is where the ink is counted.
const LEFT: f32 = 10.0;
/// See [`LEFT`].
const RIGHT: f32 = 90.0;

/// How far the device's answer may sit from the sliver's own area, as a fraction of it.
///
/// Measured rather than chosen: the five thicknesses below come back 0.0510, 0.1020, 0.2000,
/// 0.5020 and 1.0000 against 0.05, 0.1, 0.2, 0.5 and 1.0, so the worst is 2% and this is 8% —
/// wide enough that a different adapter's rounding cannot fail it, narrow enough that a sliver
/// promoted to a whole pixel (twenty times its area, at the thinnest) would.
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

/// Total ink over the sliver's own columns, in units of one fully covered row.
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

#[test]
fn the_device_draws_a_sliver_thinner_than_its_own_quantum() {
    let Ok(mut gpu) = QuorraRasterizer::new_headless() else {
        println!("skipped: no adapter on this machine");
        return;
    };
    for thickness in [0.05_f32, 0.1, 0.2, 0.5, 1.0] {
        let list = sliver(thickness);
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a 100 × 320 page");
        let raster = gpu
            .rasterize(&list, target)
            .expect("a scene of one filled rectangle");
        let drawn = ink(&raster);
        assert!(
            drawn > 0.0,
            "§10.7.4: no shape ever disappears, and a {thickness}-unit sliver did"
        );
        let error = (drawn - thickness).abs() / thickness;
        assert!(
            error < TOLERANCE,
            "a {thickness}-unit sliver drew {drawn:.4} of ink, {:.1}% from its own area — \
             run `cargo run --release -p render-quorra --example sub_pixel_marks` for both \
             backends' ladders",
            error * 100.0
        );
    }
}
