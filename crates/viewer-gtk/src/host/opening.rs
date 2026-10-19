//! The launch: the viewport's first size opens the document, the opening thread is joined, page one's read is shown on the status bar while it runs, and the preparation is taken back.

use std::collections::VecDeque;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Command, Event};

use viewer_host::native_window::NativeWindow;
use viewer_host::panel::Tab;
use viewer_host::trace::Topic;

use super::{DOCUMENT, Host, opening_commands};

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
        // §12.4.4.1's transition is drawn in the viewport rather than in a page's own rectangle,
        // so the size the core is told is also the size a frame is shaped for.
        self.state.viewport = (width, height);
        let resize = Command::Resize {
            width,
            height,
            scale,
        };
        // Page one's drawing ahead draws into the target the first frame will be asked at, which is
        // this viewport's; a size that changes while the page is read is told again, and a frame
        // asked at another target than the one drawn is drawn whole (ADR 1855).
        let window = viewer_host::loading::Window {
            width,
            height,
            inset: 0,
            scale,
        };
        self.state.extent = Some(window);
        if let Some(ahead) = self.state.ahead.as_ref() {
            ahead.window_is(window);
        }
        if !self.opened {
            self.opened = true;
            self.state.trace.say(
                Topic::Launch,
                format_args!("first allocation {width}x{height} device px, scale {scale}"),
            );
            self.join_the_opening(resize);
            return;
        }
        // A document still opening past `WARN` is told the window's size when it is taken in.
        if self.state.awaiting.is_some() {
            self.state.awaiting = Some(resize);
            return;
        }
        self.dispatch(resize);
        // A window asked to fit the first displayed page is measured again once it has taken
        // the size it was asked for, because a frame is not always drawn for a viewport the page
        // already fits the width of (ADR 1429).
        if self.state.presenting.owes_placing() {
            self.fit_the_window();
        }
    }

    /// The first allocation: the document [`Opening`](super::Opening) opened is taken in, and told its viewport.
    ///
    /// The viewport goes in before any of the open's events is answered, so that each is answered
    /// against the viewer a first open with a viewport would have left — the order this window
    /// had when it opened the document here (ADR 1539). A thread that panicked is said, and the
    /// document opened on this one instead.
    ///
    /// **It waits no longer than `viewer_host::drawing::WARN`** (ADR 1811). A document still
    /// opening then is left to its thread: the status bar carries how far page one has got, GTK's
    /// loop goes on answering the window, and [`Self::mind_the_opening`] joins it on the
    /// [`viewer_host::loading::TICK`] that finds it ready, with the window's size as it is then.
    fn join_the_opening(&mut self, resize: Command) {
        // A page that can be drawn while it is read is waited for no longer than `GLIMPSE_AFTER`,
        // from which its looks are what the window shows (ADR 1855).
        let drawing_ahead = self.state.ahead.is_some();
        if self.state.anticipated.as_ref().is_some_and(|opening| {
            !if drawing_ahead {
                opening.wait_within(viewer_host::drawing::GLIMPSE_AFTER)
            } else {
                opening.wait()
            }
        }) {
            self.state
                .trace
                .say(Topic::Launch, format_args!("document still opening"));
            self.state.awaiting = Some(resize);
            self.show_the_opening();
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
            return;
        }
        self.take_the_opening(resize);
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

    /// §7.6.4.1: opens the document, with a password where one has been supplied.
    pub(super) fn open_document(&mut self, password: Option<viewer_core::Secret>) {
        if let Some(password) = &password {
            self.state.kept.keep(DOCUMENT, password);
        }
        self.pump(opening_commands(
            self.restrictions.window(),
            self.widget_appearances,
            self.separations,
            self.state.showing.bytes.clone(),
            password,
            self.state.showing.fragment.clone(),
        ));
    }
}
