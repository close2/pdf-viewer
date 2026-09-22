//! A clipping path admits exactly what a fill of the same path paints: ISO 32000-2 §10.7.4.
//!
//! # What the expected values come from
//!
//! §10.7.4 states the clipping region by the fill:
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by
//! > a fill operation.
//!
//! and states, two paragraphs earlier, what a fill of a flat rectangle includes:
//!
//! > A zero-width or zero-height rectangle paints a line 1 pixel wide.
//!
//! So the expected value is not a number of this renderer's choosing: it is whatever
//! `zero_area_fill.rs` measures for the *same* rectangle filled, which is one device pixel of
//! ink per device pixel of length. Every assertion here is that equality, and the arithmetic
//! is the standard's rather than ours.
//!
//! # Why both halves are measured in one test
//!
//! The defect this pins had the two halves of one sentence disagreeing with each other:
//! `5 20.5 30 0 re f` painted its row of pixels while `5 20.5 30 0 re W n` over a full-page
//! fill admitted none of them. A test that asserted a count would have been satisfied by
//! either half moving; a test that asserts the *fill* and the *clip* agree cannot be.
//!
//! # And at three scales, because the rule is stated in device pixels
//!
//! §10.7.4's pixel is a device pixel, and the clipping path is stated in the document's own
//! space. A single scale cannot tell a rule applied on the device grid from one applied in the
//! path's space and scaled with it (trap 2's arithmetic, `zero_area_fill.rs`'s own reason for
//! running a ladder).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, \
              and the arithmetic is over a hundred-unit page and a raster of known size"
)]

use pdf_render::{
    BlendMode, Clip, ClipId, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand,
    Point, Raster, Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use std::sync::Arc;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units.
const PAGE: f32 = 100.0;

/// The rule's length, in PDF units.
const LENGTH: f32 = 30.0;

/// Builds a path from its commands.
fn path(commands: &[PathCommand]) -> Path {
    let mut built = Path::new();
    for command in commands {
        built.push(*command);
    }
    built
}

/// `5 y 30 0 re`, the shape ADR 1060 found a clip losing.
fn zero_height_rectangle(y: f32) -> [PathCommand; 5] {
    [
        PathCommand::MoveTo(Point::new(5.0, y)),
        PathCommand::LineTo(Point::new(5.0 + LENGTH, y)),
        PathCommand::LineTo(Point::new(5.0 + LENGTH, y)),
        PathCommand::LineTo(Point::new(5.0, y)),
        PathCommand::Close,
    ]
}

/// The same shape in the other axis.
fn zero_width_rectangle(x: f32) -> [PathCommand; 5] {
    [
        PathCommand::MoveTo(Point::new(x, 5.0)),
        PathCommand::LineTo(Point::new(x, 5.0 + LENGTH)),
        PathCommand::LineTo(Point::new(x, 5.0 + LENGTH)),
        PathCommand::LineTo(Point::new(x, 5.0)),
        PathCommand::Close,
    ]
}

/// One black fill of `commands` under `rule`, unclipped.
fn filled(commands: &[PathCommand], rule: FillRule) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Fill {
        path: Arc::new(path(commands)),
        transform: Transform::IDENTITY,
        fill_rule: rule,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// A black fill of the whole page under a clip of `commands` — `W n` then `re f`.
fn clipped(commands: &[PathCommand], rule: FillRule) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let clip: ClipId = list
        .add_clip(Clip {
            path: path(commands),
            transform: Transform::IDENTITY,
            fill_rule: rule,
            parent: None,
        })
        .expect("a clip");
    let page = path(&[
        PathCommand::MoveTo(Point::new(0.0, 0.0)),
        PathCommand::LineTo(Point::new(PAGE, 0.0)),
        PathCommand::LineTo(Point::new(PAGE, PAGE)),
        PathCommand::LineTo(Point::new(0.0, PAGE)),
        PathCommand::Close,
    ]);
    list.push(Command::Fill {
        path: Arc::new(page),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: Some(clip),
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

/// How many device pixels carry any ink at all — §10.7.4's own unit, a *set of pixels*.
fn marked(raster: &Raster) -> usize {
    raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 255)
        .count()
}

/// Renders `list` at `scale`, returning the ink and the number of pixels it reached.
fn drawn(list: &DisplayList, scale: f32) -> (f64, usize) {
    let target = TargetSpec::for_page(list, scale, GENEROUS).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a fill is supported");
    (ink(&raster), marked(&raster))
}

/// The defect ADR 1060 recorded: a clip must admit the row its fill paints.
///
/// Not "about thirty pixels" but "what the fill paints", measured in the same run — the
/// clause defines one by the other and a test that pinned a number would pass on either half
/// of it moving.
#[test]
fn a_flat_clip_admits_exactly_what_a_flat_fill_paints() {
    for commands in [zero_height_rectangle(20.5), zero_width_rectangle(20.5)] {
        for scale in [1.0_f32, 2.0, 3.5] {
            let (paints, paints_pixels) = drawn(&filled(&commands, FillRule::NonZero), scale);
            let (admits, admits_pixels) = drawn(&clipped(&commands, FillRule::NonZero), scale);
            assert!(
                paints_pixels > 0,
                "at scale {scale} the fill itself painted nothing"
            );
            assert_eq!(
                admits_pixels, paints_pixels,
                "at scale {scale} the clip admitted {admits_pixels} pixels and the fill \
                 painted {paints_pixels}"
            );
            assert!(
                (admits - paints).abs() < 0.5,
                "at scale {scale} the clip admitted {admits} of ink and the fill painted {paints}"
            );
        }
    }
}

/// The clause's own arithmetic, so that the pair above cannot agree on a wrong number.
///
/// "A zero-width or zero-height rectangle paints a line 1 pixel wide", and the line is
/// `LENGTH` units long, so at `scale` it is `LENGTH × scale` device pixels of one pixel each.
#[test]
fn what_both_halves_agree_on_is_the_clauses_own_row_of_pixels() {
    for scale in [1.0_f32, 2.0, 3.5] {
        let (_, pixels) = drawn(
            &clipped(&zero_height_rectangle(20.5), FillRule::NonZero),
            scale,
        );
        let expected = f64::from(LENGTH * scale);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a pixel count over a hundred-unit page, far inside f64's exact range"
        )]
        let counted = pixels as f64;
        assert!(
            (counted - expected).abs() < 3.0,
            "at scale {scale} the clip admits {counted} pixels, expected about {expected}"
        );
    }
}

/// Two rules that cross are two shapes, and the crossing is admitted rather than cancelled.
///
/// The discriminating case for scan-converting a wholly collapsed clipping path under the
/// **non-zero** rule whatever the operator asked. Under `W*` the two marks meet in one pixel,
/// and taking the parity there would punch a hole in the middle of a clipped grid — which is
/// the same thing `zero_area_fill.rs`'s `a_mark_inside_an_even_odd_fill_does_not_cut_a_hole_in_it`
/// keeps a *fill* from doing, one operator over.
#[test]
fn two_crossing_rules_clipped_under_the_even_odd_rule_keep_their_crossing() {
    let mut commands = zero_height_rectangle(20.5).to_vec();
    commands.extend_from_slice(&zero_width_rectangle(20.5));
    let (_, pixels) = drawn(&clipped(&commands, FillRule::EvenOdd), 2.0);
    // Two rules of `LENGTH` units at scale 2, sharing the one pixel they cross in.
    let expected = f64::from(LENGTH * 2.0) * 2.0 - 1.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a pixel count over a hundred-unit page, far inside f64's exact range"
    )]
    let counted = pixels as f64;
    assert!(
        (counted - expected).abs() < 3.0,
        "the crossed clip admits {counted} pixels, expected about {expected}"
    );
}

/// A clipping path of a single point still admits nothing, which is ADR 1060 and stays.
///
/// The boundary of the rule above, and the reason it is pinned here: §8.5.3.3.1 hedges its own
/// answer for a degenerate subpath — "the result is device-dependent and not generally useful"
/// — and this tree declines it for a fill (`pdf_render::collapsed`), so §10.7.4's definition
/// makes the clip decline it too. A clip that started admitting a pixel here would have the two
/// halves of the definition disagreeing again, in the other direction.
#[test]
fn a_clip_that_is_a_single_point_still_admits_nothing() {
    let point = [
        PathCommand::MoveTo(Point::new(10.5, 10.5)),
        PathCommand::LineTo(Point::new(10.5, 10.5)),
        PathCommand::Close,
    ];
    for scale in [1.0_f32, 2.0] {
        let (_, pixels) = drawn(&clipped(&point, FillRule::NonZero), scale);
        assert_eq!(pixels, 0, "at scale {scale} a point clip admitted {pixels}");
        let (_, painted) = drawn(&filled(&point, FillRule::NonZero), scale);
        assert_eq!(
            painted, 0,
            "at scale {scale} a point fill painted {painted}"
        );
    }
}

/// A mixed clipping path admits the **union** of two fills, which is not a rectangle.
///
/// This is the whole of §10.7.4's clipping sentence read against §8.5.4's: "For clipping, the
/// clipping region consists of the set of pixels that would be included by a fill operation",
/// and "For a given path definition, the same area that would be filled by the f operator is
/// the area that would be used for a clip". A path that encloses a square *and* rules a line
/// across it is filled as both — the square by its own rule, the rule by this subclause's
/// EXAMPLE, "A zero-width or zero-height rectangle paints a line 1 pixel wide" — so the clip
/// admits both, and the set is a square with two whiskers rather than any rectangle.
///
/// **Both operators, because the even-odd one is the discriminator.** Appending the mark to the
/// square's own path states the union under `W n` and states a *hole* under `W* n`, where the
/// crossing counts twice and the parity cancels. So a backend that concatenates the two passes
/// the first line here and fails the second, and one that composes two fills passes both.
///
/// **Every edge is on a pixel boundary, on purpose.** Where a boundary falls inside a pixel the
/// fill composites its two parts source-over while the clip's region sums their coverages, and
/// the two answers part by the product term — a comparison of this backend's anti-aliasing
/// departure with itself rather than of the clip with the fill. Whole pixels take that out, and
/// the expected count below is then the clause's own arithmetic.
#[test]
fn a_clip_that_encloses_an_area_and_rules_a_line_admits_the_union_of_both() {
    // A 30-unit square, and a rule of 55 units through it at y = 20.5 — 30 of the rule's
    // length lies inside the square and 25 outside it.
    let mut commands = vec![
        PathCommand::MoveTo(Point::new(10.0, 10.0)),
        PathCommand::LineTo(Point::new(40.0, 10.0)),
        PathCommand::LineTo(Point::new(40.0, 40.0)),
        PathCommand::LineTo(Point::new(10.0, 40.0)),
        PathCommand::Close,
    ];
    commands.extend_from_slice(&[
        PathCommand::MoveTo(Point::new(5.0, 20.5)),
        PathCommand::LineTo(Point::new(60.0, 20.5)),
        PathCommand::LineTo(Point::new(60.0, 20.5)),
        PathCommand::LineTo(Point::new(5.0, 20.5)),
        PathCommand::Close,
    ]);
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        for scale in [1.0_f32, 2.0] {
            let (paints, paints_pixels) = drawn(&filled(&commands, rule), scale);
            let (admits, admits_pixels) = drawn(&clipped(&commands, rule), scale);
            // 30 x 30 units of square, plus the 25 units of rule outside it one pixel high,
            // each at `scale` device pixels per unit.
            let side = 30.0 * f64::from(scale);
            let expected = side * side + 25.0 * f64::from(scale);
            #[expect(
                clippy::cast_precision_loss,
                reason = "a pixel count over a hundred-unit page, far inside f64's exact range"
            )]
            let counted = admits_pixels as f64;
            assert_eq!(
                admits_pixels, paints_pixels,
                "under {rule:?} at scale {scale} the clip admitted {admits_pixels} pixels and \
                 the fill painted {paints_pixels}"
            );
            assert!(
                (counted - expected).abs() < 0.5,
                "under {rule:?} at scale {scale} the clip admits {counted} pixels, and the \
                 union of the two fills is {expected}"
            );
            assert!(
                (admits - paints).abs() < 0.5,
                "under {rule:?} at scale {scale} the clip admitted {admits} of ink and the \
                 fill painted {paints}"
            );
        }
    }
}
