//! A clip's pictures shown in the GTK window, at the window: the screen annotation activated, the
//! level `open` playing it here, the confined media worker decoding `pdf-media-worker`'s hand-built
//! `I_PCM` stream, the decoding thread waking GTK's loop, each picture held up over the screen
//! annotation's rectangle as its samples converted, the clip's end taking it down, and Escape and a
//! rendition action's pause and stop reaching it (RFC 0015 section 6, round 6; ADR 3545).
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first. The clip has no sound track, so the sound beside its pictures is refused by name and its
//! pictures play on.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use pdf_model::view::WidgetAppearances;
use viewer_core::Command;
use viewer_host::sound::Sounds;
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

use super::{Host, Opening};

#[path = "../../../../tools/conformance/test-support/showing.rs"]
mod showing;

#[path = "../../../pdf-media-worker/tests/support/pcm_stream.rs"]
#[expect(
    dead_code,
    reason = "the fixture's other streams are `pdf-media-worker`'s own tests'"
)]
mod pcm_stream;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "pdf-viewer-gtk-video-{name}-{}",
        std::process::id()
    ))
}

/// The window's loop, turned until `done` holds or two minutes pass; every status line the window
/// says on the way is kept in `said`.
fn turn_until(
    host: &Rc<RefCell<Host>>,
    said: &RefCell<Vec<String>>,
    what: &str,
    done: impl Fn(&Host) -> bool,
) {
    let began = Instant::now();
    let context = glib::MainContext::default();
    loop {
        {
            let host = host.borrow();
            let line = host.ui.status.text().to_string();
            if said.borrow().last() != Some(&line) {
                said.borrow_mut().push(line);
            }
            if done(&host) {
                return;
            }
        }
        assert!(
            began.elapsed() < Duration::from_mins(2),
            "the window never {what}; it said {:?} (the worker must be built: `cargo build -p \
             pdf-media-worker --bins`)",
            said.borrow()
        );
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// The GTK window over the video document of `count` pictures at the level `open`, its sound
/// played into the recording double, the document taken in; and the pictures the clip carries.
fn a_window(
    document: &Path,
    count: u32,
) -> (Rc<RefCell<Host>>, Vec<pdf_media_worker::video::Picture>) {
    a_window_in(document, count, None)
}

/// [`a_window`] with the rendition's media screen parameters' entries `screen` where given
/// (ADR 3587).
fn a_window_in(
    document: &Path,
    count: u32,
    screen: Option<&str>,
) -> (Rc<RefCell<Host>>, Vec<pdf_media_worker::video::Picture>) {
    let (sets, units, pictures) =
        pcm_stream::stream(count, 16, 16, pcm_stream::Coding::Progressive);
    let clip = pcm_stream::mp4(&sets, &units, 16, 16);
    let bytes = match screen {
        Some(screen) => showing::a_video_document_in(&clip, screen),
        None => showing::a_video_document(&clip),
    };
    std::fs::write(document, bytes).expect("write the document");
    let opening = Opening::start(
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
    )
    .expect("the video document is on disk");
    let app = gtk4::Application::new(None::<&str>, gio::ApplicationFlags::NON_UNIQUE);
    app.register(None::<&gio::Cancellable>)
        .expect("an application without a name registers locally");
    let host = Host::open(&app, opening, Trace::off(Instant::now()));
    host.borrow_mut().resized(400, 400);
    let said = RefCell::new(Vec::new());
    turn_until(&host, &said, "took the opened document", |host| {
        host.state.anticipated.is_none() && host.state.awaiting.is_none()
    });
    // No audio device is wanted: the clip has no sound track, so its sound is refused before a
    // helper would be started, and a helper that is not there says so if it were.
    let missing = temporary("no-such-helper");
    host.borrow_mut().state.sounds = Sounds::opening_with(Box::new(move || {
        viewer_host::sound::helper::open_at(&missing)
    }));
    // As the launch tells the core where the helper is there, whatever was found beside the test
    // binary (ADRs 3417, 3545).
    host.borrow_mut().dispatch(Command::PlaysHere(true));
    host.borrow_mut().dispatch(Command::ShowsVideoHere(true));
    (host, pictures)
}

/// The picture the window holds up, as RGBA bytes, where one is up.
fn held_up(host: &Host) -> Option<Vec<u8>> {
    let widget = host.shown.widget()?;
    if !widget.is_visible() {
        return None;
    }
    let texture = widget.paintable()?.downcast::<gtk4::gdk::Texture>().ok()?;
    let downloader = gtk4::gdk::TextureDownloader::new(&texture);
    let mut downloader = downloader;
    downloader.set_format(gtk4::gdk::MemoryFormat::R8g8b8a8);
    let (bytes, _stride) = downloader.download_bytes();
    Some(bytes.to_vec())
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

/// **A clip plays to its end in the window**: a picture goes up over the screen annotation, each
/// picture held up is one of the clip's own samples converted, later pictures replace earlier ones,
/// and at the clip's end the window says so and takes the picture down. The wrong answers
/// excluded: the clip handed to this machine's player (a window that did not tell the core it shows
/// video), a picture of other pixels, and a last picture left up after the end.
#[gtk4::test]
fn a_clip_plays_to_its_end_in_the_gtk_window() {
    let document = temporary("ends.pdf");
    let (host, pictures) = a_window(&document, 40);
    let said = RefCell::new(Vec::new());
    host.borrow_mut()
        .dispatch(Command::Activate(showing::SCREEN));
    let seen = RefCell::new(Vec::new());
    turn_until(&host, &said, "ended the clip", |host| {
        if let Some(rgba) = held_up(host) {
            let index = which(&rgba, &pictures);
            assert!(index.is_some(), "the picture up is one of the clip's own");
            let mut seen = seen.borrow_mut();
            if seen.last() != index.as_ref() {
                seen.extend(index);
            }
        }
        said.borrow()
            .iter()
            .any(|line| line.contains("ended after"))
    });
    let seen = seen.into_inner();
    assert!(
        !seen.is_empty(),
        "a picture went up; said {:?}",
        said.borrow()
    );
    assert!(
        seen.windows(2).all(|pair| pair[0] < pair[1]),
        "later pictures replace earlier ones: {seen:?}"
    );
    assert!(
        said.borrow()
            .iter()
            .any(|line| line.contains("showing its pictures")),
        "{:?}",
        said.borrow()
    );
    assert!(
        held_up(&host.borrow()).is_none(),
        "the picture is taken down"
    );
    let _ = std::fs::remove_file(&document);
}

/// **A pause holds the picture, Space shows the clip again, and Escape stops it**: Table 218's
/// `/OP` 2 leaves the picture up and still, Space resumes it, and Escape takes it down and says
/// the clip stopped — before its end.
#[gtk4::test]
fn a_pause_and_escape_reach_the_clip_in_the_gtk_window() {
    let document = temporary("stops.pdf");
    let (host, _) = a_window(&document, 200);
    let said = RefCell::new(Vec::new());
    host.borrow_mut()
        .dispatch(Command::Activate(showing::SCREEN));
    turn_until(&host, &said, "held up a picture", |host| {
        held_up(host).is_some()
    });
    host.borrow_mut()
        .dispatch(Command::Activate(showing::PAUSE));
    turn_until(&host, &said, "paused the clip", |_| {
        said.borrow().iter().any(|line| line.ends_with(": paused"))
    });
    let still = held_up(&host.borrow());
    assert!(still.is_some(), "a paused clip keeps its picture up");
    let until = Instant::now()
        .checked_add(Duration::from_millis(200))
        .expect("a moment from now");
    turn_until(&host, &said, "waited", |_| Instant::now() >= until);
    assert_eq!(held_up(&host.borrow()), still, "and still");
    host.borrow_mut()
        .dispatch(Command::Activate(showing::RESUME));
    turn_until(&host, &said, "resumed the clip", |_| {
        said.borrow()
            .iter()
            .any(|line| line.contains("showing its pictures again"))
    });
    said.borrow_mut().clear();
    host.borrow_mut()
        .dispatch(Command::Activate(showing::PAUSE));
    turn_until(&host, &said, "paused the clip again", |_| {
        said.borrow().iter().any(|line| line.ends_with(": paused"))
    });
    assert!(
        host.borrow_mut()
            .key(gtk4::gdk::Key::space, viewer_host::Modifiers::default()),
        "Space takes a paused clip"
    );
    turn_until(&host, &said, "showed the clip again", |_| {
        said.borrow()
            .iter()
            .any(|line| line.contains("showing its pictures again"))
    });
    assert!(
        host.borrow_mut()
            .key(gtk4::gdk::Key::Escape, viewer_host::Modifiers::default()),
        "Escape takes a clip that shows"
    );
    turn_until(&host, &said, "stopped the clip", |_| {
        said.borrow()
            .iter()
            .any(|line| line.contains("stopped after"))
    });
    assert!(
        held_up(&host.borrow()).is_none(),
        "the picture is taken down"
    );
    host.borrow_mut().dispatch(Command::Activate(showing::STOP));
    turn_until(
        &host,
        &said,
        "said a stop with nothing associated reaches nothing",
        |_| said.borrow().iter().any(|line| line.contains("no effect")),
    );
    assert!(
        !said
            .borrow()
            .iter()
            .any(|line| line.contains("ended after")),
        "stopped before its end: {:?}",
        said.borrow()
    );
    let _ = std::fs::remove_file(&document);
}

/// The texture the clip's window of its own holds, as its width, height and RGBA bytes.
fn in_the_media_window(host: &Host) -> Option<(usize, usize, Vec<u8>)> {
    let open = host.shown.media_window.as_ref()?;
    let texture = open
        .picture()
        .paintable()?
        .downcast::<gtk4::gdk::Texture>()
        .ok()?;
    let mut downloader = gtk4::gdk::TextureDownloader::new(&texture);
    downloader.set_format(gtk4::gdk::MemoryFormat::R8g8b8a8);
    let (bytes, stride) = downloader.download_bytes();
    let width = usize::try_from(texture.width()).ok()?;
    let height = usize::try_from(texture.height()).ok()?;
    let row_bytes = width.saturating_mul(4);
    let mut rgba = Vec::with_capacity(row_bytes.saturating_mul(height));
    for row in bytes.chunks(stride).take(height) {
        rgba.extend_from_slice(&row[..row_bytes]);
    }
    Some((width, height, rgba))
}

/// **A floating window of its own** (ISO 32000-2 Table 294's `/W` 0, Table 295; ADR 3587): the
/// window opens titled by the document's `/TT` and the clip, `/D`'s 48 by 32 pixels, not
/// resizable (`/R`'s default 0); it shows Table 294's `/B` blue where the picture, met into it
/// (Table 291's default fit), does not reach — the 16-pixel-square picture scaled to 32 at the
/// left, the background at the right — and nothing over the page's rectangle; GTK 4 places no
/// window, which it says; Escape in the window stops the clip and the window closes.
///
/// Under a virtual display with one monitor and no window manager: where a window manager puts it
/// is not exercised.
#[gtk4::test]
fn a_floating_window_of_its_own_shows_the_clip_in_the_gtk_window() {
    let document = temporary("floating.pdf");
    let (host, _) = a_window_in(
        &document,
        40,
        Some("/MH << /W 0 /F << /D [48 32] /TT [() (A clip window)] >> >> /BE << /B [0 0 1] >>"),
    );
    let said = RefCell::new(Vec::new());
    host.borrow_mut()
        .dispatch(Command::Activate(showing::SCREEN));
    let drawn = RefCell::new(None);
    turn_until(&host, &said, "showed a picture in its own window", |host| {
        let picture = in_the_media_window(host).filter(|(width, _, rgba)| {
            let left = width.saturating_mul(16).saturating_add(8).saturating_mul(4);
            rgba[left..][..3] != [0, 0, 255]
        });
        let found = picture.is_some();
        *drawn.borrow_mut() = picture;
        found
    });
    let (width, height, rgba) = drawn.into_inner().expect("drawn");
    let at = |x: usize, y: usize| {
        &rgba[y.saturating_mul(width).saturating_add(x).saturating_mul(4)..][..4]
    };
    assert_eq!(
        at(width.saturating_sub(2), height.div_ceil(2)),
        &[0, 0, 255, 255],
        "/B where the picture is not"
    );
    {
        let host = host.borrow();
        let open = host.shown.media_window.as_ref().expect("open");
        let title = open.window().title().expect("a title").to_string();
        assert!(title.starts_with("A clip window — "), "{title}");
        assert!(
            title.contains("the clip"),
            "it says it holds a media clip: {title}"
        );
        assert_eq!(open.window().default_size(), (48, 32));
        assert!(!open.window().is_resizable());
        assert!(
            held_up(&host).is_none(),
            "nothing over the page's rectangle"
        );
    }
    assert!(
        said.borrow()
            .iter()
            .any(|line| line.contains("which the window manager places")),
        "{:?}",
        said.borrow()
    );
    assert!(host.borrow_mut().stop_the_pictures());
    assert!(
        host.borrow().shown.media_window.is_none(),
        "the window closes with the clip"
    );
    let _ = std::fs::remove_file(&document);
}

/// **A full-screen window** (Table 294's `/W` 1) on the monitor §13.2.7.5's specifier names —
/// Table 304's 4, the greatest area, here the virtual display's one monitor — said by name. A
/// specifier GDK cannot find (3, colour depth) is judged before any window, by `pdf-model`'s
/// `media_viability` tests against [`super::media_window::SCREENS`]' kind of host.
///
/// A second physical monitor is not exercised here: the choice between two is
/// `viewer_host::video::screens`' own tests'.
#[gtk4::test]
fn a_full_screen_window_goes_on_the_monitor_its_specifier_names_in_the_gtk_window() {
    let document = temporary("full.pdf");
    let (host, _) = a_window_in(&document, 40, Some("/MH << /W 1 /M 4 >>"));
    let said = RefCell::new(Vec::new());
    host.borrow_mut()
        .dispatch(Command::Activate(showing::SCREEN));
    turn_until(&host, &said, "opened its full-screen window", |host| {
        host.shown.media_window.is_some()
    });
    assert!(
        said.borrow()
            .iter()
            .any(|line| line.contains("shown full screen on") && line.contains("greatest area")),
        "{:?}",
        said.borrow()
    );
    assert!(host.borrow_mut().stop_the_pictures());
    let _ = std::fs::remove_file(&document);
}
