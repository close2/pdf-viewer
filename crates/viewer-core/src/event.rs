//! What the viewer tells its host.
//!
//! Everything here is either a fact about the document that the host could not have worked out
//! for itself, or a job the host has to do. Nothing here is advice.

use std::sync::Arc;

use pdf_render::{DisplayList, Rect, TargetSpec};

use crate::command::Purpose;
use crate::viewer::{DocumentId, RenderToken};

/// One thing the viewer tells its host.
#[derive(Debug)]
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
    /// §12.6.4.8: a link asks for this URI to be resolved.
    ///
    /// Handed over rather than opened, and that is not squeamishness: the string is one the
    /// *document* controls, and handing it to a browser is a decision about this machine. The
    /// URI arrives resolved — against the document's `/Base` where it states one, and with
    /// `/IsMap`'s cursor coordinates already appended.
    OpenUri {
        /// Which document asked.
        document: DocumentId,
        /// The resolved URI.
        uri: String,
    },
    /// The document asks for a file. Answer with [`crate::Command::Supply`].
    ///
    /// The name is the document's own words and is **not** a path: resolving it — or refusing
    /// to — is the host's decision, because a document naming a file is a document asking this
    /// machine for something and this crate has no filesystem (rule 2).
    NeedsFile {
        /// Which document asked.
        document: DocumentId,
        /// What the bytes are wanted for.
        purpose: Purpose,
        /// The file specification's name, as the document wrote it.
        name: String,
    },
    /// §12.4.4: show the page using this transition.
    ///
    /// Named rather than played, because a transition is a sequence of frames and this crate has
    /// no clock (rule 3). A host with one plays it; a host without one draws the page, which is
    /// the transition's own end state.
    Transition {
        /// Which document.
        document: DocumentId,
        /// Table 164's style, duration and direction.
        transition: pdf_model::navigation::Transition,
    },
    /// Whether the document now differs from the file it was opened from.
    ///
    /// Sent when the answer changes and not on every edit, because what a host does with it is
    /// mark a window and ask before closing.
    Dirty {
        /// Which document.
        document: DocumentId,
        /// Whether anything is unsaved.
        dirty: bool,
    },
    /// The document, with §7.5.6's update appended. Write it somewhere.
    ///
    /// The whole file rather than the update alone, because §7.5.6's update is only meaningful
    /// after the bytes it chains to — and because a host that had to concatenate them could get
    /// it wrong. The original bytes are the first part of it, unchanged.
    Saved {
        /// Which document.
        document: DocumentId,
        /// The file.
        bytes: Vec<u8>,
    },
    /// §7.11.4's embedded file, decoded. Write it somewhere — or do not; see [`Extraction`].
    ///
    /// The answer to [`crate::Command::Extract`], and the bytes are the file itself rather than
    /// the stream: §7.4's filters are undone here, because a host that had to decode them would
    /// be a second reader of the document.
    Extracted {
        /// Which document it came out of.
        document: DocumentId,
        /// What asked for it, which is the only thing that decides whether to ask a person first.
        asked: Extraction,
        /// Table 43's `/UF` or `/F` where the specification states one, and the
        /// `/EmbeddedFiles` key otherwise. **A name and not a path**: it is the document's own
        /// words, and turning it into somewhere on this machine is the host's decision (rule 2)
        /// — §7.11.4's own note is that `/F` is "a platform-dependent encoding".
        name: String,
        /// The file.
        bytes: Vec<u8>,
        /// The rest of the URI's fragment, where Annex O's `ef` is what asked for this file and
        /// something follows it. Undecoded, exactly as [`crate::Command::Open`]'s is.
        ///
        /// §O.2.1, Table Annex O.3's `ef` row:
        ///
        /// > Any remaining parameters after this parameter apply to the selected embedded file.
        ///
        /// **A host that opens these bytes as a document hands this straight back** as
        /// `Command::Open`'s `fragment`, and the sentence is carried out: the page, the search or
        /// the highlight after `ef` applies to the file that came out rather than to the one it
        /// came out of. A host that does not open embedded files ignores it and loses nothing it
        /// had — `Event::Reported` has already said, in words, that the fragment continued.
        ///
        /// It is here rather than left to a host to split for itself because splitting it is
        /// §O.2's grammar, and six consumers re-deriving that grammar is six chances to disagree
        /// about where a parameter ends. `None` for [`crate::Command::Extract`] and for
        /// §12.5.6.15's annotation, which have no URI behind them at all. ADR 0431.
        fragment: Option<String>,
    },
    /// An operation was not performed, because the document restricts it and this reader is
    /// obeying (§7.6.4.2, §12.8.2.2, §12.8.6).
    ///
    /// **Not [`Self::Reported`]**, and the difference is the point: `Reported` says what the
    /// *document* could not do, and this says what the *reader's own policy* did. Telling a
    /// person that a document is defective when what happened is that their program obeyed it
    /// would be a false statement about the file.
    ///
    /// It carries the operation as well as the sentences, so that it can become a **question**.
    /// [`crate::RestrictionLevel`]'s two unbuilt levels — ask and warn — are exactly this event
    /// plus a host able to answer it with a [`crate::Command`], which is the shape
    /// [`Self::PasswordRequired`] already uses; a refusal that had only prose could not.
    ///
    /// [`crate::Command::Restrict`] with [`crate::RestrictionLevel::Off`] is what stops it.
    Refused {
        /// Which document.
        document: DocumentId,
        /// What was not done.
        operation: pdf_model::restriction::Operation,
        /// One sentence per restriction that applied, already worded for a person.
        ///
        /// Every one of them rather than the first: §12.8.6 makes a permission granted only if
        /// "allowed by each permission handler that is present in the permissions dictionary as
        /// well as by the security handler", so two clauses can withhold one operation and a
        /// person being asked is owed both reasons.
        notes: Vec<String>,
    },
    /// What could not be drawn on the page that was just interpreted.
    ///
    /// Trap 5's channel: every layer of this program reports what it could not handle rather
    /// than falling back to something plausible, and this is where that reaches a person. An
    /// empty list is never sent — a page with nothing to report sends no event at all.
    Reported {
        /// Which document.
        document: DocumentId,
        /// Which page, zero-based, or `None` for something said about the document itself.
        ///
        /// Both exist and they are different statements. §12.11's requirements, §12.8's
        /// signatures and §7.11.4's attachments are claims about the *file*, and a person
        /// deciding whether to trust what they are looking at needs them before any page is
        /// drawn; everything else here is about the page on the screen.
        page: Option<usize>,
        /// One sentence per distinct thing, already worded for a person.
        notes: Vec<String>,
    },
    /// A step of [`crate::Command::Find`] read one page, and what it found there.
    ///
    /// **The only event a host has to pump.** A search's answer is not known until the pages have
    /// been read and this crate has neither a thread nor a clock to read them with (rules 3 and
    /// 4), so a step reads one page and says how many are left; a host sends
    /// [`crate::Find::Continue`] until `remaining` is zero, at whatever rate its own repaint
    /// allows.
    ///
    /// Three answers, and a host needs all three: `found` set is the occurrence, which is by then
    /// the selection and on the page being shown; `found` empty with `remaining` above zero is a
    /// search still going; and `found` empty with `remaining` zero is a document that does not
    /// contain the string.
    Searched {
        /// Which document.
        document: DocumentId,
        /// Where the occurrence is, or `None` where this step found none.
        found: Option<Found>,
        /// How many pages are still to be read before the search has an answer.
        ///
        /// What a progress indicator counts down, and what says whether to pump again.
        remaining: usize,
        /// Whether the scan has passed the end of the document and come round to the beginning.
        ///
        /// A fact only this crate holds — a host knows neither where the search began nor which
        /// pages it has read — and the one every find bar puts into words.
        /// [`crate::Command::Find`] started from Annex O's `search` never wraps, so this is
        /// always `false` for it.
        wrapped: bool,
    },
}

/// What asked for §7.11.4's bytes, and the reason [`Event::Extracted`] carries it.
///
/// **The annex that made this necessary is the one that says why.** ISO 32000-2 §O.2.1, Table
/// Annex O.3's `ef`:
///
/// > Security should be strongly considered when opening an embedded file. When opening a file
/// > that is not from a trusted source, a PDF processor may choose to prompt the user or even
/// > prevent opening of the file.
///
/// That sentence is attached to that parameter and to no other, and §O.1 says why: a fragment
/// identifier is "useful primarily when referring to them from external to the PDF such as a web
/// page or web API". A URI is somebody else's sentence far more often than a click is, so the two
/// are not the same request even though they produce the same bytes — and a host that wrote a file
/// to disk for one has to be able to decline for the other. ADR 0310.
///
/// This crate takes no view of which is safe. It says which happened, once, where a host can see
/// it; the four levels `doc/todo/38` describes are built on top of this rather than inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extraction {
    /// A person: [`crate::Command::Extract`], or activating §12.5.6.15's file attachment
    /// annotation, which the clause makes an act of the reader — "activating the annotation
    /// extracts the embedded file and gives the user an opportunity to view it or store it in the
    /// file system".
    Asked,
    /// Annex O's `ef` parameter: the URI the document was opened from named the file, and nobody
    /// pressed anything.
    Fragment,
}

/// Where a [`crate::Command::Find`] landed.
///
/// The page and the range, and deliberately not the shapes: the occurrence is now the selection,
/// so [`crate::Query::Selection`] answers with the quadrilaterals over it in the host's own
/// colour, exactly as it does for a drag. A find bar draws every occurrence on the page from
/// [`crate::Query::Find`] and lets the selection mark which one is current, which is why this
/// carries no second kind of highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    /// Which page it is on, zero-based. The page being shown by the time this arrives.
    pub page: usize,
    /// The byte range of that page's readback it covers — [`crate::Query::Selection`]'s offsets.
    ///
    /// Half-open, `from` first. The same string [`crate::Selected::text`] is a slice of.
    pub range: (usize, usize),
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
