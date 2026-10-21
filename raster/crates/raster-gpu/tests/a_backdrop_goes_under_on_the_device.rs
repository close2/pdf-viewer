//! `Target::ReadbackOnto` is the caller's three host passes over a `Readback` frame, to the
//! byte (the caller's `doc/adr/2150`).
//!
//! # What is held
//!
//! The caller put its medium under a read-back page on the processor: premultiply the
//! straight raster again, clear the pixels outside its crop, composite its opaque medium by
//! eight-bit source-over, demultiply. `onto.wgsl` does that on the device, and the claim is
//! that nothing about a page moves when it does. So each test draws one scene twice on one
//! device — once to `Readback`, whose straight bytes this file then puts through a
//! transcription of the caller's three passes, and once to `ReadbackOnto` — and asks for the
//! same bytes. The scenes are rotated and translucent so that the frame is mostly partial
//! alpha, which is the only place the arithmetic can disagree: an opaque pixel is itself
//! both ways, and an untouched one is the backdrop both ways.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Backdrop, Device, Target, Viewport};
use raster_scene::{Affine, Color, Point, Rect, Scene, SceneBuilder};

mod common;

use common::headless::{device, pixels, render};

const WIDTH: u32 = 61;
const HEIGHT: u32 = 37;

/// Three overlapping marks — two rotated, all translucent — so that most of the frame is
/// edge coverage under a partial alpha.
fn translucent_page() -> Scene {
    let mut builder = SceneBuilder::new();
    let turn = |angle: f32, x: f32, y: f32| Affine {
        a: angle.cos(),
        b: angle.sin(),
        c: -angle.sin(),
        d: angle.cos(),
        e: x,
        f: y,
    };
    for (rect, transform, colour) in [
        (
            Rect::new(Point::new(-20.0, -6.0), Point::new(20.0, 6.0)),
            turn(0.37, 30.3, 18.6),
            Color::new(0.9, 0.2, 0.1, 0.55),
        ),
        (
            Rect::new(Point::new(-9.0, -14.0), Point::new(9.0, 14.0)),
            turn(-0.81, 22.7, 15.2),
            Color::new(0.1, 0.4, 0.95, 0.3),
        ),
        (
            Rect::new(Point::new(40.25, 3.5), Point::new(58.75, 30.125)),
            Affine::IDENTITY,
            Color::new(0.2, 0.8, 0.3, 0.8),
        ),
    ] {
        builder
            .rect(rect, transform, colour, None, None)
            .expect("an admissible rectangle");
    }
    builder.finish()
}

/// The caller's three passes over a straight-alpha `Readback` frame, transcribed from
/// `render_raster`'s `premultiply` and `demultiply` and `pdf_render`'s `crop_to_page` and
/// `impose_on_medium`, in the order its `rasterize` runs them.
fn on_the_processor(straight: &[u8], backdrop: &Backdrop) -> Vec<u8> {
    let [red, green, blue] = backdrop.colour;
    let below = [red, green, blue, 255_u8];
    let [left, top, right, bottom] = backdrop.keep.unwrap_or([0, 0, WIDTH, HEIGHT]);
    let mut out = straight.to_vec();
    for (index, pixel) in (0_u32..).zip(out.chunks_exact_mut(4)) {
        let (x, y) = (index % WIDTH, index / WIDTH);
        let alpha = u16::from(pixel[3]);
        if alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = u8::try_from((u16::from(*channel) * alpha + 127) / 255).unwrap();
            }
        }
        if !((left..right).contains(&x) && (top..bottom).contains(&y)) {
            pixel.fill(0);
        }
        if pixel[3] == 255 {
            continue;
        }
        let clear = 255 - u16::from(pixel[3]);
        for (channel, under) in pixel.iter_mut().zip(below) {
            let sum = u16::from(*channel) + (u16::from(under) * clear + 127) / 255;
            *channel = u8::try_from(sum.min(255)).unwrap();
        }
        if pixel[3] != 0 && pixel[3] != 255 {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel[..3] {
                *channel = u8::try_from((u16::from(*channel) * 255 / alpha).min(255)).unwrap();
            }
        }
    }
    out
}

fn onto(device: &mut Device, scene: &Scene, backdrop: Backdrop) -> raster_gpu::Frame {
    device
        .render(
            scene,
            &Viewport::full(WIDTH, HEIGHT, Affine::IDENTITY),
            Target::ReadbackOnto(backdrop),
        )
        .expect("renders onto its backdrop")
}

/// How many pixels of `raster` are neither transparent nor opaque — the population the claim
/// is about, which a scene that drew nothing translucent would leave empty.
fn partial(raster: &[u8]) -> usize {
    raster
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0 && pixel[3] != 255)
        .count()
}

#[test]
fn a_page_on_white_is_the_processor_s_bytes() {
    let mut device = device();
    let scene = translucent_page();
    let straight = render(&mut device, &scene, WIDTH, HEIGHT);
    assert!(
        partial(&straight) > 300,
        "the scene is mostly partial alpha"
    );
    let backdrop = Backdrop {
        colour: [255, 255, 255],
        keep: None,
    };
    let drawn = pixels(onto(&mut device, &scene, backdrop));
    assert_eq!(drawn, on_the_processor(&straight, &backdrop));
    assert_eq!(
        partial(&drawn),
        0,
        "every pixel on an opaque backdrop is opaque"
    );
}

#[test]
fn a_coloured_backdrop_and_a_crop_are_the_processor_s_bytes() {
    let mut device = device();
    let scene = translucent_page();
    let straight = render(&mut device, &scene, WIDTH, HEIGHT);
    for backdrop in [
        Backdrop {
            colour: [200, 180, 40],
            keep: Some([5, 3, 50, 33]),
        },
        Backdrop {
            colour: [0, 0, 0],
            keep: Some([0, 0, 0, 0]),
        },
        Backdrop {
            colour: [17, 99, 254],
            keep: Some([30, 0, WIDTH, HEIGHT]),
        },
    ] {
        let drawn = pixels(onto(&mut device, &scene, backdrop));
        assert_eq!(
            drawn,
            on_the_processor(&straight, &backdrop),
            "{backdrop:?}"
        );
    }
}

/// The pipeline is compiled by the first frame that asks for a backdrop, and that frame says
/// so in its phases; the next one compiles nothing.
#[test]
fn the_first_frame_onto_a_backdrop_names_its_compile_and_the_second_does_not() {
    let mut device = device();
    let scene = translucent_page();
    let backdrop = Backdrop {
        colour: [255, 255, 255],
        keep: None,
    };
    let compiled = |frame: &raster_gpu::Frame| {
        frame
            .timings()
            .phases
            .iter()
            .any(|(name, _)| *name == "pipeline compile (first use)")
    };
    // A `Readback` frame first, so that every render pipeline the scene needs is compiled
    // and the only first use left is the backdrop's.
    render(&mut device, &scene, WIDTH, HEIGHT);
    assert!(compiled(&onto(&mut device, &scene, backdrop)));
    assert!(!compiled(&onto(&mut device, &scene, backdrop)));
}

/// A zero-size frame is still a frame, onto a backdrop as without one.
#[test]
fn a_zero_size_frame_onto_a_backdrop_is_empty() {
    let mut device = device();
    let frame = device
        .render(
            &translucent_page(),
            &Viewport::full(0, 0, Affine::IDENTITY),
            Target::ReadbackOnto(Backdrop {
                colour: [255, 255, 255],
                keep: None,
            }),
        )
        .expect("a zero-size readback is a legitimate frame");
    assert!(pixels(frame).is_empty());
    assert!(device.backdrop_fits(WIDTH, HEIGHT));
}
