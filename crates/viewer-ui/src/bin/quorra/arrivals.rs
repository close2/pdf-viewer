//! The documents a reader names beside the one showing: a path typed after Ctrl + O, and every
//! path on the command line after the first.
//!
//! Both go through `viewer_host::Arrivals`, one at a time, and both become bytes in
//! `viewer_host::open_chosen` — the one place the three windows turn a path a person named into a
//! document, so that `CLAUDE.md`'s levels have one place to attach (ADR 1275). What is this
//! window's is the line a path is typed into: the other two put a file dialogue there, and this
//! one has no toolkit to ask for one.

use viewer_core::Command;
use winit::keyboard::{Key, NamedKey};

use crate::app::App;

/// The word in front of the box, which says what the typed line is for.
pub(crate) const OPEN_LABEL: &str = "Open beside:";

impl App {
    /// Ctrl + O: the line a path is typed into, over the page where the find bar goes.
    ///
    /// The two bars are one band of this window's chrome and never up together, so opening this
    /// one closes the other and stops the search it held.
    pub(crate) fn choose_a_document(&mut self) {
        if self.find.shown {
            self.find.toggle();
            self.pages_left = 0;
            self.dispatch(Command::Find(viewer_core::Find::Stop));
        }
        if !self.opener.shown {
            self.opener.toggle();
            // Beside the document already showing, which is where a person reaching for a second
            // one most often looks; a path typed after it is still whatever the person types.
            if let Some(directory) = self.directory.as_deref() {
                let mut beside = directory.to_string_lossy().into_owned();
                if !beside.is_empty() && !beside.ends_with(std::path::MAIN_SEPARATOR) {
                    beside.push(std::path::MAIN_SEPARATOR);
                }
                self.opener.typed(&beside);
            }
        }
        self.redraw();
    }

    /// A key press while the line is open. Every key is the line's, which is the find bar's rule:
    /// a `/` typed into a path is a slash.
    pub(crate) fn open_key(&mut self, key: &Key<&str>) {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.opener.toggle();
            }
            Key::Named(NamedKey::Enter) => {
                let typed = std::mem::take(&mut self.opener.needle);
                self.opener.toggle();
                if !typed.is_empty() {
                    self.arrivals
                        .wait(viewer_host::Named::file(typed.into()), false);
                    self.open_the_next();
                }
            }
            Key::Named(NamedKey::Backspace) => {
                self.opener.backspace();
            }
            Key::Named(NamedKey::Space) => {
                self.opener.typed(" ");
            }
            Key::Character(text) if !text.is_empty() => {
                self.opener.typed(text);
            }
            // An arrow, a function key: taken anyway, so that nothing moves the document out from
            // under a path being typed.
            _ => {}
        }
        self.redraw();
    }

    /// The line, where it is open, drawn where the find bar is drawn.
    pub(crate) fn opener_list(&self, width: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.opener.draw_labelled(chrome, width, scale, OPEN_LABEL)
    }

    /// Starts the next document waiting to open, where none is on its way.
    ///
    /// `viewer_core::Command::Open` under the name `viewer_host::Arrivals` reserved, so the core
    /// hands the document every answer this reader has given exactly as it did the first
    /// (`Viewer::adopt`, ADR 1263); the tab is added when `Event::Opened` names it.
    pub(crate) fn open_the_next(&mut self) {
        loop {
            let Some(arriving) = self.arrivals.start(&mut self.documents) else {
                return;
            };
            let id = arriving.id;
            let fragment = arriving.named.fragment.clone();
            match arriving.bytes() {
                Ok(bytes) => {
                    self.dispatch(Command::Open {
                        id,
                        bytes,
                        password: None,
                        fragment,
                    });
                    return;
                }
                Err(sentence) => {
                    println!("note: {sentence}");
                    self.arrivals.settle(id);
                }
            }
        }
    }

    /// §7.6.4.1's second attempt, for the document on its way to a tab rather than the one in
    /// front.
    pub(crate) fn open_arriving(&mut self, password: Option<viewer_core::Secret>) {
        let Some((id, bytes, fragment)) = self.arrivals.current().map(|arriving| {
            (
                arriving.id,
                arriving.bytes(),
                arriving.named.fragment.clone(),
            )
        }) else {
            return;
        };
        match bytes {
            Ok(bytes) => self.dispatch(Command::Open {
                id,
                bytes,
                password,
                fragment,
            }),
            Err(sentence) => {
                println!("note: {sentence}");
                self.given_up(id);
            }
        }
    }

    /// A document on its way to a tab that will not arrive — cancelled, refused, declined — and
    /// the next one waiting, which may now start. The first document's giving up starts them too,
    /// since no first frame is coming to do it.
    pub(crate) fn given_up(&mut self, id: viewer_core::DocumentId) {
        if self.arrivals.settle(id).is_some() || id == crate::DOCUMENT {
            self.arrival_due = true;
        }
    }
}
