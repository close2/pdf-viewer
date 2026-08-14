//! A clip is a set of pixels, so a clip that contains a mark takes nothing from it.
//!
//! ISO 32000-2 §10.7.4's clipping paragraph — "[s]ubsequent painting operations shall affect a
//! region that is the intersection of the set of pixels defined by the clipping region with the
//! set of pixels for the region to be painted" — and §8.5.4 from the transparent imaging model's
//! side: "[t]he effective shape is the intersection of the object's intrinsic shape with the
//! clipping path; the source shape value shall be 0.0 outside this intersection."
//!
//! **The closed form these scenes assert is the set identity and nothing else**: `S ∩ C = S`
//! where `S ⊆ C`. So a mark drawn under a clip that contains it must be the mark drawn under no
//! clip at all, pixel for pixel — the boundary pixels included, which is the whole point, since
//! that is where this backend's anti-aliasing (departure (1) of §10.7.4's ledger row) gives both
//! the mark and the clip a fraction and where a product and an intersection part company. No
//! number here comes from another renderer, and none comes from this one either: the assertion
//! is one render against another render of the same geometry.
//!
//! `clip_bands.rs` asserts the same identity for the pixels a clip *admits* whole, and skips the
//! boundary column for exactly the reason this file exists. ADR 0280 took the composition of a
//! clip with another clip; ADR 0355 took the composition of a clip with the mark.
//!
//! # What each scene is placed to catch
//!
//! - The mark's edges land at fractional device coordinates, and at none of `tiny-skia`'s
//!   quarter-pixel sample rows: an integer placement makes every coverage 0 or 1, where a
//!   product and a minimum are the same function and any construction passes (ADR 0285's "the
//!   test that had to move").
//! - **The paint is a gradient running up the page**, because the composition draws the mark
//!   through a mask over a device rectangle rather than through its own path, and a paint is
//!   positioned in the path's space (trap 2). A shader transform applied twice, dropped, or
//!   mirrored moves the gradient inside the rectangle while every coverage stays as it was, and
//!   a solid fill cannot see it: a clipped rectangle of one colour is a clipped rectangle of one
//!   colour.
//! - Three scales and a rotation, because a defect that cancels at one magnitude need not at
//!   another and an axis-aligned scene cannot see a transposition.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "test code: a panic with a message is the intended failure mode, the arithmetic is \
              on literal page dimensions that cannot overflow, the casts are pixel indices \
              non-negative by the page's own geometry, and the single-character names are a \
              pixel's four channels"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, ClipId, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand,
    Point, Ramp, Raster, Rasterizer, Shading, ShadingKind, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units. Square, so a transposed transform would not go unnoticed.
const PAGE: f32 = 200.0;

/// The mark, in page space: a rectangle on no whole and no quarter device coordinate.
const MARK: (f32, f32, f32, f32) = (30.4, 40.6, 170.6, 90.4);

/// A rectangle as a closed path.
fn rect((x0, y0, x1, y1): (f32, f32, f32, f32)) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// The mark: [`MARK`] filled with a red-to-blue gradient running *up* the page.
fn mark(list: &mut DisplayList, transform: Transform, clip: Option<ClipId>) {
    list.push(Command::Fill {
        path: Arc::new(rect(MARK)),
        transform,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(Shading {
            kind: Arc::new(ShadingKind::Axial {
                start: Point::new(0.0, MARK.1),
                end: Point::new(0.0, MARK.3),
                ramp: Ramp::sample(|t| Color::rgb(1.0 - t, 0.0, t)),
                extend: (true, true),
            }),
            transform: Transform::IDENTITY,
        })),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    });
}

/// Renders the mark under `rungs` statements of a clip that is the mark's own rectangle.
///
/// Zero rungs is the unclipped render the others are judged against.
fn render(rungs: usize, transform: Transform, scale: f32) -> Raster {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut parent = None;
    for _ in 0..rungs {
        parent = Some(
            list.add_clip(Clip {
                path: rect(MARK),
                transform,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .expect("a clip"),
        );
    }
    mark(&mut list, transform, parent);
    let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("an axial shading under a clip is supported")
}

/// One pixel of a raster as a raster of its own, so that a named pixel and a whole page go
/// through the same comparison.
fn one_pixel(raster: &Raster, x: u32, y: u32) -> Raster {
    let (r, g, b, a) = pixel(raster, x, y);
    Raster {
        width: 1,
        height: 1,
        data: vec![r, g, b, a],
        format: raster.format,
    }
}

/// Reads a pixel as `(r, g, b, a)`.
fn pixel(raster: &Raster, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 4];
    (p[0], p[1], p[2], p[3])
}

/// Asserts that this scene can tell the two compositions apart before comparing them.
///
/// Two things have to be true of the *unclipped* render, and neither is implied by the other: a
/// partly covered boundary pixel, which is where a product and an intersection differ at all,
/// and an interior that varies down the page, which is what a displaced shader would move.
fn assert_it_discriminates(what: &str, unclipped: &Raster, scale: f32) {
    let left = (MARK.0 * scale) as u32;
    let middle = ((PAGE - f32::midpoint(MARK.1, MARK.3)) * scale) as u32;
    let boundary = pixel(unclipped, left, middle);
    let interior = pixel(unclipped, left + 1, middle);
    // The raster is opaque over a white page, so partial coverage shows as a colour between the
    // mark's own and the background's rather than in the alpha channel.
    assert!(
        boundary != interior && boundary != (255, 255, 255, 255),
        "{what}: the mark's own left edge must be partly covered — {boundary:?} against an \
         interior of {interior:?}"
    );
    let top = ((PAGE - MARK.3) * scale) as u32 + 2;
    let bottom = ((PAGE - MARK.1) * scale) as u32 - 2;
    let inside = (MARK.0 * scale) as u32 + 2;
    assert_ne!(
        pixel(unclipped, inside, top),
        pixel(unclipped, inside, bottom),
        "{what}: the gradient must vary down the page for a displaced shader to show"
    );
}

/// Compares two renders pixel for pixel, naming the first that differs by more than `levels`.
///
/// **One level of 255 is allowed and the reason is the composition itself.** The clip's mask and
/// the mark's coverage are two rasterisations of the same edge, each quantised to eight bits,
/// and `min` takes the smaller of the two: where the two quantisations of one coverage disagree
/// by a level — which they may, since they are built by separate calls with separate rounding —
/// the composed value is that level under the mark's own. It is a bound rather than a
/// measurement, and it is two orders below what this scene is about: the product this replaces
/// is tens of levels away, which `a_clip_coincident_with_the_mark_takes_nothing_from_it` asserts
/// by name at the boundary pixel.
fn assert_agrees_within(what: &str, left: &Raster, right: &Raster, levels: i32) {
    assert_eq!((left.width, left.height), (right.width, right.height));
    for y in 0..left.height {
        for x in 0..left.width {
            let (a, b) = (pixel(left, x, y), pixel(right, x, y));
            let apart = [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)]
                .into_iter()
                .map(|(p, q)| (i32::from(p) - i32::from(q)).abs())
                .max()
                .unwrap_or(0);
            assert!(
                apart <= levels,
                "{what}: device pixel ({x},{y}) is {apart} levels apart — {a:?} against {b:?}"
            );
        }
    }
}

/// The identity itself, at three magnitudes, and the boundary pixel named.
///
/// The named pixel is what makes this a test of the *composition*: at the mark's own left edge
/// the clip and the mark carry the same fraction `c`, so a product paints `c²` and an
/// intersection paints `c`. Naming it separately from the whole-page comparison is ADR 0285's
/// lesson — a scene must fail at the defect's magnitude and not only in its axis.
#[test]
fn a_clip_coincident_with_the_mark_takes_nothing_from_it() {
    for scale in [1.0_f32, 2.0, 4.0] {
        let unclipped = render(0, Transform::IDENTITY, scale);
        assert_it_discriminates(&format!("scale {scale}"), &unclipped, scale);
        let clipped = render(1, Transform::IDENTITY, scale);
        let left = (MARK.0 * scale) as u32;
        let middle = ((PAGE - f32::midpoint(MARK.1, MARK.3)) * scale) as u32;
        assert_agrees_within(
            &format!("the boundary pixel at scale {scale}"),
            &one_pixel(&clipped, left, middle),
            &one_pixel(&unclipped, left, middle),
            1,
        );
        assert_agrees_within(
            &format!("a coincident clip at scale {scale}"),
            &clipped,
            &unclipped,
            1,
        );
    }
}

/// The same clip stated over and over, which is `S ∩ C ∩ C ∩ …` and is still `S`.
///
/// `issue21346.pdf` states one device rectangle six times: three of them compose as a chain
/// (ADR 0280) and the rest meet the mark, which is what this rung ladder walks. The rungs are
/// compared with **each other** exactly as well as with the unclipped render within a level:
/// restating a clip may not move a single bit, whatever the two quantisations of one edge do.
#[test]
fn restating_the_clip_takes_nothing_either() {
    let unclipped = render(0, Transform::IDENTITY, 2.0);
    assert_it_discriminates("the ladder", &unclipped, 2.0);
    let once = render(1, Transform::IDENTITY, 2.0);
    assert_agrees_within("one clip", &once, &unclipped, 1);
    for rungs in 2..=6 {
        assert_agrees_within(
            &format!("{rungs} coincident clips"),
            &render(rungs, Transform::IDENTITY, 2.0),
            &once,
            0,
        );
    }
}

/// A mark whose boundary is not parallel to a device axis, where a coverage is fractional along
/// the whole of two edges rather than along a column.
#[test]
fn a_turned_clip_takes_nothing_from_a_turned_mark() {
    // About the page's centre, so the mark stays on the page; 20 degrees is off both axes and
    // off the diagonal, where a transposition would be invisible.
    let angle = 20.0_f32.to_radians();
    let (sin, cos) = angle.sin_cos();
    let centre = PAGE / 2.0;
    // A rotation about the page's centre, written out: translate the centre to the origin,
    // turn, and put it back.
    let turned = Transform::new(
        cos,
        sin,
        -sin,
        cos,
        centre - centre * cos + centre * sin,
        centre - centre * sin - centre * cos,
    );
    for scale in [1.0_f32, 2.0] {
        let unclipped = render(0, turned, scale);
        assert_agrees_within(
            &format!("a turned coincident clip at scale {scale}"),
            &render(1, turned, scale),
            &unclipped,
            1,
        );
    }
}
