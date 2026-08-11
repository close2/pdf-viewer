//! A clip restricts *where* a command draws and must not change *what* it draws.
//!
//! The backend narrows the drawing surface to the rows a clip can admit, so that a
//! command costs what it can change rather than what the page is. That makes the band
//! arithmetic load-bearing for correctness as well as speed: an off-by-one in the row
//! offset shifts everything the command draws, and a shading is the only paint that
//! makes such a shift visible — a solid fill looks identical however far it slid,
//! because a clipped rectangle of one colour is a clipped rectangle of one colour.
//!
//! So these scenes are filled with a gradient running up the page, and each asserts the
//! same property: **inside the clip, the page is exactly what it would have been with no
//! clip at all.** That is the definition of a clip, and it is what a band must preserve.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a panic with a message is the intended failure mode, the \
              arithmetic is on literal page dimensions that cannot overflow, and the \
              casts are pixel indices far inside f32's exact range"
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

/// A rectangle as a closed path.
fn rect(min: Point, max: Point) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(min));
    path.push(PathCommand::LineTo(Point::new(max.x, min.y)));
    path.push(PathCommand::LineTo(max));
    path.push(PathCommand::LineTo(Point::new(min.x, max.y)));
    path.push(PathCommand::Close);
    path
}

/// The whole page filled with a red-to-blue gradient running up it, under `clip`.
fn gradient_page(list: &mut DisplayList, clip: Option<ClipId>) {
    list.push(Command::Fill {
        path: Arc::new(rect(Point::new(0.0, 0.0), Point::new(PAGE, PAGE))),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(Shading {
            kind: Arc::new(ShadingKind::Axial {
                start: Point::new(0.0, 0.0),
                end: Point::new(0.0, PAGE),
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

fn render(list: &DisplayList, scale: f32) -> Raster {
    let target = TargetSpec::for_page(list, scale, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("an axial shading under a clip is supported")
}

/// Reads a pixel as `(r, g, b, a)`.
fn pixel(raster: &Raster, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 4];
    (p[0], p[1], p[2], p[3])
}

/// Compares a clipped render against an unclipped one over the whole page.
///
/// `admits` says whether a *device* pixel is strictly inside the clip. Pixels on the
/// clip's boundary are excluded by the caller rather than tested with a tolerance:
/// antialiasing gives them a genuinely intermediate value, and a test that accepted an
/// intermediate value everywhere would accept a page-wide shift of half a pixel.
fn assert_clip_only_restricts(
    what: &str,
    clipped: &Raster,
    unclipped: &Raster,
    admits: impl Fn(u32, u32) -> Option<bool>,
) {
    let mut inside = 0_u32;
    for y in 0..clipped.height {
        for x in 0..clipped.width {
            let Some(admitted) = admits(x, y) else {
                continue;
            };
            let actual = pixel(clipped, x, y);
            if admitted {
                inside += 1;
                assert_eq!(
                    actual,
                    pixel(unclipped, x, y),
                    "{what}: ({x},{y}) is inside the clip and must be untouched by it"
                );
            } else {
                assert_eq!(
                    actual,
                    (255, 255, 255, 255),
                    "{what}: ({x},{y}) is outside the clip and must stay background"
                );
            }
        }
    }
    assert!(inside > 0, "{what}: the clip admitted nothing to compare");
}

/// The base case: one rectangular clip well inside the page.
///
/// Run at two scales because the band is measured in device rows, so a defect that
/// happens to cancel at one scale need not at another.
#[test]
fn a_rectangular_clip_leaves_what_it_admits_untouched() {
    for scale in [1.0f32, 2.0] {
        let mut unclipped = DisplayList::new(Size::new(PAGE, PAGE));
        gradient_page(&mut unclipped, None);

        let mut clipped = DisplayList::new(Size::new(PAGE, PAGE));
        // Page y 40..90, which is device rows 110..160 at scale 1 — nowhere near the
        // page's centre line, so a mirror about it cannot pass.
        let id = clipped
            .add_clip(Clip {
                path: rect(Point::new(30.0, 40.0), Point::new(170.0, 90.0)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("first clip");
        gradient_page(&mut clipped, Some(id));

        let clipped = render(&clipped, scale);
        let unclipped = render(&unclipped, scale);

        assert_clip_only_restricts(
            &format!("rectangular clip at scale {scale}"),
            &clipped,
            &unclipped,
            |x, y| device_rect_admits(x, y, (30.0, 40.0, 170.0, 90.0), scale),
        );
    }
}

/// Nested clips, where the child's band is a strict subset of its parent's.
///
/// The effective clip is the intersection of the chain, so the parent must still be
/// obeyed where the child alone would admit.
#[test]
fn nested_clips_intersect() {
    let mut unclipped = DisplayList::new(Size::new(PAGE, PAGE));
    gradient_page(&mut unclipped, None);

    let mut clipped = DisplayList::new(Size::new(PAGE, PAGE));
    let parent = clipped
        .add_clip(Clip {
            path: rect(Point::new(20.0, 20.0), Point::new(180.0, 120.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("parent clip");
    let child = clipped
        .add_clip(Clip {
            path: rect(Point::new(60.0, 60.0), Point::new(140.0, 160.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: Some(parent),
        })
        .expect("child clip");
    gradient_page(&mut clipped, Some(child));

    let clipped = render(&clipped, 1.0);
    let unclipped = render(&unclipped, 1.0);

    // The intersection: x 60..140, y 60..120.
    assert_clip_only_restricts("nested clips", &clipped, &unclipped, |x, y| {
        device_rect_admits(x, y, (60.0, 60.0, 140.0, 120.0), 1.0)
    });
}

/// Restating a clip is intersecting a set with itself, so it must change nothing.
///
/// ISO 32000-2 §10.7.4: "For clipping, the clipping region consists of the set of pixels that
/// would be included by a fill operation. Subsequent painting operations shall affect a region
/// that is the intersection of the set of pixels defined by the clipping region with the set of
/// pixels for the region to be painted." A set intersected with itself is that set, however
/// many times it is taken.
///
/// The rectangle's edges are deliberately at fractional device coordinates: on integer ones
/// every coverage is 0 or 1, where a product and a minimum agree and the test would pass
/// against either composition. `issue21346.pdf` states one rectangle six times over and its
/// edge was painted at a twentieth of the mark before this assertion existed (ADR 0280).
#[test]
fn restating_a_clip_changes_nothing() {
    let ladder: Vec<Raster> = (1..=6)
        .map(|rungs| {
            let mut list = DisplayList::new(Size::new(PAGE, PAGE));
            let mut parent = None;
            for _ in 0..rungs {
                parent = Some(
                    list.add_clip(Clip {
                        path: rect(Point::new(30.4, 40.6), Point::new(170.6, 90.4)),
                        transform: Transform::IDENTITY,
                        fill_rule: FillRule::NonZero,
                        parent,
                    })
                    .expect("a clip"),
                );
            }
            gradient_page(&mut list, parent);
            render(&list, 1.0)
        })
        .collect();

    let (one, restated) = ladder.split_first().expect("six rungs");
    // The discriminating pixel, named rather than trusted to the whole-page comparison below:
    // the clip's left edge falls in device column 30, which must carry the same fraction
    // whether the clip is stated once or six times.
    let edge = pixel(one, 30, 120);
    assert!(
        edge != (255, 255, 255, 255),
        "the boundary column must be partly painted for this test to discriminate, not {edge:?}"
    );
    for (rungs, raster) in restated.iter().enumerate() {
        assert_eq!(
            pixel(raster, 30, 120),
            edge,
            "the boundary column under {} coincident clips",
            rungs + 2
        );
        assert!(
            raster.data == one.data,
            "{} coincident clips must draw what one draws",
            rungs + 2
        );
    }
}

/// A clip that lies entirely off the page admits nothing at all.
///
/// Worth its own case because it is the one clip that produces no mask: there are no
/// rows to build one in, and the command has to be dropped rather than drawn unclipped.
#[test]
fn a_clip_beyond_the_page_draws_nothing() {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let id = list
        .add_clip(Clip {
            path: rect(
                Point::new(10.0, PAGE + 50.0),
                Point::new(190.0, PAGE + 100.0),
            ),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("first clip");
    gradient_page(&mut list, Some(id));

    let raster = render(&list, 1.0);
    assert!(
        raster
            .data
            .chunks_exact(4)
            .all(|p| p == [255, 255, 255, 255]),
        "a clip entirely off the page must leave the page blank"
    );
}

/// Whether a device pixel is strictly inside or strictly outside a page-space rectangle.
///
/// `None` for the row or column a clip edge passes through, where antialiasing makes the
/// value neither one thing nor the other.
fn device_rect_admits(x: u32, y: u32, page_rect: (f32, f32, f32, f32), scale: f32) -> Option<bool> {
    let (x0, y0, x1, y1) = page_rect;
    // Device rows count down from the page's top edge.
    let left = x0 * scale;
    let right = x1 * scale;
    let top = (PAGE - y1) * scale;
    let bottom = (PAGE - y0) * scale;

    let x = x as f32;
    let y = y as f32;
    let strictly_in = x > left && x + 1.0 < right && y > top && y + 1.0 < bottom;
    let strictly_out = x + 1.0 < left || x > right || y + 1.0 < top || y > bottom;
    match (strictly_in, strictly_out) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}
