//! An image's texture filled only where a frame samples it draws the frame a whole texture
//! draws (ADR 1493).
//!
//! The device writes an image's texture — its own samples or an area-averaged reduction — in
//! the squares a frame's image ops can read, widened by the linear filter's neighbour, and
//! leaves the rest unwritten until a later frame reads it. A texel the shader reads and the
//! fill missed would be zero, so a missed margin is a darker or transparent edge. Each view
//! here is drawn twice: on a device that has drawn nothing before it, which fills only what
//! the view samples, and on one that first drew the whole image at the same magnification,
//! which filled every texel. The two frames are the same to the byte. The views cut the image
//! at fractional offsets, under both filters, magnified, at native size and reduced; a second
//! view scrolled from the first is drawn on the first device too, so the strip it uncovers is
//! the only fill that frame makes.

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

mod common;

use std::sync::Arc;

use raster_gpu::{Device, Options, Target, Viewport};
use raster_scene::{Affine, BlendMode, ImageFilter, ImageSpec, SceneBuilder};

/// Wide enough for several fill squares on each side, and not a multiple of one.
const SIDE: u32 = 1030;

/// The target every view is drawn into: a small window onto a large image.
const WINDOW: (u32, u32) = (300, 200);

fn device() -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        encode_threads: 4,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

/// Samples that vary in every channel, alpha included, so a premultiplied texel differs from
/// its neighbours and from zero.
fn image() -> ImageSpec {
    let data: Vec<u8> = (0..SIDE * SIDE * 4)
        .map(|index| (index.wrapping_mul(2_654_435_761) >> 24) as u8 | 1)
        .collect();
    ImageSpec {
        width: SIDE,
        height: SIDE,
        data: Arc::from(data),
    }
}

/// The image drawn over `extent` × `extent` scene units at the origin, seen through a
/// `width` × `height` window whose top-left corner is at `(x, y)` in scene space.
fn view(
    device: &mut Device,
    id: raster_scene::ImageId,
    extent: f32,
    interpolate: bool,
    (x, y): (f32, f32),
    (width, height): (u32, u32),
) -> Vec<u8> {
    let mut builder = SceneBuilder::new();
    builder
        .image(
            id,
            Affine {
                a: extent,
                b: 0.0,
                c: 0.0,
                d: extent,
                e: 0.0,
                f: 0.0,
            },
            1.0,
            ImageFilter::Auto { interpolate },
            None,
            BlendMode::Normal,
            None,
        )
        .unwrap();
    let scene = builder.finish();
    let scroll = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: -x,
        f: -y,
    };
    common::headless::pixels(
        device
            .render(
                &scene,
                &Viewport::full(width, height, scroll),
                Target::Readback,
            )
            .expect("renders"),
    )
}

/// How many bytes of two frames differ, and whether they are the same length.
fn difference(one: &[u8], other: &[u8]) -> (bool, usize) {
    (
        one.len() == other.len(),
        one.iter().zip(other).filter(|(a, b)| a != b).count(),
    )
}

/// Each view of the image at `extent`, drawn on a device that fills only what it samples and
/// on one that filled every texel first.
fn assert_views_agree(extent: f32, interpolate: bool) {
    let corners = [
        (412.25, 333.5),
        (0.5, 0.0),
        (extent - 150.75, extent - 120.25),
    ];
    let scrolled = |(x, y): (f32, f32)| (x + 37.5, y + 81.25);
    let mut partial = device();
    let partial_id = partial.upload_image(&image()).unwrap();
    let mut whole = device();
    let whole_id = whole.upload_image(&image()).unwrap();
    // The whole image in one frame, at the magnification every view below uses, so that every
    // texel of the texture those views bind is written before any of them is drawn.
    let side = extent.ceil() as u32;
    view(
        &mut whole,
        whole_id,
        extent,
        interpolate,
        (0.0, 0.0),
        (side, side),
    );
    for corner in corners {
        for at in [corner, scrolled(corner)] {
            let filled_here = view(&mut partial, partial_id, extent, interpolate, at, WINDOW);
            let filled_first = view(&mut whole, whole_id, extent, interpolate, at, WINDOW);
            assert_eq!(
                difference(&filled_here, &filled_first),
                (true, 0),
                "extent {extent}, interpolate {interpolate}, window at {at:?}"
            );
        }
    }
}

#[test]
fn a_reduced_image_filled_where_sampled_draws_the_whole_textures_frame() {
    for interpolate in [false, true] {
        assert_views_agree(SIDE as f32 / 3.0, interpolate);
    }
}

#[test]
fn an_image_at_its_own_size_filled_where_sampled_draws_the_whole_textures_frame() {
    for interpolate in [false, true] {
        assert_views_agree(SIDE as f32, interpolate);
    }
}

#[test]
fn a_magnified_image_filled_where_sampled_draws_the_whole_textures_frame() {
    for interpolate in [false, true] {
        assert_views_agree(SIDE as f32 * 2.5, interpolate);
    }
}
