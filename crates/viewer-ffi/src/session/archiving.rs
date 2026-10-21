//! Archive conversion commands and the retained conversion result.

use super::Session;
use crate::events::Events;
use viewer_core::Command;

impl Session {
    /// Asks for the focused document as ISO 19005 at one target, under one shipped profile.
    ///
    /// Table 22's bit 11 is asked at the reader's level; where it is granted the conversion runs
    /// before this returns and [`Self::archived`] holds it (ADR 1954).
    #[must_use]
    pub fn archive(&mut self, asked: viewer_core::Archiving) -> Events {
        self.handle(Command::Archive(asked))
    }

    /// The last conversion a grant released, where one has been.
    #[must_use]
    pub const fn archived(&self) -> Option<&viewer_host::archiving::Converted> {
        self.archived.as_ref()
    }
}
