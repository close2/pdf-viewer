//! The launch: the document the opening thread opened is taken in, page one is shown as it is
//! read, and what page one does not need is read beside it (ADRs 1539, 1553, 1867; filed here by
//! ADR 1908).

use std::collections::VecDeque;

use viewer_core::{Command, Event};
use viewer_host::native_window::NativeWindow;
use viewer_host::panel::Tab;
use viewer_host::trace::Topic;

use super::panels::flatten;
use super::{DOCUMENT, Host, opening_commands};

impl Host {
    /// The first resize: the document the opening thread opened is taken in, and told its
    /// viewport.
    ///
    /// The viewport goes in before any of the open's events is answered, so that each is answered
    /// against the viewer a first open with a viewport would have left — the order this window
    /// had when it opened the document here (ADR 1539). A thread that panicked is said, and the
    /// document opened on this one instead.
    ///
    /// **It waits no longer than `viewer_host::drawing::WARN`** (ADR 1811). A document still
    /// opening then is left to its thread: the status bar carries how far page one has got, Qt's
    /// loop goes on answering the window, and [`Self::drawing_pump`] joins it on the
    /// [`viewer_host::loading::TICK`] that finds it ready, with the window's size as it is then.
    pub(super) fn join_the_opening(&mut self, resize: Command) {
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
            return;
        }
        self.take_the_opening(resize);
    }

    /// Puts how far page one has got on the status bar: the sentence on the label, and the share
    /// [`Self::loading_permille`] hands Qt's own bar.
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
        if sentence != self.message {
            // Said once, when the bar goes up: the label moves every tick and the terminal is not
            // a label.
            if !std::mem::replace(&mut self.bar_said, true) {
                println!("note: {sentence}");
            }
            self.message = sentence;
            self.update.status = true;
        }
    }

    /// How much of page one has been read, in thousandths, while the document is still opening past
    /// `WARN` — `-1` where there is no bar to show, and `0` for a wait with no share yet.
    ///
    /// **And while a page is shown as it stands, how much of its drawing there is**: the bar that
    /// showed the page being read hands over to the page being drawn, and goes when it lands
    /// (ADR 1825).
    pub(crate) fn loading_permille(&self) -> i32 {
        if self.state.awaiting.is_none() {
            return self.state.drawing.partial().map_or(-1, |partial| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a share in [0, 1], times a thousand"
                )]
                let permille = (partial.fraction() * 1000.0).round() as i32;
                permille
            });
        }
        let showing = self
            .state
            .anticipated
            .as_ref()
            .and_then(viewer_host::Loading::showing);
        showing.map_or(-1, |reading| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a share in [0, 1], times a thousand"
            )]
            reading
                .fraction()
                .map_or(0, |fraction| (fraction * 1000.0).round() as i32)
        })
    }

    /// Reads what page one does not need beside it: §12.3.3's outline and §7.7.3's page tree,
    /// placed, which the open no longer reads (ADRs 1543, 1553).
    ///
    /// The thread holds a handle to the immutable file and nothing of the core's. Its answer is
    /// looked for on the drawing timer, which [`Host::drawing_wait`] keeps running until it lands.
    pub(super) fn prepare(&mut self) {
        self.preparing = self.state.viewer.preparation().map(|preparation| {
            let (send, answer) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // A window that has closed has nobody to give the answer to.
                let _ = send.send(preparation.run());
            });
            answer
        });
    }

    /// The launch document's outline and page tree, if the thread reading them has answered.
    ///
    /// The caption's section comes back as an `Event::PageChanged`, and the contents panel is
    /// built again from what has now been read.
    pub(super) fn take_the_preparation(&mut self) {
        let Some(answer) = self.preparing.as_ref() else {
            return;
        };
        let prepared = match answer.try_recv() {
            Ok(prepared) => Some(prepared),
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            // The thread ended without an answer; the first use reads it instead.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
        };
        self.preparing = None;
        let mut queue = VecDeque::new();
        if let Some(prepared) = prepared {
            self.state
                .trace
                .say(Topic::Launch, format_args!("outline and page tree read"));
            let events: Vec<Event> = self.state.viewer.prepared(prepared).collect();
            for event in events {
                self.react(event, &mut queue);
            }
        }
        if self.state.documents.focused() == DOCUMENT {
            let rows = flatten(&self.panel_of(Tab::Contents));
            if let Some(tree) = self.trees.get_mut(Tab::Contents.index()) {
                *tree = rows;
                self.update.panels = true;
            }
        }
        self.drain(queue);
    }

    /// §7.6.4.1: opens the document, with a password where one has been supplied.
    pub(super) fn open_document(&mut self, password: Option<viewer_core::Secret>) {
        if let Some(password) = &password {
            self.state.kept.keep(DOCUMENT, password);
        }
        self.drain(opening_commands(
            self.restrictions.window(),
            self.widget_appearances,
            self.separations,
            self.state.showing.bytes.clone(),
            password,
            self.state.showing.fragment.clone(),
        ));
    }
}
