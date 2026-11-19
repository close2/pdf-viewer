//! Shift + T on a signed document whose policy references a trusted list, at the GTK window: the
//! reference reaches the question the window puts and the refresh its yes starts (ADR 2936, ETSI
//! TS 119 172-2 clause 4.22.2.1 b)).
//!
//! **The window is the one `quorra-gtk` builds** (ADR 3098): [`Host::open`] on an application that
//! is registered — GIO's `startup`, the step after which GTK takes an application's windows — and
//! never run, so the test's own loop iterations stand where `Application::run`'s would. The key
//! goes through [`Host::key`], which the window's key controller calls, and the question is the
//! dialogue the window presents, read from its labels and answered by its own buttons.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use pdf_model::view::WidgetAppearances;
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

use super::{Host, Opening};

#[path = "../../../../tools/conformance/test-support/signed_under_a_policy.rs"]
mod signed_under_a_policy;

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

/// The question dialogue the window presents over itself, if it presents one.
fn the_question(host: &Rc<RefCell<Host>>) -> Option<gtk4::Window> {
    let window = host.borrow().ui.window.clone();
    gtk4::Window::list_toplevels()
        .into_iter()
        .filter_map(|toplevel| toplevel.downcast::<gtk4::Window>().ok())
        .find(|toplevel| {
            toplevel.is_visible()
                && toplevel
                    .transient_for()
                    .is_some_and(|over| &over == window.upcast_ref::<gtk4::Window>())
        })
}

/// Every widget under `widget`, depth first.
fn descendants(widget: &gtk4::Widget) -> Vec<gtk4::Widget> {
    let mut found = Vec::new();
    let mut child = widget.first_child();
    while let Some(here) = child {
        child = here.next_sibling();
        found.extend(descendants(&here));
        found.push(here);
    }
    found
}

/// Presses Shift + T and answers the question's text, or `None` where no question was put.
fn shift_t(host: &Rc<RefCell<Host>>) -> Option<(gtk4::Window, String)> {
    let taken = host
        .borrow_mut()
        .key(gtk4::gdk::Key::T, viewer_host::Modifiers::SHIFT);
    assert!(taken, "Shift + T is a key the window binds");
    let dialogue = the_question(host)?;
    let text = descendants(dialogue.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk4::Label>().ok())
        .map(|label| label.text().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    Some((dialogue, text))
}

/// The dialogue's button labelled `label`, pressed as a click presses it.
fn press(dialogue: &gtk4::Window, label: &str) {
    descendants(dialogue.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk4::Button>().ok())
        .find(|button| button.label().as_deref() == Some(label))
        .unwrap_or_else(|| panic!("the question has a {label:?} button"))
        .emit_clicked();
}

/// **With the policy's copy bound, the question the GTK window puts at `ask` names the list the
/// policy references, and its yes starts the refresh.** The calibration is the same document
/// before the copy is bound, whose question names none: only a bound policy is anybody's
/// agreement.
#[gtk4::test]
fn a_bound_policy_s_reference_goes_with_shift_t_to_the_refresh() {
    let directory = temporary("trusted-lists");
    // A directory left by an earlier run is the one thing to clear; its absence is the usual case.
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    std::fs::write(directory.join("OJEU-LOTL-Certs-Set.der"), [0x30, 0x00])
        .expect("the certificate file");
    std::fs::write(
        directory.join(pdf_signature::trusted_list::authentication::OJEU_LOTL_LOC),
        "http://127.0.0.1:9/never-answered.xml",
    )
    .expect("the location file");
    let document = temporary("signed.pdf");
    std::fs::write(&document, signed_under_a_policy::document()).expect("write the document");

    let opening = Opening::start(
        &document,
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
        },
        ReaderWords {
            trusted_lists: Some(directory.clone()),
            ..ReaderWords::default()
        },
    )
    .expect("the signed document is on disk");
    // `quorra-gtk`'s flags, and no identifier, so registering asks no session bus for a name.
    let app = gtk4::Application::new(None::<&str>, gio::ApplicationFlags::NON_UNIQUE);
    app.register(None::<&gio::Cancellable>)
        .expect("an application without a name registers locally");
    let host = Host::open(&app, opening, Trace::off(Instant::now()));
    // The first allocation joins the open (ADR 1539); the size is asked for here rather than
    // waited for from a window manager.
    host.borrow_mut().resized(400, 400);
    turn_until(&host, "took the opened document", |host| {
        host.state.anticipated.is_none() && host.state.awaiting.is_none()
    });
    // `ask` after the open, as the menu would set it: at the open it would first put the question
    // about fetching the policy itself, and Shift + T is declined while that one waits.
    {
        let mut host = host.borrow_mut();
        host.state.restrictions = host
            .state
            .restrictions
            .with(viewer_host::ActLevel::Submissions(
                viewer_host::Submissions::Ask,
            ));
    }

    let (dialogue, unbound) = shift_t(&host).expect("ask puts the question");
    assert!(
        !unbound.contains(signed_under_a_policy::REFERENCE),
        "an unbound policy is nobody's agreement: {unbound}"
    );
    // Closed unanswered, as Escape closes it, which is the decline.
    dialogue.close();
    assert!(
        the_question(&host).is_none(),
        "the close answers the question"
    );

    {
        let mut host = host.borrow_mut();
        let focused = host.state.documents.focused();
        assert_eq!(
            signed_under_a_policy::bind(&mut host.state.viewer, focused),
            1,
            "the signature publishes its policy"
        );
    }
    let (dialogue, text) = shift_t(&host).expect("ask puts the question");
    assert!(
        text.contains(signed_under_a_policy::REFERENCE),
        "the question names the referenced list: {text}"
    );
    assert!(
        host.borrow().state.refresher.interval().is_none(),
        "nothing yet"
    );
    press(&dialogue, viewer_host::restriction::GO_AHEAD);
    assert!(
        host.borrow().state.refresher.interval().is_some(),
        "the yes starts the refresh"
    );
    host.borrow().ui.window.destroy();
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_dir_all(&directory);
}
