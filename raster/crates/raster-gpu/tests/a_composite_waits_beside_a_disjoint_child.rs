//! A child's composite waits past the next child, and the two are drawn in one pass, where
//! their rectangles stand apart and the frame's priced peak holds both (ADR 1631).
//!
//! Two things can go wrong, and each test holds one. The bytes: the later child's backdrop
//! is copied before the earlier composite is drawn, which is the same backdrop only where
//! the two rectangles meet nowhere — so every expected pixel is §11.3.6's closed form for
//! opaque marks under §11.3.5.2's `Multiply`, `cb × cs`, read on both children. And the
//! budget: a waiting composite keeps its child's texture and its copy alive while the next
//! child renders, which the frame budget priced only where a heavier sibling's price covers
//! them — so `Counters::layer_textures` says whether the wait happened, against a control
//! scene in which it cannot.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Device, Options, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, GroupSpec, Point, Rect, Scene, SceneBuilder,
};

const SIZE: u32 = 64;

fn device() -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

fn group(blend: BlendMode) -> GroupSpec {
    GroupSpec {
        alpha: 1.0,
        blend,
        clip: None,
        knockout: false,
        mask: None,
        isolated: true,
        compose: Compose::SrcOver,
    }
}

fn square(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new(Point::new(x0, y0), Point::new(x1, y1))
}

const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
const YELLOW: Color = Color::new(1.0, 1.0, 0.0, 1.0);
const CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);
const MAGENTA: Color = Color::new(1.0, 0.0, 1.0, 1.0);

/// A parent whose left half is red and right half yellow, then three `Multiply` children
/// in a row: a heavy cyan band across the top, a cyan square at the lower left and, at
/// `b`, a magenta square. Red × cyan is black, yellow × cyan green, yellow × magenta red.
fn scene(heavy: bool, b: Rect) -> Scene {
    let mut builder = SceneBuilder::new();
    builder
        .group(group(BlendMode::Normal), |parent| {
            parent.rect(
                square(0.0, 0.0, 32.0, 64.0),
                Affine::IDENTITY,
                RED,
                None,
                None,
            )?;
            parent.rect(
                square(32.0, 0.0, 64.0, 64.0),
                Affine::IDENTITY,
                YELLOW,
                None,
                None,
            )?;
            if heavy {
                parent.group(group(BlendMode::Multiply), |band| {
                    band.rect(
                        square(4.0, 4.0, 60.0, 30.0),
                        Affine::IDENTITY,
                        CYAN,
                        None,
                        None,
                    )
                })?;
            }
            parent.group(group(BlendMode::Multiply), |a| {
                a.rect(
                    square(8.0, 40.0, 20.0, 52.0),
                    Affine::IDENTITY,
                    CYAN,
                    None,
                    None,
                )
            })?;
            parent.group(group(BlendMode::Multiply), |second| {
                second.rect(b, Affine::IDENTITY, MAGENTA, None, None)
            })
        })
        .unwrap();
    builder.finish()
}

fn render(device: &mut Device, scene: &Scene) -> (Vec<u8>, u32) {
    let frame = device
        .render(
            scene,
            &Viewport::full(SIZE, SIZE, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders");
    let textures = frame.counters().layer_textures;
    (frame.into_raster().unwrap().into_pixels(), textures)
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().unwrap()
}

/// The magenta square apart from the cyan one, at the lower right.
fn apart() -> Rect {
    square(44.0, 40.0, 56.0, 52.0)
}

/// The magenta square over the cyan one's right edge, so that its backdrop holds the cyan
/// composite and it may not be copied before that composite is drawn.
fn over_a() -> Rect {
    square(16.0, 44.0, 28.0, 56.0)
}

/// **Where the two squares stand apart and a heavier sibling's price covers both, the
/// cyan square's composite waits for the magenta one's**: two more textures are alive at
/// once than where the magenta square covers the cyan one's edge and cannot wait — the
/// cyan square and its copy, held while the magenta one renders — and every pixel is the
/// closed form either way.
#[test]
fn a_composite_waits_beside_a_child_whose_rectangle_it_does_not_meet() {
    let mut device = device();
    let (waited, waited_textures) = render(&mut device, &scene(true, apart()));
    let (control, control_textures) = render(&mut device, &scene(true, over_a()));
    for (pixels, what) in [(&waited, "apart"), (&control, "over the cyan square")] {
        assert_eq!(
            pixel(pixels, 10, 10),
            [0, 0, 0, 255],
            "{what}: red × cyan, the band"
        );
        assert_eq!(
            pixel(pixels, 50, 10),
            [0, 255, 0, 255],
            "{what}: yellow × cyan, the band"
        );
        assert_eq!(
            pixel(pixels, 10, 45),
            [0, 0, 0, 255],
            "{what}: red × cyan, the square"
        );
        assert_eq!(
            pixel(pixels, 30, 35),
            [255, 0, 0, 255],
            "{what}: the parent alone"
        );
    }
    assert_eq!(pixel(&waited, 50, 45), [255, 0, 0, 255], "yellow × magenta");
    assert_eq!(pixel(&waited, 20, 45), [255, 0, 0, 255], "the parent alone");
    // Over the cyan square the magenta one multiplies black, and past it red.
    assert_eq!(pixel(&control, 18, 46), [0, 0, 0, 255], "black × magenta");
    assert_eq!(pixel(&control, 24, 46), [255, 0, 0, 255], "red × magenta");
    assert_eq!(
        waited_textures,
        control_textures + 2,
        "the cyan square and its copy, alive beside the magenta square"
    );
}

/// **Without a heavier sibling nothing waits**, because holding the cyan square beside the
/// magenta one would take the frame past the peak it was priced at, which is the heavier of
/// the two children alone — so the squares standing apart cost what they cost touching.
#[test]
fn nothing_waits_where_the_frames_price_does_not_hold_both() {
    let mut device = device();
    let (apart_pixels, apart_textures) = render(&mut device, &scene(false, apart()));
    let (_, touching_textures) = render(&mut device, &scene(false, over_a()));
    assert_eq!(
        apart_textures, touching_textures,
        "the priced peak holds one child"
    );
    assert_eq!(pixel(&apart_pixels, 10, 45), [0, 0, 0, 255], "red × cyan");
    assert_eq!(
        pixel(&apart_pixels, 50, 45),
        [255, 0, 0, 255],
        "yellow × magenta"
    );
}
