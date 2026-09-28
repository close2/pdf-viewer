//! An image's edge under a clip that states the same edge is the image's own edge — ISO 32000-2
//! §10.7.4's clipping paragraph, for the third painting operator.
//!
//! # What the expected value comes from
//!
//! §10.7.4: "For clipping, the clipping region consists of the set of pixels that would be
//! included by a fill operation. Subsequent painting operations shall affect a region that is the
//! intersection of the set of pixels defined by the clipping region with the set of pixels for the
//! region to be painted." An image is a painting operation like a fill and a stroke, and a region
//! that contains the image's own region takes nothing from it: `S ∩ C = S`. Under this backend's
//! anti-aliasing (§10.7.1's NOTE) a boundary pixel carries a fraction rather than a membership,
//! and the identity is what that fraction must keep — the image's edge column under a clip whose
//! edge is the image's own reads what it reads unclipped. A product of the two fractions reads
//! their square; `doc/todo/11` item 4 named this operator as the one still composed that way.
//!
//! The clip's other edge cuts the image, so the region does not contain it and is composed with it
//! rather than left off. The edge is placed at device 30.504 at one device pixel per unit, so the fraction is 0.504 and
//! its square 0.254 — sixty-four levels of 255 apart, where the identity allows the arithmetic's
//! one.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, and an \
              index outside a raster this file sized, forty pixels square, is the test failing"
)]

use pdf_render::{
    BlendMode, Clip, Command, DisplayList, FillRule, Image, Path, PathCommand, Point, Raster,
    Rasterizer, SampleAlpha, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything here.
const GENEROUS: u64 = 1 << 30;

/// The page, at one device pixel per unit.
const PAGE: Size = Size {
    width: 40.0,
    height: 40.0,
};

/// The image's rectangle on the page: its right edge falls 0.504 into device column 30.
const LEFT: f32 = 10.0;
/// See [`LEFT`].
const RIGHT: f32 = 30.504;
/// See [`LEFT`].
const BOTTOM: f32 = 10.0;
/// See [`LEFT`].
const TOP: f32 = 30.0;

/// One opaque black sample.
fn black() -> Image {
    Image {
        width: 1,
        height: 1,
        data: vec![0, 0, 0, 255].into(),
        interpolate: false,
        sample_alpha: SampleAlpha::Shape,
    }
}

/// The clip's left edge, inside the image: the region cuts the image's left part away, so it does
/// not contain the image and is composed with it rather than left off (ADR 1095), and its right
/// edge is the image's own.
const CLIP_LEFT: f32 = 15.0;

/// The clip's rectangle as a path on the page.
fn rectangle() -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(CLIP_LEFT, BOTTOM)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, BOTTOM)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, TOP)));
    path.push(PathCommand::LineTo(Point::new(CLIP_LEFT, TOP)));
    path.push(PathCommand::Close);
    path
}

/// The image placed on its rectangle, under `clips` restatements of the clip.
fn drawn(clips: usize) -> Raster {
    let mut list = DisplayList::new(PAGE);
    let mut parent = None;
    for _ in 0..clips {
        parent = Some(
            list.add_clip(Clip {
                path: rectangle(),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .expect("a clip"),
        );
    }
    list.push(Command::Image {
        image: black().into(),
        transform: Transform::new(RIGHT - LEFT, 0.0, 0.0, TOP - BOTTOM, LEFT, BOTTOM),
        alpha: 1.0,
        clip: parent,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("an image under a clip is supported")
}

/// The ink at device pixel `(x, y)`, as a fraction of a whole pixel.
fn ink(raster: &Raster, (x, y): (usize, usize)) -> f32 {
    let red = raster.data[(y * raster.width as usize + x) * 4];
    f32::from(255 - red) / 255.0
}

/// The edge column reads the image's own fraction however many times the clip restates it, and
/// the clip's own left edge still cuts.
#[test]
fn an_images_edge_under_a_clip_sharing_it_is_its_own_edge() {
    let alone = ink(&drawn(0), (30, 20));
    assert!(
        (alone - 0.504).abs() < 1.5 / 255.0,
        "the image's own edge covers 0.504 of column 30; drawn {alone}"
    );
    for clips in 1..=3 {
        let clipped = ink(&drawn(clips), (30, 20));
        assert!(
            (clipped - alone).abs() <= 1.0 / 255.0,
            "under {clips} restatement(s) the edge reads {clipped} where alone it reads {alone}"
        );
        assert!(
            ink(&drawn(clips), (12, 20)) < 1.0 / 255.0,
            "the clip still cuts the image's left part away"
        );
    }
}
