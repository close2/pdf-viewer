//! The two rules for what is inside a path: ISO 32000-2 §8.5.3.3.2 and §8.5.3.3.3.
//!
//! # Why this file exists
//!
//! Nothing checked them. `FillRule` has been in the display list since the first commit,
//! both rasterisers implement both rules natively, and every test in the tree used the
//! non-zero one — so the even-odd rule reached pixels through four crates with no assertion
//! anywhere that it produces a *different* picture. The conformance ledger asked the
//! question the pages could not: §8.5.3.3.3 is a normative rule, and the row for it had no
//! test to name.
//!
//! # What the expected values come from
//!
//! Both clauses illustrate their rule with the same figure, two concentric circles.
//! §8.5.3.3.2, the non-zero winding number rule:
//!
//! > For a path composed of two concentric circles, the areas enclosed by both circles are
//! > considered to be inside, provided that both are drawn in the same direction. If the
//! > circles are drawn in opposite directions, only the doughnut shape between them is
//! > inside, according to the rule; the doughnut hole is outside.
//!
//! §8.5.3.3.3, the even-odd rule:
//!
//! > For the two concentric circles, only the doughnut shape between the two circles is
//! > considered inside, regardless of the directions in which the circles are drawn.
//!
//! Squares stand in for the circles so that every expected area is an exact integer: an
//! outer square of side 80 and an inner one of side 40 enclose 6400 and 1600 device pixels,
//! and the three answers the two clauses give are 6400, 4800 and 4800.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, \
              and the arithmetic is over a hundred-unit page and a raster of known size"
)]

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use std::sync::Arc;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units.
const PAGE: f32 = 100.0;

/// A square subpath, wound clockwise or anticlockwise about its centre.
fn square(path: &mut Path, side: f32, clockwise: bool) {
    let low = (PAGE - side) / 2.0;
    let high = low + side;
    let mut corners = [
        Point::new(low, low),
        Point::new(high, low),
        Point::new(high, high),
        Point::new(low, high),
    ];
    if clockwise {
        corners.reverse();
    }
    path.push(PathCommand::MoveTo(corners[0]));
    for corner in &corners[1..] {
        path.push(PathCommand::LineTo(*corner));
    }
    path.push(PathCommand::Close);
}

/// Two concentric squares filled as one path under `rule`.
fn scene(rule: FillRule, same_direction: bool) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    square(&mut path, 80.0, false);
    square(&mut path, 40.0, !same_direction);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: rule,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Total darkness on the page, in units of one fully black pixel.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| u64::from(255 - pixel[0]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the sum is bounded by four bytes per pixel of a raster under a megapixel, \
                  far inside f64's exact integer range"
    )]
    let sum = sum as f64;
    sum / 255.0
}

fn ink_of(list: &DisplayList) -> f64 {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a fill is supported");
    ink(&raster)
}

/// The non-zero rule fills both squares when they are wound the same way.
#[test]
fn the_non_zero_rule_fills_a_nested_subpath_wound_the_same_way() {
    let got = ink_of(&scene(FillRule::NonZero, true));
    assert!(
        (got - 6400.0).abs() < 40.0,
        "the whole outer square is 6400 pixels; got {got:.0}"
    );
}

/// Wound the other way, the same rule leaves the hole out — the clause's doughnut.
#[test]
fn the_non_zero_rule_leaves_a_hole_when_the_windings_oppose() {
    let got = ink_of(&scene(FillRule::NonZero, false));
    assert!(
        (got - 4800.0).abs() < 40.0,
        "6400 less the 1600 of the hole is 4800; got {got:.0}"
    );
}

/// The even-odd rule leaves the hole either way, which is the whole of §8.5.3.3.3.
///
/// The assertion that matters is the *first* of the two: it is the case where the two rules
/// give different answers, so a backend that quietly used the non-zero rule for both — or a
/// display list that dropped the rule on the way to one — fails here at 6400 against 4800
/// and nowhere else in the tree.
#[test]
fn the_even_odd_rule_leaves_the_hole_regardless_of_direction() {
    for same_direction in [true, false] {
        let got = ink_of(&scene(FillRule::EvenOdd, same_direction));
        assert!(
            (got - 4800.0).abs() < 40.0,
            "same_direction={same_direction}: expected the doughnut's 4800, got {got:.0}"
        );
    }
}
