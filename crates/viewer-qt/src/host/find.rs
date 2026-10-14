//! The find bar: Annex O's document-wide search, a page at a time on Qt's timer, and the matches
//! and highlights drawn over the page (ADR 0250; filed here by ADR 1908).

use viewer_core::{Answer, Command, Find, FindDirection, Query};

use crate::bridge::ffi::QtQuad;

use super::{Host, quad};

impl Host {
    /// Every occurrence of the find bar's string on the page being shown.
    ///
    /// Asked on every repaint, because `Query::Find` reads a readback that is already there.
    /// `Command::Find` is the other question and the expensive one — see `find`.
    pub(crate) fn matches(&self) -> Vec<QtQuad> {
        if self.needle.is_empty() {
            return Vec::new();
        }
        match self.viewer.query(Query::Find(&self.needle)) {
            Answer::Found(occurrences) => occurrences.iter().flatten().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

    /// ISO 32000-2 Annex O's `highlight`: the rectangles the URI's fragment named.
    ///
    /// Table Annex O.4: "Open the document with the specified rectangle highlighted … [t]he nature
    /// of the highlighting is implementation-dependent." So the shapes cross and the colour is
    /// Qt's, which is the whole argument for chrome crossing as geometry. Empty for every document
    /// opened without a fragment naming one.
    pub(crate) fn highlights(&self) -> Vec<QtQuad> {
        match self.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => quads.iter().copied().map(quad).collect(),
            _ => Vec::new(),
        }
    }

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

    /// The find bar's string changed: highlight this page, and look no further.
    ///
    /// Deliberately not a search per keystroke — see `viewer_gtk`'s `retype`, which makes the same
    /// choice for the same reason and is the agreement between the two hosts that
    /// `doc/ui-boundary.md` is about.
    pub(crate) fn retype(&mut self, needle: &str) {
        needle.clone_into(&mut self.needle);
        self.update.chrome = true;
    }

    /// The next occurrence anywhere in the document, or the previous one.
    pub(crate) fn find(&mut self, backward: bool) {
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

    /// One more page of the search in progress. The C++ calls this from a zero-delay timer.
    pub(crate) fn find_continue(&mut self) {
        self.dispatch(Command::Find(Find::Continue));
    }

    /// The find bar was closed: forget the plan and the highlights.
    pub(crate) fn find_stop(&mut self) {
        self.needle.clear();
        self.pages_left = 0;
        self.dispatch(Command::Find(Find::Stop));
        self.update.chrome = true;
    }

    /// Whether a search still has pages to read, which is what the C++ pumps on.
    pub(crate) fn searching(&self) -> bool {
        self.pages_left > 0
    }
}
