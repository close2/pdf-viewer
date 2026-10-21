//! The launch: the viewport's first size opens the document, the opening thread is joined, page one's read is shown on the status bar while it runs, and the preparation is taken back.

use std::collections::VecDeque;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::Event;

use viewer_host::native_window::{FirstSize, Fitting, Joining, NativeWindow, Sizing};
use viewer_host::panel::Tab;
use viewer_host::trace::Topic;

use super::{DOCUMENT, Host};

/// GTK's part of the sizes a window takes: its first is an allocation, and it fits itself to
/// Table 147's `/FitWindow` (ADRs 1429, 2172).
const SIZING: Sizing = Sizing {
    first: FirstSize::Allocation,
    fitting: Fitting::Measured,
};

impl Host {
    /// The viewport changed size, in logical pixels.
    ///
    /// The first one opens the document, which is what makes page one's raster the size of the
    /// window it will be shown in rather than of a guess.
    pub(super) fn resized(&mut self, width: i32, height: i32) {
        self.scale = self.ui.chrome.scale_factor().max(1);
        let scale = f64::from(self.scale);
        self.chrome.borrow_mut().scale = scale;
        let (Ok(width), Ok(height)) = (
            u32::try_from(width.saturating_mul(self.scale)),
            u32::try_from(height.saturating_mul(self.scale)),
        ) else {
            return;
        };
        if width == 0 || height == 0 {
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small integer, so f64 to f32 here loses \
                      nothing a viewport can express"
        )]
        let scale = scale as f32;
        if self.sized(width, height, scale, SIZING) == Joining::StillOpening {
            self.look_at_the_opening();
        }
    }

    /// A look at the document still opening on every [`viewer_host::loading::TICK`], until
    /// [`NativeWindow::mind_the_opening`] has joined it: GTK's loop has no timer of its own already
    /// looking, as `viewer-qt`'s drawing pump and the C window's tick are (ADR 1811).
    fn look_at_the_opening(&self) {
        let me = self.me.clone();
        glib::timeout_add_local(viewer_host::loading::TICK, move || {
            let Some(host) = me.upgrade() else {
                return glib::ControlFlow::Break;
            };
            // A host busy with one of its own writes is asked again at the next look.
            let Ok(mut host) = host.try_borrow_mut() else {
                return glib::ControlFlow::Continue;
            };
            if host.mind_the_opening() {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    /// Hides GTK's own bar where a document still opening put it up, and answers whether it was.
    pub(super) fn take_down_the_bar(&mut self) -> bool {
        let up = self.ui.loading.is_visible();
        if up {
            self.ui.loading.set_visible(false);
        }
        up
    }

    /// Puts how far page one has got on the status bar: the sentence on the label, the share on
    /// GTK's own bar beside it.
    pub(super) fn show_the_opening(&mut self) {
        let Some(reading) = self
            .state
            .anticipated
            .as_ref()
            .and_then(viewer_host::Loading::showing)
        else {
            return;
        };
        let sentence = viewer_host::loading::reading(reading);
        if self.ui.status.text() != sentence.as_str() {
            // Not `say`: a sentence that changes every tick is not one for the terminal.
            if !self.ui.loading.is_visible() {
                println!("note: {sentence}");
            }
            self.ui.status.set_text(&sentence);
            self.ui.status.set_tooltip_text(Some(&sentence));
        }
        match reading.fraction() {
            Some(fraction) => self.ui.loading.set_fraction(fraction),
            None => self.ui.loading.pulse(),
        }
        self.ui.loading.set_visible(true);
    }

    /// Reads what page one does not need beside it: §12.3.3's outline and §7.7.3's page tree,
    /// placed, which the open no longer reads (ADRs 1543, 1553).
    ///
    /// The thread holds a handle to the immutable file and nothing of the core's, and GTK's main
    /// loop looks for its answer at [`viewer_host::drawing::POLL`] — the interval chosen for a
    /// page's own latency, here for the caption's — and stops looking once it has one.
    pub(super) fn prepare(&mut self) {
        let Some(preparation) = self.state.viewer.preparation() else {
            return;
        };
        let (send, answer) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // A window that has closed has nobody to give the answer to.
            let _ = send.send(preparation.run());
        });
        self.preparing = Some(answer);
        let me = self.me.clone();
        glib::timeout_add_local(viewer_host::drawing::POLL, move || {
            let Some(host) = me.upgrade() else {
                return glib::ControlFlow::Break;
            };
            // A host busy with one of its own writes is asked again at the next look.
            let Ok(mut host) = host.try_borrow_mut() else {
                return glib::ControlFlow::Continue;
            };
            host.take_the_preparation();
            if host.preparing.is_some() {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    /// The launch document's outline and page tree, if the thread reading them has answered.
    ///
    /// The caption's section comes back as an `Event::PageChanged`, and the contents panel is
    /// built again from what has now been read.
    fn take_the_preparation(&mut self) {
        let Some(answer) = self.preparing.as_ref() else {
            return;
        };
        let prepared = match answer.try_recv() {
            Ok(prepared) => prepared,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The thread ended without an answer; the first use reads it instead.
                self.preparing = None;
                self.rebuild_panel(Tab::Contents);
                return;
            }
        };
        self.preparing = None;
        self.state
            .trace
            .say(Topic::Launch, format_args!("outline and page tree read"));
        let events: Vec<Event> = self.state.viewer.prepared(prepared).collect();
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue);
        if self.state.documents.focused() == DOCUMENT {
            self.rebuild_panel(Tab::Contents);
        }
    }
}
