//! §12.4.4's presentation: the full-screen window, the timer `viewer_host::presentation_pump` is
//! turned on, and the transition's frame as a texture.

use std::cell::RefCell;
use std::rc::Weak;

use gtk4::prelude::*;
use gtk4::{gdk, glib};
use viewer_core::{Command, PresentationMode};
use viewer_host::native_window::NativeWindow;
use viewer_host::{PresentationPump, Turned};

use crate::page;

use super::stand_in::Picture;
use super::{Host, with};

impl Host {
    /// Enters or leaves §12.4.4's presentation, which for this program is the full-screen window.
    ///
    /// `p`, the same letter `viewer-ui` and `viewer-qt` bind. That the window and §12.4.4.2's mode
    /// are one act is a documented choice, argued in `viewer_host::presentation`; what crosses the
    /// boundary is `Command::Present`, which has existed since ADR 0316 and needed no change.
    pub(super) fn present_or_stop(&mut self) {
        if self.state.presenting.toggle() {
            self.start_the_clock();
            self.dispatch(Command::Present(PresentationMode::On));
            self.say(
                "presenting full screen (§7.7.2's FullScreen page mode) — §12.4.4's /Dur advances \
                 the page, its /Trans is drawn, and Escape comes back",
            );
        } else {
            self.stop_the_clock();
            self.dispatch(Command::Present(PresentationMode::Off));
            // §12.2: "[t]he document's page mode, specifying how to display the document on
            // exiting full-screen mode". `None` is Table 147's own condition unmet, and then what
            // goes back is what the reader had — a choice rather than a reading (ADR 0470).
            if let Some(mode) = self.state.presenting.on_exit() {
                self.say(&format!("§12.2's /NonFullScreenPageMode asks for {mode:?}"));
                self.show_page_mode(mode);
            } else {
                self.say("presentation stopped");
            }
        }
        self.apply_chrome();
    }

    /// Takes down the pending timer, if there still is one.
    ///
    /// **`SourceId::remove` panics on a source that is already gone** — it unwraps a `Result` — so
    /// the id is looked up before it is used. A one-shot destroys itself when it fires, and the
    /// callback that fires it clears [`Host::armed`]; this is the case where it could not, because
    /// the host was borrowed by a nested main loop (§7.6.4.1's password dialogue runs one).
    fn disarm(&mut self) {
        let Some(armed) = self.armed.take() else {
            return;
        };
        if let Some(source) = glib::MainContext::default().find_source_by_id(&armed) {
            source.destroy();
        }
    }

    /// Stops it, and takes the timer and the frame in flight with it.
    ///
    /// **The source is removed rather than left to fire on a `None` clock.** A window that is not
    /// presenting has nothing to advance, and `CLAUDE.md`'s principle 2 makes a wakeup with
    /// nothing behind it a defect rather than a rounding error.
    fn stop_the_clock(&mut self) {
        self.drop_the_clock();
        self.playing = None;
        self.disarm();
    }

    /// Arms the next turn of `viewer_host::presentation_pump` on GTK's own main loop: §12.4.4's
    /// clock while a presentation or a transition runs, a document's script timers while none does,
    /// and nothing at all for a still window or a hidden one whose documents set no timer.
    ///
    /// A one-shot that is re-armed after every pump, which is the shape [`Host::pump_search`]
    /// already uses and which answers what a repeating source cannot: the interval changes when a
    /// transition starts and stops, and a presentation that ends leaves nothing armed at all.
    pub(super) fn pump_presentation(&mut self) {
        self.disarm();
        let Some(wait) = self.presentation_wait(std::time::Instant::now()) else {
            return;
        };
        let me = self.me.clone();
        self.armed = Some(glib::timeout_add_local_once(wait, move || {
            with(&me, |host| {
                host.armed = None;
                host.turn_the_clock();
            });
        }));
    }

    /// One turn of the pump: what is due on the screen is drawn, and the next turn armed.
    fn turn_the_clock(&mut self) {
        if let Turned::Frame(frame) = self.turn_the_presentation(std::time::Instant::now()) {
            self.playing = frame;
            self.refresh();
        }
        self.pump_presentation();
    }

    /// The window was minimised or came back: the pump parks the presentation while nobody can see
    /// it, and the frame of a transition in flight goes with the effect (ADR 2300).
    fn shown_or_hidden(&mut self, shown: bool) {
        if !self.window_shown(shown, std::time::Instant::now()) {
            return;
        }
        if self.playing.take().is_some() {
            self.refresh();
        }
        self.pump_presentation();
    }

    /// The frame of a transition in flight, as one texture filling the viewport.
    ///
    /// `None` where there is nothing being drawn, and then the window shows the page — which is
    /// the transition's own end state, so the two answers meet without a seam.
    pub(super) fn transition_placement(&mut self) -> Option<Picture> {
        let raster = self.playing.as_ref()?;
        match page::texture(raster) {
            Ok(texture) => Some(Picture {
                extent: (raster.width, raster.height),
                texture,
                origin: (0.0, 0.0),
                bytes: raster.data.len(),
            }),
            Err(error) => {
                eprintln!("note: {error}");
                None
            }
        }
    }
}

/// `GDK_TOPLEVEL_STATE_SUSPENDED`, `gdktoplevel.h`'s `1 << 16` since GTK 4.12, which `gdk4` names
/// only behind its `v4_12` feature: a GTK before 4.12 never sets the bit, so reading it under the
/// `v4_10` floor parks on Wayland where the library can say so and changes nothing where it cannot
/// (ADRs 0508, 2317).
const SUSPENDED: u32 = 1 << 16;

/// Whether `toplevel`'s state holds [`SUSPENDED`], read off the `state` property's own number:
/// `gdk4` converts the state to `ToplevelState` by truncation, which drops every bit its feature set
/// does not name, so `Toplevel::state` can never answer it under the `v4_10` floor. A `GLib` that
/// cannot state the flags as a number leaves the window to `MINIMIZED` alone, which is how it
/// parked before this bit was read.
fn suspended(toplevel: &gdk::Toplevel) -> bool {
    toplevel
        .property_value("state")
        .transform::<u32>()
        .ok()
        .and_then(|bits| bits.get::<u32>().ok())
        .is_some_and(|bits| bits & SUSPENDED != 0)
}

/// Parks the presentation while the window is minimised, and turns it again when it comes back
/// (ADR 2300).
///
/// GDK states a toplevel's minimised state as `ToplevelState::MINIMIZED`, and under X11 an unmap
/// of a mapped window reads as one where no window manager states `_NET_WM_STATE_HIDDEN`. Under
/// Wayland, xdg-shell states no minimised window, and GDK 4.12 reads the compositor's
/// `suspended` instead — a window no one can see — as [`SUSPENDED`], which is read by its value so
/// the `v4_10` floor stays (ADR 2317). The
/// notice is taken on the idle queue, because GDK may raise it from inside a call this host makes
/// while borrowed — the find bar's reason — and the state is read when the idle runs, so two
/// notices in one turn of the loop cannot be read out of order.
pub(super) fn watch_visibility(window: &gtk4::ApplicationWindow, me: &Weak<RefCell<Host>>) {
    let me = me.clone();
    window.connect_realize(move |window| {
        let Some(toplevel) = window.surface().and_downcast::<gdk::Toplevel>() else {
            return;
        };
        let me = me.clone();
        toplevel.connect_state_notify(move |toplevel| {
            let toplevel = toplevel.clone();
            let me = me.clone();
            glib::idle_add_local_once(move || {
                let shown = !toplevel.state().contains(gdk::ToplevelState::MINIMIZED)
                    && !suspended(&toplevel);
                with(&me, |host| host.shown_or_hidden(shown));
            });
        });
    });
}
