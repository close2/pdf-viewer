//! Opening and focusing documents, and accepting deferred preparation.

use super::Session;
use crate::events::Events;
use viewer_core::{Command, DocumentId, Event};

impl Session {
    /// §7.6.4.1 and Annex O: opens a document from bytes the caller owns.
    ///
    /// The password arrives as a [`viewer_core::Secret`] rather than a `String` so that the text
    /// the caller's NUL-terminated bytes became **moves** into it: a conversion here would leave a
    /// second copy in a buffer nothing clears. What the *caller's* memory holds is the caller's,
    /// and the header says so.
    #[must_use]
    pub fn open(
        &mut self,
        id: u64,
        bytes: Vec<u8>,
        password: Option<viewer_core::Secret>,
        fragment: Option<String>,
    ) -> Events {
        // A document opens at the window's levels, so the mirror of its departures starts empty —
        // the core's `Open` does exactly this, and a mirror that did not would build the next
        // override on the last document's (ADR 1145).
        self.departures = viewer_core::RestrictionOverride::NONE;
        match &password {
            Some(password) => self.kept.keep(DocumentId(id), password),
            None => self.kept.forget(DocumentId(id)),
        }
        self.handle(Command::Open {
            id: DocumentId(id),
            bytes: bytes.into(),
            password,
            fragment,
        })
    }

    /// Closes a document and forgets everything derived from it.
    #[must_use]
    pub fn close(&mut self, id: u64) -> Events {
        self.kept.forget(DocumentId(id));
        self.handle(Command::Close(DocumentId(id)))
    }

    /// Makes an already-open document the one commands apply to.
    #[must_use]
    pub fn focus(&mut self, id: u64) -> Events {
        self.handle(Command::Focus(DocumentId(id)))
    }

    /// A name held in reserve for a document opened beside the one showing.
    ///
    /// ISO 32000-2 Table 203's and Table 204's `/NewWindow true`, answered by the only party that
    /// knows whether this caller has a second place to put a document. `None` empties the reserve,
    /// under which a destination in another file replaces the one it was reached from. ADR 1263.
    pub fn beside(&mut self, name: Option<u64>) -> Events {
        self.handle(Command::Beside(name.map(DocumentId)))
    }

    /// What the focused document still reads after page one, for the caller to run on a thread
    /// of its own: §12.3.3's outline and §7.7.3's page tree, placed (ADRs 1543, 1553).
    ///
    /// `None` where there is nothing left to read. The caller runs it with
    /// [`viewer_core::Preparation::run`] wherever it likes and hands the answer to
    /// [`Self::prepared`]; a caller that never asks loses only the timing, because the first use
    /// reads both on the thread that asked.
    #[must_use]
    pub fn preparation(&self) -> Option<viewer_core::Preparation> {
        self.viewer.preparation()
    }

    /// Takes in what a preparation read: `Event::PageChanged` again, with the page's section,
    /// where the document is the one in front and its page has one.
    #[must_use]
    pub fn prepared(&mut self, prepared: viewer_core::Prepared) -> Events {
        Events::new(self.viewer.prepared(prepared).collect::<Vec<Event>>())
    }
}
