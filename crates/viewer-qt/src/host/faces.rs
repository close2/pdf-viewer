//! The machine's faces for chrome strings the compiled-in face cannot draw, asked for off this
//! thread and handed to the window as they land (ADRs 1406, 1418; filed here by ADR 1908).

use viewer_host::trace::Topic;

use super::Host;

impl Host {
    /// Queues every character of `text` the compiled-in face lacks on `viewer_host::machine_faces`'
    /// thread, which never searches on this one (ADR 1406); the answer is taken by
    /// [`Host::take_face`] when it lands.
    pub(super) fn ask_machine(&self, text: &str) {
        for character in text.chars() {
            if viewer_host::machine_faces::compiled_in_lacks(character) {
                let _answer = self.faces.ask(viewer_host::machine_faces::Wanted {
                    character,
                    bold: false,
                    italic: false,
                });
            }
        }
    }

    /// How long to wait before asking whether a machine face has landed, in milliseconds, or `-1`
    /// where none is being searched for and none is waiting to be taken.
    ///
    /// [`Host::drawing_wait`]'s shape, for the same reason: the search is on a thread of its own and
    /// there is no path from it into a Qt object that would not cost this crate an `unsafe` token.
    ///
    /// **Nothing is handed over before the first frame is on the screen.** The search is off the
    /// window's thread, but registering what it found is not: `addApplicationFontFromData` reads
    /// a face of megabytes on the thread that paints, and measured under `Xvfb` with a Chinese
    /// outline title at launch a face that landed first put that registration in front of the
    /// first frame in one run of four. So a face found early waits for the first frame, and the
    /// rows are drawn by Qt's own fallback until then (trap 70, ADR 1429).
    pub(crate) fn faces_wait(&self) -> i32 {
        let poll = i32::try_from(viewer_host::drawing::POLL.as_millis()).unwrap_or(i32::MAX);
        let waiting = self.faces.found() > self.faces_taken;
        if waiting && self.presented {
            return 0;
        }
        if waiting || self.faces.searching() {
            poll
        } else {
            -1
        }
    }

    /// The next machine face the window has not registered, as the bytes of its file, or empty
    /// where there is none.
    ///
    /// The file the covering search named (§9.7.4.2's search, ADR 1382), which the window hands to
    /// `QFontDatabase::addApplicationFontFromData` and sets the strip and the panels in by the
    /// family it registers (ADRs 1406, 1418).
    pub(crate) fn take_face(&mut self) -> Vec<u8> {
        if !self.presented {
            return Vec::new();
        }
        let Some(bytes) = self.faces.face(self.faces_taken) else {
            return Vec::new();
        };
        self.faces_taken = self.faces_taken.saturating_add(1);
        // Trap 70's instrument: whether a machine face reaches the window before or after the
        // first frame is the question `--trace=launch` answers, and the launch gate cannot.
        self.state.trace.say(
            Topic::Launch,
            format_args!(
                "machine face {} handed to Qt, {} bytes",
                self.faces_taken,
                bytes.len()
            ),
        );
        bytes.to_vec()
    }
}
