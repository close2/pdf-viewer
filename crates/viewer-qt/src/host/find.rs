//! The find bar: Annex O's document-wide search, a page at a time on Qt's timer, and the matches
//! and highlights drawn over the page (ADR 0250; filed here by ADR 1908).

use viewer_core::{Answer, Command, Find, Query};
use viewer_host::native_window::NativeWindow;

use crate::bridge::ffi::QtQuad;

use super::{Host, quad};

impl Host {
    /// Every occurrence of the find bar's string on the page being shown.
    ///
    /// Asked on every repaint, because `Query::Find` reads a readback that is already there.
    /// `Command::Find` is the other question and the expensive one — see `find`.
    pub(crate) fn matches(&self) -> Vec<QtQuad> {
        if self.state.needle.is_empty() {
            return Vec::new();
        }
        match self.state.viewer.query(Query::Find(&self.state.needle)) {
            Answer::Found(occurrences) => occurrences.iter().flatten().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// ISO 32000-2 Annex O's `highlight`: the rectangles the URI's fragment named.
    ///
    /// Table Annex O.4: "Open the document with the specified rectangle highlighted … \[t]he nature
    /// of the highlighting is implementation-dependent." So the shapes cross and the colour is
    /// Qt's, which is the whole argument for chrome crossing as geometry. Empty for every document
    /// opened without a fragment naming one.
    pub(crate) fn highlights(&self) -> Vec<QtQuad> {
        match self.state.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => quads.iter().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// The find bar's string changed: highlight this page, and look no further.
    ///
    /// Deliberately not a search per keystroke — see `viewer_gtk`'s `retype`, which makes the same
    /// choice for the same reason and is the agreement between the two hosts that
    /// `doc/ui-boundary.md` is about.
    pub(crate) fn retype(&mut self, needle: &str) {
        needle.clone_into(&mut self.state.needle);
        self.update.chrome = true;
    }

    /// The next occurrence anywhere in the document, or the previous one: the bridge's name for
    /// [`NativeWindow::find`], which `cxx` calls as a method of `Host` itself (ADR 1920).
    pub(crate) fn find(&mut self, backward: bool) {
        NativeWindow::find(self, backward);
    }

    /// One more page of the search in progress. The C++ calls this from a zero-delay timer.
    pub(crate) fn find_continue(&mut self) {
        self.dispatch(Command::Find(Find::Continue));
    }

    /// The find bar was closed: forget the plan and the highlights.
    pub(crate) fn find_stop(&mut self) {
        self.state.needle.clear();
        self.state.pages_left = 0;
        self.dispatch(Command::Find(Find::Stop));
        self.update.chrome = true;
    }

    /// Whether a search still has pages to read, which is what the C++ pumps on.
    pub(crate) fn searching(&self) -> bool {
        self.state.pages_left > 0
    }
}
