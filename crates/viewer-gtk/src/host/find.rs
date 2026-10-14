//! The find bar and the search it drives.

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Command, Find, FindDirection};

use super::{Host, with};

impl Host {
    /// A step of the search reported. Says what it found, and how much is left to read.
    pub(super) fn searched(
        &mut self,
        found: Option<viewer_core::Found>,
        remaining: usize,
        wrapped: bool,
    ) {
        self.pages_left = remaining;
        match found {
            Some(found) => self.say(&format!(
                "found {:?} on page {}{}",
                self.needle,
                found.page.saturating_add(1),
                if wrapped { " (wrapped)" } else { "" }
            )),
            None if remaining == 0 => {
                self.say(&format!("{:?} is not in this document", self.needle));
            }
            None => self.say(&format!(
                "searching for {:?}: {remaining} page(s) left",
                self.needle
            )),
        }
    }

    /// The find bar's string changed: highlight what is on this page, and look no further.
    ///
    /// Deliberately **not** a `Command::Find` per keystroke. Typing changes what is highlighted on
    /// the page — which is a query over a readback that already exists — and a search across the
    /// document is what pressing Enter asks for. A host that started one on every character would
    /// be interpreting pages while a person was still deciding what to look for.
    pub(super) fn retype(&mut self, needle: String) {
        self.needle = needle;
        self.refresh();
    }

    /// Enter, Ctrl+G and Ctrl+Shift+G: the next occurrence anywhere in the document.
    pub(super) fn find(&mut self, backward: bool) {
        if self.needle.is_empty() {
            return;
        }
        let needle = self.needle.clone();
        self.dispatch(Command::Find(Find::Start {
            needle,
            direction: if backward {
                FindDirection::Backward
            } else {
                FindDirection::Forward
            },
        }));
    }

    /// The bar was revealed or closed. Closing it forgets the plan and the highlights.
    pub(super) fn find_bar_shown(&mut self, shown: bool) {
        if shown {
            self.ui.find_entry.grab_focus();
            return;
        }
        self.needle.clear();
        self.pages_left = 0;
        self.dispatch(Command::Find(Find::Stop));
    }

    /// One more page of the search, scheduled on GTK's idle queue.
    ///
    /// **The whole reason a step is one page.** `viewer-core` has no thread to read a thousand
    /// pages on (rule 4) and this host must not block the main loop for the 5.84 s that would
    /// cost, so each step is posted back through `glib::idle_add_local_once`: the window keeps
    /// repainting, the status line counts down, and the search finishes when it finishes.
    pub(super) fn pump_search(&mut self) {
        if self.pages_left == 0 {
            return;
        }
        let me = self.me.clone();
        glib::idle_add_local_once(move || {
            with(&me, |host| host.dispatch(Command::Find(Find::Continue)));
        });
    }
}

/// The find bar, and it is somebody else's widget: a `GtkSearchBar` with a `GtkSearchEntry` in it,
/// so Ctrl+F, Escape, the clear icon and Ctrl+G all behave the way they do in every other GTK
/// application.
///
/// Nothing about it is drawn by this program — which is `doc/ui-boundary.md`'s whole argument,
/// applied to a find bar: what crosses from the core is the geometry of the matches and the
/// vocabulary of the search, and the *bar* is the platform's.
pub(super) fn find_bar() -> (gtk4::SearchBar, gtk4::SearchEntry) {
    let entry = gtk4::SearchEntry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Find in document"));
    let bar = gtk4::SearchBar::new();
    bar.set_child(Some(&entry));
    bar.connect_entry(&entry);
    bar.set_show_close_button(true);
    (bar, entry)
}
