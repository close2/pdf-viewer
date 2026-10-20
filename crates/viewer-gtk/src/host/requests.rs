//! The timer that looks for a server's answer on GTK's loop: what a document sends or fetches is
//! carried out by `viewer_host::native_window` for every window, and this window's part of it is
//! the main loop's timeout (ADR 2105).

use gtk4::glib;
use viewer_host::native_window::NativeWindow;

use super::{Host, with};

impl Host {
    /// Looks for the answers on GTK's main loop, at `viewer_host::submit::LOOK`, until none is out.
    pub(super) fn look_for_answers(&self) {
        let me = self.me.clone();
        glib::timeout_add_local(viewer_host::submit::LOOK, move || {
            let mut more = false;
            with(&me, |host| {
                // A conversion's report is collected inside it too (ADR 1954).
                host.take_the_answers();
                more = host.state.looking();
            });
            if more {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }
}
