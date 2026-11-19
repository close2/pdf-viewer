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

    /// A copy of a signature policy's document this caller fetched, handed to `document`'s
    /// validation, which binds it from the signature's own identifier and enforces §12.8.3.4.4's
    /// constraints from it as from a copy the file stores (ADRs 2624, 3169).
    ///
    /// The copy the three windows hand on after their own fetch, as
    /// `viewer_host::submit::Returned::follow_up` does; whether to fetch was the caller's, at its
    /// reader's network level (ADR 1738).
    #[must_use]
    pub fn policy_copy(
        &mut self,
        document: DocumentId,
        policy: pdf_signature::policy::PublishedPolicy,
        copy: Vec<u8>,
    ) -> Events {
        self.handle(Command::PolicyCopy {
            document,
            policy,
            copy,
        })
    }

    /// §12.8.1's third question: which certification authorities this reader believes.
    ///
    /// RFC 5280 section 6.1.1's inputs (d) and (b), which ADR 1039 made a host's to supply and
    /// this library's never to invent. An empty set is the default and is what a caller that never
    /// calls this gets.
    #[must_use]
    pub fn trust(&mut self, policy: viewer_core::TrustPolicy) -> Events {
        self.trust = policy.clone();
        self.handle(Command::Trust(policy))
    }

    /// [`Self::trust`] with the anchors and the acceptance replaced and the lists this caller
    /// supplied before kept.
    #[must_use]
    pub fn trust_anchors(
        &mut self,
        anchors: pdf_signature::trust::Supply,
        acceptance: pdf_signature::verdict::Acceptance,
    ) -> Events {
        let policy = viewer_core::TrustPolicy {
            anchors,
            acceptance,
            lists: self.trust.lists.clone(),
        };
        self.trust(policy)
    }

    /// [`Self::trust`] with the trusted lists replaced and the anchors kept (ETSI TS 119 612, ADR
    /// 2879).
    ///
    /// A verdict through a list, and ETSI TS 119 615 clause 4.1's "has passed", are asked at the
    /// instant the anchors carry; where this caller supplied no anchor, the lists bring `at`, the
    /// caller's clock, with them — `viewer-host`'s rule for a reader given `--trusted-lists` and no
    /// `--trust-anchors` (ADR 2842).
    #[must_use]
    pub fn trusted_lists(
        &mut self,
        lists: pdf_signature::trusted_list::Lists,
        at: pdf_signature::x509::Instant,
    ) -> Events {
        let mut policy = self.trust.clone();
        if policy.anchors.is_empty() && !lists.is_empty() {
            policy.anchors = pdf_signature::trust::Supply::of(Vec::new(), String::new(), at);
        }
        policy.lists = lists;
        self.trust(policy)
    }

    /// §7.6.5.1's reader identity: the certificate and key a document encrypted to recipients is
    /// opened with, or `None` for none — the default, and what a caller that never calls this
    /// gets (ADR 2822).
    #[must_use]
    pub fn recipient(&mut self, key: Option<viewer_core::RecipientSecret>) -> Events {
        self.handle(Command::Recipient(key))
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
