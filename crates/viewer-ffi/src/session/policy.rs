//! The reader's restrictions, trust inputs and rendering preferences.

use super::Session;
use crate::events::Events;
use viewer_core::{Command, DocumentId};

impl Session {
    /// The person's answer to [`viewer_core::Event::Asking`].
    #[must_use]
    pub fn answer(&mut self, document: u64, proceed: bool) -> Events {
        self.handle(Command::Answer {
            document: DocumentId(document),
            proceed,
        })
    }

    /// How much of what a document asserts about its reader this viewer obeys.
    #[must_use]
    pub fn restrict(&mut self, level: viewer_core::RestrictionLevel) -> Events {
        self.restrictions = viewer_core::RestrictionPolicy::uniform(level);
        let policy = self.restrictions;
        self.handle(Command::Restrict(viewer_core::RestrictionScope::Window(
            policy,
        )))
    }

    /// The same, for one operation alone — `CLAUDE.md`'s four levels, per restriction.
    #[must_use]
    pub fn restrict_operation(
        &mut self,
        operation: pdf_model::restriction::Operation,
        level: viewer_core::RestrictionLevel,
    ) -> Events {
        self.restrictions = self.restrictions.with(operation, level);
        let policy = self.restrictions;
        self.handle(Command::Restrict(viewer_core::RestrictionScope::Window(
            policy,
        )))
    }

    /// The same, for the document in front of this caller alone — the scope a window's policy
    /// cannot express.
    ///
    /// `None` is not a fifth level: it gives the operation back to the window's policy, which is
    /// where every document starts and where the next one opened will be. ADR 1145.
    #[must_use]
    pub fn restrict_document(
        &mut self,
        operation: pdf_model::restriction::Operation,
        level: Option<viewer_core::RestrictionLevel>,
    ) -> Events {
        self.departures = self.departures.with(operation, level);
        let departures = self.departures;
        self.handle(Command::Restrict(viewer_core::RestrictionScope::Document(
            departures,
        )))
    }

    /// §6.3.2.2's "unless otherwise instructed": who draws §12.7's widget appearances.
    #[must_use]
    pub fn delegate(&mut self, appearances: pdf_model::view::WidgetAppearances) -> Events {
        self.handle(Command::Delegate(appearances))
    }

    /// §12.8.1's third question: which certification authorities this reader believes.
    ///
    /// RFC 5280 section 6.1.1's inputs (d) and (b), which ADR 1039 made a host's to supply and
    /// this library's never to invent. An empty set is the default and is what a caller that never
    /// calls this gets.
    #[must_use]
    pub fn trust(&mut self, policy: viewer_core::TrustPolicy) -> Events {
        self.handle(Command::Trust(policy))
    }

    /// ISO 32000-2 §8.10.4's target documents: which files a reference `XObject` may import from.
    ///
    /// The clause's second processor class, made reachable. Empty is the default and is what a
    /// caller that never calls this gets: every reference `XObject` draws §8.10.4.1's proxy, which
    /// is what that clause asks of a reader with no target file.
    #[must_use]
    pub fn reference_files(&mut self, files: viewer_core::ReferenceFiles) -> Events {
        self.handle(Command::References(files))
    }

    /// ISO 32000-2 §8.11.4.4's answers about this reader: who they are, and in what language.
    pub fn audience(&mut self, audience: pdf_model::optional_content::Audience) -> Events {
        self.handle(Command::Audience(audience))
    }

    /// ISO 32000-2 §10.8.3's separation simulation, as this reader has asked for it.
    pub fn separations(&mut self, simulate: bool) -> Events {
        self.handle(Command::Separations(simulate))
    }
}
