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
    Counters, Coverage, Device, EncodeSource, Frame, Options, RetainedScene, Target, UploadSplit,
};
use raster_scene::{
    Affine, BlendMode, Color, Compose, FillRule, GroupSpec, LineCap, Point, Rect, Scene,
    SceneBuilder, Segment,
};

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

/// **A zoom step draws the line stream the view before it left on the device** (viewer ADRs
/// 2022, 2056): a line's instance is the page's, so where the first walk culled none the encode at
/// the new view keeps the stream the encode before it wrote, and the frame sends none of it — and
/// draws, to the byte, what a device that never saw the first view draws at the second. A larger
/// view, a translated one that puts part of the page off the target, and a quarter turn, each from
/// the view before.
#[test]
fn a_zoom_step_draws_the_line_stream_the_view_before_it_left() {
    let mut device = device();
    let mut retained = RetainedScene::new(line_page(&mut device));
    // A first view the whole page fits, so the first walk culls no line and its stream is the
    // page's (ADR 2056).
    let whole =
        raster_gpu::Viewport::full(viewport().width, viewport().height, Affine::scale(0.7, 0.7));
    let first = retained_frame(
        &mut device,
        &mut retained,
        &whole,
        EncodeSource::Encoded,
        "first frame",
    )
    .counters();
    assert_eq!(first.commands_culled, 0, "{first:?}");
    assert!(first.uploads.line_instances > 0, "{:?}", first.uploads);
    let base = viewport();
    let steps = [
        Affine::scale(1.25, 1.25),
        Affine {
            a: 1.5,
            b: 0.0,
            c: 0.0,
            d: 1.5,
            e: -40.0,
            f: -24.0,
        },
        Affine {
            a: 0.0,
            b: 1.0,
            c: -1.0,
            d: 0.0,
            e: base.width as f32,
            f: 0.0,
        },
    ];
    for view in steps {
        let at = raster_gpu::Viewport::full(base.width, base.height, view);
        let stepped = retained_frame(
            &mut device,
            &mut retained,
            &at,
            EncodeSource::Encoded,
            "a new view is a new encode",
        );
        let counters = stepped.counters();
        assert_eq!(counters.lanes.line, first.lanes.line, "{view:?}");
        assert_eq!(
            counters.uploads.line_instances, 0,
            "{view:?}: the stream is the one already on the device: {:?}",
            counters.uploads
        );
        let fresh = device_with(&Options::default())
            .render(retained.scene(), &at, Target::Readback)
            .expect("the frame must draw");
        assert_eq!(
            pixels(stepped),
            pixels(fresh),
            "{view:?}: the step draws what a fresh device draws at its view"
        );
    }
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

/// A rectangle under a triangular clip inside a group: a child layer, which takes the frame off
/// the record replay (ADR 0087) as the owner's witness leaves it, so the compute lane's step walks.
fn residue_in_a_group(device: &mut Device, builder: &mut SceneBuilder) {
    let triangle = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(20.0, 200.0)),
            Segment::LineTo(Point::new(120.0, 140.0)),
            Segment::LineTo(Point::new(110.0, 236.0)),
            Segment::Close,
        ])
        .unwrap();
    let residue = builder
        .clip(triangle, Affine::IDENTITY, FillRule::NonZero, None)
        .unwrap();
    builder
        .group(
            GroupSpec {
                alpha: 0.8,
                blend: BlendMode::Normal,
                clip: None,
                knockout: false,
                mask: None,
                isolated: true,
                compose: Compose::SrcOver,
            },
            |inside| {
                inside.rect(
                    Rect::new(Point::new(0.0, 130.0), Point::new(130.0, 240.0)),
                    Affine::IDENTITY,
                    Color::new(0.2, 0.7, 0.2, 0.6),
                    Some(residue),
                    None,
                )
            },
        )
        .unwrap();
}

/// A page whose lines run under two rectangular clips, with no clip, under a transform of their
/// own, and once down the stroke road, between rectangles that leave and enter the target as the
/// view moves — so that a step's batches break where a fresh walk's would — and none of whose lines
/// the first view culls.
fn kept_line_page(device: &mut Device) -> Scene {
    let mut builder = SceneBuilder::new();
    let clips = [
        Rect::new(Point::new(0.0, 0.0), Point::new(150.0, 120.0)),
        Rect::new(Point::new(60.0, 40.0), Point::new(220.0, 180.0)),
    ]
    .map(|rect| {
        let outline = device
            .upload_outline(&common::scene::rect_outline(rect))
            .unwrap();
        builder
            .clip(outline, Affine::IDENTITY, FillRule::NonZero, None)
            .unwrap()
    });
    let colour = |i: u8| Color::new(0.1, f32::from(i % 7) / 7.0, 0.6, 0.85);
    residue_in_a_group(device, &mut builder);
    for i in 0..120_u8 {
        let (x, y) = (
            4.0 + f32::from(i % 12) * 17.0,
            4.0 + f32::from(i / 12) * 16.0,
        );
        // Every line meets its chain and the first view's target, so the first frame culls none
        // and its stream is the page's.
        let transform = if i % 30 >= 20 {
            Affine::scale(0.9, 1.1)
        } else {
            Affine::IDENTITY
        };
        let clip = if transform != Affine::IDENTITY {
            None
        } else if x < 140.0 && y < 110.0 {
            Some(clips[0])
        } else if x >= 60.0 && y >= 40.0 {
            Some(clips[1])
        } else {
            None
        };
        builder
            .line(
                (
                    Point::new(x, y),
                    Point::new(x + 15.0, y + f32::from(i % 5) * 3.0),
                ),
                transform,
                ([0.0, 0.7, 2.5][usize::from(i % 3)], i % 2 == 0),
                [LineCap::Butt, LineCap::Round, LineCap::Square][usize::from(i % 3)],
                colour(i),
                clip,
                None,
            )
            .unwrap();
        if i % 17 == 0 {
            builder
                .rect(
                    Rect::new(Point::new(x - 30.0, y - 2.0), Point::new(x - 10.0, y + 9.0)),
                    Affine::IDENTITY,
                    Color::new(0.8, 0.3, 0.1, 0.7),
                    None,
                    None,
                )
                .unwrap();
        }
        if i == 64 {
            // A line whose own transform turns the axes takes the stroke road at every view.
            builder
                .line(
                    (Point::new(30.0, 30.0), Point::new(90.0, 50.0)),
                    Affine {
                        a: 0.92,
                        b: 0.39,
                        c: -0.39,
                        d: 0.92,
                        e: 0.0,
                        f: 0.0,
                    },
                    (1.5, false),
                    LineCap::Round,
                    colour(3),
                    None,
                    None,
                )
                .unwrap();
        }
    }
    builder.finish()
}

/// **A step to a new view takes the line runs of the encode before and draws a fresh device's
/// frame** (viewer ADR 2056): under both lanes a larger view, one that puts part of the page off
/// the target, a quarter turn and a smaller view each keep every closed-form line in runs, send
/// none of the line stream, and draw what a device that never saw the views before draws.
#[test]
fn a_step_keeps_the_line_runs_and_draws_a_fresh_devices_frame() {
    for coverage in [Coverage::Cpu, Coverage::Compute] {
        let mut device = device();
        device.set_coverage(coverage);
        let mut retained = RetainedScene::new(kept_line_page(&mut device));
        let first = retained_frame(
            &mut device,
            &mut retained,
            &viewport(),
            EncodeSource::Encoded,
            "first frame",
        )
        .counters();
        assert_eq!(first.lines_kept, 0, "a first frame walks its lines");
        let base = viewport();
        let steps = [
            Affine {
                a: 1.6,
                b: 0.0,
                c: 0.0,
                d: 1.6,
                e: -60.0,
                f: -50.0,
            },
            Affine {
                a: 2.5,
                b: 0.0,
                c: 0.0,
                d: 2.5,
                e: -300.0,
                f: -20.0,
            },
            Affine {
                a: 0.0,
                b: 1.0,
                c: -1.0,
                d: 0.0,
                e: base.width as f32,
                f: 0.0,
            },
            Affine::scale(0.7, 0.7),
        ];
        for view in steps {
            let at = raster_gpu::Viewport::full(base.width, base.height, view);
            let stepped = retained_frame(
                &mut device,
                &mut retained,
                &at,
                EncodeSource::Encoded,
                "a new view is a new encode",
            );
            let counters = stepped.counters();
            assert_eq!(
                counters.lines_kept, first.lanes.line,
                "{coverage:?} {view:?}: every closed-form line is kept"
            );
            assert_eq!(counters.uploads.line_instances, 0, "{coverage:?} {view:?}");
            let mut fresh_device = device_with(&Options::default());
            fresh_device.set_coverage(coverage);
            // The same scene, its clips' outlines uploaded to this device under the same ids.
            let scene = kept_line_page(&mut fresh_device);
            let fresh = fresh_device
                .render(&scene, &at, Target::Readback)
                .expect("the frame must draw");
            assert_eq!(fresh.counters().lines_kept, 0);
            assert_eq!(
                counters.lanes.line, first.lanes.line,
                "{coverage:?} {view:?}: the line lane is the first walk's, which culled no line"
            );
            assert_eq!(
                pixels(stepped),
                pixels(fresh),
                "{coverage:?} {view:?}: the step draws what a fresh device draws at its view"
            );
        }
    }
}

/// A line inside a group is walked into a plan the walk makes again at every view, so an encode
/// holding one keeps no runs, and the step walks the page whole — and draws the same frame.
#[test]
fn a_line_inside_a_group_keeps_nothing() {
    let mut device = device();
    let mut builder = SceneBuilder::new();
    let line = |builder: &mut SceneBuilder, y: f32| {
        builder.line(
            (Point::new(10.0, y), Point::new(200.0, y + 5.0)),
            Affine::IDENTITY,
            (1.0, false),
            LineCap::Butt,
            Color::new(0.2, 0.3, 0.7, 1.0),
            None,
            None,
        )
    };
    line(&mut builder, 20.0).unwrap();
    builder
        .group(
            GroupSpec {
                alpha: 0.5,
                blend: BlendMode::Normal,
                clip: None,
                knockout: false,
                mask: None,
                isolated: true,
                compose: Compose::SrcOver,
            },
            |inside| line(inside, 60.0),
        )
        .unwrap();
    let mut retained = RetainedScene::new(builder.finish());
    retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Encoded,
        "first frame",
    );
    let at =
        raster_gpu::Viewport::full(viewport().width, viewport().height, Affine::scale(1.5, 1.5));
    let stepped = retained_frame(
        &mut device,
        &mut retained,
        &at,
        EncodeSource::Encoded,
        "a new view is a new encode",
    );
    assert_eq!(stepped.counters().lines_kept, 0);
    let fresh = device_with(&Options::default())
        .render(retained.scene(), &at, Target::Readback)
        .expect("the frame must draw");
    assert_eq!(pixels(stepped), pixels(fresh));
}

/// A first frame that culls a line wrote a stream of its view, not the page's, so it keeps no runs:
/// the step walks the page again, culling as the walk does, and draws a fresh device's frame.
#[test]
fn a_first_frame_that_culls_a_line_keeps_nothing() {
    let mut device = device();
    let mut builder = SceneBuilder::new();
    for (from, to) in [
        ((10.0, 20.0), (200.0, 25.0)),
        ((400.0, 40.0), (500.0, 60.0)),
    ] {
        builder
            .line(
                (Point::new(from.0, from.1), Point::new(to.0, to.1)),
                Affine::IDENTITY,
                (1.0, false),
                LineCap::Butt,
                Color::new(0.2, 0.3, 0.7, 1.0),
                None,
                None,
            )
            .unwrap();
    }
    let mut retained = RetainedScene::new(builder.finish());
    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Encoded,
        "first frame",
    )
    .counters();
    assert_eq!(first.lanes.line, 1, "the line off the target is culled");
    assert_eq!(first.commands_culled, 1);
    let at = raster_gpu::Viewport::full(
        viewport().width,
        viewport().height,
        Affine {
            a: 0.5,
            b: 0.0,
            c: 0.0,
            d: 0.5,
            e: 0.0,
            f: 0.0,
        },
    );
    let stepped = retained_frame(
        &mut device,
        &mut retained,
        &at,
        EncodeSource::Encoded,
        "a new view is a new encode",
    );
    assert_eq!(stepped.counters().lines_kept, 0);
    assert_eq!(
        stepped.counters().lanes.line,
        2,
        "the step walks the page at its own view, which admits both"
    );
    let fresh = device_with(&Options::default())
        .render(retained.scene(), &at, Target::Readback)
        .expect("the frame must draw");
    assert_eq!(pixels(stepped), pixels(fresh));
}

/// **The runs the line makers count are the walk's** (ADR 2056): a page of more lines than the
/// threads are started for (`lines_ahead`'s floor), under two chains that alternate in long runs,
/// on a device given four encode threads, keeps every line from a first view that culls none,
/// and the step draws what a one-threaded fresh device draws there.
#[test]
fn a_step_after_a_walk_with_line_makers_keeps_their_runs() {
    let page = |device: &mut Device| {
        let mut builder = SceneBuilder::new();
        let clips = [
            Rect::new(Point::new(0.0, 0.0), Point::new(220.0, 120.0)),
            Rect::new(Point::new(0.0, 60.0), Point::new(220.0, 180.0)),
        ]
        .map(|rect| {
            let outline = device
                .upload_outline(&common::scene::rect_outline(rect))
                .unwrap();
            builder
                .clip(outline, Affine::IDENTITY, FillRule::NonZero, None)
                .unwrap()
        });
        for i in 0..20_000_u32 {
            let (column, row) = ((i % 200) as f32, (i / 200) as f32);
            let (x, y) = (2.0 + column * 1.08, 62.0 + row * 0.55);
            builder
                .line(
                    (Point::new(x, y), Point::new(x + 0.9, y + 0.3)),
                    Affine::IDENTITY,
                    (0.4, i % 3 == 0),
                    LineCap::Butt,
                    Color::new(0.2, (i % 11) as f32 / 11.0, 0.5, 0.8),
                    Some(clips[((i / 3_000) % 2) as usize]),
                    None,
                )
                .unwrap();
        }
        builder.finish()
    };
    let threaded = Options {
        encode_threads: 4,
        ..Options::default()
    };
    let mut device = device_with(&threaded);
    let mut retained = RetainedScene::new(page(&mut device));
    let first = retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Encoded,
        "first frame",
    )
    .counters();
    assert_eq!(first.commands_culled, 0, "{first:?}");
    let at = raster_gpu::Viewport::full(
        viewport().width,
        viewport().height,
        Affine {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 2.0,
            e: -100.0,
            f: -150.0,
        },
    );
    let stepped = retained_frame(
        &mut device,
        &mut retained,
        &at,
        EncodeSource::Encoded,
        "a new view is a new encode",
    );
    assert_eq!(stepped.counters().lines_kept, first.lanes.line);
    assert_eq!(stepped.counters().uploads.line_instances, 0);
    let mut fresh_device = device_with(&Options::default());
    let scene = page(&mut fresh_device);
    let fresh = fresh_device
        .render(&scene, &at, Target::Readback)
        .expect("the frame must draw");
    assert_eq!(pixels(stepped), pixels(fresh));
}

/// A view of [`kept_line_page`] that puts most of the page off the target, so its walk culls lines.
fn zoomed() -> raster_gpu::Viewport<'static> {
    raster_gpu::Viewport::full(
        viewport().width,
        viewport().height,
        Affine {
            a: 2.5,
            b: 0.0,
            c: 0.0,
            d: 2.5,
            e: -300.0,
            f: -20.0,
        },
    )
}

/// **The idle build of a window opened zoomed keeps the page's lines, and the step from it takes
/// them in runs** (viewer ADR 2134): under both lanes a first frame that culls keeps nothing, the
/// idle build at its view walks every line, draws the first frame's pixels and leaves a replay of
/// that view drawing them too, and a step from it keeps every line, sends none of the stream and
/// draws what a fresh device draws there. Asked again, it builds nothing.
#[test]
fn the_idle_build_of_a_window_opened_zoomed_keeps_the_lines_for_the_step() {
    for coverage in [Coverage::Cpu, Coverage::Compute] {
        let mut device = device();
        device.set_coverage(coverage);
        let mut retained = RetainedScene::new(kept_line_page(&mut device));
        let first = retained_frame(
            &mut device,
            &mut retained,
            &zoomed(),
            EncodeSource::Encoded,
            "first frame",
        );
        let first_counters = first.counters();
        assert!(
            first_counters.commands_culled > 0,
            "{coverage:?}: the view culls"
        );
        assert!(retained.wants_whole_walk(), "{coverage:?}: it kept nothing");
        let first_pixels = pixels(first);
        let whole = device
            .build_whole(&mut retained, &zoomed(), Target::Readback)
            .expect("the idle build draws")
            .expect("a frame that culled is built whole");
        let whole_lines = whole.counters().lanes.line;
        assert!(
            whole_lines > first_counters.lanes.line,
            "{coverage:?}: the idle build culls no line"
        );
        assert_eq!(
            pixels(whole),
            first_pixels,
            "{coverage:?}: the idle build draws the frame it repeats"
        );
        assert!(!retained.wants_whole_walk(), "{coverage:?}");
        assert!(
            device
                .build_whole(&mut retained, &zoomed(), Target::Readback)
                .expect("asked again")
                .is_none(),
            "{coverage:?}: a view whose encode keeps its runs is not built again"
        );
        let replayed = retained_frame(
            &mut device,
            &mut retained,
            &zoomed(),
            EncodeSource::Replayed,
            "the view the idle build was made at replays it",
        );
        assert_eq!(pixels(replayed), first_pixels, "{coverage:?}");
        let at = raster_gpu::Viewport::full(
            viewport().width,
            viewport().height,
            Affine {
                a: 3.0,
                b: 0.0,
                c: 0.0,
                d: 3.0,
                e: -380.0,
                f: -40.0,
            },
        );
        let stepped = retained_frame(
            &mut device,
            &mut retained,
            &at,
            EncodeSource::Encoded,
            "a new view is a new encode",
        );
        let counters = stepped.counters();
        assert_eq!(counters.lines_kept, whole_lines, "{coverage:?}");
        assert_eq!(counters.uploads.line_instances, 0, "{coverage:?}");
        let mut fresh_device = device_with(&Options::default());
        fresh_device.set_coverage(coverage);
        let scene = kept_line_page(&mut fresh_device);
        let fresh = fresh_device
            .render(&scene, &at, Target::Readback)
            .expect("the frame must draw");
        assert_eq!(
            pixels(stepped),
            pixels(fresh),
            "{coverage:?}: the step draws what a fresh device draws at its view"
        );
    }
}

/// **The idle build is declined once where it would keep nothing**: a line inside a group keeps no
/// runs at any view, and streams past the residency budget would be sent every frame, so the held
/// encode stays — a replay of its view still replays — and the scene is not built whole again.
#[test]
fn the_idle_build_is_declined_once_where_it_would_keep_nothing() {
    // Past the budget: one byte resident, which no stream fits.
    let mut tight = device_with(&Options {
        max_resident_bytes: 1,
        ..Options::default()
    });
    let mut retained = RetainedScene::new(kept_line_page(&mut tight));
    retained_frame(
        &mut tight,
        &mut retained,
        &zoomed(),
        EncodeSource::Encoded,
        "first frame",
    );
    assert!(retained.wants_whole_walk());
    let built = tight
        .build_whole(&mut retained, &zoomed(), Target::Readback)
        .expect("the idle build is asked");
    assert!(built.is_none(), "the whole stream is past the budget");
    assert!(!retained.wants_whole_walk(), "and is not asked again");
    retained_frame(
        &mut tight,
        &mut retained,
        &zoomed(),
        EncodeSource::Replayed,
        "the held encode stays",
    );

    // A line inside a group.
    let mut device = device();
    let mut builder = SceneBuilder::new();
    let line = |builder: &mut SceneBuilder, y: f32| {
        builder.line(
            (Point::new(10.0, y), Point::new(600.0, y + 5.0)),
            Affine::IDENTITY,
            (1.0, false),
            LineCap::Butt,
            Color::new(0.2, 0.3, 0.7, 1.0),
            None,
            None,
        )
    };
    line(&mut builder, 20.0).unwrap();
    line(&mut builder, 400.0).unwrap();
    builder
        .group(
            GroupSpec {
                alpha: 0.5,
                blend: BlendMode::Normal,
                clip: None,
                knockout: false,
                mask: None,
                isolated: true,
                compose: Compose::SrcOver,
            },
            |inside| line(inside, 60.0),
        )
        .unwrap();
    let mut retained = RetainedScene::new(builder.finish());
    retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Encoded,
        "first frame",
    );
    assert!(retained.wants_whole_walk());
    assert!(
        device
            .build_whole(&mut retained, &viewport(), Target::Readback)
            .expect("the idle build is asked")
            .is_none(),
        "a line in a group keeps nothing"
    );
    assert!(!retained.wants_whole_walk());
    retained_frame(
        &mut device,
        &mut retained,
        &viewport(),
        EncodeSource::Replayed,
        "the held encode stays",
    );
}
