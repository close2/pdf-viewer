//! ISO 32000-2 §11.7.4.3's special overprinting blend mode: `Compose::DestOver` and
//! `Compose::DestOverIn`, measured against the clause's own closed form (`doc/adr/1295`).
//!
//! The clause decides the blend function per component and no document names it:
//!
//! > If the overprint mode is 1 (nonzero overprint mode) and the current colour space and
//! > group colour space are both DeviceCMYK , then process colour components with nonzero
//! > values shall replace the corresponding component values of the backdrop; components
//! > with zero values leave the existing backdrop value unchanged.
//!
//! So `B(Cb, Cs)` is `Cb` in a channel the mark leaves alone and `Cs` in one it replaces.
//! The expectation every test here is written against is §11.3.3's basic compositing
//! formula with §11.3.7.3's union alpha, in premultiplied form, with that `B` substituted
//! and nothing else assumed:
//!
//! ```text
//! cr = (1 − αs)·cb + (1 − αb)·cs + αs·αb·B(Cb, Cs)
//! αr = αb + αs − αb·αs
//! ```
//!
//! Every quantity in it is stated by the test — the backdrop's colour and alpha, the mark's
//! colour and alpha — except the mark's **coverage**, which is geometry and not the clause's
//! business; it is read off the same wedge drawn opaque white onto transparency, so that
//! `αs = alpha × coverage` at every pixel, partially covered ones included.
//!
//! The backdrop is deliberately **not opaque**. Onto an opaque backdrop destination-over
//! leaves the backdrop exactly as it was, which is the overprinting story and also a case
//! a wrong operator can pass by writing nothing at all; at `αb = 0.6` the source shows
//! through in proportion `1 − αb`, and only the clause's arithmetic lands on it.
//!
//! This file was written from the two clauses and the table alone; the caller's CPU backend
//! is the second reading, and the first time the two meet is the caller's cross-backend run.

// Test-file lint policy as in m1.rs; the reference math mirrors clause arithmetic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    // Pixel indexing and the clause's own arithmetic, over rasters this file just drew.
    clippy::arithmetic_side_effects
)]

use std::sync::Arc;

use raster_gpu::Device;
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, GroupSpec, ImageFilter, ImageSpec, OutlineId,
    OverprintComposeReason, Paint, Point, Rect, SceneBuilder, SceneError, Segment,
};

mod common;

use common::clause::premul;
use common::headless::{device, render};

const SIZE: u32 = 64;

/// The backdrop's straight colour and alpha: every channel different, and translucent.
const BACKDROP: Color = Color::new(0.9, 0.2, 0.5, 0.6);
/// The mark's straight colour and alpha: every channel different from the backdrop's.
const MARK: Color = Color::new(0.1, 0.7, 0.9, 0.8);

/// Premultiplied units of 255 a readback may stand from the closed form: the eight-bit
/// store of a straight colour, converted back, is within one step of the truth, and the
/// premultiplication doubles that at worst.
const TOLERANCE: f32 = 2.0;

/// A triangle with a diagonal edge, so that partially covered pixels exist.
fn wedge(device: &mut Device) -> OutlineId {
    device
        .upload_outline(&[
            Segment::MoveTo(Point::new(8.0, 8.0)),
            Segment::LineTo(Point::new(56.0, 8.0)),
            Segment::LineTo(Point::new(8.0, 56.0)),
            Segment::Close,
        ])
        .unwrap()
}

fn fill(
    builder: &mut SceneBuilder,
    outline: OutlineId,
    colour: Color,
    blend: BlendMode,
    compose: Compose,
) -> Result<(), SceneError> {
    builder.fill(
        outline,
        Affine::IDENTITY,
        FillRule::NonZero,
        Paint::Solid(colour),
        None,
        blend,
        compose,
        None,
    )
}

fn backdrop(builder: &mut SceneBuilder) {
    builder
        .rect(
            Rect::new(Point::new(0.0, 0.0), Point::new(SIZE as f32, SIZE as f32)),
            Affine::IDENTITY,
            BACKDROP,
            None,
            None,
        )
        .unwrap();
}

/// An isolated group of one element at alpha 1 under no mask or clip, composited under
/// `compose` — how a caller states §11.7.4.3's mode for a mark with no operator of its own.
fn one_element_group(compose: Compose) -> GroupSpec {
    GroupSpec {
        alpha: 1.0,
        blend: BlendMode::Normal,
        clip: None,
        knockout: false,
        mask: None,
        isolated: true,
        compose,
    }
}

/// The wedge's coverage per pixel, read from the wedge drawn opaque white onto
/// transparency.
fn coverage(device: &mut Device, outline: OutlineId) -> Vec<f32> {
    let mut builder = SceneBuilder::new();
    fill(
        &mut builder,
        outline,
        Color::new(1.0, 1.0, 1.0, 1.0),
        BlendMode::Normal,
        Compose::SrcOver,
    )
    .unwrap();
    render(device, &builder.finish(), SIZE, SIZE)
        .chunks_exact(4)
        .map(|pixel| f32::from(pixel[3]) / 255.0)
        .collect()
}

/// The premultiplied pixel §11.3.3 gives under §11.7.4.3's selection, in units of 255:
/// `kept[i]` is whether channel `i` takes `B = Cb`.
fn clause(kept: [bool; 3], coverage: f32) -> [f32; 4] {
    let (ab, as_) = (BACKDROP.a, MARK.a * coverage);
    let cb = [BACKDROP.r, BACKDROP.g, BACKDROP.b];
    let cs = [MARK.r, MARK.g, MARK.b];
    let mut out = [0.0; 4];
    for channel in 0..3 {
        let b = if kept[channel] {
            cb[channel]
        } else {
            cs[channel]
        };
        out[channel] = 255.0
            * ((1.0 - as_) * ab * cb[channel] + (1.0 - ab) * as_ * cs[channel] + as_ * ab * b);
    }
    out[3] = 255.0 * (ab + as_ - ab * as_);
    out
}

/// Worst deviation of `actual` from [`clause`] over every pixel, and how many of those
/// pixels are partially covered.
fn deviation(actual: &[u8], kept: [bool; 3], coverage: &[f32]) -> (f32, u32) {
    let (mut worst, mut partial) = (0.0_f32, 0_u32);
    for (pixel, &f) in coverage.iter().enumerate() {
        if f > 0.0 && f < 1.0 {
            partial += 1;
        }
        let at = pixel * 4;
        let expected = clause(kept, f);
        for (channel, &want) in expected.iter().enumerate().take(3) {
            worst = worst.max((premul(actual, at, channel) - want).abs());
        }
        worst = worst.max((f32::from(actual[at + 3]) - expected[3]).abs());
    }
    (worst, partial)
}

/// A fill under `compose` over the translucent backdrop.
fn over_backdrop(device: &mut Device, outline: OutlineId, compose: Compose) -> Vec<u8> {
    let mut builder = SceneBuilder::new();
    backdrop(&mut builder);
    fill(&mut builder, outline, MARK, BlendMode::Normal, compose).unwrap();
    render(device, &builder.finish(), SIZE, SIZE)
}

/// Keeping every channel is §11.3.3's formula with `B = Cb` everywhere, which is Porter-Duff
/// destination-over — and the fixed-function blend state states it.
#[test]
fn dest_over_is_the_clause_with_every_channel_kept() {
    let mut device = device();
    let outline = wedge(&mut device);
    let coverage = coverage(&mut device, outline);
    let drawn = over_backdrop(&mut device, outline, Compose::DestOver);
    let (worst, partial) = deviation(&drawn, [true; 3], &coverage);
    eprintln!("destination-over: worst {worst:.2} over {partial} partial pixels");
    assert!(
        partial > 30,
        "the wedge must have partially covered pixels: {partial}"
    );
    assert!(
        worst <= TOLERANCE,
        "§11.7.4.3 with every channel kept; worst premultiplied deviation {worst}"
    );

    // And it is not source-over, by the margin the translucent backdrop makes.
    let over = over_backdrop(&mut device, outline, Compose::SrcOver);
    let (from_over, _) = deviation(&over, [true; 3], &coverage);
    assert!(
        from_over >= 30.0,
        "source-over must miss the kept channels' clause by far more than the tolerance: \
         {from_over}"
    );
}

/// Every proper subset of the channels: kept channels are destination-over, the rest
/// source-over, and the alpha is the union in all of them.
#[test]
fn dest_over_in_selects_the_blend_function_per_channel() {
    let mut device = device();
    let outline = wedge(&mut device);
    let coverage = coverage(&mut device, outline);
    for bits in 1_u8..7 {
        let kept = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
        let drawn = over_backdrop(&mut device, outline, Compose::DestOverIn(kept));
        let (worst, partial) = deviation(&drawn, kept, &coverage);
        eprintln!("kept {kept:?}: worst {worst:.2} over {partial} partial pixels");
        assert!(partial > 30);
        assert!(
            worst <= TOLERANCE,
            "§11.7.4.3 keeping {kept:?}; worst premultiplied deviation {worst}"
        );
    }
}

/// The two uniform selections through the per-channel operator are the operators they
/// collapse to: the layer's §11.3.6 composite and the lanes' blend states are two
/// derivations of one clause, and they meet within the readback's step.
#[test]
fn the_two_derivations_meet_on_the_uniform_selections() {
    let mut device = device();
    let outline = wedge(&mut device);
    let coverage = coverage(&mut device, outline);
    for (kept, lane) in [
        ([true; 3], Compose::DestOver),
        ([false; 3], Compose::SrcOver),
    ] {
        let through_layer = over_backdrop(&mut device, outline, Compose::DestOverIn(kept));
        let through_lane = over_backdrop(&mut device, outline, lane);
        let (layer_worst, _) = deviation(&through_layer, kept, &coverage);
        let (lane_worst, _) = deviation(&through_lane, kept, &coverage);
        assert!(
            layer_worst <= TOLERANCE && lane_worst <= TOLERANCE,
            "kept {kept:?}: layer {layer_worst}, lane {lane_worst}"
        );
    }
}

/// A group of one element composited under the mode is the element composited under it:
/// the route a caller takes for a mark with no operator of its own. Stated here with a fill
/// inside, so the expectation is the same closed form, and with an image inside, which is
/// the mark that route exists for.
#[test]
fn a_one_element_group_carries_the_mode() {
    let mut device = device();
    let outline = wedge(&mut device);
    let coverage = coverage(&mut device, outline);
    for kept in [[true; 3], [true, false, true]] {
        let mut builder = SceneBuilder::new();
        backdrop(&mut builder);
        builder
            .group(one_element_group(Compose::keeping(kept)), |body| {
                fill(body, outline, MARK, BlendMode::Normal, Compose::SrcOver)
            })
            .unwrap();
        let drawn = render(&mut device, &builder.finish(), SIZE, SIZE);
        let (worst, _) = deviation(&drawn, kept, &coverage);
        assert!(
            worst <= TOLERANCE,
            "a group of one under {kept:?}; worst premultiplied deviation {worst}"
        );
    }

    // One opaque image sample of the mark's straight colour at the mark's alpha as the
    // command's constant, over the whole target: coverage 1 everywhere.
    let byte = |v: f32| (v * 255.0).round() as u8;
    let texel = [byte(MARK.r), byte(MARK.g), byte(MARK.b), 255];
    let image = device
        .upload_image(&ImageSpec {
            width: 1,
            height: 1,
            data: Arc::from(texel.as_slice()),
        })
        .unwrap();
    let kept = [false, true, false];
    let mut builder = SceneBuilder::new();
    backdrop(&mut builder);
    builder
        .group(one_element_group(Compose::keeping(kept)), |body| {
            body.image(
                image,
                Affine::scale(SIZE as f32, SIZE as f32),
                MARK.a,
                ImageFilter::Nearest,
                None,
                BlendMode::Normal,
                None,
            )
        })
        .unwrap();
    let drawn = render(&mut device, &builder.finish(), SIZE, SIZE);
    let everywhere = vec![1.0; (SIZE * SIZE) as usize];
    let (worst, _) = deviation(&drawn, kept, &everywhere);
    // The image's colour is stored in eight bits before it is composited, so the mark's
    // colour is itself one step from `MARK`: one more step of tolerance.
    assert!(
        worst <= TOLERANCE + 1.0,
        "an image in a group of one under {kept:?}; worst premultiplied deviation {worst}"
    );
}

/// Onto an opaque backdrop a kept channel is the backdrop exactly — the overprinting
/// behaviour §11.7.4.3 exists to be compatible with: "components with zero values leave the
/// existing backdrop value unchanged".
#[test]
fn onto_an_opaque_backdrop_a_kept_channel_is_the_backdrop() {
    let mut device = device();
    let outline = wedge(&mut device);
    let opaque = Color::new(0.9, 0.2, 0.5, 1.0);
    let mut plain = SceneBuilder::new();
    plain
        .rect(
            Rect::new(Point::new(0.0, 0.0), Point::new(SIZE as f32, SIZE as f32)),
            Affine::IDENTITY,
            opaque,
            None,
            None,
        )
        .unwrap();
    let before = render(&mut device, &plain.finish(), SIZE, SIZE);
    for kept in [[true; 3], [true, false, false]] {
        let mut builder = SceneBuilder::new();
        builder
            .rect(
                Rect::new(Point::new(0.0, 0.0), Point::new(SIZE as f32, SIZE as f32)),
                Affine::IDENTITY,
                opaque,
                None,
                None,
            )
            .unwrap();
        fill(
            &mut builder,
            outline,
            MARK,
            BlendMode::Normal,
            Compose::keeping(kept),
        )
        .unwrap();
        let after = render(&mut device, &builder.finish(), SIZE, SIZE);
        for (b, a) in before.chunks_exact(4).zip(after.chunks_exact(4)) {
            for channel in 0..3 {
                if kept[channel] {
                    assert!(
                        b[channel].abs_diff(a[channel]) <= 1,
                        "kept {kept:?}: channel {channel} moved from {} to {}",
                        b[channel],
                        a[channel]
                    );
                }
            }
            assert_eq!(a[3], 255, "the union with an opaque backdrop is opaque");
        }
    }
}

/// A *mark* under the mode inside a knockout group composites with the group's transparent
/// initial backdrop, and §11.3.6 gives every blend function Normal's arithmetic there — "An
/// alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect" — so it draws exactly
/// what the same mark under source-over draws in that position.
#[test]
fn inside_a_knockout_group_a_mark_under_the_mode_is_normal() {
    let mut device = device();
    let outline = wedge(&mut device);
    let knockout = GroupSpec {
        knockout: true,
        ..one_element_group(Compose::SrcOver)
    };
    let draw = |device: &mut Device, compose: Compose| {
        let mut builder = SceneBuilder::new();
        backdrop(&mut builder);
        builder
            .group(knockout, |body| {
                fill(body, outline, BACKDROP, BlendMode::Normal, Compose::SrcOver)?;
                fill(body, outline, MARK, BlendMode::Normal, compose)
            })
            .unwrap();
        render(device, &builder.finish(), SIZE, SIZE)
    };
    let normal = draw(&mut device, Compose::SrcOver);
    for compose in [Compose::DestOver, Compose::DestOverIn([true, false, true])] {
        assert_eq!(
            draw(&mut device, compose),
            normal,
            "{compose:?} in a knockout group is Normal's arithmetic"
        );
    }
}

/// The positions where the mode would meet a second compositing rule refuse it by name
/// rather than draw one of the two.
#[test]
fn the_mode_is_refused_where_it_meets_a_second_rule() {
    let mut device = device();
    let outline = wedge(&mut device);

    let mut builder = SceneBuilder::new();
    let error = fill(
        &mut builder,
        outline,
        MARK,
        BlendMode::Multiply,
        Compose::DestOver,
    )
    .expect_err("§11.7.4.3 puts the document's own mode on an implicit group");
    assert_eq!(
        error,
        SceneError::OverprintComposeUnsupported {
            compose: Compose::DestOver,
            reason: OverprintComposeReason::BlendNotNormal,
        }
    );

    // A group under the mode as an element of a knockout group: its §11.4.6 shape is the
    // union of its elements', which a finished layer does not carry apart from its opacity.
    let mut builder = SceneBuilder::new();
    let error = builder
        .group(
            GroupSpec {
                knockout: true,
                ..one_element_group(Compose::SrcOver)
            },
            |body| {
                body.group(one_element_group(Compose::DestOver), |inner| {
                    fill(inner, outline, MARK, BlendMode::Normal, Compose::SrcOver)
                })
            },
        )
        .expect_err("§11.4.6 weights a group element by a shape no layer carries");
    assert_eq!(
        error,
        SceneError::OverprintComposeUnsupported {
            compose: Compose::DestOver,
            reason: OverprintComposeReason::KnockoutElement,
        }
    );

    let mut builder = SceneBuilder::new();
    let error = builder
        .group(
            GroupSpec {
                isolated: false,
                ..one_element_group(Compose::DestOver)
            },
            |body| fill(body, outline, MARK, BlendMode::Normal, Compose::SrcOver),
        )
        .expect_err("§11.4.4 seeds a non-isolated group with its own backdrop");
    assert_eq!(
        error,
        SceneError::OverprintComposeUnsupported {
            compose: Compose::DestOver,
            reason: OverprintComposeReason::NonIsolated,
        }
    );
}
