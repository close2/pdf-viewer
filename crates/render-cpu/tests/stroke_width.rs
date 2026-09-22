//! The one thickness a device cannot go below: ISO 32000-2 §8.4.3.2 and §10.7.5.
//!
//! # What the expected values come from
//!
//! §8.4.3.2: "A line width of 0 shall denote the thinnest line that can be rendered at
//! device resolution: 1 device pixel wide." §10.7.5 says the same of a stroke under half a
//! pixel while `/SA` is enabled — "the stroke shall be rendered as a single-pixel line" — and
//! its NOTE says the two cases are the same width.
//!
//! Both are decided by `Stroke::device_width` in `pdf-render` rather than by a rasteriser,
//! and these tests are what make that decision checkable in pixels. They measure *ink* — the
//! total darkness a horizontal line deposits — because a one-pixel line of length `n` on a
//! white page deposits exactly `n` pixels' worth however it is drawn, while a stroke drawn at
//! a fifth of a pixel's coverage deposits a fifth of that. Ink separates the two by a factor
//! of five, where a single pixel's value separates them by a level or two.
//!
//! # Why the scale varies
//!
//! The rule is stated in *device* pixels and applied in the path's own space, so it is a
//! reciprocal of the scale. A test at one scale cannot tell a reciprocal from a constant —
//! trap 2's argument, which cost this project two mirrored gradients and a doubled image
//! transform — so every case here runs at three.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, \
              and the arithmetic is over a hundred-unit page and a raster of known size"
)]

use pdf_render::{
    BlendMode, Color, Command, DisplayList, Paint, Path, PathCommand, Point, Raster, Rasterizer,
    Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use std::sync::Arc;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units.
const PAGE: f32 = 100.0;

/// The stroked line's length, in PDF units.
const LENGTH: f32 = 80.0;

/// The turned rule's extent along each axis, in PDF units.
///
/// Its device length is therefore `DIAGONAL * scale * sqrt(2)`, which is what a thickness
/// measured as ink-over-length divides by.
const DIAGONAL: f32 = 70.0;

/// A horizontal black line across the middle of the page, stroked with `stroke`.
///
/// Horizontal and at a half-integer y so that at scale 1.0 the line covers one row of pixels
/// exactly. A line straddling a row boundary would spread the same ink over two rows, which
/// measures identically and would make a reader wonder why.
fn line(stroke: Stroke) -> DisplayList {
    line_at(50.5, stroke)
}

/// The same line, at a chosen `y`, so that a caller can vary where it falls on the pixel grid.
///
/// The two ends are at whole units and the page is a whole number of them, so at scale 1.0 the
/// only thing a caller's `y` changes is the fraction of a device pixel the rule's two edges sit
/// at — which is exactly the variable ISO 32000-2 §10.7.5's first requirement is about.
fn line_at(y: f32, stroke: Stroke) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, y)));
    path.push(PathCommand::LineTo(Point::new(10.0 + LENGTH, y)));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// A 45° line, its two ends both shifted in `y` by `shift`, stroked with `stroke`.
///
/// A turned rule is a different population from a horizontal one, not a variation on it: an
/// axis-aligned rule's two long edges run along pixel rows, while a turned rule's cross every
/// column it passes, so the two go through different halves of the scan converter (ADR 1082).
///
/// Shifting both ends by the same `shift` slides the rule along its own normal without turning
/// it, and a shift of one *device* pixel in `y` carries the pixel grid onto itself — so eight
/// shifts an eighth of a device pixel apart are one whole period of the placement variable, the
/// same period `line_at`'s caller walks.
fn diagonal_at(shift: f32, stroke: Stroke) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0 + shift)));
    path.push(PathCommand::LineTo(Point::new(
        10.0 + DIAGONAL,
        10.0 + DIAGONAL + shift,
    )));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Total darkness on the page, in units of one fully black pixel.
///
/// The page is white, so `255 - red` is how much ink a pixel carries and the sum over the
/// raster is how much the stroke deposited. Divided by 255 so that the answer is comparable
/// with a length in device pixels regardless of the scale.
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
        .expect("a stroke is supported");
    ink(&raster)
}

/// A zero-width stroke deposits one device pixel of ink per device pixel of length.
///
/// §8.4.3.2's "1 device pixel wide", measured. The expected ink is the line's length in
/// device units, which is `LENGTH * scale`, and the tolerance is the two end pixels a cap
/// may or may not round off.
#[test]
fn a_zero_width_stroke_is_one_device_pixel_at_every_scale() {
    let list = line(Stroke {
        width: 0.0,
        ..Stroke::default()
    });
    for scale in [1.0_f32, 2.0, 3.5] {
        let expected = f64::from(LENGTH * scale);
        let got = ink_at(&list, scale);
        assert!(
            (got - expected).abs() < 3.0,
            "at scale {scale}: {got} pixels of ink, expected about {expected}"
        );
    }
}

/// `/SA` promotes a stroke under half a device pixel to a whole one, and only then.
///
/// §10.7.5's rule in pixels. A width of 0.2 units is a fifth of a pixel at scale 1.0 and
/// deposits a fifth of the ink without adjustment; with adjustment it deposits a whole
/// pixel's worth. At scale 4.0 the same stroke is 0.8 of a pixel — over the clause's half —
/// and adjustment must leave it exactly where it was, which is the half of this rule that a
/// test firing on the key alone would not check.
#[test]
fn stroke_adjustment_promotes_only_a_sub_half_pixel_line() {
    let plain = line(Stroke {
        width: 0.2,
        ..Stroke::default()
    });
    let adjusted = line(Stroke {
        width: 0.2,
        adjust: true,
        ..Stroke::default()
    });

    for scale in [1.0_f32, 2.0] {
        let full = f64::from(LENGTH * scale);
        assert!(
            (ink_at(&adjusted, scale) - full).abs() < 3.0,
            "at scale {scale}: an adjusted 0.2-unit stroke should deposit about {full}, got {}",
            ink_at(&adjusted, scale)
        );
        // Without adjustment the ink is the stroke's own coverage: a stroke `w` device
        // pixels wide over a line `n` device pixels long deposits `w × n`, which is what
        // anti-aliasing means and is a fifth of a whole pixel's worth at scale 1.0.
        let faint = f64::from(0.2 * scale) * full;
        assert!(
            (ink_at(&plain, scale) - faint).abs() < 3.0,
            "at scale {scale}: an unadjusted 0.2-unit stroke deposited {} of the {faint} its \
             own coverage asks for",
            ink_at(&plain, scale)
        );
    }

    // 0.2 * 4.0 = 0.8 of a device pixel, which is over half: nothing to adjust.
    assert!(
        (ink_at(&adjusted, 4.0) - ink_at(&plain, 4.0)).abs() < 1.0,
        "at scale 4.0 the two should agree: {} against {}",
        ink_at(&adjusted, 4.0),
        ink_at(&plain, 4.0)
    );
}

/// The substituted width draws what `tiny-skia`'s own hairline draws.
///
/// This is the claim `convert::stroke`'s comment makes and the reason it is safe to stop
/// relying on the rasteriser's convention: handing `tiny-skia` one device pixel expressed in
/// path space produces the same pixels as handing it the `0.0` that selects hairline
/// stroking. Asserted at three scales, and exactly rather than within a tolerance — if the
/// two ever diverge, the GPU backend's agreement with this one is what pays for it.
#[test]
fn the_substituted_width_matches_the_rasterisers_hairline() {
    let width = |scale: f32| 1.0 / scale;
    for scale in [1.0_f32, 2.0, 3.5] {
        let hairline = line(Stroke {
            width: 0.0,
            ..Stroke::default()
        });
        let explicit = line(Stroke {
            width: width(scale),
            ..Stroke::default()
        });
        let target = TargetSpec::for_page(&hairline, scale, GENEROUS).expect("a valid target");
        let one = CpuRasterizer::new()
            .rasterize(&hairline, target)
            .expect("supported");
        let two = CpuRasterizer::new()
            .rasterize(&explicit, target)
            .expect("supported");
        assert_eq!(
            one.data, two.data,
            "at scale {scale} the hairline and the explicit width differ"
        );
    }
}

/// §10.7.5's *first* requirement, measured: the thickness a stroke achieves is within half a
/// device pixel of the requested width wherever the rule falls on the grid.
///
/// > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
/// > automatically be adjusted as necessary to produce lines of uniform thickness. The
/// > thickness shall be as near as possible to the requested line width -no more than half a
/// > pixel different.
///
/// The clause names an adjustment as the means and bounds an outcome; the bound is what a test
/// can hold, and it is a number rather than an appearance. Thickness here is ink divided by the
/// rule's device length, which is the width the device actually laid down — a rule `w` pixels
/// wide over `n` pixels of length deposits `w × n` on an area-sampling rasteriser however it
/// falls, and that is the property being asserted.
///
/// Eight placements an eighth of a device pixel apart is a whole period of the only variable
/// the requirement is about. Two scales, for the reason the module comment gives.
///
/// The tight bound is what makes this discriminating rather than vacuous, and it is a ratchet:
/// the test prints the worst deviation this ladder produces and `MEASURED` is that worst
/// doubled, for the slack the turned rung's comment states. The reference that grid-fits
/// instead — `poppler`, which snaps both edges to whole pixels and so draws a 0.6-pixel rule as
/// one whole one — reads 0.4 here and would fail it while staying inside the clause's own
/// bound. ADR 0848 has the ladder for all five renderers, ADR 1189 the re-measurement this
/// number is taken from.
#[test]
fn stroke_adjustment_holds_the_thickness_within_half_a_pixel_at_every_placement() {
    /// The clause's own bound, in device pixels of thickness.
    const CLAUSE: f64 = 0.5;
    /// What this backend holds on axis-aligned rules: the worst this ladder measures, doubled.
    const MEASURED: f64 = 0.016;

    let mut population = 0_u32;
    let mut worst = 0.0_f64;
    let mut worst_rung = String::new();
    for scale in [1.0_f32, 2.0] {
        for width in [0.6_f32, 1.0, 2.5] {
            let mut thicknesses = Vec::new();
            for eighth in 0_u8..8 {
                // An eighth of a *device* pixel, so the offset is a reciprocal of the scale —
                // trap 2's argument, which is why the module runs everything at two scales.
                let y = 50.0 + f32::from(eighth) / (8.0 * scale);
                let list = line_at(
                    y,
                    Stroke {
                        width,
                        adjust: true,
                        ..Stroke::default()
                    },
                );
                thicknesses.push(ink_at(&list, scale) / f64::from(LENGTH * scale));
            }
            // The rule is stated in *device* pixels and the width in the path's own space, so
            // the quantity the clause bounds is the width times the scale — the reciprocal the
            // module comment warns about, met from the other side.
            let requested = f64::from(width * scale);
            for (eighth, got) in thicknesses.iter().enumerate() {
                population += 1;
                let off = (got - requested).abs();
                assert!(
                    off <= CLAUSE,
                    "scale {scale}, width {width}, placement {eighth}/8: thickness {got} is more \
                     than half a device pixel from the requested {requested}"
                );
                assert!(
                    off <= MEASURED,
                    "scale {scale}, width {width}, placement {eighth}/8: thickness {got} against \
                     the requested {requested} is {off} out, past the {MEASURED} this backend \
                     held when ADR 1189 measured it"
                );
                if off > worst {
                    worst = off;
                    worst_rung = format!("scale {scale}, width {width}, {eighth}/8");
                }
            }
        }
    }
    println!(
        "axis-aligned rungs: {population} measured (2 scales x 3 widths x 8 placements), worst \
         {worst:.4} device px of thickness at {worst_rung}, against {MEASURED} held and {CLAUSE} \
         allowed by ISO 32000-2 10.7.5"
    );
}

/// §10.7.5's first requirement on a *turned* rung: a 45° rule's thickness stays within a
/// fraction of a device pixel of the requested width wherever it falls on the grid, with `/SA`
/// enabled and with it absent.
///
/// > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
/// > automatically be adjusted as necessary to produce lines of uniform thickness. The
/// > thickness shall be as near as possible to the requested line width -no more than half a
/// > pixel different.
///
/// # Why a turned rung is a separate population
///
/// The rung above is axis-aligned, and an axis-aligned rectangle goes to a closed form that
/// never enters the scan converter (ADR 0476, ADR 0226). A 45° rule does, and it is the shape
/// the analytic area integral of ADR 1082 was written for. The two measure different code, so a
/// ladder of horizontal rules says nothing about a turned one — which is how a 47-fold
/// improvement on turned rules went unreported: the gate held only the half that had not moved.
///
/// # Both sides of the parameter
///
/// Every width here is above §10.7.5's half a device pixel, so the clause's *substitution* has
/// no antecedent and `/SA true` must draw exactly what a document that sets nothing draws. That
/// is asserted rung for rung rather than assumed, because a promotion that fired where the
/// clause does not ask for one would otherwise pass this test as an improvement in uniformity.
///
/// # Where the tolerance comes from
///
/// `MEASURED` is a ratchet, so it is the measurement plus a stated slack rather than a round
/// number: the worst deviation this ladder produces is printed by the test itself, and the
/// constant is that worst rounded up to twice it. Twice is the slack, and what it buys is the
/// two things that legitimately move a coverage measurement without a defect: a different
/// rounding of the page's device size at a scale this ladder does not run, and the
/// last-bit differences an `f32` accumulation may shift. It is not slack for a change of
/// algorithm — one of those is meant to fail this and be re-measured. The clause's own bound is
/// asserted alongside it so that a reader can see which of the two is the standard's.
#[test]
fn stroke_thickness_holds_on_a_turned_rung_at_every_placement() {
    /// The clause's own bound, in device pixels of thickness.
    const CLAUSE: f64 = 0.5;
    /// What this backend holds on turned rungs: the worst this ladder measures, doubled.
    const MEASURED: f64 = 0.008;
    /// How far the two settings of `/SA` may differ, every width here being above half a pixel.
    ///
    /// `Stroke::device_width` returns the requested width unchanged in both cases, so the two
    /// rasters are in fact identical and the measured difference is zero. The bound is what
    /// makes the assertion discriminating rather than exact: a promotion that fired where the
    /// clause states no antecedent would move a 0.6-pixel rule to 1.0, which this catches by
    /// nearly three orders of magnitude.
    const AGREEMENT: f64 = 0.0005;

    let mut population = 0_u32;
    let mut worst = 0.0_f64;
    let mut worst_rung = String::new();
    for scale in [1.0_f32, 2.0] {
        let length = f64::from(DIAGONAL) * f64::from(scale) * f64::from(std::f32::consts::SQRT_2);
        for requested in [0.6_f64, 1.0, 3.0] {
            // The width is stated in *device* pixels and applied in the path's own space, so
            // it is a reciprocal of the scale — trap 2's argument, met from the other side, as
            // the module comment describes.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "test code: three literal widths of one decimal digit"
            )]
            let width = (requested / f64::from(scale)) as f32;
            let thickness = |adjust: bool| -> Vec<f64> {
                (0_u8..8)
                    .map(|eighth| {
                        let shift = f32::from(eighth) / (8.0 * scale);
                        let list = diagonal_at(
                            shift,
                            Stroke {
                                width,
                                adjust,
                                ..Stroke::default()
                            },
                        );
                        ink_at(&list, scale) / length
                    })
                    .collect()
            };
            let plain = thickness(false);
            let adjusted = thickness(true);
            for (eighth, (got, also)) in plain.iter().zip(adjusted.iter()).enumerate() {
                for (state, measured) in [("absent", got), ("/SA true", also)] {
                    population += 1;
                    let off = (measured - requested).abs();
                    assert!(
                        off <= CLAUSE,
                        "scale {scale}, {requested} device px, {state}, placement {eighth}/8: \
                         thickness {measured} is more than half a device pixel from the request"
                    );
                    assert!(
                        off <= MEASURED,
                        "scale {scale}, {requested} device px, {state}, placement {eighth}/8: \
                         thickness {measured} against the requested {requested} is {off} out, \
                         past the {MEASURED} this backend held when ADR 1189 measured it"
                    );
                    if off > worst {
                        worst = off;
                        worst_rung =
                            format!("scale {scale}, {requested} device px, {state}, {eighth}/8");
                    }
                }
                assert!(
                    (got - also).abs() <= AGREEMENT,
                    "scale {scale}, {requested} device px, placement {eighth}/8: /SA changed a \
                     width already above half a device pixel, {got} against {also}"
                );
            }
        }
    }
    println!(
        "turned rungs: {population} measured (2 scales x 3 widths x 2 /SA settings x 8 \
         placements), worst {worst:.4} device px of thickness at {worst_rung}, against \
         {MEASURED} held and {CLAUSE} allowed by ISO 32000-2 10.7.5"
    );
}
