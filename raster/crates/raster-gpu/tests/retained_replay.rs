//! A replayed frame is the frame that was encoded — in its pixels, in its counters, and
//! in what it says it spent.
//!
//! ADR 0048. `Device::render_retained` skips phase 1 when nothing an encode reads has
//! changed, which is a machine for producing a plausible-looking wrong page — principle
//! 6's worst outcome. This file is the equality half of the answer: the bytes a replay
//! draws are the bytes an encode drew, and everything the replayed `Frame` reports about
//! itself is true of *that* frame rather than of the one it replays. What makes an encode
//! stop being replayable is `retained_invalidation.rs`; what a page too large for the
//! atlas does is `retained_atlas.rs`.
//!
//! Every assertion here is an observable — `Frame::encode_source`, a raster, a counter, a
//! phase — and never a duration. A test that concluded "the second frame was faster"
//! would pass on a machine where it was faster for another reason, and would say nothing
//! at all about which encode drew the pixels.

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

use raster_gpu::{
    Counters, Device, EncodeSource, Frame, Options, RetainedScene, Target, UploadSplit,
};
use raster_scene::{Affine, Color, LineCap, Point, Scene, SceneBuilder};

mod common;

use common::headless::pixels;
use common::retained::{artwork_page, device, device_with, retained_frame, text_page, viewport};

/// The three renderings of one scene at one viewport — immediate, retained-and-encoded,
/// retained-and-replayed — are the same bytes.
///
/// On **two page shapes** on purpose: the atlas lane retains nothing but instance bytes
/// and reads tiles the device already holds, while the artwork page's encode carries a
/// coverage sheet, a clip residue, a layer plan and a composite. A replay that dropped
/// the sheet would pass the first and fail the second.
fn identical_across_replay(build: impl Fn(&mut Device) -> Scene, name: &str) {
    let mut device = device();
    let scene = build(&mut device);
    let viewport = viewport();

    let immediate = device
        .render(&scene, &viewport, Target::Readback)
        .expect("the frame must draw");
    assert_eq!(
        immediate.encode_source(),
        EncodeSource::Encoded,
        "{name}: `render` retains nothing and always encodes"
    );
    let immediate = pixels(immediate);

    let mut retained = RetainedScene::new(scene);
    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Encoded,
        "the first frame of a handle has nothing to replay",
    );
    let second = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Replayed,
        "nothing changed, so the encode is the one already made",
    );
    let third = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Replayed,
        "a replay does not consume what it replays",
    );

    assert_eq!(
        pixels(first),
        immediate,
        "{name}: the retained path's own first frame must equal an immediate one"
    );
    assert_eq!(
        pixels(second),
        immediate,
        "{name}: a replayed frame must equal the frame that was encoded, byte for byte"
    );
    assert_eq!(
        pixels(third),
        immediate,
        "{name}: and it must still be equal on the frame after that"
    );
}

/// A page of the line lane's marks (ADR 0097) — every cap, four directions, a hairline and a
/// dot — whose instance stream is the one a replay now reads from the device (viewer ADR
/// 1993) rather than sends again.
fn line_page(_: &mut Device) -> Scene {
    let mut builder = SceneBuilder::new();
    let caps = [LineCap::Butt, LineCap::Round, LineCap::Square];
    for row in 0..12_u8 {
        for column in 0..12_u8 {
            let (x, y) = (8.0 + f32::from(column) * 20.0, 8.0 + f32::from(row) * 20.0);
            let to = match (row + column) % 4 {
                0 => Point::new(x + 14.0, y),
                1 => Point::new(x, y + 14.0),
                2 => Point::new(x + 0.2, y + 0.2),
                _ => Point::new(x + 14.0, y + 14.0),
            };
            let cap = caps[usize::from((row * 7 + column) % 3)];
            let to = if cap == LineCap::Round && row == 5 {
                Point::new(x, y)
            } else {
                to
            };
            let width = [0.0, 0.4, 1.0, 3.5][usize::from(column % 4)];
            builder
                .line(
                    (Point::new(x, y), to),
                    Affine::IDENTITY,
                    (width, row % 2 == 0),
                    cap,
                    Color::new(0.1, 0.2 + f32::from(row) / 30.0, 0.6, 0.9),
                    None,
                    None,
                )
                .unwrap();
        }
    }
    builder.finish()
}

#[test]
fn a_replayed_line_page_is_the_page_that_was_encoded_and_sends_no_line() {
    identical_across_replay(line_page, "line page");
    let mut device = device();
    let mut retained = RetainedScene::new(line_page(&mut device));
    let viewport = viewport();
    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Encoded,
        "first frame",
    )
    .counters();
    let second = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Replayed,
        "second frame",
    )
    .counters();
    assert!(
        first.lanes.line > 50,
        "the page must be the line lane's: {:?}",
        first.lanes
    );
    assert_eq!(
        first.uploads.line_instances,
        u64::from(first.lanes.line) * 60
    );
    assert_eq!(second.uploads.line_instances, 0, "{:?}", second.uploads);
    assert_eq!(
        retained.resident_instance_bytes(),
        first.uploads.line_instances + first.uploads.rect_instances + first.uploads.quad_instances
    );
    // A new encode is a new cell: the streams are the new encode's, sent once.
    retained.forget();
    assert_eq!(retained.resident_instance_bytes(), 0);
    let third = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Encoded,
        "after forget",
    )
    .counters();
    assert_eq!(third.uploads, first.uploads);
}

#[test]
fn a_replayed_text_page_is_the_page_that_was_encoded() {
    identical_across_replay(|device| text_page(device, 240, 20, 10.0).0, "text page");
}

#[test]
fn a_replayed_artwork_page_is_the_page_that_was_encoded() {
    identical_across_replay(artwork_page, "artwork page");
}

/// The counters of a replayed frame are the counters of the frame it replays — **except
/// the one that is about this frame's transfers rather than about the encode**.
///
/// `bytes_uploaded` is honestly smaller on a replay, and by a knowable amount: the frame
/// that encoded also uploaded the glyph tiles it rasterised into the atlas texture and the
/// encode's instance streams, and the frame that replayed uploaded neither, because the
/// tiles are resident in the atlas and the streams beside the retained encode (viewer ADR
/// 1993). That is a counter telling the truth about two different frames, and
/// it is why this test compares the encode-derived fields by equality and that one by
/// inequality rather than blurring the two.
#[test]
fn a_replay_counts_what_the_encode_counted() {
    let mut device = device();
    let (scene, _) = text_page(&mut device, 240, 20, 10.0);
    let viewport = viewport();
    let mut retained = RetainedScene::new(scene);
    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Encoded,
        "first frame",
    );
    let second = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Replayed,
        "second frame",
    );
    let (first, second) = (first.counters(), second.counters());
    assert_eq!(
        Counters {
            bytes_uploaded: 0,
            uploads: UploadSplit::default(),
            ..first
        },
        Counters {
            bytes_uploaded: 0,
            uploads: UploadSplit::default(),
            ..second
        },
        "everything a counter reads out of the encode must be the encode's"
    );
    assert!(
        second.bytes_uploaded < first.bytes_uploaded,
        "the replay uploaded no instances and no glyph tiles: {} against {}",
        second.bytes_uploaded,
        first.bytes_uploaded
    );
    // The same sentence stream by stream (viewer ADR 1992): the atlas rows and the instance
    // streams are the whole of the difference, and the streams the replay did not send are
    // the ones the retained encode keeps on the device (ADR 1993).
    assert!(first.uploads.atlas_rows > 0, "{:?}", first.uploads);
    assert!(first.uploads.quad_instances > 0, "{:?}", first.uploads);
    assert_eq!(
        second.uploads,
        UploadSplit {
            atlas_rows: 0,
            rect_instances: 0,
            quad_instances: 0,
            line_instances: 0,
            ..first.uploads
        },
        "a replay sends none of the encode's instances again"
    );
    assert_eq!(
        retained.resident_instance_bytes(),
        first.uploads.rect_instances + first.uploads.quad_instances + first.uploads.line_instances,
        "what the replay did not send is what the retained encode holds"
    );
    for counters in [first, second] {
        assert_eq!(counters.uploads.total(), counters.bytes_uploaded);
    }
}

/// A replayed frame's **encode subdivision is zero, not the retained encode's**
/// (ADR 0023's instrument, ADR 0048's honesty).
///
/// The clock lives inside the `Encoded`, so the geometry and staging totals a retained
/// encode carries are real durations spent by the frame that made it — possibly hundreds
/// of frames ago. A replay that reported them would be a `Frame` saying it spent time it
/// did not spend, which is the one thing a `Frame` may never do.
#[test]
fn a_replay_reports_no_geometry_and_no_staging() {
    let mut device = device_with(&Options {
        instrument_encode: true,
        ..Options::default()
    });
    let scene = artwork_page(&mut device);
    let viewport = viewport();
    let mut retained = RetainedScene::new(scene);

    let phase = |frame: &Frame, name: &str| {
        frame
            .timings()
            .phases
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, d)| *d)
    };

    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Encoded,
        "first frame",
    );
    assert!(
        phase(&first, "encode: geometry").is_some_and(|d| d > std::time::Duration::ZERO),
        "the encoding frame rasterised this page's coverage and must say so: {:?}",
        first.timings().phases
    );

    let second = retained_frame(
        &mut device,
        &mut retained,
        &viewport,
        EncodeSource::Replayed,
        "second frame",
    );
    for name in ["encode: geometry", "encode: staging", "encode: recording"] {
        assert_eq!(
            phase(&second, name),
            Some(std::time::Duration::ZERO),
            "a replayed frame spent nothing on {name}: {:?}",
            second.timings().phases
        );
    }
}
