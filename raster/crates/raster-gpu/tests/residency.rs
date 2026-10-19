//! What the device keeps of a page between frames is held to a figure it states, and a page past
//! it is drawn the same with its streams sent every frame (viewer ADR 2030).
//!
//! A retained encode's instance streams stay on the device after its first frame so that a replay
//! sends none of them again (viewer ADR 1993); `retained_replay.rs` holds that. This file holds
//! the budget they are charged against: the figure `Limits` reports, the charge given back when an
//! encode goes, and the refusal — which names its three numbers, is decided once for the encode,
//! and changes nothing a frame draws.

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

use raster_gpu::{Device, EncodeSource, Options, ResidencyRefused, RetainedScene, Target};
use raster_scene::{Affine, Color, LineCap, Point, Scene, SceneBuilder};

mod common;

use common::headless::pixels;
use common::retained::{device, device_with, retained_frame, viewport};

/// A page of the line lane's marks, with `offset` moving every one of them, so that two pages are
/// two encodes with streams of the same size.
fn line_page(offset: f32) -> Scene {
    let mut builder = SceneBuilder::new();
    for row in 0..8_u8 {
        for column in 0..8_u8 {
            let (x, y) = (
                offset + 8.0 + f32::from(column) * 24.0,
                8.0 + f32::from(row) * 20.0,
            );
            builder
                .line(
                    (Point::new(x, y), Point::new(x + 16.0, y + 6.0)),
                    Affine::IDENTITY,
                    (1.0 + f32::from(row % 3), false),
                    LineCap::Butt,
                    Color::new(0.1, 0.3, 0.6, 0.9),
                    None,
                    None,
                )
                .unwrap();
        }
    }
    builder.finish()
}

/// The bytes the device keeps of an encode whose first frame sent `uploads`.
fn instance_bytes(uploads: raster_gpu::UploadSplit) -> u64 {
    uploads.rect_instances + uploads.quad_instances + uploads.line_instances
}

/// By default the device keeps what it admits one frame, and states it; the streams are charged
/// while their encode is held and given back when it goes.
#[test]
fn the_default_budget_is_the_frame_ceiling_and_a_dropped_encode_gives_its_bytes_back() {
    let mut device = device();
    let limits = device.limits();
    assert_eq!(limits.max_resident_bytes, limits.max_frame_bytes);
    assert_eq!(device.resident_bytes(), 0);

    let mut first = RetainedScene::new(line_page(0.0));
    let sent = retained_frame(
        &mut device,
        &mut first,
        &viewport(),
        EncodeSource::Encoded,
        "first frame",
    )
    .counters()
    .uploads;
    let kept = instance_bytes(sent);
    assert!(kept > 0, "the page must have instance streams: {sent:?}");
    assert_eq!(first.resident_instance_bytes(), kept);
    assert_eq!(first.residency_refused(), None);
    assert_eq!(device.resident_bytes(), kept);

    let mut second = RetainedScene::new(line_page(3.0));
    retained_frame(
        &mut device,
        &mut second,
        &viewport(),
        EncodeSource::Encoded,
        "the second handle's first frame",
    );
    assert_eq!(
        device.resident_bytes(),
        kept + second.resident_instance_bytes()
    );

    first.forget();
    assert_eq!(device.resident_bytes(), second.resident_instance_bytes());
    drop(second);
    assert_eq!(device.resident_bytes(), 0);
}

/// A device asked to keep less than a page's streams draws the page byte for byte as it draws it
/// immediately, sends the streams on every frame including the replay, and says why once, with
/// the numbers.
#[test]
fn a_page_past_the_budget_is_drawn_the_same_and_sends_its_streams_every_frame() {
    let mut device = device_with(&Options {
        max_resident_bytes: 1,
        ..Options::default()
    });
    assert_eq!(device.limits().max_resident_bytes, 1);
    let scene = line_page(0.0);
    let immediate = pixels(
        device
            .render(&scene, &viewport(), Target::Readback)
            .expect("the frame must draw"),
    );

    let mut retained = RetainedScene::new(scene);
    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Encoded,
        "first frame",
    );
    let first_uploads = first.counters().uploads;
    let streams = instance_bytes(first_uploads);
    assert_eq!(
        retained.residency_refused(),
        Some(ResidencyRefused {
            needed: streams,
            in_use: 0,
            budget: 1,
        })
    );
    assert_eq!(retained.resident_instance_bytes(), 0);
    assert_eq!(device.resident_bytes(), 0, "a refusal holds nothing");
    assert_eq!(pixels(first), immediate);

    let replay = retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Replayed,
        "a refused encode still replays its host half",
    );
    assert_eq!(
        replay.counters().uploads.line_instances,
        first_uploads.line_instances,
        "the streams cross again on a replay"
    );
    assert_eq!(
        retained.residency_refused().map(|r| r.needed),
        Some(streams)
    );
    assert_eq!(pixels(replay), immediate);
}

/// The budget is the device's, across every handle drawn on it: a second page that would take the
/// device past it is refused with the first page's bytes as what was held, and once the first is
/// forgotten the second's next encode is kept.
#[test]
fn a_second_page_is_refused_by_what_the_first_holds_and_kept_once_it_is_given_back() {
    let mut probe = device();
    let mut sized = RetainedScene::new(line_page(0.0));
    let one = instance_bytes(
        retained_frame(
            &mut probe,
            &mut sized,
            &viewport(),
            EncodeSource::Encoded,
            "sizing frame",
        )
        .counters()
        .uploads,
    );
    drop(sized);
    drop(probe);

    // Room for one page's streams and not two.
    let mut device = device_with(&Options {
        max_resident_bytes: one + one / 2,
        ..Options::default()
    });
    let mut first = RetainedScene::new(line_page(0.0));
    let mut second = RetainedScene::new(line_page(3.0));
    retained_frame(
        &mut device,
        &mut first,
        &viewport(),
        EncodeSource::Encoded,
        "first page",
    );
    assert_eq!(first.resident_instance_bytes(), one);
    retained_frame(
        &mut device,
        &mut second,
        &viewport(),
        EncodeSource::Encoded,
        "second page",
    );
    let refused = second
        .residency_refused()
        .expect("the second page's streams would pass the budget");
    assert_eq!(refused.in_use, one);
    assert_eq!(refused.budget, one + one / 2);
    assert_eq!(device.resident_bytes(), one);

    first.forget();
    second.forget();
    retained_frame(
        &mut device,
        &mut second,
        &viewport(),
        EncodeSource::Encoded,
        "the second page's new encode",
    );
    assert_eq!(second.residency_refused(), None);
    assert_eq!(device.resident_bytes(), second.resident_instance_bytes());
}

/// A request above the frame's ceiling is held to it: what the device keeps is never more than
/// what it admits one frame, whatever the host asked for.
#[test]
fn a_request_above_the_frame_ceiling_is_held_to_it() {
    let device: Device = device_with(&Options {
        max_resident_bytes: u64::MAX,
        ..Options::default()
    });
    let limits = device.limits();
    assert_eq!(limits.max_resident_bytes, limits.max_frame_bytes);
}
