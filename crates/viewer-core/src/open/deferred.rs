//! What a document reads after page one, and says about itself when first asked.
//!
//! The outline, the whole page tree placed, and the document's own report are each a function of
//! the immutable file that page one does not need, so each is a [`OnceLock`] filled at first use
//! or by a [`crate::Preparation`] a host ran on a thread of its own (ADRs 1044, 1543, 1855).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use pdf_model::Pages;
use pdf_model::outline::Outline;
use pdf_syntax::ObjectId;

use super::{Open, Preparation, Prepared};

impl Open {
    /// §12.3.3's outline, read now if nothing has read it yet.
    ///
    /// The first use pays for it, on whichever thread asks; [`crate::Preparation`] is how a host
    /// makes that a thread other than the one that draws (ADR 1543).
    pub(crate) fn outline(&self) -> &Outline {
        self.outline
            .get_or_init(|| Outline::read(&self.document, &Pages::new(&self.document)))
    }

    /// §7.7.3's page tree placed — which page each object is — built now if nothing has built it.
    pub(crate) fn page_indices(&self) -> &BTreeMap<ObjectId, usize> {
        self.page_indices
            .get_or_init(|| Pages::new(&self.document).indices())
    }

    /// The §12.3.3 section covering `index`, where the outline names one.
    ///
    /// `read` false answers only from what is already read, which is what the opening
    /// announcement asks: a section is not worth the outline and the whole page tree in front of
    /// page one (ADR 1543).
    pub(crate) fn section_at(&self, index: usize, read: bool) -> Option<String> {
        let (outline, indices) = if read {
            (self.outline(), self.page_indices())
        } else {
            (self.outline.get()?, self.page_indices.get()?)
        };
        outline
            .section_at_with(&self.document, indices, index)
            .map(ToOwned::to_owned)
    }

    /// What this document says about itself, worded once and kept.
    ///
    /// The clauses [`crate::notes::about`] answers, produced the first time
    /// [`crate::Command::Report`] asks and read from [`Self::about`] every time after. `&self`
    /// rather than `&mut self` because a [`OnceLock`] is what makes "once" a property of the
    /// value rather than of the caller's discipline — and because the document is immutable, so
    /// the sentences are a function of the file and are the same sentences every time.
    pub(crate) fn about(&self, trust: &crate::TrustPolicy) -> &crate::notes::About {
        self.about
            .get_or_init(|| crate::notes::about(&self.document, trust))
    }

    /// Drops the wording, because the policy it was worded under has changed.
    ///
    /// [`crate::Command::Trust`] is the one thing that can invalidate it: the sentences are a function of
    /// the document *and* of whom this reader believes, and the first of those two is immutable
    /// while the second is a host's to change at any moment.
    pub(crate) fn forget_about(&mut self) {
        self.about = OnceLock::new();
    }

    /// The work [`Preparation`] carries, where it is still to do.
    pub(crate) fn preparation(&self, id: crate::DocumentId) -> Option<Preparation> {
        (self.outline.get().is_none() || self.page_indices.get().is_none()).then(|| Preparation {
            id,
            document: Arc::clone(&self.document),
        })
    }

    /// Takes in what a [`Preparation`] read, where it is this document's and still wanted.
    ///
    /// Returns whether it was this document's. A cell already filled — a first use that came
    /// before the answer did — keeps what it holds: both are functions of one immutable file.
    pub(crate) fn prepared(&mut self, prepared: Prepared) -> bool {
        if !Arc::ptr_eq(&self.document, &prepared.document) {
            return false;
        }
        let Prepared {
            outline, indices, ..
        } = prepared;
        self.outline.get_or_init(|| outline);
        self.page_indices.get_or_init(|| indices);
        true
    }
}
