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
            result: Ok(Duration::from_millis(120)),
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
