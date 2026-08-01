//! What the viewer tells its host.
//!
//! Everything here is either a fact about the document that the host could not have worked out
//! for itself, or a job the host has to do. Nothing here is advice.

use std::sync::Arc;

use pdf_render::{DisplayList, Rect, TargetSpec};

use crate::viewer::{DocumentId, RenderToken};

/// One thing the viewer tells its host.
#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    /// A document opened, and this is how many pages it has.
    Opened {
        /// The identity the host gave it.
        document: DocumentId,
        /// How many pages, counting §12.7.8.3.3's imported template pages.
        pages: usize,
    },
    /// A document could not be opened, and this is why.
    ///
    /// Distinct from [`Self::PasswordRequired`], which is not a failure: one is a file this
    /// program cannot read and the other is a file it has not been given the key to.
    OpenFailed {
        /// The identity the host gave it.
        document: DocumentId,
        /// What went wrong, in the words `pdf_syntax` used.
        reason: String,
    },
    /// §7.6.4.1: the document is encrypted and neither the empty password nor the one supplied
    /// opens it.
    ///
    /// The host prompts and sends [`crate::Command::Open`] again with what it was told. The
    /// viewer keeps nothing in the meantime — an unopened document is not an open one — so a
    /// host that never asks has simply not opened a file.
    PasswordRequired {
        /// The identity the host gave it.
        document: DocumentId,
    },
    /// A document closed and everything derived from it was dropped.
    Closed(DocumentId),
    /// The focused document is showing a different page.
    ///
    /// Carries §12.4.2's label as well as the index, because "[p]age labels and page indices
    /// need not coincide": a page of front matter is *iv*, and a host that showed only the
    /// index would be contradicting the document.
    PageChanged {
        /// Which document.
        document: DocumentId,
        /// The zero-based index.
        index: usize,
        /// §12.4.2's label, where the document states one for this page.
        label: Option<String>,
        /// How many pages there are.
        of: usize,
        /// §12.3.3's outline section covering this page, where the outline names one.
        section: Option<String>,
    },
    /// Rasterise this, and send [`crate::Command::RenderReady`] back with the token.
    NeedsRender(RenderRequest),
    /// This part of the viewport, in device pixels, no longer shows what it should.
    ///
    /// A tier-1 host blits the frame [`crate::Query::Frame`] hands it; a tier-2 host re-renders
    /// the request it kept. The rectangle is a bound on what changed and not a promise that
    /// everything inside it did.
    Damage(Rect),
    /// What could not be drawn on the page that was just interpreted.
    ///
    /// Trap 5's channel: every layer of this program reports what it could not handle rather
    /// than falling back to something plausible, and this is where that reaches a person. An
    /// empty list is never sent — a page with nothing to report sends no event at all.
    Reported {
        /// Which document.
        document: DocumentId,
        /// Which page, zero-based.
        page: usize,
        /// One sentence per distinct thing, already worded for a person.
        notes: Vec<String>,
    },
}

/// A page, resolved to drawing commands, and the resolution to draw it at.
///
/// Self-contained on purpose: everything a worker needs is here, so it can be sent to another
/// thread or another process without reaching back into the viewer for anything. That is what
/// makes rule 4 hold — the viewer hands out work rather than lending out state.
#[derive(Debug, Clone)]
pub struct RenderRequest {
    /// Identifies this request. Send it back unchanged.
    ///
    /// A [`crate::Command::RenderReady`] whose token is not the one outstanding is dropped, so
    /// a worker that is slow costs a wasted render and never a wrong frame.
    pub token: RenderToken,
    /// Which document the page belongs to.
    pub document: DocumentId,
    /// Which page, zero-based.
    pub page: usize,
    /// The page's drawing commands, resolution-independent.
    ///
    /// Shared rather than owned because the viewer keeps it: a zoom or a scroll produces a new
    /// [`TargetSpec`] over the same list, which is the difference between re-rasterising and
    /// re-interpreting.
    pub list: Arc<DisplayList>,
    /// What to draw into, and the transform from page space to it.
    pub target: TargetSpec,
}
