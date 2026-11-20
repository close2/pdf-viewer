//! §13.3's sound played in the Qt window: the sound annotation activated, the level `open` playing
//! it here, the confined media worker decoding it, the window's timer taking in what the player
//! said, the title naming Escape, Escape stopping it, the document's close ending it, and a missing
//! sound helper said in the window (RFC 0015 section 5, step 4; ADR 3329). Drives the Qt host's
//! Rust callbacks without a `QApplication`, as the window's timer and `keyPressEvent` call them.
//!
//! The audio device is a recording double (`sounding.rs` in `tools/conformance/test-support`):
//! no person has heard a sound through this window. The worker is the program beside the test
//! binary: `cargo build -p pdf-media-worker --bins` first (trap 10's construction).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::super::Host;
use pdf_model::view::WidgetAppearances;
use viewer_core::Command;
use viewer_host::sound::Sounds;
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

#[path = "../../../../../tools/conformance/test-support/sounding.rs"]
mod sounding;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pdf-viewer-qt-{name}-{}", std::process::id()))
}

/// The Qt window over the sounding document at the level `open`, with `sounds` in place of the
/// helper's, page one drawn.
fn a_window(document: &Path, sounds: Sounds) -> Host {
    std::fs::write(document, sounding::a_sounding_document()).expect("write the document");
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
    .expect("the sounding document opens in the Qt host");
    host.resized(400, 400, 1.0);
    super::drawn(&mut host);
    host.state.sounds = sounds;
    host
}

/// The window's timer, as `MainWindow::pumpDrawing` arms it, turned until `done` holds or two
/// minutes pass. A timer that is not armed is a window that would never look again, which fails.
fn timer_until(host: &mut Host, what: &str, done: impl Fn(&Host) -> bool) {
    let began = Instant::now();
    while !done(host) {
        assert!(
            began.elapsed() < Duration::from_mins(2),
            "the window never {what}"
        );
        let wait = host.drawing_wait();
        assert!(
            wait >= 0,
            "the window's timer is armed while it waits to have {what}"
        );
        std::thread::sleep(Duration::from_millis(u64::from(wait.unsigned_abs())));
        host.drawing_pump();
    }
}

/// **The sound reaches the device as §13.3's arithmetic gives it, the window's timer takes the
/// player's word in, the title names the key that stops it, and Escape does.** The wrong answers
/// excluded: the WAVE file handed to this machine's player instead, a device opened at the
/// launch, a timer left unarmed while the sound is decoded (the window would never say it), and
/// Escape left to the key table while a sound sounds. A second sound and the document's close
/// each stop the one sounding.
#[test]
fn a_sound_plays_in_the_qt_window_and_escape_stops_it() {
    let document = temporary("sounding.pdf");
    let (open, record) = sounding::recording();
    let mut host = a_window(&document, Sounds::opening_with(open));
    assert!(
        !host.state.sounds.started(),
        "nothing is made before the first sound"
    );
    assert_eq!(sounding::record(&record).opened, 0, "no device at launch");

    host.dispatch(Command::Activate(sounding::ANNOTATION));
    timer_until(&mut host, "said the sound was sounding", |host| {
        host.title().contains("playing a sound, Escape stops it")
    });
    {
        let record = sounding::record(&record);
        assert_eq!(record.opened, 1, "the device opened once, at this sound");
        assert_eq!(record.started, [sounding::SAMPLES.to_vec()]);
    }

    // Escape, as the find bar's close action hands it over.
    host.key(crate::keys::ESCAPE, false, false);
    timer_until(&mut host, "said the sound was stopped", |host| {
        !host.title().contains("playing a sound")
    });
    assert!(
        sounding::record(&record).stops >= 1,
        "the device was stopped"
    );
    assert!(
        host.message.contains("stopped"),
        "the window says so: {}",
        host.message
    );
    assert_eq!(
        host.drawing_wait(),
        -1,
        "nothing left to look at, so the timer stops"
    );

    // Again, and the document's close stops it: its last document, so the window goes too.
    host.dispatch(Command::Activate(sounding::ANNOTATION));
    timer_until(&mut host, "said the second sound was sounding", |host| {
        host.title().contains("playing a sound")
    });
    host.close_document();
    assert!(
        host.closing_the_window(),
        "the last document takes the window"
    );
    assert!(
        !host.state.sounds.started(),
        "the window's close ends the player"
    );
    let _ = std::fs::remove_file(&document);
}

/// **A missing sound helper is said in the window, and the window goes on**: the sentence names
/// the helper and how to build it, in the status line and the title, and the timer stops.
#[test]
fn a_missing_sound_helper_is_said_in_the_qt_window() {
    let document = temporary("helperless.pdf");
    let missing = temporary("no-such-helper");
    let mut host = a_window(
        &document,
        Sounds::opening_with(Box::new(move || {
            viewer_host::sound::helper::open_at(&missing)
        })),
    );
    host.dispatch(Command::Activate(sounding::ANNOTATION));
    timer_until(&mut host, "said the sound was not played", |host| {
        host.title().contains("no-such-helper")
    });
    assert!(
        host.message.contains("no-such-helper"),
        "the status line says it: {}",
        host.message
    );
    assert_eq!(
        host.drawing_wait(),
        -1,
        "a refused sound leaves the timer nothing"
    );
    let _ = std::fs::remove_file(&document);
}
