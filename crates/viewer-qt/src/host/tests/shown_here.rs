//! A clip's pictures shown in the Qt window: the screen annotation activated, the level `open`
//! playing it here, the confined media worker decoding `pdf-media-worker`'s hand-built `I_PCM`
//! stream, the window's timer looking at the clip with no wake from the decoding thread, each
//! picture handed to the frame sink over the screen annotation's rectangle as its samples
//! converted, the clip's end taking it down, and Escape, Space, Table 218's pause, resume and stop
//! and the document's close reaching it (RFC 0015 section 6, round 7; ADR 3562). Drives the Qt
//! host's Rust callbacks without a `QApplication`, as the window's timer and `keyPressEvent` call
//! them — `viewer-gtk`'s `shown_here`, in this window.
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first. The clip has no sound track, so the sound beside its pictures is refused by name and its
//! pictures play on.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::super::Host;
use pdf_model::view::WidgetAppearances;
use viewer_core::{Answer, Command, Query};
use viewer_host::sound::Sounds;
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

#[path = "../../../../../tools/conformance/test-support/showing.rs"]
mod showing;

#[path = "../../../../pdf-media-worker/tests/support/pcm_stream.rs"]
#[expect(
    dead_code,
    reason = "the fixture's other streams are `pdf-media-worker`'s own tests'"
)]
mod pcm_stream;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pdf-viewer-qt-video-{name}-{}", std::process::id()))
}

/// The Qt window over the video document of `count` pictures at the level `open`, page one
/// drawn; and the pictures the clip carries.
fn a_window(document: &Path, count: u32) -> (Host, Vec<pdf_media_worker::video::Picture>) {
    a_window_of(document, count, showing::a_video_document)
}

/// [`a_window`] over the document `of` makes of the clip.
fn a_window_of(
    document: &Path,
    count: u32,
    of: impl FnOnce(&[u8]) -> Vec<u8>,
) -> (Host, Vec<pdf_media_worker::video::Picture>) {
    let (sets, units, pictures) =
        pcm_stream::stream(count, 16, 16, pcm_stream::Coding::Progressive);
    let clip = pcm_stream::mp4(&sets, &units, 16, 16);
    std::fs::write(document, of(&clip)).expect("write the document");
    let mut host = Host::open(
        document,
        None,
        WidgetAppearances::Delegated,
        Settings {
            restrictions: viewer_core::RestrictionPolicy::uniform(
                viewer_core::RestrictionLevel::Off,
            ),
            links: viewer_host::Links::Refuse,
            remote_documents: viewer_host::RemoteDocuments::Refuse,
            embedded_documents: viewer_host::EmbeddedDocuments::Refuse,
            submissions: viewer_host::Submissions::Refuse,
            separations: false,
            scripts: Scripts::Off,
            launched_documents: viewer_host::LaunchedDocuments::Refuse,
            media_files: viewer_host::MediaFiles::Open,
        },
        ReaderWords::default(),
        Trace::off(Instant::now()),
    )
    .expect("the video document opens in the Qt host");
    host.resized(400, 400, 1.0);
    super::drawn(&mut host);
    // No audio device is wanted: the clip has no sound track, so its sound is refused before a
    // helper would be started, and a helper that is not there says so if it were.
    let missing = temporary("no-such-helper");
    host.state.sounds = Sounds::opening_with(Box::new(move || {
        viewer_host::sound::helper::open_at(&missing)
    }));
    // As the launch tells the core what this window does, whatever was found beside the test
    // binary (ADRs 3417, 3562).
    host.dispatch(Command::PlaysHere(true));
    host.dispatch(Command::ShowsVideoHere(true));
    (host, pictures)
}

/// The window's timer, as `MainWindow::pumpDrawing` arms it, turned until `done` holds or two
/// minutes pass; every sentence the window says on the way is kept in `said`. A timer that is not
/// armed is a window that would never look again, which fails.
fn timer_until(host: &mut Host, said: &mut Vec<String>, what: &str, done: impl Fn(&Host) -> bool) {
    let began = Instant::now();
    loop {
        if said.last() != Some(&host.message) {
            said.push(host.message.clone());
        }
        if done(host) {
            return;
        }
        assert!(
            began.elapsed() < Duration::from_mins(2),
            "the window never {what}; it said {said:?} (the worker must be built: `cargo build \
             -p pdf-media-worker --bins`)"
        );
        let wait = host.drawing_wait();
        assert!(
            wait >= 0,
            "the window's timer is armed while it waits to have {what}; it said {said:?}"
        );
        std::thread::sleep(Duration::from_millis(u64::from(wait.unsigned_abs())));
        host.drawing_pump();
    }
}

/// The picture the window hands the frame sink after the pages, as RGBA bytes, where one is up.
fn held_up(host: &Host) -> Option<Vec<u8>> {
    host.shown_picture()
        .map(|shown| shown.raster.into_owned().data)
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

/// **A clip plays to its end in the Qt window**: a picture goes to the frame sink over the screen
/// annotation's rectangle, each picture is one of the clip's own samples converted, later pictures
/// replace earlier ones, and at the clip's end the window says so and takes the picture down. The
/// wrong answers excluded: the clip handed to this machine's player (a window that did not tell
/// the core it shows video), a timer left unarmed while the decoding thread works (nothing wakes
/// this window), a picture of other pixels or somewhere else, and a last picture left up.
#[test]
fn a_clip_plays_to_its_end_in_the_qt_window() {
    let document = temporary("ends.pdf");
    let (mut host, pictures) = a_window(&document, 40);
    let mut said = Vec::new();
    host.dispatch(Command::Activate(showing::SCREEN));
    let seen = std::cell::RefCell::new(Vec::new());
    let placed = std::cell::Cell::new(false);
    timer_until(&mut host, &mut said, "ended the clip", |host| {
        if let Some(shown) = host.shown_picture() {
            let index = which(&shown.raster.data, &pictures);
            assert!(index.is_some(), "the picture up is one of the clip's own");
            // Over the screen annotation: meet fits the square picture inside its rectangle.
            let Answer::Screen(quad) = host.state.viewer.query(Query::Screen(showing::SCREEN))
            else {
                panic!("the screen annotation is on the page placed");
            };
            let ((left, top), _) = super::super::pictures::bounding_box(quad);
            assert!(
                shown.origin.0 >= left - 0.5 && shown.origin.1 >= top - 0.5,
                "the picture stands inside the rectangle: {:?} in {quad:?}",
                shown.origin
            );
            assert!(
                shown.extent.0 > 0 && shown.extent.1 > 0,
                "and has an extent"
            );
            placed.set(true);
            let mut seen = seen.borrow_mut();
            if seen.last() != index.as_ref() {
                seen.extend(index);
            }
        }
        host.message.contains("ended after")
    });
    let seen = seen.into_inner();
    assert!(
        placed.get() && !seen.is_empty(),
        "a picture went up; said {said:?}"
    );
    assert!(
        seen.windows(2).all(|pair| pair[0] < pair[1]),
        "later pictures replace earlier ones: {seen:?}"
    );
    assert!(
        said.iter()
            .any(|line| line.contains("showing its pictures")),
        "{said:?}"
    );
    assert!(held_up(&host).is_none(), "the picture is taken down");
    assert_eq!(
        host.drawing_wait(),
        -1,
        "nothing left to look at, so the timer stops"
    );
    let _ = std::fs::remove_file(&document);
}

/// **A pause holds the picture, Space shows the clip again, Escape stops it, and the document's
/// close ends it**: Table 218's `/OP` 2 leaves the picture up and still, `/OP` 3 and Space resume
/// it, Escape takes it down and says the clip stopped — before its end — and a stop with nothing
/// associated says it reached nothing.
#[test]
fn a_pause_space_escape_and_close_reach_the_clip_in_the_qt_window() {
    let document = temporary("stops.pdf");
    let (mut host, _) = a_window(&document, 200);
    let mut said = Vec::new();
    host.dispatch(Command::Activate(showing::SCREEN));
    timer_until(&mut host, &mut said, "held up a picture", |host| {
        held_up(host).is_some()
    });
    host.dispatch(Command::Activate(showing::PAUSE));
    timer_until(&mut host, &mut said, "paused the clip", |host| {
        host.message.ends_with(": paused")
    });
    let still = held_up(&host);
    assert!(still.is_some(), "a paused clip keeps its picture up");
    std::thread::sleep(Duration::from_millis(200));
    host.drawing_pump();
    assert_eq!(held_up(&host), still, "and still");
    host.dispatch(Command::Activate(showing::RESUME));
    timer_until(&mut host, &mut said, "resumed the clip", |host| {
        host.message.contains("showing its pictures again")
    });
    host.dispatch(Command::Activate(showing::PAUSE));
    timer_until(&mut host, &mut said, "paused the clip again", |host| {
        host.message.ends_with(": paused")
    });
    host.key(crate::keys::SPACE, false, false);
    timer_until(
        &mut host,
        &mut said,
        "showed the clip again on Space",
        |host| host.message.contains("showing its pictures again"),
    );
    host.key(crate::keys::ESCAPE, false, false);
    timer_until(&mut host, &mut said, "stopped the clip on Escape", |host| {
        host.message.contains("stopped after")
    });
    assert!(held_up(&host).is_none(), "the picture is taken down");
    host.dispatch(Command::Activate(showing::STOP));
    timer_until(
        &mut host,
        &mut said,
        "said a stop with nothing associated reaches nothing",
        |host| host.message.contains("no effect"),
    );
    assert!(
        !said.iter().any(|line| line.contains("ended after")),
        "stopped before its end: {said:?}"
    );

    // Again, and the document's close ends it: its last document, so the window goes too.
    host.dispatch(Command::Activate(showing::SCREEN));
    timer_until(&mut host, &mut said, "held up a picture again", |host| {
        held_up(host).is_some()
    });
    host.close_document();
    assert!(
        !host.state.pictures.in_hand(),
        "the document's close ends its clip"
    );
    assert!(held_up(&host).is_none(), "and takes its picture down");
    let _ = std::fs::remove_file(&document);
}

/// ISO 32000-2 §13.4's Table 306 `/Poster` `true` in the Qt window: once the window says it shows
/// video, the movie's first picture is decoded beside it and handed to the frame sink over the
/// movie annotation as its samples converted (ADR 3609); activated, the movie plays in its place
/// and no poster is handed, and a press on Table 307's bar pauses it (ADR 3610).
#[test]
fn a_movie_s_poster_is_handed_until_it_plays_in_the_qt_window() {
    let document = temporary("poster.pdf");
    let (mut host, pictures) = a_window_of(&document, 8, |clip| {
        showing::a_movie_document(clip, "/Poster true", "<< /ShowControls true /Mode /Open >>")
    });
    let mut said = Vec::new();
    timer_until(&mut host, &mut said, "handed the poster", |host| {
        !host.shown_posters().is_empty()
    });
    let poster = host.shown_posters().remove(0).raster.into_owned().data;
    assert_eq!(
        which(&poster, &pictures),
        Some(0),
        "the movie's first picture"
    );
    host.dispatch(Command::Activate(showing::MOVIE));
    timer_until(
        &mut host,
        &mut said,
        "played the movie in its place",
        |host| held_up(host).is_some() && host.shown_posters().is_empty(),
    );
    let Answer::Screen(quad) = host.state.viewer.query(Query::Screen(showing::MOVIE)) else {
        panic!("the movie annotation is on the page placed");
    };
    let left = quad[0].min(quad[2]).min(quad[4]).min(quad[6]);
    let bottom = quad[1].max(quad[3]).max(quad[5]).max(quad[7]);
    assert!(host.press_the_bar((left + 5.0, bottom - 5.0)));
    assert!(
        host.state
            .pictures
            .controller(Instant::now())
            .is_some_and(|bar| !bar.playing),
        "the bar's button paused the movie"
    );
    let _ = std::fs::remove_file(&document);
}

/// Whether the page the core last interpreted reports Table 344's snapshot drawn in place of the
/// rich media annotation's appearance.
fn snapshot_drawn(host: &Host) -> bool {
    let Answer::Reports(reports) = host.state.viewer.query(Query::Reports) else {
        return false;
    };
    reports
        .iter()
        .flat_map(|page| page.notes.iter())
        .any(|note| note.contains(showing::SNAPSHOT_SHOWN))
}

/// **A rich media snapshot stands while the clip plays in the Qt window** (ISO 32000-2
/// §13.7.2.3.5, ADR 3889): a rich media annotation's `Video` configuration activated plays in its
/// rectangle, the window tells the core its player holds the clip, and the page is drawn with
/// Table 344's image in place of the appearance until the clip has ended, when the window tells
/// the core the clip has left its player.
#[test]
fn a_rich_media_snapshot_stands_while_the_clip_plays_in_the_qt_window() {
    let document = temporary("snapshot.pdf");
    let (mut host, _) = a_window_of(&document, 12, showing::a_rich_media_document);
    let mut said = Vec::new();
    assert!(!snapshot_drawn(&host), "the appearance before");
    host.dispatch(Command::Activate(showing::RICH_MEDIA));
    timer_until(&mut host, &mut said, "drawn the snapshot", snapshot_drawn);
    timer_until(&mut host, &mut said, "ended the clip", |host| {
        !snapshot_drawn(host)
    });
    assert!(
        said.iter().any(|line| line.contains("ended after"))
            || host.message.contains("ended after"),
        "the snapshot went with the clip's end: {said:?}"
    );
    let _ = std::fs::remove_file(&document);
}

/// **Table 291's `/F 3` scrolls in the Qt window** (ADR 3901): a clip whose `MH` must honour `/F
/// 3` plays here, and its sixteen-sample picture, unscaled over a rectangle two points square,
/// overflows it; the wheel with the pointer over the rectangle moves the part shown and leaves the
/// page where it was, and the picture handed to the frame sink is the part. The wrong answers
/// excluded: the clip handed off as non-viable, and the wheel scrolling the page under the
/// picture.
#[test]
fn an_overflowing_scroll_clip_is_scrolled_by_the_wheel_in_the_qt_window() {
    let document = temporary("scrolls.pdf");
    let (mut host, _) = a_window_of(&document, 200, showing::a_scrolling_video_document);
    let mut said = Vec::new();
    host.dispatch(Command::Activate(showing::SCREEN));
    timer_until(&mut host, &mut said, "handed the clip's picture", |host| {
        held_up(host).is_some()
    });
    let screen = |host: &Host| {
        let Answer::Screen(quad) = host.state.viewer.query(Query::Screen(showing::SCREEN)) else {
            panic!("the screen annotation is on the page placed");
        };
        quad
    };
    let quad = screen(&host);
    let before = host.state.pictures.placed(quad).expect("placed");
    assert_eq!((before.left, before.top), (0, 0));
    assert!(
        before.right < 16 && before.bottom < 16,
        "unscaled and cut: {before:?}"
    );
    host.pointer(
        f32::midpoint(quad[0], quad[4]),
        f32::midpoint(quad[1], quad[5]),
        0,
    );
    // The boundary's travel is turned towards the person: negative is down and to the right.
    host.scrolled(-3.0, -2.0, true);
    let after = host.state.pictures.placed(quad).expect("placed");
    assert!(after.left > 0 && after.top > 0, "the part moved: {after:?}");
    assert_eq!(screen(&host), quad, "the page did not scroll");
    let shown = host.shown_picture().expect("a picture is handed");
    assert_eq!(
        (shown.raster.width, shown.raster.height),
        (after.right - after.left, after.bottom - after.top)
    );
    let _ = std::fs::remove_file(&document);
}
