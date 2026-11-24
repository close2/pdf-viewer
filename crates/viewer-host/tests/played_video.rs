//! A clip's pictures played through a window's [`VideoPlayer`] end to end: the confined media
//! worker decoding `pdf-media-worker`'s hand-built `I_PCM` stream (ADR 3526), the decoding thread,
//! the clock, and each picture converted on the processor as a window that draws on none converts
//! it (RFC 0015 section 6, rounds 5 and 6; ADRs 3538, 3545).
//!
//! **What a picture must be is known before it is decoded.** Every macroblock is `I_PCM`, which
//! carries its samples as raw bytes (Rec. ITU-T H.264 7.3.5), so picture `k` is the samples the
//! fixture wrote for it, and its pixels are those samples converted by `render-cpu`'s oracle
//! conversion — checked here against the samples, not against the decoder's own output.
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first, as `pdf-sandbox`'s worker is built before `pdf-model`'s tests (trap 10).

#![expect(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: the fixture is a few kilobytes and a failure should fail loudly"
)]

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use pdf_media_worker::video::Picture;
use pdf_render::video::Planes;
use pdf_syntax::ObjectId;
use viewer_core::{DocumentId, MediaOperation};
use viewer_host::video::{Frame, Seen, Shown, VideoPlayer};

#[path = "../../pdf-media-worker/tests/support/pcm_stream.rs"]
#[expect(
    dead_code,
    reason = "the fixture's other streams are `pdf-media-worker`'s own tests'"
)]
mod pcm_stream;

/// The screen annotation the pictures are drawn over, and the one the rendition is associated with.
const SCREEN: ObjectId = ObjectId::new(4, 0);

/// One picture's interval: the fixture's `stts` states 1000 ticks a sample at 25 000 a second.
const INTERVAL: Duration = Duration::from_millis(40);

/// The longest any wait here may take before the test says what it was waiting for.
const PATIENCE: Duration = Duration::from_secs(30);

/// An MP4 file of `count` sixteen-by-sixteen pictures and the samples each must decode to.
fn clip(count: u32) -> (Arc<[u8]>, Vec<Picture>) {
    let (sets, units, pictures) =
        pcm_stream::stream(count, 16, 16, pcm_stream::Coding::Progressive);
    (Arc::from(pcm_stream::mp4(&sets, &units, 16, 16)), pictures)
}

/// The clip shown in the screen annotation, started at once and played once.
fn shown(file: Arc<[u8]>) -> Shown {
    Shown {
        document: DocumentId(1),
        label: "the clip".to_owned(),
        file,
        screen: Some(SCREEN),
        associated: Some(SCREEN),
        auto_play: true,
        repeat: 1.0,
        duration: pdf_model::media::SimpleDuration::Intrinsic,
        section: pdf_model::media::Section::WHOLE,
        controls: false,
        fit: pdf_model::media::Fit::PlayersDefault,
        window: None,
    }
}

/// A player whose wake is a channel the test waits on, as a window's loop waits on its waker.
fn player() -> (VideoPlayer, Receiver<()>) {
    let (woken, wakes) = channel();
    let player = VideoPlayer::new(Box::new(move || {
        let _ = woken.send(());
    }));
    (player, wakes)
}

/// What a window's loop does between two looks: rests until the player's next time or its wake,
/// whichever comes first — never longer than a picture's interval, so a lost wake costs a frame.
fn rest(player: &VideoPlayer, wakes: &Receiver<()>) {
    let now = Instant::now();
    let until = player
        .next_due(now)
        .unwrap_or(now + INTERVAL)
        .min(now + INTERVAL);
    let _ = wakes.recv_timeout(until.saturating_duration_since(now));
}

/// The picture's pixels as the oracle converts `samples`, the planes the fixture wrote.
fn converted(samples: &Picture, frame: &Frame) -> pdf_render::Raster {
    let planes = Planes {
        width: samples.width as u32,
        height: samples.height as u32,
        y: &samples.y,
        cb: &samples.u,
        cr: &samples.v,
    };
    render_cpu::video::convert_picture(&planes, &frame.conversion).unwrap()
}

/// Plays until the player says something matching `done`, checking every picture on the way;
/// answers the indices of the pictures shown and everything said.
fn play_until(
    player: &mut VideoPlayer,
    wakes: &Receiver<()>,
    samples: &[Picture],
    done: impl Fn(&Seen) -> bool,
) -> (Vec<usize>, Vec<Seen>) {
    let began = Instant::now();
    let mut indices = Vec::new();
    let mut said = Vec::new();
    loop {
        if let Some(frame) = player.due(Instant::now()) {
            let index = (frame.at.as_millis() / INTERVAL.as_millis()) as usize;
            assert_eq!(
                frame.at,
                INTERVAL * index as u32,
                "a picture at its sample's time"
            );
            assert_eq!(
                frame.picture, samples[index],
                "picture {index} is the samples its slice carries"
            );
            let raster = viewer_host::video::on_the_processor(&frame).unwrap();
            assert_eq!(
                raster,
                converted(&samples[index], &frame),
                "picture {index}'s pixels are its samples converted"
            );
            indices.push(index);
        }
        let seen = player.seen();
        let finished = seen.iter().any(&done);
        said.extend(seen);
        if finished {
            return (indices, said);
        }
        assert!(
            began.elapsed() < PATIENCE,
            "the worker must be built: `cargo build -p pdf-media-worker --bins`; said {said:?}"
        );
        rest(player, wakes);
    }
}

/// **A clip plays to its end**: each picture handed on is the one its time names, its pixels are
/// its samples converted, the pictures come in order, every picture is shown or counted as
/// dropped late, the end is said with those two counts, and the rectangle is the page's again.
/// The wrong answers excluded: a picture shown out of its time, a stream decoded into other
/// samples, a clip whose last picture is never taken down (a pass end nothing wakes the window
/// for), and an end counted short.
#[test]
fn a_clip_plays_to_its_end_with_every_picture_its_samples() {
    let (file, samples) = clip(6);
    let (mut player, wakes) = player();
    player.play(shown(file));
    assert!(player.holds_a_picture());
    let (indices, said) = play_until(&mut player, &wakes, &samples, |seen| {
        matches!(seen, Seen::Ended(_) | Seen::Refused(_) | Seen::Stopped(_))
    });
    assert!(!indices.is_empty(), "a picture was shown; said {said:?}");
    assert!(
        indices.windows(2).all(|pair| pair[0] < pair[1]),
        "in order: {indices:?}"
    );
    let Some(Seen::Ended(ended)) = said.last() else {
        panic!("the clip ended: {said:?}");
    };
    let dropped = 6 - indices.len();
    assert_eq!(
        ended,
        &format!(
            "the clip: ended after {} pictures, {dropped} dropped late",
            indices.len()
        )
    );
    assert!(
        !player.holds_a_picture(),
        "the rectangle is the page's again"
    );
    assert!(player.due(Instant::now()).is_none());
}

/// **A stop mid-clip ends it at once**: Table 218's `/OP` 1 on the associated annotation stops
/// the pictures, says so, and leaves nothing to show; the same operation on an annotation the clip
/// is not associated with reaches nothing.
#[test]
fn a_stop_mid_clip_ends_its_pictures() {
    let (file, samples) = clip(40);
    let (mut player, wakes) = player();
    player.play(shown(file));
    let (indices, _) = play_until(&mut player, &wakes, &samples, |seen| {
        matches!(seen, Seen::Showing(_))
    });
    let began = Instant::now();
    let mut shown = indices.len();
    while shown == 0 {
        assert!(began.elapsed() < PATIENCE, "a first picture");
        rest(&player, &wakes);
        shown += usize::from(player.due(Instant::now()).is_some());
    }
    assert!(
        !player.operate(DocumentId(1), ObjectId::new(9, 0), MediaOperation::Stop),
        "another annotation's stop reaches nothing"
    );
    assert!(player.operate(DocumentId(1), SCREEN, MediaOperation::Stop));
    let said = player.seen();
    assert!(
        matches!(said.as_slice(), [Seen::Stopped(stopped)] if stopped.starts_with("the clip: stopped after")),
        "{said:?}"
    );
    assert!(!player.holds_a_picture());
    std::thread::sleep(INTERVAL * 3);
    assert!(
        player.due(Instant::now()).is_none(),
        "no picture after a stop"
    );
    assert!(player.next_due(Instant::now()).is_none());
}

/// **A pause mid-clip holds its place**: while paused no picture comes however long the wait, and
/// a resume goes on from where the pause left it — no picture before that place is shown again —
/// to the clip's end.
#[test]
fn a_pause_mid_clip_holds_its_place_and_a_resume_goes_on() {
    let (file, samples) = clip(20);
    let (mut player, wakes) = player();
    player.play(shown(file));
    let mut before = Vec::new();
    let began = Instant::now();
    while before.len() < 3 {
        assert!(
            began.elapsed() < PATIENCE,
            "three pictures before the pause"
        );
        rest(&player, &wakes);
        if let Some(frame) = player.due(Instant::now()) {
            before.push((frame.at.as_millis() / INTERVAL.as_millis()) as usize);
        }
    }
    assert!(player.pause());
    std::thread::sleep(INTERVAL * 5);
    assert!(
        player.due(Instant::now()).is_none(),
        "no picture while paused"
    );
    assert!(player.next_due(Instant::now()).is_none());
    assert!(player.resume());
    let (after, said) = play_until(&mut player, &wakes, &samples, |seen| {
        matches!(seen, Seen::Ended(_) | Seen::Refused(_))
    });
    assert!(
        matches!(said.last(), Some(Seen::Ended(_))),
        "the resumed clip ended: {said:?}"
    );
    let paused_at = *before.last().unwrap();
    assert!(
        after.iter().all(|index| *index > paused_at),
        "nothing before the pause's place again: paused at {paused_at}, then {after:?}"
    );
}

/// **A section cuts the pictures through the real worker** (§13.2.4.3, ADR 3593): a clip of ten
/// pictures 40 ms apart with a section from 120 ms to 280 ms shows pictures 3 to 6 and no other —
/// the decode starts at the section's beginning and stops at its end — and its end is said with
/// those four. The wrong answer excluded: the whole clip shown from picture 0, as it was while
/// only the sound was cut.
#[test]
fn a_section_shows_only_its_pictures() {
    let (file, samples) = clip(10);
    let (mut player, wakes) = player();
    let mut cut = shown(file);
    cut.section = pdf_model::media::Section {
        begin: 0.12,
        end: Some(0.28),
    };
    player.play(cut);
    let (indices, said) = play_until(&mut player, &wakes, &samples, |seen| {
        matches!(seen, Seen::Ended(_) | Seen::Refused(_) | Seen::Stopped(_))
    });
    assert!(
        indices.iter().all(|index| (3..=6).contains(index)),
        "only the section's pictures: {indices:?}"
    );
    assert!(!indices.is_empty(), "a picture was shown; said {said:?}");
    let Some(Seen::Ended(ended)) = said.last() else {
        panic!("the clip ended: {said:?}");
    };
    let dropped = 4 - indices.len();
    assert_eq!(
        ended,
        &format!(
            "the clip: ended after {} pictures, {dropped} dropped late",
            indices.len()
        )
    );
}
