//! A clip's pictures in a window driven from C: the screen annotation activated, the level `open`
//! playing it here, the confined media worker decoding `pdf-media-worker`'s hand-built `I_PCM`
//! stream, the host's timer at `quorra_window_interval` looking at the clip through
//! `quorra_window_tick` with no wake from the decoding thread, each picture read through
//! `quorra_window_picture` and `quorra_window_picture_copy` over the screen annotation's rectangle
//! as its samples converted, a `QUORRA_REPAINT_PICTURE` record for each change, the clip's end
//! taking it down, and Escape, Space, Table 218's pause, resume and stop and the document's close
//! reaching it (RFC 0015 section 6, round 8; ADR 3578) — `viewer-qt`'s `shown_here`, in this
//! window.
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first. The clip has no sound track, so the sound beside its pictures is refused by name and its
//! pictures play on.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use viewer_core::{Answer, Command, Query};
use viewer_host::native_window::NativeWindow as _;
use viewer_host::sound::Sounds;
use viewer_host::{ActLevel, MediaFiles};

use super::{OwedKind, Repainted, Window};
use crate::Status;

#[path = "../../../../tools/conformance/test-support/showing.rs"]
mod showing;

#[path = "../../../pdf-media-worker/tests/support/pcm_stream.rs"]
#[expect(
    dead_code,
    reason = "the fixture's other streams are `pdf-media-worker`'s own tests'"
)]
mod pcm_stream;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("quorra-ffi-video-{name}-{}", std::process::id()))
}

/// The window over the video document of `count` pictures at the level `open`, page one drawn,
/// its host having said it draws pictures; and the pictures the clip carries.
fn a_window(document: &Path, count: u32) -> (Window, Vec<pdf_media_worker::video::Picture>) {
    a_window_over(document, count, showing::a_video_document)
}

/// [`a_window`] over the document `built` makes of the clip.
fn a_window_over(
    document: &Path,
    count: u32,
    built: fn(&[u8]) -> Vec<u8>,
) -> (Window, Vec<pdf_media_worker::video::Picture>) {
    let (sets, units, pictures) =
        pcm_stream::stream(count, 16, 16, pcm_stream::Coding::Progressive);
    let clip = pcm_stream::mp4(&sets, &units, 16, 16);
    std::fs::write(document, built(&clip)).expect("write the document");
    let mut window = Window::open_hearing(
        document,
        None,
        viewer_host::policy::Words::default(),
        viewer_host::sound::Hearing::HereWithPictures,
    )
    .expect("the video document opens");
    window.resize(400, 400, 1.0).expect("resize");
    rest(&mut window);
    window.state.restrictions = window
        .state
        .restrictions
        .with(ActLevel::MediaFiles(MediaFiles::Open));
    // No audio device is wanted: the clip has no sound track, so its sound is refused before a
    // helper would be started, and a helper that is not there says so if it were.
    let missing = temporary("no-such-helper");
    window.state.sounds = Sounds::opening_with(Box::new(move || {
        viewer_host::sound::helper::open_at(&missing)
    }))
    .heard_by(viewer_host::sound::Hearing::HereWithPictures);
    let _ = window.take_owed();
    (window, pictures)
}

/// The host's timer until nothing beside the window runs.
fn rest(window: &mut Window) {
    for _ in 0..20_000 {
        if window.interval().is_none() {
            return;
        }
        window.tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("the window never came to rest");
}

/// What a host carries out of the records: every sentence said, and how many repaints said the
/// picture changed.
#[derive(Default)]
struct Host {
    said: Vec<String>,
    picture_repaints: usize,
}

impl Host {
    fn take(&mut self, window: &mut Window) {
        let owed = window.take_owed();
        for index in 0..owed.len() {
            match owed.kind(index) {
                Some(OwedKind::Said) => {
                    self.said
                        .push(owed.text(index).unwrap_or_default().to_owned());
                }
                Some(OwedKind::Repaint) => {
                    let changed = owed.repaint(index).expect("a repaint").changed();
                    if changed & Repainted::PICTURE != 0 {
                        self.picture_repaints = self.picture_repaints.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
    }

    fn last(&self) -> &str {
        self.said.last().map_or("", String::as_str)
    }
}

/// The host's timer, armed at `quorra_window_interval`, turned until `done` holds or two minutes
/// pass. A timer that is not armed is a host that would never call again, which fails.
fn timer_until(
    window: &mut Window,
    host: &mut Host,
    what: &str,
    done: impl Fn(&Window, &Host) -> bool,
) {
    let began = Instant::now();
    loop {
        host.take(window);
        if done(window, host) {
            return;
        }
        assert!(
            began.elapsed() < Duration::from_mins(2),
            "the window never {what}; it said {:?} (the worker must be built: `cargo build -p \
             pdf-media-worker --bins`)",
            host.said
        );
        let wait = window.interval();
        assert!(
            wait.is_some(),
            "the host's timer is armed while it waits to have {what}; it said {:?}",
            host.said
        );
        std::thread::sleep(Duration::from_millis(u64::from(wait.unwrap_or(0))));
        window.tick();
    }
}

/// The picture as the host copies it, where one is up.
fn held_up(window: &Window) -> Option<Vec<u8>> {
    let info = window.picture().ok()?;
    let mut pixels = vec![0; info.bytes];
    let written = window
        .picture_copy(&mut pixels)
        .expect("the copy the facts sized");
    assert_eq!(
        written, info.bytes,
        "the copy writes the bytes the facts said"
    );
    Some(pixels)
}

/// Which of the clip's pictures `rgba` is: the one whose samples, converted by the oracle, are
/// those pixels.
fn which(rgba: &[u8], pictures: &[pdf_media_worker::video::Picture]) -> Option<usize> {
    pictures.iter().position(|samples| {
        let planes = pdf_render::video::Planes {
            width: 16,
            height: 16,
            y: &samples.y,
            cb: &samples.u,
            cr: &samples.v,
        };
        let conversion =
            pdf_render::video::Conversion::for_signal(pdf_render::video::Signal::default(), 16)
                .expect("the default signal converts");
        render_cpu::video::convert_picture(&planes, &conversion)
            .is_ok_and(|raster| raster.data == rgba)
    })
}

/// **A clip plays to its end in a window driven from C**: a picture is read over the screen
/// annotation's rectangle, each picture is one of the clip's own samples converted, later pictures
/// replace earlier ones, each change is a `QUORRA_REPAINT_PICTURE`, and at the clip's end the
/// window says so and takes the picture down. The wrong answers excluded: the clip handed to the
/// host's opener (a window whose host did not say it draws pictures), a timer left unarmed while
/// the decoding thread works (nothing wakes the host), a picture of other pixels or somewhere
/// else, a copy into a short buffer, and a last picture left up.
#[test]
fn a_clip_plays_to_its_end_in_a_window_driven_from_c() {
    let document = temporary("ends.pdf");
    let (mut window, pictures) = a_window(&document, 40);
    let mut host = Host::default();
    window.dispatch(Command::Activate(showing::SCREEN));
    let seen = std::cell::RefCell::new(Vec::new());
    timer_until(&mut window, &mut host, "ended the clip", |window, host| {
        if let Some(rgba) = held_up(window) {
            let index = which(&rgba, &pictures);
            assert!(index.is_some(), "the picture up is one of the clip's own");
            let info = window.picture().expect("the picture is up");
            // Over the screen annotation: meet fits the square picture inside its rectangle.
            let Answer::Screen(quad) = window.state.viewer.query(Query::Screen(showing::SCREEN))
            else {
                panic!("the screen annotation is on the page placed");
            };
            let left = quad[0].min(quad[2]).min(quad[4]).min(quad[6]);
            let top = quad[1].min(quad[3]).min(quad[5]).min(quad[7]);
            let (x, y) = info.origin();
            assert!(
                x >= left - 0.5 && y >= top - 0.5,
                "the picture stands inside the rectangle: {:?} in {quad:?}",
                info.origin()
            );
            assert!(info.extent.0 > 0 && info.extent.1 > 0, "and has an extent");
            assert_eq!(
                window.picture_copy(&mut [0; 4]),
                Err(Status::BufferTooSmall),
                "a short buffer is refused, nothing written"
            );
            let mut seen = seen.borrow_mut();
            if seen.last() != index.as_ref() {
                seen.extend(index);
            }
        }
        host.last().contains("ended after")
    });
    let seen = seen.into_inner();
    assert!(!seen.is_empty(), "a picture went up; said {:?}", host.said);
    assert!(
        seen.windows(2).all(|pair| pair[0] < pair[1]),
        "later pictures replace earlier ones: {seen:?}"
    );
    assert!(
        host.picture_repaints > seen.len(),
        "each picture up and its taking down was a QUORRA_REPAINT_PICTURE: {} for {seen:?}",
        host.picture_repaints
    );
    assert!(
        host.said
            .iter()
            .any(|line| line.contains("showing its pictures")),
        "{:?}",
        host.said
    );
    assert_eq!(
        window.picture(),
        Err(Status::NoAnswer),
        "the picture is taken down"
    );
    assert_eq!(
        window.interval(),
        None,
        "nothing left to look at, so the timer stops"
    );
    let _ = std::fs::remove_file(&document);
}

/// **A pause holds the picture, Space shows the clip again, Escape stops it, and the document's
/// close ends it**: Table 218's `/OP` 2 leaves the picture up and still, `/OP` 3 and Space resume
/// it, Escape takes it down and says the clip stopped — before its end — and a stop with nothing
/// associated says it reached nothing.
#[test]
fn a_pause_space_escape_and_close_reach_the_clip_in_a_window_driven_from_c() {
    let document = temporary("stops.pdf");
    let (mut window, _) = a_window(&document, 200);
    let mut host = Host::default();
    let none = viewer_host::Modifiers::default();
    window.dispatch(Command::Activate(showing::SCREEN));
    timer_until(&mut window, &mut host, "held up a picture", |window, _| {
        held_up(window).is_some()
    });
    window.dispatch(Command::Activate(showing::PAUSE));
    timer_until(&mut window, &mut host, "paused the clip", |_, host| {
        host.last().ends_with(": paused")
    });
    let still = held_up(&window);
    assert!(still.is_some(), "a paused clip keeps its picture up");
    std::thread::sleep(Duration::from_millis(200));
    window.tick();
    assert_eq!(held_up(&window), still, "and still");
    window.dispatch(Command::Activate(showing::RESUME));
    timer_until(&mut window, &mut host, "resumed the clip", |_, host| {
        host.last().contains("showing its pictures again")
    });
    window.dispatch(Command::Activate(showing::PAUSE));
    timer_until(
        &mut window,
        &mut host,
        "paused the clip again",
        |_, host| host.last().ends_with(": paused"),
    );
    window.press(viewer_host::Key::Space, none);
    timer_until(
        &mut window,
        &mut host,
        "showed the clip again on Space",
        |_, host| host.last().contains("showing its pictures again"),
    );
    window.press(viewer_host::Key::Escape, none);
    timer_until(
        &mut window,
        &mut host,
        "stopped the clip on Escape",
        |_, host| host.last().contains("stopped after"),
    );
    assert!(held_up(&window).is_none(), "the picture is taken down");
    window.dispatch(Command::Activate(showing::STOP));
    timer_until(
        &mut window,
        &mut host,
        "said a stop with nothing associated reaches nothing",
        |_, host| host.last().contains("no effect"),
    );
    assert!(
        !host.said.iter().any(|line| line.contains("ended after")),
        "stopped before its end: {:?}",
        host.said
    );

    // Again, and the document's close ends it: its last document, so the window goes too.
    window.dispatch(Command::Activate(showing::SCREEN));
    timer_until(
        &mut window,
        &mut host,
        "held up a picture again",
        |window, _| held_up(window).is_some(),
    );
    assert!(window.close_document(), "its last document");
    assert!(
        !window.state.pictures.in_hand(),
        "the document's close ends its clip"
    );
    assert!(held_up(&window).is_none(), "and takes its picture down");
    let _ = std::fs::remove_file(&document);
}

/// **A host that does not say it draws pictures is handed the clip**, as before (ADR 3217): no
/// picture is read and the clip's file is a `QUORRA_OWED_OPEN_FILE` record.
#[test]
fn a_host_that_draws_no_pictures_is_handed_the_clip() {
    let document = temporary("handed.pdf");
    let (sets, units, _) = pcm_stream::stream(4, 16, 16, pcm_stream::Coding::Progressive);
    let clip = pcm_stream::mp4(&sets, &units, 16, 16);
    std::fs::write(&document, showing::a_video_document(&clip)).expect("write the document");
    let mut window =
        Window::open(&document, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    rest(&mut window);
    window.state.restrictions = window
        .state
        .restrictions
        .with(ActLevel::MediaFiles(MediaFiles::Open));
    let _ = window.take_owed();
    window.dispatch(Command::Activate(showing::SCREEN));
    let owed = window.take_owed();
    let kinds: Vec<_> = (0..owed.len())
        .filter_map(|index| owed.kind(index))
        .collect();
    assert!(
        kinds.contains(&OwedKind::OpenFile),
        "the clip goes to the host's opener: {kinds:?}"
    );
    assert_eq!(
        window.picture(),
        Err(Status::NoAnswer),
        "and no picture is up"
    );
    assert_eq!(window.picture_wait(), None, "nor looked for");
    let _ = std::fs::remove_file(&document);
}

/// ISO 32000-2 §13.4's Table 306 `/Poster` `true` in a window driven from C: the page placed, the
/// movie's first picture is decoded beside the window and read through `quorra_window_picture` as
/// its samples converted, with a `QUORRA_REPAINT_PICTURE` when it comes up (ADR 3609). Activated,
/// the movie plays with Table 307's bar, and a press through `quorra_window_pointer` on the bar's
/// button pauses it and never reaches the page (ADR 3610); `quorra_window_controller` says so.
#[test]
fn a_movie_s_poster_and_its_bar_reach_a_window_driven_from_c() {
    let document = temporary("poster.pdf");
    let (sets, units, pictures) = pcm_stream::stream(8, 16, 16, pcm_stream::Coding::Progressive);
    let clip = pcm_stream::mp4(&sets, &units, 16, 16);
    std::fs::write(
        &document,
        showing::a_movie_document(
            &clip,
            "/Poster true",
            "<< /ShowControls true /Mode /Open >>",
        ),
    )
    .expect("write the document");
    let mut window = Window::open_hearing(
        &document,
        None,
        viewer_host::policy::Words::default(),
        viewer_host::sound::Hearing::HereWithPictures,
    )
    .expect("the movie document opens");
    window.state.restrictions = window
        .state
        .restrictions
        .with(ActLevel::MediaFiles(MediaFiles::Open));
    let missing = temporary("no-such-helper-poster");
    window.state.sounds = Sounds::opening_with(Box::new(move || {
        viewer_host::sound::helper::open_at(&missing)
    }))
    .heard_by(viewer_host::sound::Hearing::HereWithPictures);
    window.resize(400, 400, 1.0).expect("resize");
    let mut host = Host::default();
    timer_until(&mut window, &mut host, "put the poster up", |window, _| {
        held_up(window).is_some()
    });
    let rgba = held_up(&window).expect("the poster");
    assert_eq!(
        which(&rgba, &pictures),
        Some(0),
        "the movie's first picture"
    );
    assert!(host.picture_repaints > 0, "a repaint said it came up");
    assert_eq!(
        window.controller().err(),
        Some(Status::NoAnswer),
        "a poster has no bar"
    );
    window.dispatch(Command::Activate(showing::MOVIE));
    timer_until(&mut window, &mut host, "played the movie", |window, _| {
        window.controller().is_ok_and(|bar| bar.playing)
    });
    let Answer::Screen(quad) = window.state.viewer.query(Query::Screen(showing::MOVIE)) else {
        panic!("the movie annotation is on the page placed");
    };
    let left = quad[0].min(quad[2]).min(quad[4]).min(quad[6]);
    let bottom = quad[1].max(quad[3]).max(quad[5]).max(quad[7]);
    let over = window.pointer(
        (left + 5.0, bottom - 5.0),
        viewer_core::PointerAction::Pressed,
    );
    assert!(!over, "the bar is not a link");
    assert!(
        window.controller().is_ok_and(|bar| !bar.playing),
        "the button paused the movie"
    );
    let _ = std::fs::remove_file(&document);
}

/// Whether the page the core last interpreted reports Table 344's snapshot drawn in place of the
/// rich media annotation's appearance.
fn snapshot_drawn(window: &Window) -> bool {
    let Answer::Reports(reports) = window.state.viewer.query(Query::Reports) else {
        return false;
    };
    reports
        .iter()
        .flat_map(|page| page.notes.iter())
        .any(|note| note.contains(showing::SNAPSHOT_SHOWN))
}

/// ISO 32000-2 §13.7.2.3.5's snapshot in a window driven from C: a rich media annotation's `Video`
/// configuration activated plays in its rectangle, the window tells the core its player holds the
/// clip, and the page is drawn with Table 344's image in place of the annotation's appearance
/// until the clip has ended, when the window tells the core it has left the player's hands and the
/// appearance is drawn again (ADR 3889).
#[test]
fn a_rich_media_snapshot_stands_while_the_clip_plays_in_a_window_driven_from_c() {
    let document = temporary("snapshot.pdf");
    let (mut window, _) = a_window_over(&document, 12, showing::a_rich_media_document);
    let mut host = Host::default();
    assert!(!snapshot_drawn(&window), "the appearance before activation");
    window.dispatch(Command::Activate(showing::RICH_MEDIA));
    timer_until(&mut window, &mut host, "drawn the snapshot", |window, _| {
        snapshot_drawn(window)
    });
    timer_until(&mut window, &mut host, "ended the clip", |_, host| {
        host.last().contains("ended after")
    });
    rest(&mut window);
    assert!(
        !snapshot_drawn(&window),
        "the appearance once the clip has left the player; said {:?}",
        host.said
    );
    let _ = std::fs::remove_file(&document);
}
