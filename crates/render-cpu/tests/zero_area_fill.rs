//! A filled shape with no area may not disappear: ISO 32000-2 §10.7.4.
//!
//! # What the expected values come from
//!
//! §10.7.4: "A shape shall be scan-converted by painting any pixel whose half-open square
//! region intersects the shape, no matter how small the intersection is. This ensures that no
//! shape ever disappears". `pdf_render::collapsed` is where that sentence is read against
//! §8.5.3.3.1's degenerate subpath and §10.7.4's own half-open convention, and the answer it
//! reaches is the one §8.4.3.2 already gives a line: the thinnest mark the device has.
//!
//! So the expected ink is the same quantity `stroke_width.rs` measures for a zero-width
//! stroke — one device pixel per device pixel of length — and it is measured the same way,
//! at three scales, because the rule is stated in device pixels and applied in the path's own
//! space. A test at one scale cannot tell a reciprocal from a constant (trap 2).
//!
//! # Why ink rather than a pixel
//!
//! A rule of no thickness is exactly what an antialiasing rasteriser spreads over two rows
//! when it lands on a row boundary, so no single pixel's value is the assertion. The total
//! darkness is, and the *discriminating* case is the one this rule exists for: before the
//! rule, this page's ink is zero.

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

/// The rule's length, in PDF units.
const LENGTH: f32 = 80.0;

/// A filled path of `commands`, in black, under the given rule.
fn filled(commands: &[PathCommand], rule: FillRule) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    for command in commands {
        path.push(*command);
    }
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

/// `x y w 0 re`, which is how `issue4260_reduced.pdf` rules every line of its grid.
fn zero_height_rectangle(y: f32) -> [PathCommand; 5] {
    [
        PathCommand::MoveTo(Point::new(10.0, y)),
        PathCommand::LineTo(Point::new(10.0 + LENGTH, y)),
        PathCommand::LineTo(Point::new(10.0 + LENGTH, y)),
        PathCommand::LineTo(Point::new(10.0, y)),
        PathCommand::Close,
    ]
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

/// Renders `list` at `scale` and returns the ink it deposited.
fn ink_at(list: &DisplayList, scale: f32) -> f64 {
    let target = TargetSpec::for_page(list, scale, GENEROUS).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a fill is supported");
    ink(&raster)
}

/// A rectangle of no height deposits one device pixel of ink per device pixel of length.
///
/// The measurement §10.7.4 asks for, and the one that was zero before the rule existed.
#[test]
fn a_rectangle_of_no_height_marks_the_page_at_every_scale() {
    let list = filled(&zero_height_rectangle(50.5), FillRule::NonZero);
    for scale in [1.0_f32, 2.0, 3.5] {
        let expected = f64::from(LENGTH * scale);
        let got = ink_at(&list, scale);
        assert!(
            (got - expected).abs() < 3.0,
            "at scale {scale}: {got} pixels of ink, expected about {expected}"
        );
    }
}

/// The same shape in the other axis, which is the other half of a ruled grid.
#[test]
fn a_rectangle_of_no_width_marks_the_page_too() {
    let column = [
        PathCommand::MoveTo(Point::new(50.5, 10.0)),
        PathCommand::LineTo(Point::new(50.5, 10.0 + LENGTH)),
        PathCommand::Close,
    ];
    let list = filled(&column, FillRule::NonZero);
    let expected = f64::from(LENGTH * 2.0);
    let got = ink_at(&list, 2.0);
    assert!(
        (got - expected).abs() < 3.0,
        "{got} pixels of ink, expected about {expected}"
    );
}

/// The mark carries the ink a zero-width stroke down the same line carries, and not its pixels.
///
/// **This was an assertion of byte-identity until the three-hundred-and-sixty-eighth session,
/// and it is an ink test now because the identity was broken on purpose.** What the two
/// constructions still share is the *width*, which is the thing that must not drift: §8.4.3.2's
/// answer for a line and §10.7.4's for a flat fill are one device pixel, stated once in
/// `pdf_render::thinnest_line`, so each lays down one pixel of ink per device pixel of length.
///
/// What they no longer share is placement. The fill's mark is the whole device pixel row
/// §10.7.4 says the shape's boundary passes through — the clause states that outright, and it
/// has no width of its own for anything else to be derived from. The stroke keeps the
/// coordinates the document gave it, because moving a stroke's coordinates onto the grid is
/// §10.7.5's automatic stroke adjustment, which is conditioned on the graphics state's `/SA`
/// and which this tree therefore does not do unasked. ADR 0208 argues it.
///
/// The line is placed off a pixel boundary deliberately: on one, the snapped row and the
/// centred band are the same rectangle and the divergence this test pins would be invisible.
#[test]
fn a_flat_fill_carries_a_hairline_strokes_ink_at_its_own_placement() {
    const OFF_THE_GRID: f32 = 50.3;

    let fill = filled(&zero_height_rectangle(OFF_THE_GRID), FillRule::NonZero);
    let mut stroked = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, OFF_THE_GRID)));
    path.push(PathCommand::LineTo(Point::new(10.0 + LENGTH, OFF_THE_GRID)));
    stroked.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: pdf_render::Stroke {
            width: 0.0,
            ..pdf_render::Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    for scale in [1.0_f32, 2.0] {
        let target = TargetSpec::for_page(&fill, scale, GENEROUS).expect("a valid target");
        let one = CpuRasterizer::new()
            .rasterize(&fill, target)
            .expect("supported");
        let two = CpuRasterizer::new()
            .rasterize(&stroked, target)
            .expect("supported");
        let expected = f64::from(LENGTH * scale);
        let (mark, hairline) = (ink(&one), ink(&two));
        assert!(
            (mark - hairline).abs() < 1.0,
            "at scale {scale} the mark carries {mark} and the hairline {hairline}"
        );
        assert!(
            (mark - expected).abs() < 3.0,
            "at scale {scale}: {mark} pixels of ink, expected about {expected}"
        );
        assert_ne!(
            one.data, two.data,
            "at scale {scale} the snapped mark and the unsnapped hairline should differ"
        );
    }
}

/// A mark does not join the winding of the path it came out of.
///
/// The discriminating case for filling the marks separately: under the even-odd rule a
/// rectangle drawn *inside* a filled square toggles parity, so a mark added to the path would
/// punch a hole through the square rather than lying on it. The square's ink is its own area
/// and the flat subpath cannot subtract from it.
#[test]
fn a_mark_inside_an_even_odd_fill_does_not_cut_a_hole_in_it() {
    let mut commands = vec![
        PathCommand::MoveTo(Point::new(10.0, 10.0)),
        PathCommand::LineTo(Point::new(90.0, 10.0)),
        PathCommand::LineTo(Point::new(90.0, 90.0)),
        PathCommand::LineTo(Point::new(10.0, 90.0)),
        PathCommand::Close,
    ];
    commands.extend_from_slice(&zero_height_rectangle(50.5));
    let list = filled(&commands, FillRule::EvenOdd);
    // 80 × 80 units at scale 1.0, and the mark lies on top of it rather than beside it.
    let expected = 80.0 * 80.0;
    let got = ink_at(&list, 1.0);
    assert!(
        (got - expected).abs() < 3.0,
        "{got} pixels of ink, expected the square's {expected}"
    );
}

/// An ordinary shape is untouched, including one thinner than the mark this rule makes.
///
/// The rule's boundary: a sliver half a pixel high has an area, so it gets the coverage that
/// area implies — this renderer's documented departure over antialiased edges — and not a
/// whole pixel's worth. Only a shape with *no* extent is one that cannot appear at any
/// placement or any scale, which is what this rule is about.
///
/// # A second shape that disappeared, measured here and closed elsewhere
///
/// Half a unit was chosen because it was above `tiny-skia`'s own floor, and that floor was
/// worth writing down: this sliver at scale 1.0 used to deposit 39.8 of the 40 pixels its area
/// asks for at 0.5 units and **0** at 0.1, because the scan converter supersamples four times
/// per pixel row and takes each sub-row's sample at its centre. The ladder was 0.05 → 0,
/// 0.1 → 0, 0.2 → 19.8 of 16, 0.5 → 39.8 of 40 — a *device*-dependent disappearance rather than
/// a geometric one, and a rule of its own rather than an extension of this one.
///
/// It is `pdf_render::sub_pixel`'s since the three-hundred-and-eighty-ninth session, and the
/// whole ladder is now within one level of 255 of the area (ADR 0226): 0.05 → 0.0510 of a row,
/// 0.1 → 0.1020, 0.2 → 0.2000, 0.5 → 0.5020, where an eight-bit raster's own steps are 1/255
/// apart and there is nowhere finer to put them. This sliver's own answer moves from 39.8 to
/// **40.157** of its 40, which is 128 of 255 per column where the area asks for 127.5. The
/// tolerance below is unchanged, because what this test is for is unchanged: that an ordinary
/// thin shape is *not* given a whole pixel's mark — 80 pixels, which it is still forty away
/// from.
#[test]
fn a_sliver_keeps_the_coverage_its_area_asks_for() {
    let sliver = [
        PathCommand::MoveTo(Point::new(10.0, 50.0)),
        PathCommand::LineTo(Point::new(10.0 + LENGTH, 50.0)),
        PathCommand::LineTo(Point::new(10.0 + LENGTH, 50.5)),
        PathCommand::LineTo(Point::new(10.0, 50.5)),
        PathCommand::Close,
    ];
    let got = ink_at(&filled(&sliver, FillRule::NonZero), 1.0);
    let expected = f64::from(LENGTH * 0.5);
    assert!(
        (got - expected).abs() < 1.0,
        "{got} pixels of ink, expected the sliver's own coverage, about {expected}"
    );
}
