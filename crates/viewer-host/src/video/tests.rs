//! The window's half of the frame sink against a queue the test fills: which picture is due, which
//! are dropped late, and what stop, pause and the end say (ADR 3538). The decoding thread's half is
//! `pdf-media-worker`'s own tests.

use super::*;

/// A 2 by 2 picture with no VUI, which Annex E reads as limited range and an unspecified matrix.
fn picture() -> pdf_media_worker::video::Picture {
    pdf_media_worker::video::Picture {
        width: 2,
        height: 2,
        y: vec![16; 4],
        u: vec![128],
        v: vec![128],
        signal: pdf_media_worker::video::Signal::default(),
    }
}

fn shown(repeat: f64) -> Shown {
    Shown {
        document: DocumentId(1),
        label: "the clip \"c.mp4\"".to_owned(),
        file: Arc::from(Vec::new()),
        screen: Some(ObjectId::new(9, 0)),
        associated: Some(ObjectId::new(9, 0)),
        auto_play: true,
        repeat,
        cut: false,
        fit: pdf_model::media::Fit::PlayersDefault,
    }
}

/// A player holding `shown` as pass 1 of a queue the test writes, its clock started at the
/// answered instant.
fn holding(shown: Shown) -> (VideoPlayer, SyncSender<Decoded>, Instant) {
    let (orders, _) = std::sync::mpsc::channel();
    let (outbox, decoded) = std::sync::mpsc::sync_channel(16);
    let mut player = VideoPlayer::new(Box::new(|| {}));
    player.unstarted = None;
    player.generation = 1;
    player.running = Some(Running {
        orders,
        decoded,
        wanted: Arc::new(AtomicU64::new(1)),
        thread: None,
    });
    let now = Instant::now();
    let mut clock = MediaClock::new(CLOCK_RATE);
    clock.freeze(now);
    player.in_hand = Some(InHand {
        shown,
        clock,
        sound: None,
        time: Duration::ZERO,
        size: None,
        running: true,
        generation: 1,
        pass_start: Duration::ZERO,
        passes: 1,
        next: None,
        last_at: Duration::ZERO,
        interval: Duration::ZERO,
        over: None,
        shown_count: 0,
        dropped: 0,
    });
    (player, outbox, now)
}

fn send(outbox: &SyncSender<Decoded>, generation: u64, millis: u64) {
    outbox
        .send(Decoded::Picture {
            generation,
            picture: picture(),
            at: Duration::from_millis(millis),
        })
        .expect("queued");
}

#[test]
fn the_newest_due_picture_is_shown_and_the_ones_it_passes_are_dropped_late() {
    let (mut player, outbox, began) = holding(shown(1.0));
    for millis in [0, 40, 80, 120] {
        send(&outbox, 1, millis);
    }
    let first = player.due(began).expect("the first picture is due at once");
    assert_eq!(first.at, Duration::ZERO);
    assert!(
        player.due(began).is_none(),
        "the picture at 40 ms is not yet due"
    );
    let late = player
        .due(began + Duration::from_millis(130))
        .expect("120 ms is due");
    assert_eq!(late.at, Duration::from_millis(120));
    assert!(
        late.conversion.kr > 0.0,
        "Annex E's unspecified matrix took the stated choice"
    );
    outbox
        .send(Decoded::Over {
            generation: 1,
            result: Ok((Duration::from_millis(120), None)),
        })
        .expect("queued");
    // The last picture stays up for its interval, 40 ms, before the clip ends.
    assert!(player.due(began + Duration::from_millis(140)).is_none());
    assert!(player.holds_a_picture());
    assert!(player.due(began + Duration::from_millis(170)).is_none());
    assert!(!player.holds_a_picture());
    let seen = player.seen();
    assert_eq!(
        seen.last(),
        Some(&Seen::Ended(
            "the clip \"c.mp4\": ended after 2 pictures, 2 dropped late".to_owned()
        ))
    );
}

#[test]
fn a_picture_of_a_pass_the_window_has_left_is_never_shown() {
    let (mut player, outbox, began) = holding(shown(1.0));
    send(&outbox, 0, 0);
    send(&outbox, 1, 0);
    assert!(player.due(began).is_some());
    let hand = player.in_hand.as_ref().expect("in hand");
    assert_eq!((hand.shown_count, hand.dropped), (1, 0));
}

#[test]
fn a_pause_stops_the_clock_and_another_annotation_s_operation_reaches_nothing() {
    let (mut player, outbox, began) = holding(shown(1.0));
    send(&outbox, 1, 0);
    let document = DocumentId(1);
    assert!(!player.operate(document, ObjectId::new(10, 0), MediaOperation::Pause));
    assert!(player.operate(document, ObjectId::new(9, 0), MediaOperation::Pause));
    assert!(player.due(began + Duration::from_secs(1)).is_none());
    assert!(
        !player.operate(document, ObjectId::new(9, 0), MediaOperation::Pause),
        "a pause finds it paused: no effect"
    );
    assert!(player.operate(document, ObjectId::new(9, 0), MediaOperation::Stop));
    assert!(!player.in_hand());
    let seen = player.seen();
    assert!(matches!(seen.first(), Some(Seen::Paused(_))), "{seen:?}");
    assert!(matches!(seen.last(), Some(Seen::Stopped(_))), "{seen:?}");
}

#[test]
fn a_clip_in_no_rectangle_is_said_and_not_decoded() {
    let mut player = VideoPlayer::new(Box::new(|| {}));
    player.play(Shown {
        screen: None,
        ..shown(1.0)
    });
    assert!(!player.in_hand());
    assert!(
        player.running.is_none(),
        "no thread and no worker for a clip not shown"
    );
    assert!(matches!(player.seen().as_slice(), [Seen::Refused(_)]));
}

/// A window that looks only once the pass is over takes its last pictures and the pass's end in
/// one look: none of them was up for any of its interval, so each is dropped late, nothing is
/// handed on, and the end counts every picture. The wrong answer excluded: the newest picture
/// lost from both counts (an end said "after 0 pictures, 2 dropped late" of three).
#[test]
fn a_look_after_the_pass_ended_counts_every_picture_it_passes() {
    let (mut player, outbox, began) = holding(shown(1.0));
    drop(player.seen());
    for millis in [0, 40, 80] {
        send(&outbox, 1, millis);
    }
    outbox
        .send(Decoded::Over {
            generation: 1,
            result: Ok((Duration::from_millis(80), None)),
        })
        .expect("queued");
    assert!(player.due(began + Duration::from_millis(500)).is_none());
    assert!(!player.holds_a_picture());
    assert_eq!(
        player.seen().last(),
        Some(&Seen::Ended(
            "the clip \"c.mp4\": ended after 0 pictures, 3 dropped late".to_owned()
        ))
    );
}

/// Table 291's `/F` over a 100 by 50 pixel rectangle at (10, 20), for a picture of 40 by 40
/// samples (ADR 3553): meet scales it by 1.25 to 50 by 50 at the rectangle's top-left; slice by 2.5
/// to cover the rectangle, showing 40 by 20 samples; fill stretches it to the rectangle; hidden
/// shows it unscaled, 40 by 40 pixels, all of it.
#[test]
fn each_fit_places_the_picture_as_table_291_says() {
    use pdf_model::media::Fit;
    let quad = [10.0, 20.0, 110.0, 20.0, 110.0, 70.0, 10.0, 70.0];
    let at = |fit| placed(fit, (40, 40), quad).expect("placed");
    let meet = at(Fit::Meet);
    assert_eq!((meet.right, meet.bottom), (40, 40));
    assert_eq!(meet.quad, [10.0, 20.0, 60.0, 20.0, 60.0, 70.0, 10.0, 70.0]);
    assert_eq!(
        at(Fit::PlayersDefault),
        meet,
        "this player's default is meet"
    );
    let slice = at(Fit::Slice);
    assert_eq!((slice.right, slice.bottom), (40, 20));
    assert_eq!(slice.quad, quad);
    let fill = at(Fit::Fill);
    assert_eq!((fill.right, fill.bottom), (40, 40));
    assert_eq!(fill.quad, quad);
    let hidden = at(Fit::Hidden);
    assert_eq!((hidden.right, hidden.bottom), (40, 40));
    assert_eq!(
        hidden.quad,
        [10.0, 20.0, 50.0, 20.0, 50.0, 60.0, 10.0, 60.0]
    );
    assert_eq!(
        at(Fit::Scroll),
        hidden,
        "scroll's picture is hidden's, less its interface"
    );
    // A picture larger than the rectangle, unscaled, is cut to it.
    let cut = placed(Fit::Hidden, (300, 30), quad).expect("placed");
    assert_eq!((cut.right, cut.bottom), (100, 30));
    assert_eq!(cut.quad, [10.0, 20.0, 110.0, 20.0, 110.0, 50.0, 10.0, 50.0]);
    // The affine puts the part's corners on the quadrilateral's.
    let affine = slice.affine();
    let map = |x: f32, y: f32| {
        (
            affine[0] * x + affine[2] * y + affine[4],
            affine[1] * x + affine[3] * y + affine[5],
        )
    };
    assert_eq!(map(0.0, 0.0), (10.0, 20.0));
    assert_eq!(map(40.0, 20.0), (110.0, 70.0));
}

/// A rectangle turned by a quarter fits by its own edges: meet in a 50 wide, 100 tall rectangle
/// whose first edge runs down the window.
#[test]
fn a_turned_rectangle_is_fitted_along_its_own_edges() {
    let quad = [0.0, 0.0, 0.0, 50.0, -100.0, 50.0, -100.0, 0.0];
    let meet = placed(pdf_model::media::Fit::Meet, (40, 40), quad).expect("placed");
    assert_eq!(meet.quad, [0.0, 0.0, 0.0, 50.0, -50.0, 50.0, -50.0, 0.0]);
}

/// The part of a raster a placement shows: hidden's top-left corner of a picture wider than its
/// rectangle.
#[test]
fn the_part_shown_is_cut_from_the_picture_s_top_left() {
    let raster = pdf_render::Raster {
        width: 3,
        height: 2,
        format: pdf_render::RasterFormat::Rgba8,
        data: (0..24).collect(),
    };
    let part = Placed {
        left: 0,
        top: 0,
        right: 2,
        bottom: 1,
        quad: [0.0; 8],
    }
    .part_of(&raster);
    assert_eq!((part.width, part.height), (2, 1));
    assert_eq!(part.data, (0..8).collect::<Vec<u8>>());
}
