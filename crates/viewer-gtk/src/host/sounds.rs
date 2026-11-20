//! §13.3's sound on GTK's loop: the player's thread wakes GTK's main context, and the window takes
//! what the player said on GTK's next turn (RFC 0015 section 5, step 3; ADR 3329).
//!
//! The player, the media worker and the sound helper are `viewer_host`'s and shared with the Qt
//! window ([`viewer_host::sound::Sounds`]); what is GTK's is the wake, and the window's close.
//! This program links no audio library: the helper, `quorra-sound`, does (ADR 3317).

use std::cell::RefCell;
use std::rc::Weak;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_host::native_window::NativeWindow;

use super::Host;

thread_local! {
    /// The windows that have played a sound, on GTK's own thread: the wake is called on the
    /// player's thread, where no `Rc` may go, so what crosses is a call to [`heard`] and this is
    /// where it finds its windows. `drawing`'s `WOKEN` is the same construction for the device.
    static HEARING: RefCell<Vec<Weak<RefCell<Host>>>> = const { RefCell::new(Vec::new()) };
}

/// The wake for the player `me` makes at its first sound: GTK's main context, invoked from the
/// player's thread.
pub(super) fn wake(me: Weak<RefCell<Host>>) -> viewer_host::sound::Wake {
    HEARING.with(|hearing| {
        let mut hearing = hearing.borrow_mut();
        hearing.retain(|window| window.strong_count() > 0);
        hearing.push(me);
    });
    Box::new(|| glib::MainContext::default().invoke(heard))
}

/// What the wake runs on GTK's thread: one look at each window's player.
fn heard() {
    let windows: Vec<_> =
        HEARING.with(|hearing| hearing.borrow().iter().filter_map(Weak::upgrade).collect());
    for window in windows {
        // A window already borrowed is inside a turn of its own, and the answers timer that turn
        // left armed takes the sentence in at its next look, so it is that look's rather than
        // lost.
        if let Ok(mut host) = window.try_borrow_mut() {
            host.take_the_sounds();
        }
    }
}

/// The window's close ends its sounds: the player's thread is ended and waited for, which stops
/// the sound, closes the helper's pipe and kills the worker before the window is gone.
///
/// A window already borrowed is closing from inside its own turn — the last document closed —
/// and that turn ended the sounds before it asked GTK to close the window.
pub(super) fn end_on_close(window: &gtk4::ApplicationWindow, me: &Weak<RefCell<Host>>) {
    let me = me.clone();
    window.connect_close_request(move |_| {
        if let Some(host) = me.upgrade()
            && let Ok(mut host) = host.try_borrow_mut()
        {
            host.state.sounds.end();
        }
        glib::Propagation::Proceed
    });
}
