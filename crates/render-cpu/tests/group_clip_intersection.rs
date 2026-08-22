//! A group's shape meets the clip at its blit as a set, not as a second factor.
//!
//! ISO 32000-2 §8.5.4 says it of a group in its own sentence, after saying it of a mark:
//!
//! > Similarly, the shape of a transparency group (defined as the union of the shapes of its
//! > constituent objects) shall be influenced both by the clipping path in effect when each of
//! > the objects is painted
//!
//! — and, the sentence goes on, by the one in effect at the time the group's results are
//! painted onto its backdrop. (The quotation stops where it does because the extraction in
//! `doc/md/` breaks that last word across a space, so the rest cannot be quoted verbatim.)
//!
//! and §10.7.4 makes that influence an intersection of sets rather than a product:
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by
//! > a fill operation. Subsequent painting operations shall affect a region that is the
//! > intersection of the set of pixels defined by the clipping region with the set of pixels
//! > for the region to be painted.
//!
//! `clip_intersection.rs` asserts the identity `S ∩ C = S` for a *mark* whose clip contains it
//! (ADR 0355). This file asserts it for a **group's blit**, which is the composition ADR 0492
//! took and which `doc/todo/11` item 4 carried for nineteen sessions with no small witness:
//! §11.4.4's NOTE 5 flattens a group away unless something is applied to it as a whole, so a
//! probe written without a mask or a group alpha never reaches the blit that multiplies at all.
//!
//! **No number here comes from another renderer, and none comes from this one either.** Every
//! assertion is one render against another render of the same geometry.
//!
//! # What the field is for, and why one test asserts the *old* arithmetic
//!
//! Table 139 returns a group's shape `f` beside its alpha `α` and a raster of premultiplied
//! samples holds one number. `pdf_render::Command::Group`'s `alpha_is_shape` is `pdf-model`'s
//! statement that the two coincide — §11.3.7.1's `α = f × q` with the group's opacity 1.0
//! everywhere — and it is what makes the intersection expressible. So the last test states a
//! group whose flag is `false` and asserts the product is still what comes out: the exact
//! composition is conditioned on the claim rather than applied to every group, and a fix that
//! ignored the flag would be wrong for a group whose opacity is *not* 1.0 and would pass every
//! other test in this file.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is on literal page dimensions and pixel indices that cannot overflow"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, ClipId, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand,
    Point, Raster, Rasterizer, Size, SoftMask, SoftMaskId, SoftMaskKind, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// What the identity is allowed to cost, in levels of 255.
///
/// **One, and it is the eight bits rather than the composition.** `min(f, C)` is exact where
/// the two coverages coincide — which they do here, since one rectangle states both — but only
/// while the two are *rounded* the same way, which is the sentence ADR 0363 already writes one
/// layer down. This tree rasterises a clipping region and a filled mark through two different
/// entry points, and a rectangle's edge can land a level apart in them; where it does, the
/// minimum takes the lower of the two. So a level survives on the mark's left column and
/// nowhere else, against the sixty-four the product cost the same edge (the last test names
/// that figure from the geometry).
const LEVEL: u8 = 1;

/// Page side, in PDF units. Square, so a transposed transform would not go unnoticed.
const PAGE: f32 = 60.0;

/// The rectangle every rung states, in page space.
///
/// Placed off every whole *and* every quarter device coordinate at each scale below, because
/// `tiny-skia` samples four times per pixel row: an edge on a sample line has coverage 0 or 1,
/// where a product and a minimum are the same function and any construction passes. ADR 0285's
/// "the test that had to move" is what this constant is guarding against.
const MARK: (f32, f32, f32, f32) = (10.3, 10.3, 40.504, 40.504);

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

/// One opaque black fill of [`MARK`].
///
/// Opaque because §11.6.4.2 gives every elementary object "an intrinsic opacity q j of 1.0
/// everywhere", which is what makes the group's accumulated alpha its shape.
fn fill(transform: Transform) -> Command {
    Command::Fill {
        path: Arc::new(rect(MARK)),
        transform,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// A soft mask worth 1.0 at every pixel of the page (§11.6.5.2, §11.5.3).
///
/// A luminosity mask over a group that paints the whole page white on a white backdrop, so
/// §11.5.3's derivation gives 1.0 inside the group's box and 1.0 outside it. It cannot change
/// what any pixel should be, and it changes which composition the group goes through — which is
/// exactly the property the eighth rung of `pdf-model`'s `coincident_edge_probe` rests on.
fn unit_mask(list: &mut DisplayList) -> SoftMaskId {
    list.add_soft_mask(SoftMask {
        commands: vec![Command::Fill {
            path: Arc::new(rect((0.0, 0.0, PAGE, PAGE))),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::WHITE),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }],
        kind: SoftMaskKind::Luminosity {
            backdrop: Color::WHITE,
        },
        transfer: None,
    })
    .expect("the first soft mask")
}

/// How the rectangle is stated a second time, if at all.
#[derive(Clone, Copy)]
enum Restated {
    /// The bare fill, with no group at all.
    Flat,
    /// A group holding the fill, clipped by nothing — the control every clipped rung is
    /// judged against, so that both sides of the identity pay the buffer round trip below.
    Alone,
    /// As the clip on a group holding the fill — §8.10.1 step c)'s `/BBox`, which is what a
    /// transparency group `XObject`'s box becomes.
    GroupClip {
        /// What `pdf-model` says about Table 139's two results for this group.
        alpha_is_shape: bool,
    },
}

/// Renders one rung, with `masked` deciding whether a unit soft mask stands beside the clip.
fn render(restated: Restated, masked: bool, transform: Transform, scale: f32) -> Raster {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mask = masked.then(|| unit_mask(&mut list));
    match restated {
        Restated::Flat => {
            let Command::Fill {
                path,
                fill_rule,
                paint,
                blend,
                ..
            } = fill(transform)
            else {
                unreachable!("`fill` builds exactly one variant")
            };
            list.push(Command::Fill {
                path,
                transform,
                fill_rule,
                paint,
                clip: None,
                mask,
                blend,
            });
        }
        Restated::Alone | Restated::GroupClip { .. } => {
            let clip: Option<ClipId> = match restated {
                Restated::GroupClip { .. } => Some(
                    list.add_clip(Clip {
                        path: rect(MARK),
                        transform,
                        fill_rule: FillRule::NonZero,
                        parent: None,
                    })
                    .expect("a clip"),
                ),
                _ => None,
            };
            let alpha_is_shape = match restated {
                Restated::GroupClip { alpha_is_shape } => alpha_is_shape,
                _ => true,
            };
            list.push(Command::Group {
                commands: vec![fill(transform)],
                alpha: 1.0,
                clip,
                mask,
                blend: BlendMode::Normal,
                isolated: true,
                knockout: false,
                alpha_is_shape,
                blending: None,
            });
        }
    }
    let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("an opaque fill in a group is supported")
}

/// Reads a pixel as `(r, g, b, a)`.
fn pixel(raster: &Raster, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 4];
    (p[0], p[1], p[2], p[3])
}

/// The largest per-channel difference between two rasters of one geometry, with where it is.
fn worst(left: &Raster, right: &Raster) -> (u8, u32, u32) {
    assert_eq!(
        (left.width, left.height),
        (right.width, right.height),
        "two renders of one page"
    );
    let mut worst = (0, 0, 0);
    for y in 0..left.height {
        for x in 0..left.width {
            let (a, b) = (pixel(left, x, y), pixel(right, x, y));
            let difference = [
                a.0.abs_diff(b.0),
                a.1.abs_diff(b.1),
                a.2.abs_diff(b.2),
                a.3.abs_diff(b.3),
            ]
            .into_iter()
            .max()
            .unwrap_or(0);
            if difference > worst.0 {
                worst = (difference, x, y);
            }
        }
    }
    worst
}

/// The rotation one scene uses, so a defect that cancels on an axis cannot hide.
fn turned() -> Transform {
    // Seventeen degrees: off the axes and off the diagonal, where a transposition would be
    // invisible. Written out as a rotation about the page's centre — translate the centre to
    // the origin, turn, and put it back — because `Transform` states the six numbers.
    let angle = 17.0_f32.to_radians();
    let (sin, cos) = angle.sin_cos();
    let centre = PAGE / 2.0;
    Transform::new(
        cos,
        sin,
        -sin,
        cos,
        centre - centre * cos + centre * sin,
        centre - centre * sin - centre * cos,
    )
}

/// A group of one mark is that mark, to within the buffer round trip — the control.
///
/// §11.4.4's NOTE 5 states when a group changes nothing: the same page results "if the group is
/// non-isolated and has the same knockout attribute as its parent group" and the shape and
/// opacity inputs at its blit "are always 1.0". This scene is one opaque fill in a group with
/// no clip, no mask and alpha 1.0, so nothing should move — and one level of 255 does, because
/// this backend accumulates the group's coverage into a premultiplied buffer and composites
/// *that*, where the flat mark's coverage is interpolated towards the destination in one step.
/// Two roundings against one.
///
/// **This is what the tests below are judged against rather than the flat mark**, so that both
/// sides of §8.5.4's identity pay the same round trip and the assertion is the clause rather
/// than the arithmetic underneath it.
#[test]
fn a_group_of_one_mark_costs_one_level_and_that_is_the_control() {
    for scale in [1.0, 2.0, 3.7] {
        let flat = render(Restated::Flat, false, Transform::IDENTITY, scale);
        let grouped = render(Restated::Alone, false, Transform::IDENTITY, scale);
        let (difference, x, y) = worst(&flat, &grouped);
        assert!(
            difference <= 1,
            "at scale {scale}, a group of one mark moved the page by {difference} at ({x}, {y})"
        );
    }
}

/// A group's `/BBox` on the mark's own rectangle takes nothing from the mark.
///
/// The set identity `S ∩ C = S` where `S ⊆ C`, asked of the composition at a group's blit.
/// Without a soft mask beside it the clip arrives as `scan::Clip::Region` and the composition
/// is `min(f, C)`.
#[test]
fn a_groups_own_box_takes_nothing_from_its_only_mark() {
    for scale in [1.0, 2.0, 3.7] {
        for transform in [Transform::IDENTITY, turned()] {
            let alone = render(Restated::Alone, false, transform, scale);
            let grouped = render(
                Restated::GroupClip {
                    alpha_is_shape: true,
                },
                false,
                transform,
                scale,
            );
            let (difference, x, y) = worst(&alone, &grouped);
            assert!(
                difference <= LEVEL,
                "at scale {scale}, a group clipped by its own content's rectangle differed \
                 from the same content unclipped by {difference} at ({x}, {y})"
            );
        }
    }
}

/// The same identity with §11.6.5's soft mask standing beside the clip.
///
/// The mask is 1.0 everywhere, so it cannot change what any pixel should be — but it changes
/// the route: the clip now arrives as `scan::Clip::Both`, whose composition is
/// `min(f · S, C · S)` rather than `min(f, C)`, and the two are equal only because rounding is
/// monotone (ADR 0363's identity, one layer up).
#[test]
fn a_unit_soft_mask_beside_the_box_changes_the_route_and_no_pixel() {
    for scale in [1.0, 2.0, 3.7] {
        let alone = render(Restated::Alone, true, Transform::IDENTITY, scale);
        let grouped = render(
            Restated::GroupClip {
                alpha_is_shape: true,
            },
            true,
            Transform::IDENTITY,
            scale,
        );
        let (difference, x, y) = worst(&alone, &grouped);
        assert!(
            difference <= LEVEL,
            "at scale {scale}, a group under a unit soft mask and its own box differed from \
             the same content under the mask alone by {difference} at ({x}, {y})"
        );
    }
}

/// The boundary pixel is the one that decides it, and it is worth naming.
///
/// At scale 1 the mark's lower edge falls at device y `60 − 40.504`, so device row 19 is
/// covered `0.504` of a pixel. A product paints it at `0.504²` — 0.254 of the mark, 65 levels
/// of black on white against 129 — and the set identity paints it at its own coverage. The
/// assertion is against the unclipped render rather than against either number, so it states
/// the identity and not a constant.
#[test]
fn the_boundary_row_is_the_marks_own_coverage_and_not_its_square() {
    let column = 20;
    let row = 19;
    let alone = render(Restated::Alone, false, Transform::IDENTITY, 1.0);
    let grouped = render(
        Restated::GroupClip {
            alpha_is_shape: true,
        },
        false,
        Transform::IDENTITY,
        1.0,
    );
    let bare = pixel(&alone, column, row);
    assert!(
        bare.0 > 0 && bare.0 < 255,
        "the row this test is about must be a fractional edge, and it read {bare:?}"
    );
    assert_eq!(
        pixel(&grouped, column, row),
        bare,
        "the group's boundary row is not the mark's own coverage"
    );
}

/// A group whose alpha is **not** its shape keeps the product, because nothing else is stated.
///
/// §11.3.7.1 gives alpha as the product of shape and opacity, so a group with an opacity below
/// 1.0 somewhere has a shape this raster does not hold and `min(α, C)` would be a different
/// approximation rather than the clause: where `C < f` it overstates by as much as the product
/// understates. `pdf-model` answers `false` there, and this asserts the backend obeys the
/// answer rather than the flag being decoration.
#[test]
fn a_group_whose_alpha_is_not_its_shape_is_composited_through_the_mask() {
    let column = 20;
    let row = 19;
    let alone = render(Restated::Alone, false, Transform::IDENTITY, 1.0);
    let stated = render(
        Restated::GroupClip {
            alpha_is_shape: false,
        },
        false,
        Transform::IDENTITY,
        1.0,
    );
    let bare = pixel(&alone, column, row);
    let product = pixel(&stated, column, row);
    assert_ne!(
        product, bare,
        "with the shape unstated the boundary row should still be the product"
    );
    // Black on white, so the ink is `255 − red`. The product's is the square of the identity's,
    // to within the rounding of two eight-bit steps.
    let ink = |sample: (u8, u8, u8, u8)| f64::from(255 - sample.0) / 255.0;
    let squared = ink(bare) * ink(bare);
    assert!(
        (ink(product) - squared).abs() < 0.01,
        "the unstated group's boundary ink {} is not the identity's {} squared ({squared})",
        ink(product),
        ink(bare)
    );
}
