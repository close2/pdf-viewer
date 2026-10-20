//! What the window does once its first frames are on screen: settling the clock's cadence, and
//! noting that the device's pipelines have compiled.

use std::sync::Arc;

use super::{State, Surface};
use crate::app::App;
use crate::trace::Topic;

impl App {
    /// Asks the surface once more what it refreshes at, until it answers.
    ///
    /// **A Wayland surface enters no output until it has been drawn to**, and the cadence is read
    /// in `resumed`, which is strictly before that — so a cadence read only there takes
    /// `doc/todo/36`'s floor on every Wayland session and puts 120 Hz out of reach in principle. Called after a present
    /// rather than before one for exactly that reason, and it stops asking the moment the window's
    /// own output answers ([`crate::cadence::Cadence::ask`], ADR 0384).
    pub(super) fn settle_cadence(&mut self) {
        if self.cadence.settled() {
            return;
        }
        // Cloned rather than borrowed because the clock and the window are two fields of the same
        // value; one `Arc` bump per frame, and only until the surface has answered once.
        let Some(window) = self.state.as_ref().map(|state| Arc::clone(&state.window)) else {
            return;
        };
        if self.cadence.ask(&window) {
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "presenting on a cadence of {} — re-asked now that the window has been drawn to",
                    self.cadence.described()
                ),
            );
        }
    }

    /// Closes the one line the launch timeline never had: when the pipelines finished compiling.
    ///
    /// **ADR 0227.** Bring-up prints `pipelines still compiling` because
    /// `CLAUDE.md` forbids waiting for warmth on the launch path, and nothing ever said when the
    /// wait nobody did would have ended — so a first frame that absorbed a shader compilation
    /// and one that did not read identically. Polled once a frame, which behind the topic check
    /// is one `Option` read; *noticed* rather than *finished*, because the compilation ends on
    /// raster's own thread and this is only the first frame to look.
    ///
    /// **Read off the last finished frame since ADR 0391**, because the device is no longer on
    /// this thread to ask: [`crate::renderer::Window`] keeps whatever the render thread saw when
    /// it last drew, which is the same answer one tick later.
    pub(super) fn pipelines_compiled(&mut self) {
        if self.frames.pipelines || !self.trace.on(Topic::Launch) {
            return;
        }
        let Some(State {
            surface: Surface::Device(window),
            ..
        }) = self.state.as_ref()
        else {
            return;
        };
        let Some(compiling) = window.pipelines() else {
            return;
        };
        self.frames.pipelines = true;
        self.trace.say(
            Topic::Launch,
            format_args!(
                "pipelines compiled in {compiling:?}, noticed at this frame — nothing on the \
                 launch path waited for them, so every frame before this one drew with whatever \
                 was ready"
            ),
        );
    }
}
