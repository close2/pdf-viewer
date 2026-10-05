//! A whole coverage tile and a chain's region one render made are handed to the next render
//! that asks for them under the same words, and the bytes are the bytes a fresh device computes
//! (ADR 1529).
//!
//! # Where the expected values come from
//!
//! ISO 32000-2 §10.7.4:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the
//! > set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! Both sets are functions of geometry alone: a mark's of its outline under its transform and
//! its rule (§8.5.3.3), a clip's of its chain's paths (§8.5.4). The colour a mark is painted
//! in is not among them. So the page this exists for — a four-component group drawn as two
//! frames of the same elements in two different sets of components (ADR 1471) — asks the
//! second frame for exactly the coverage the first one made, and every expected value here is
//! the frame a fresh device draws for the same scene: the memo may change what a render costs
//! and nothing it draws. The second and third cases change one input the coverage *does*
//! depend on, the rule and the chain, and must then draw what a fresh device draws for that.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Counters, Device, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, GroupSpec, Paint, Scene, SceneBuilder,
};

mod common;

use common::meet::{ARMS, clip_polygon, device_with, polygon_path, residue_clip};

/// The target: the 64-gon's box and a margin.
const SIZE: (u32, u32) = (20, 22);

/// A five-pointed star of circumradius 2.2 about `(x, y)`, drawn as one self-crossing
/// pentagram, so that §8.5.3.3's two rules fill different sets: the centre winds twice.
fn star((x, y): (f64, f64)) -> Vec<(f64, f64)> {
    (0..5)
        .map(|i| {
            let angle =
                std::f64::consts::TAU * f64::from(i * 2) / 5.0 - std::f64::consts::FRAC_PI_2;
            (x + 2.2 * angle.cos(), y + 2.2 * angle.sin())
        })
        .collect()
}

/// Stars on a 4.1 × 3.7 lattice, some under the 64-gon's edge and some inside it.
fn stars() -> Vec<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    for i in 0..4 {
        for j in 0..5 {
            out.push(star((2.3 + 4.1 * f64::from(i), 2.1 + 3.7 * f64::from(j))));
        }
    }
    out
}

/// One interpretation of the elements: the stars painted in `ink`, every outline uploaded for
/// this scene alone — half of them clipped by the 64-gon shifted by `shift` on their own, and
/// all of them inside a group clipped by the same 64-gon, whose region the frame fills once.
fn scene(device: &mut Device, ink: Color, rule: FillRule, shift: f64) -> Scene {
    let mut builder = SceneBuilder::new();
    let clip_points: Vec<(f64, f64)> = clip_polygon()
        .into_iter()
        .map(|(x, y)| (x + shift, y))
        .collect();
    let clip = residue_clip(device, &mut builder, &polygon_path(&clip_points));
    let outlines: Vec<_> = stars()
        .iter()
        .map(|points| device.upload_outline(&polygon_path(points)).unwrap())
        .collect();
    let spec = GroupSpec {
        alpha: 1.0,
        blend: BlendMode::Normal,
        clip: Some(clip),
        knockout: false,
        mask: None,
        compose: Compose::SrcOver,
        isolated: true,
    };
    builder
        .group(spec, |body| {
            for (index, &outline) in outlines.iter().enumerate() {
                body.fill(
                    outline,
                    Affine::IDENTITY,
                    rule,
                    Paint::Solid(ink),
                    (index % 2 == 0).then_some(clip),
                    BlendMode::Normal,
                    Compose::SrcOver,
                    None,
                )?;
            }
            Ok(())
        })
        .unwrap();
    builder.finish()
}

fn render(device: &mut Device, scene: &Scene) -> (Vec<u8>, Counters) {
    let frame = device
        .render(
            scene,
            &Viewport::full(SIZE.0, SIZE.1, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders");
    let counters = frame.counters();
    (frame.into_raster().unwrap().into_pixels(), counters)
}

/// The chromatic half's components and the black half's, as ADR 1471's two frames carry them.
fn halves() -> (Color, Color) {
    (
        Color::new(0.75, 0.25, 0.5, 1.0),
        Color::new(0.3, 0.3, 0.3, 1.0),
    )
}

/// What a fresh device draws for the scene `make` builds, with the counters it reports.
fn fresh(
    arm: (raster_gpu::Coverage, usize),
    make: impl Fn(&mut Device) -> Scene,
) -> (Vec<u8>, Counters) {
    let mut device = device_with(arm.0, arm.1);
    let scene = make(&mut device);
    render(&mut device, &scene)
}

/// **The second frame of the same elements in other components draws what a fresh device
/// draws, and rasterises no residue for them**: the group's region is handed over and every
/// clipped star is handed its tile, on the walk's lane and the fan-out's at one thread and four.
#[test]
fn the_same_elements_in_other_components_are_drawn_from_the_render_before() {
    let (chromatic, black) = halves();
    for arm in ARMS {
        let (expected, alone) = fresh(arm, |d| scene(d, black, FillRule::NonZero, 0.0));
        assert!(
            alone.clip_residue_regions > 0,
            "{arm:?}: a fresh device fills the group's region"
        );
        let mut device = device_with(arm.0, arm.1);
        let first = scene(&mut device, chromatic, FillRule::NonZero, 0.0);
        let _ = render(&mut device, &first);
        let second = scene(&mut device, black, FillRule::NonZero, 0.0);
        let (drawn, kept) = render(&mut device, &second);
        assert_eq!(drawn, expected, "{arm:?}: the kept answers are the bytes");
        assert_eq!(
            (kept.clip_residue_regions, kept.clip_residue_tiles),
            (0, 0),
            "{arm:?}: nothing of the chain was rasterised again"
        );
    }
}

/// **A mark under the other rule is filled afresh**: the pentagram's centre winds twice, so
/// even-odd leaves it out where non-zero fills it, and the render after a non-zero one must
/// draw what a fresh device draws for even-odd.
#[test]
fn a_mark_under_the_other_rule_is_filled_afresh() {
    let (chromatic, black) = halves();
    for arm in ARMS {
        let (expected, _) = fresh(arm, |d| scene(d, black, FillRule::EvenOdd, 0.0));
        let (other, _) = fresh(arm, |d| scene(d, black, FillRule::NonZero, 0.0));
        assert_ne!(expected, other, "the two rules fill different sets here");
        let mut device = device_with(arm.0, arm.1);
        let first = scene(&mut device, chromatic, FillRule::NonZero, 0.0);
        let _ = render(&mut device, &first);
        let second = scene(&mut device, black, FillRule::EvenOdd, 0.0);
        assert_eq!(render(&mut device, &second).0, expected, "{arm:?}");
    }
}

/// **A mark under another chain is met afresh, and the chain's region filled afresh**: the
/// 64-gon a third of a pixel across meets every star it cuts in other pixels.
#[test]
fn a_mark_under_another_chain_is_met_afresh() {
    let (chromatic, black) = halves();
    for arm in ARMS {
        let (expected, _) = fresh(arm, |d| scene(d, black, FillRule::NonZero, 0.33));
        let mut device = device_with(arm.0, arm.1);
        let first = scene(&mut device, chromatic, FillRule::NonZero, 0.0);
        let _ = render(&mut device, &first);
        let second = scene(&mut device, black, FillRule::NonZero, 0.33);
        assert_eq!(render(&mut device, &second).0, expected, "{arm:?}");
    }
}
