//! An image's area-averaged reduction divided among the host's threads draws the frame one
//! thread draws.
//!
//! The reduction's output rows are averaged by up to [`Options::encode_threads`] threads (the
//! caller's ADR 1433), which is claimed to move no byte: each output row is a function of its own
//! band of source rows. The image here is over the reduction's parallel floor, with a row that
//! is not a multiple of any band and alphas that vary, so the premultiplied sums and the
//! straight-alpha readback are in the comparison. It is minified threefold on a one-thread
//! device and on a four-thread one, and drawn at its own size as the control, which reduces
//! nothing.

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

/// Over the reduction's floor of 65 536 samples, and not a multiple of the rows one thread takes
/// at a time.
const SIDE: u32 = 1030;

fn device(threads: usize) -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        encode_threads: threads,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

fn image() -> ImageSpec {
    let data: Vec<u8> = (0..SIDE * SIDE * 4)
        .map(|index| (index.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    ImageSpec {
        width: SIDE,
        height: SIDE,
        data: Arc::from(data),
    }
}

/// The image drawn over `extent` device pixels in a square target of that size.
fn drawn(threads: usize, extent: u32) -> Vec<u8> {
    let mut device = device(threads);
    let id = device.upload_image(&image()).unwrap();
    let mut builder = SceneBuilder::new();
    builder
        .image(
            id,
            Affine {
                a: extent as f32,
                b: 0.0,
                c: 0.0,
                d: extent as f32,
                e: 0.0,
                f: 0.0,
            },
            1.0,
            ImageFilter::Auto { interpolate: false },
            None,
            BlendMode::Normal,
            None,
        )
        .unwrap();
    let scene = builder.finish();
    common::headless::pixels(
        device
            .render(
                &scene,
                &Viewport::full(extent, extent, Affine::IDENTITY),
                Target::Readback,
            )
            .expect("renders"),
    )
}

/// How many bytes of two frames differ, and whether they are the same length — a count
/// rather than the frames, which are megabytes each.
fn difference(one: &[u8], other: &[u8]) -> (bool, usize) {
    (
        one.len() == other.len(),
        one.iter().zip(other).filter(|(a, b)| a != b).count(),
    )
}

#[test]
fn an_unreduced_image_draws_the_same_on_either_device() {
    assert_eq!(difference(&drawn(1, SIDE), &drawn(4, SIDE)), (true, 0));
}

#[test]
fn a_reduction_on_four_threads_draws_the_one_thread_frame() {
    assert_eq!(
        difference(&drawn(1, SIDE / 3), &drawn(4, SIDE / 3)),
        (true, 0)
    );
}
