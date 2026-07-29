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

/// A horizontal black line across the middle of the page, stroked with `stroke`.
///
/// Horizontal and at a half-integer y so that at scale 1.0 the line covers one row of pixels
/// exactly. A line straddling a row boundary would spread the same ink over two rows, which
/// measures identically and would make a reader wonder why.
fn line(stroke: Stroke) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 50.5)));
    path.push(PathCommand::LineTo(Point::new(10.0 + LENGTH, 50.5)));
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
