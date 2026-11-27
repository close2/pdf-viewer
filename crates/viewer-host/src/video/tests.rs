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
        duration: pdf_model::media::SimpleDuration::Intrinsic,
        section: pdf_model::media::Section::WHOLE,
        controls: false,
        fit: pdf_model::media::Fit::PlayersDefault,
        window: None,
        quarter_turns: 0,
        backward: false,
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
    let timeline = timeline::Timeline::new(&pdf_model::media::Playing {
        duration: shown.duration,
        section: shown.section,
        repeat: shown.repeat,
        ..pdf_model::media::Playing::default()
    });
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
        timeline,
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

/// ISO 32000-2 Table 306's `/Rotate`: "The number of degrees by which the movie shall be rotated
/// clockwise relative to the page." A 40 by 20 picture turned a quarter clockwise in a 50 wide,
/// 100 tall rectangle at the origin meets it 50 tall: its top-left sample at the rectangle's
/// top-right corner, its rows running down the right side and its columns leftward; the part a
/// window draws upright is 20 wide and 40 tall, its first sample the picture's bottom-left.
#[test]
fn a_movie_turned_by_its_rotate_is_placed_turned_clockwise() {
    use pdf_model::media::Fit;
    let quad = [0.0, 0.0, 50.0, 0.0, 50.0, 100.0, 0.0, 100.0];
    let turned = placed_turned(Fit::Meet, (40, 20), quad, 1).expect("placed");
    assert_eq!(turned.quarter_turns, 1);
    assert_eq!((turned.right, turned.bottom), (40, 20));
    // Scale 50 ÷ 20 = 2.5 across the rectangle's width, which the picture's height now lies on.
    assert_eq!(turned.quad, [50.0, 0.0, 50.0, 100.0, 0.0, 100.0, 0.0, 0.0]);
    let affine = turned.affine();
    let map = |x: f32, y: f32| {
        (
            affine[0] * x + affine[2] * y + affine[4],
            affine[1] * x + affine[3] * y + affine[5],
        )
    };
    assert_eq!(map(0.0, 0.0), (50.0, 0.0), "the top-left at the top-right");
    assert_eq!(
        map(40.0, 0.0),
        (50.0, 100.0),
        "the top row down the right side"
    );
    assert_eq!(map(0.0, 20.0), (0.0, 0.0), "the left column leftward");
    // The samples, numbered by position, as a window that draws upright gets them.
    let picture = pdf_render::Raster {
        width: 40,
        height: 20,
        format: pdf_render::RasterFormat::Rgba8,
        data: (0..40 * 20)
            .flat_map(|index: u32| index.to_le_bytes())
            .collect(),
    };
    assert!(
        !turned.is_whole(&picture),
        "a turned part is never handed on as it is"
    );
    let part = turned.part_of(&picture);
    assert_eq!((part.width, part.height), (20, 40));
    let sample = |raster: &pdf_render::Raster, x: u32, y: u32| {
        let at = usize::try_from((y * raster.width + x) * 4).expect("small");
        u32::from_le_bytes(raster.data[at..at + 4].try_into().expect("four bytes"))
    };
    assert_eq!(
        sample(&part, 0, 0),
        19 * 40,
        "the picture's bottom-left sample"
    );
    assert_eq!(
        sample(&part, 19, 0),
        0,
        "the picture's top-left sample at the top-right"
    );
    assert_eq!(
        sample(&part, 19, 39),
        39,
        "the top row's end at the bottom-right"
    );
    assert_eq!(
        turned.part_quad(),
        quad,
        "the upright part covers the rectangle from its corner"
    );
    // A half turn and three quarters keep the corners going clockwise; none is the unturned fit.
    assert_eq!(
        placed_turned(Fit::Fill, (40, 20), quad, 2).map(|placed| placed.quad),
        Some([50.0, 100.0, 0.0, 100.0, 0.0, 0.0, 50.0, 0.0])
    );
    assert_eq!(
        placed_turned(Fit::Fill, (40, 20), quad, 0),
        placed(Fit::Fill, (40, 20), quad)
    );
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
        quarter_turns: 0,
    }
    .part_of(&raster);
    assert_eq!((part.width, part.height), (2, 1));
    assert_eq!(part.data, (0..8).collect::<Vec<u8>>());
}

/// A clip shown in a window of its own (Table 294's `/W` 0 or 1, ADR 3587): the window shows
/// Table 294's `/B` alone before the first picture, and the picture fitted over it after — here a
/// two-by-two grey picture met (Table 291's `/F` 0) in a four-by-two window, so it stands in the
/// left half and the background shows in the right half.
#[test]
fn a_window_of_its_own_shows_its_background_and_then_its_picture() {
    let window = pdf_model::media::Window {
        kind: pdf_model::media::WindowKind::FullScreen,
        monitor: pdf_model::media::MonitorSpecifier::Largest,
        colour: [0.0, 0.0, 1.0],
    };
    let (mut player, _outbox, _) = holding(Shown {
        screen: None,
        window: Some(window),
        fit: pdf_model::media::Fit::Meet,
        ..shown(1.0)
    });
    assert!(player.window().is_some());
    let empty = player
        .window_picture(None, (4, 2))
        .expect("a window clip")
        .expect("drawn");
    assert!(
        empty
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [0, 0, 255, 255]),
        "Table 294's /B alone"
    );
    if let Some(hand) = player.in_hand.as_mut() {
        hand.size = Some((2, 2));
    }
    let grey = pdf_render::Raster {
        width: 2,
        height: 2,
        format: pdf_render::RasterFormat::Rgba8,
        data: [128, 128, 128, 255].repeat(4),
    };
    let drawn = player
        .window_picture(Some(&grey), (4, 2))
        .expect("a window clip")
        .expect("drawn");
    let at = |x: usize, y: usize| &drawn.data[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4];
    assert_eq!(at(0, 0), &[128, 128, 128, 255], "the picture at the left");
    assert_eq!(at(3, 1), &[0, 0, 255, 255], "the background at the right");
}

/// §13.2.4.3's section from 1 s to 1.1 s (ADR 3593): the pass's first picture is the one at 1 s,
/// shown at once; the picture at 1.12 s is past the section's end and is cut, never shown; and the
/// clip ends when the last picture before the cut has been up until the cut. The wrong answer
/// excluded: the whole clip shown from its start, as before the section reached the pictures.
#[test]
fn a_section_places_and_cuts_the_pictures() {
    let mut shown = shown(1.0);
    shown.section = pdf_model::media::Section {
        begin: 1.0,
        end: Some(1.1),
    };
    let (mut player, outbox, began) = holding(shown);
    drop(player.seen());
    for millis in [1000, 1040, 1080, 1120] {
        send(&outbox, 1, millis);
    }
    let ms = Duration::from_millis;
    assert_eq!(player.due(began).expect("the section's start").at, ms(1000));
    assert_eq!(player.due(began + ms(50)).expect("1.04 s").at, ms(1040));
    assert_eq!(player.due(began + ms(90)).expect("1.08 s").at, ms(1080));
    assert!(player.holds_a_picture(), "up until the cut at 100 ms");
    assert!(player.due(began + ms(100)).is_none(), "1.12 s is cut");
    assert!(!player.holds_a_picture());
    assert_eq!(
        player.seen().last(),
        Some(&Seen::Ended(
            "the clip \"c.mp4\": ended after 3 pictures, 0 dropped late".to_owned()
        ))
    );
}

/// Table 291's `/D` of 0.2 s over 0.12 s of pictures, twice (ADR 3593): the last picture is held up
/// from 120 ms until the duration has elapsed — "the player shall freeze the media in its final
/// state until the simple duration has elapsed" — and the second pass begins at 200 ms on the
/// clock, where the sound's second pass begins, rather than where the pictures ran out.
#[test]
fn a_longer_duration_holds_the_last_picture_until_it_has_elapsed() {
    let mut shown = shown(2.0);
    shown.duration = pdf_model::media::SimpleDuration::Seconds(0.2);
    let (mut player, outbox, began) = holding(shown);
    for millis in [0, 40, 80] {
        send(&outbox, 1, millis);
    }
    outbox
        .send(Decoded::Over {
            generation: 1,
            result: Ok((Duration::from_millis(80), None)),
        })
        .expect("queued");
    let ms = Duration::from_millis;
    assert!(player.due(began + ms(85)).is_some());
    assert!(player.due(began + ms(150)).is_none());
    let hand = player.in_hand.as_ref().expect("held, frozen");
    assert_eq!(hand.passes, 1);
    assert_eq!(
        player.next_due(began + ms(150)),
        Some(began + ms(200)),
        "the window is woken when the duration has elapsed"
    );
    assert!(player.due(began + ms(205)).is_none());
    let hand = player.in_hand.as_ref().expect("the second pass");
    assert_eq!((hand.passes, hand.pass_start), (2, ms(200)));
}

/// A duration of 0.1 s over pictures that run longer (ADR 3593): the picture at 0.12 s is cut and
/// the pass ends at 0.1 s.
#[test]
fn a_shorter_duration_cuts_the_pictures() {
    let mut shown = shown(1.0);
    shown.duration = pdf_model::media::SimpleDuration::Seconds(0.1);
    let (mut player, outbox, began) = holding(shown);
    drop(player.seen());
    for millis in [0, 40, 80, 120] {
        send(&outbox, 1, millis);
    }
    let ms = Duration::from_millis;
    assert!(player.due(began + ms(85)).is_some());
    assert!(player.holds_a_picture());
    assert!(player.due(began + ms(100)).is_none());
    assert_eq!(
        player.seen().last(),
        Some(&Seen::Ended(
            "the clip \"c.mp4\": ended after 1 pictures, 2 dropped late".to_owned()
        ))
    );
}

/// Table 307's `/ShowControls` (ADR 3595): a clip that asks for the bar answers what it shows —
/// running, at its place in the pass, the pass's length once a stated duration gives it — and a
/// press on its button pauses the clip, after which the bar says so; a clip that does not ask has
/// no bar.
#[test]
fn a_clip_that_asks_for_the_bar_answers_it_and_a_press_pauses_it() {
    let mut asked = shown(1.0);
    asked.controls = true;
    asked.duration = pdf_model::media::SimpleDuration::Seconds(2.0);
    let (mut player, outbox, began) = holding(asked);
    send(&outbox, 1, 0);
    let ms = Duration::from_millis;
    assert!(player.due(began + ms(50)).is_some());
    let controller = player.controller(began + ms(50)).expect("a bar");
    assert!(controller.playing);
    assert_eq!(controller.position, ms(50));
    assert_eq!(controller.length, Some(ms(2000)));
    let quad = [0.0, 0.0, 120.0, 0.0, 120.0, 90.0, 0.0, 90.0];
    let (bar, covered) = player
        .controller_bar(quad, began + ms(50))
        .expect("the bar over the rectangle");
    assert_eq!((bar.width, bar.height), (120, BAR_HEIGHT));
    assert_eq!(covered[1], 70.0);
    assert!(player.press(2), "the button pauses the clip");
    assert!(
        !player
            .controller(Instant::now())
            .expect("still a bar")
            .playing
    );
    let (plain, _, _) = holding(shown(1.0));
    assert!(plain.controller(began).is_none());
}
