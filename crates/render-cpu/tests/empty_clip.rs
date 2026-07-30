//! A clipping path that encloses nothing: ISO 32000-2 §8.5.4 with §8.5.3.3.1.
//!
//! # Why this file exists
//!
//! §8.5.4 defines the clipping region as the area a fill would cover — "the same area that
//! would be filled by the `f` operator" — and §8.5.3.3.1 says a path whose last subpath is a
//! single-point open one "shall be disregarded and not considered to be part of the path". A
//! path that is *only* such a subpath therefore encloses nothing, and intersecting the
//! current region with nothing leaves nothing to mark.
//!
//! `issue9017_reduced.pdf` writes exactly that — `568.938 673.022 m W n` around a shading —
//! and it is the reason this is a test rather than a note. The page reached `tiny-skia` as an
//! empty path, which `tiny-skia` refuses, so the whole page failed to rasterise and nobody
//! could see what we did with the clip. Once §8.5.3.3.1's rule made the page drawable, the
//! oracle contradicted it: all three reference renderers leave the shading undrawn, and this
//! renderer painted it across the enclosing rectangle.
//!
//! The trap it belongs to is the second one. An empty path is a shape each rasteriser answers
//! for itself — `tiny-skia` refuses it, `kurbo` clips to an empty region — so the answer is
//! `Clip::admits_nothing`'s, in the crate both backends consume.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, \
              and the arithmetic is over a hundred-unit page and a raster of known size"
)]
#![expect(
    clippy::float_cmp,
    reason = "a clip that admits nothing leaves exactly zero ink, not nearly zero: a \
              tolerance here would pass for a fill that had been merely attenuated"
)]

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use std::sync::Arc;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units.
const PAGE: f32 = 100.0;

/// A page filled edge to edge, inside `clip`.
fn scene(clip: Clip) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let id = list.add_clip(clip).expect("one clip is within the limit");
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(PAGE, 0.0)));
    path.push(PathCommand::LineTo(Point::new(PAGE, PAGE)));
    path.push(PathCommand::LineTo(Point::new(0.0, PAGE)));
    path.push(PathCommand::Close);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: Some(id),
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
        .expect("a clipped fill is supported");
    ink(&raster)
}

/// A clip with an empty path admits no pixel, so the fill inside it marks nothing.
#[test]
fn a_fill_inside_an_empty_clip_marks_nothing() {
    let got = ink_of(&scene(Clip {
        path: Path::new(),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        parent: None,
    }));
    assert_eq!(
        got, 0.0,
        "§8.5.4's region is what an empty path would fill, which is nothing"
    );
}

/// The same page with a clip that admits something, so that the scene above is a *clip*
/// result rather than a scene that draws nothing whatever it is given.
///
/// Deleting `Clip::admits_nothing`'s use in the backend fails the test above; without this
/// one, so would a backend that dropped every clipped command on the floor.
#[test]
fn the_same_fill_inside_an_ordinary_clip_marks_the_page() {
    let mut quarter = Path::new();
    let half = PAGE / 2.0;
    quarter.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    quarter.push(PathCommand::LineTo(Point::new(half, 0.0)));
    quarter.push(PathCommand::LineTo(Point::new(half, half)));
    quarter.push(PathCommand::LineTo(Point::new(0.0, half)));
    quarter.push(PathCommand::Close);
    let got = ink_of(&scene(Clip {
        path: quarter,
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        parent: None,
    }));
    assert!(
        (got - 2500.0).abs() < 40.0,
        "a 50 by 50 quarter of the page is 2500 pixels; got {got:.0}"
    );
}
