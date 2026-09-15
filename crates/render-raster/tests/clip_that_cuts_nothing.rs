//! ISO 32000-2 §10.7.4's clip is a **set of pixels**, and a set that contains a mark takes
//! nothing from it.
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by
//! > a fill operation. Subsequent painting operations shall affect a region that is the
//! > intersection of the set of pixels defined by the clipping region with the set of pixels for
//! > the region to be painted.
//!
//! §8.5.4 states the same of the shape — "[t]he effective shape is the intersection of the
//! object's intrinsic shape with the clipping path" — so where the mark lies inside the region,
//! the intersection is the mark, boundary pixels included.
//!
//! **Both backends compose a clip with a mark by an arithmetic that is not an intersection.**
//! `render-cpu` takes the smaller of the two coverages (ADR 0355), which has the property; raster
//! multiplies them inside the graphics library (`coverage.wgsl`, `cov * extent.x * extent.y`),
//! which does not — a mark whose own edge shares a pixel with its clip's keeps the *square* of
//! its coverage there. `render_raster::scene`'s `cuts_nothing` answers the clause on this side by
//! not stating a clip that cuts nothing, which is exact for every such mark and needs no change
//! in the device.
//!
//! # What the numbers here are, and how they were calibrated (trap 13)
//!
//! Every figure is the mark's own area in whole device pixels, computed from the geometry below
//! and not from either backend. The rule was planted back — `cuts_nothing` made to answer `false`
//! — to confirm this file names the defect rather than passing either way:
//!
//! ```text
//!                       geometry    cpu oracle    raster, rule off    raster, rule on
//!   contained, 1x         4.0000        3.9961              3.0353             3.9961
//!   contained, 2x         4.0000        4.0069              3.2853             4.0069
//! ```
//!
//! **The mark is a triangle and not a rectangle, and that is the calibration's own finding**: the
//! first draft of this file used a rectangle and passed with the rule off, because an
//! axis-aligned rectangular fill takes the device's rectangle path, where a clip rectangle is met
//! by a geometric intersection and never by a product. It is every *other* shape — which is what
//! a glyph, a curve and a stroke's outline are — whose coverage comes off the atlas and is
//! multiplied by the clip's. Trap 13: an uncalibrated instrument's clean answer is a sentence
//! about the instrument.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a backend that cannot draw one rectangle on an 8 x 4 page is the failure \
              this file reports, and the sums are over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// The page every mark below is drawn on.
const PAGE: Size = Size {
    width: 8.0,
    height: 4.0,
};

/// The mark: a rectangle whose four sides all fall a quarter of the way into a device pixel, so
/// that every side of it is a boundary the two compositions can disagree about.
const MARK: (f32, f32, f32, f32) = (1.25, 1.25, 5.25, 3.25);

/// [`MARK`]'s own area, in whole device pixels at the page's own scale.
const AREA: f64 = 4.0;

/// A clip whose left edge stands half a unit inside [`MARK`]'s, so that it cuts.
///
/// What it removes is the trapezoid between `x = 1.25` and `x = 1.75` under the triangle's
/// hypotenuse: heights 2 and 1.75 over a width of a half, so **0.9375** of the mark's 4.
const CUTTING: (f32, f32, f32, f32) = (1.75, 1.25, 5.25, 3.25);

/// How much of [`CUTTING`]'s removal a backend must at least show, in whole device pixels.
///
/// Neither composition owes the removal exactly — `min` over-states an intersection where both
/// coverages are fractional in one pixel, and a product under-states it — so what is asserted is
/// that the clip was *applied*, at half the geometry's own figure. A containment test that
/// answered `true` for every clip would show nothing here at all.
const AT_LEAST_CUT: f64 = 0.9375 / 2.0;

/// How far an answer may sit from the area, as a fraction of it.
///
/// Eight-bit coverage is what sets it: a boundary pixel's 0.75 is stored as 191 or 192 of 255, and
/// over the mark's boundary that is a few thousandths of its area either way — the unclipped
/// figures above are 3.9961 and 4.0069 against a geometric 4. This is 0.5%, which is room for an
/// adapter's rounding on top of that and still a fiftieth of the 24% the product costs at page
/// scale.
const TOLERANCE: f64 = 0.005;

/// The mark as a closed path: a right triangle on [`MARK`]'s lower-left corner.
fn triangle((x0, y0, x1, y1): (f32, f32, f32, f32)) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// A rectangle as a closed path.
fn rectangle((x0, y0, x1, y1): (f32, f32, f32, f32)) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// [`MARK`] filled, under `clip` where one is given.
fn scene(clip: Option<(f32, f32, f32, f32)>) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    let clip = clip.map(|box_| {
        list.add_clip(Clip {
            path: rectangle(box_),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("one clip fits in a display list")
    });
    list.push(Command::Fill {
        path: Arc::new(triangle(MARK)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The ink on one raster, in whole device pixels of full coverage.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| 765 - u64::from(pixel[0]) - u64::from(pixel[1]) - u64::from(pixel[2]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "765 times a raster of 8 x 4 pixels is a small integer"
    )]
    let sum = sum as f64;
    sum / 765.0
}

/// Both backends' ink for one scene at one scale, scale-normalised so that the two rungs are
/// comparable with the geometry's own area.
fn drawn(list: &DisplayList, scale: f32) -> (f64, f64) {
    let target = TargetSpec::for_page(list, scale, 1 << 20).expect("a target for an 8 x 4 page");
    let oracle = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws one rectangle");
    let mut backend =
        QuorraRasterizer::with_options(&render_raster::options()).expect("a graphics device");
    backend.set_coverage(raster_gpu::Coverage::Cpu);
    let ours = backend
        .rasterize(list, target)
        .expect("the device draws one rectangle");
    let area = f64::from(scale) * f64::from(scale);
    (ink(&oracle) / area, ink(&ours) / area)
}

/// The clause: a clip that contains a mark leaves it exactly as it was, on both backends and at
/// both scales.
///
/// The clip here is [`MARK`]'s own rectangle and the mark is the triangle on two of its sides, so
/// those two sides are coincident with the clip's — the placement where a product of coverages is
/// at its worst and an intersection costs nothing at all.
#[test]
fn a_clip_that_contains_a_mark_takes_nothing_from_it() {
    for scale in [1.0_f32, 2.0] {
        let (oracle, ours) = drawn(&scene(Some(MARK)), scale);
        for (backend, ink) in [("cpu oracle", oracle), ("raster", ours)] {
            assert!(
                (ink - AREA).abs() <= AREA * TOLERANCE,
                "at {scale}x the {backend} drew {ink} of a mark whose own area is {AREA}, and \
                 its clip is that same rectangle — §10.7.4's intersection with a region that \
                 contains the mark is the mark"
            );
        }
    }
}

/// And the other half, which is what stops the rule above from being too generous: a clip that
/// **does** cut takes what it cuts.
///
/// Without this, a containment test that answered `true` for every clip would pass the test above
/// and lose nothing in it. [`CUTTING`] takes 0.9375 of the mark's 4, and both backends are held
/// to at least half of that — see [`AT_LEAST_CUT`] for why not to the figure itself.
#[test]
fn a_clip_that_cuts_still_cuts() {
    for scale in [1.0_f32, 2.0] {
        let (oracle, ours) = drawn(&scene(Some(CUTTING)), scale);
        for (backend, ink) in [("cpu oracle", oracle), ("raster", ours)] {
            assert!(
                ink <= AREA - AT_LEAST_CUT,
                "at {scale}x the {backend} drew {ink} of a mark whose area is {AREA} under a clip \
                 that removes 0.9375 of it: a clip that cuts may not be left off the mark"
            );
        }
    }
}

/// The unclipped mark, so that the figure the two tests above are held to is measured rather
/// than only computed.
#[test]
fn the_marks_own_area_is_what_both_backends_draw_unclipped() {
    for scale in [1.0_f32, 2.0] {
        let (oracle, ours) = drawn(&scene(None), scale);
        for (backend, ink) in [("cpu oracle", oracle), ("raster", ours)] {
            assert!(
                (ink - AREA).abs() <= AREA * TOLERANCE,
                "at {scale}x the {backend} drew {ink} of an unclipped mark whose area is {AREA}"
            );
        }
    }
}
