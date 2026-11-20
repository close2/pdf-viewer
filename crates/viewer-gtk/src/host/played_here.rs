//! §13.3's sound played in the GTK window, at the window: the sound annotation activated, the
//! level `open` playing it here, the confined media worker decoding it, the player's thread waking
//! GTK's loop, the title naming Escape, Escape stopping it, and a missing sound helper said in the
//! window (RFC 0015 section 5, step 3; ADR 3329).
//!
//! **The window is the one `quorra-gtk` builds** (ADR 3098), on a registered application that is
//! never run, as `trusted_lists_shift_t` builds it. The audio device is a recording double
//! (`sounding.rs` in `tools/conformance/test-support`): no person has heard a sound through
//! this window. The worker is the program beside the test binary: `cargo build -p
//! pdf-media-worker --bins` first (trap 10's construction).

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

#[path = "../../../../tools/conformance/test-support/sounding.rs"]
mod sounding;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pdf-viewer-gtk-{name}-{}", std::process::id()))
}

/// The window's loop, turned until `done` holds or two minutes pass.
fn turn_until(host: &Rc<RefCell<Host>>, what: &str, done: impl Fn(&Host) -> bool) {
    let began = Instant::now();
    let context = glib::MainContext::default();
    while !done(&host.borrow()) {
        assert!(
            began.elapsed() < Duration::from_mins(2),
            "the window never {what}"
        );
        if !context.iteration(false) {
            std::thread::sleep(viewer_host::loading::TICK);
        }
    }
}

/// The GTK window over the sounding document at the level `open`, with `sounds` in place of the
/// helper's, the document taken in.
fn a_window(document: &Path, sounds: Sounds) -> Rc<RefCell<Host>> {
    std::fs::write(document, sounding::a_sounding_document()).expect("write the document");
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
    .expect("the sounding document is on disk");
    // `quorra-gtk`'s flags, and no identifier, so registering asks no session bus for a name.
    let app = gtk4::Application::new(None::<&str>, gio::ApplicationFlags::NON_UNIQUE);
    app.register(None::<&gio::Cancellable>)
        .expect("an application without a name registers locally");
    let host = Host::open(&app, opening, Trace::off(Instant::now()));
    host.borrow_mut().resized(400, 400);
    turn_until(&host, "took the opened document", |host| {
        host.state.anticipated.is_none() && host.state.awaiting.is_none()
    });
    host.borrow_mut().state.sounds = sounds;
    host
}

/// **The sound reaches the device as §13.3's arithmetic gives it, the player's thread wakes the
/// window, the title names the key that stops it, and Escape does.** The wrong answers excluded:
/// the WAVE file handed to this machine's player instead (which `open` would do in a window that
/// does not play sounds), a device opened at the launch, a title that never says the sound is
/// sounding, and Escape left to the key table while a sound sounds. Closing the window ends the
/// player.
#[gtk4::test]
fn a_sound_plays_in_the_gtk_window_and_escape_stops_it() {
    let document = temporary("sounding.pdf");
    let (open, record) = sounding::recording();
    let host = a_window(&document, Sounds::opening_with(open));
    assert!(
        !host.borrow().state.sounds.started(),
        "nothing is made for a sound before the first"
    );
    assert_eq!(sounding::record(&record).opened, 0, "no device at launch");

    host.borrow_mut()
        .dispatch(Command::Activate(sounding::ANNOTATION));
    turn_until(&host, "said the sound was sounding", |host| {
        host.state
            .title_notes()
            .contains("playing a sound, Escape stops it")
    });
    {
        let record = sounding::record(&record);
        assert_eq!(record.opened, 1, "the device opened once, at this sound");
        assert_eq!(record.started, [sounding::SAMPLES.to_vec()]);
    }
    let title = host
        .borrow()
        .ui
        .window
        .title()
        .unwrap_or_default()
        .to_string();
    assert!(
        title.contains("playing a sound, Escape stops it"),
        "the title bar carries it: {title}"
    );

    let taken = host
        .borrow_mut()
        .key(gtk4::gdk::Key::Escape, viewer_host::Modifiers::default());
    assert!(taken, "a sound that sounds takes Escape");
    turn_until(&host, "said the sound was stopped", |host| {
        !host.state.title_notes().contains("playing a sound")
    });
    assert!(
        sounding::record(&record).stops >= 1,
        "the device was stopped"
    );
    let said = host.borrow().ui.status.text().to_string();
    assert!(said.contains("stopped"), "the window says so: {said}");
    assert!(
        !host.borrow_mut().state.sounds.stop(),
        "with nothing sounding, Escape is the key table's again"
    );

    let window = host.borrow().ui.window.clone();
    window.close();
    assert!(
        !host.borrow().state.sounds.started(),
        "the window's close ends the player"
    );
    let _ = std::fs::remove_file(&document);
}

/// **A missing sound helper is said in the window, and the window goes on**: the sentence names
/// the helper and how to build it, in the status line and the title.
#[gtk4::test]
fn a_missing_sound_helper_is_said_in_the_gtk_window() {
    let document = temporary("helperless.pdf");
    let missing = temporary("no-such-helper");
    let host = a_window(
        &document,
        Sounds::opening_with(Box::new(move || {
            viewer_host::sound::helper::open_at(&missing)
        })),
    );
    host.borrow_mut()
        .dispatch(Command::Activate(sounding::ANNOTATION));
    turn_until(&host, "said the sound was not played", |host| {
        host.state.title_notes().contains("no-such-helper")
    });
    let said = host.borrow().ui.status.text().to_string();
    assert!(
        said.contains("no-such-helper"),
        "the status line says it: {said}"
    );
    assert!(
        host.borrow().state.answers_interval().is_none(),
        "a refused sound leaves nothing for the timer to look at"
    );
    let _ = std::fs::remove_file(&document);
}
