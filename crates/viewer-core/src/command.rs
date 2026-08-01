//! What a host asks of the viewer.
//!
//! One enum, and every variant is something a person did or something a worker finished. There
//! is deliberately no command that means "recompute": what has to be redrawn is this crate's
//! deduction, not the host's, because a host that had to know would be reimplementing the
//! scheduler.

use pdf_render::Raster;
use pdf_syntax::ObjectId;

use crate::viewer::{DocumentId, RenderToken};

/// One thing a host asks the viewer to do.
#[derive(Debug)]
pub enum Command {
    /// Open a document from bytes, under a host-chosen identity.
    ///
    /// Bytes rather than a path, because rule 2 says the host owns the filesystem. The identity
    /// is the host's so that it can name the same document in a later command without waiting
    /// for an event to tell it what the document was called.
    ///
    /// The password is §7.6.4.1's, where the host already has one. A document that needs one
    /// and was given none — or the wrong one — produces [`crate::Event::PasswordRequired`] and
    /// no open document, which is the prompt this project has owed since encryption landed.
    Open {
        /// What the host will call this document.
        id: DocumentId,
        /// The file, whole.
        bytes: Vec<u8>,
        /// §7.6.4.1's user or owner password, where one is already known.
        password: Option<String>,
    },
    /// Close a document and forget everything derived from it.
    Close(DocumentId),
    /// Make an already-open document the one commands apply to.
    Focus(DocumentId),
    /// The viewport changed size, or moved to a display with a different scale.
    ///
    /// Width and height are **device** pixels and `scale` is the ratio of device pixels to
    /// logical ones — 2.0 on a doubled display. Both, rather than logical pixels alone, because
    /// a page must be rasterised at the resolution it will be shown at: rendering at the
    /// logical size and letting the compositor scale it up is exactly the blur this project's
    /// first principle exists to avoid.
    Resize {
        /// Viewport width in device pixels.
        width: u32,
        /// Viewport height in device pixels.
        height: u32,
        /// Device pixels per logical pixel.
        scale: f32,
    },
    /// Show another page of the focused document.
    GoTo(PageTarget),
    /// Change the magnification of the focused document.
    Zoom(Zoom),
    /// Move the page under the viewport by a device-pixel delta.
    ///
    /// Positive `dy` moves the *content* up, which is what a wheel scrolling down does. Clamped
    /// to the rendered page: a viewer that let a page be scrolled out of sight would be
    /// answering a question nobody asked.
    Scroll {
        /// Horizontal delta in device pixels.
        dx: f32,
        /// Vertical delta in device pixels.
        dy: f32,
    },
    /// §8.11: switch an optional content group on or off.
    ///
    /// The group is named by object identity because that is what §8.11.2.2's `/OCGs` and Table
    /// 99's `/Order` hold; [`crate::Query::Layers`] is where a host gets the identities and the
    /// names to show beside them.
    SetGroup {
        /// The optional content group's object.
        group: ObjectId,
        /// Whether it is on.
        on: bool,
    },
    /// The pointer moved, or a button went down or up, at a point in the viewport.
    ///
    /// Device pixels, measured from the viewport's top-left corner — what a host has, rather
    /// than what the document uses; the viewer maps it back through the transform the frame on
    /// the screen was drawn with.
    ///
    /// One button, because that is all §12.5.5 assumes. NOTE 2 of that clause: "the term mouse
    /// denotes a generic pointing device that controls the location of a cursor on the screen
    /// and has at least one button". A host with several sends the primary one and keeps the
    /// rest for its own menus.
    Pointer {
        /// Where, in device pixels from the viewport's top-left corner.
        at: (f32, f32),
        /// What the pointer did.
        action: PointerAction,
    },
    /// The bytes the viewer asked for with [`crate::Event::NeedsFile`].
    ///
    /// `None` is a host that will not or cannot supply them, which is a legitimate answer and
    /// not an error: the policy about which files a document may name belongs to whoever owns
    /// the filesystem, and by rule 2 that is never this crate. The refusal is said out loud
    /// rather than swallowed.
    Supply {
        /// What the bytes were wanted for.
        purpose: Purpose,
        /// The file, or nothing.
        bytes: Option<Vec<u8>>,
    },
    /// A worker finished — or failed — the request it was handed.
    ///
    /// A token that does not match the request outstanding is **dropped**, which is the whole
    /// reason the token exists: a page turned while a render was in flight must not be
    /// overwritten by the frame the previous page produced.
    RenderReady {
        /// The token from [`crate::Event::NeedsRender`].
        token: RenderToken,
        /// What the worker did with it.
        rendered: Rendered,
    },
}

/// What the pointer did.
///
/// Three, because §12.5.5 names three situations and two of them are the ends of a press: "[t]he
/// normal appearance shall be used when the annotation is not interacting with the user … [t]he
/// rollover appearance shall be used when the user moves the cursor into the annotation's active
/// area without pressing the mouse button … [t]he down appearance shall be used when the mouse
/// button is pressed or held down within the annotation's active area".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerAction {
    /// The pointer moved with no button down.
    Moved,
    /// The button went down.
    Pressed,
    /// The button came up.
    ///
    /// **This is what activates a link**, and only where the button went down on the same
    /// annotation it came up on. The clause states no rule for that — it describes appearances,
    /// not activation — so this is a choice, and it is the one every pointing interface makes:
    /// a press that is dragged away before release is a press the person changed their mind
    /// about.
    Released,
}

/// What a file the viewer asks a host for is wanted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// §12.7.6.4's import-data action: the file holds §12.7.8's form data.
    ImportData,
}

/// What a worker did with a [`crate::RenderRequest`].
///
/// Three outcomes rather than a `Result<Raster, E>`, because a host that draws straight to its
/// own surface has no raster to hand back and has not failed either. That is the difference
/// between tier 1 and tier 2 of this project's three pixel tiers, and it is the only place in
/// the protocol where the two differ.
#[derive(Debug)]
pub enum Rendered {
    /// Tier 1: the pixels, for the viewer to hold and the host to blit when it is told to.
    ///
    /// The viewer keeps them, so scrolling and exposing repaint from memory rather than
    /// re-rendering. 1920×1080 RGBA is 8.3 MB; one frame per open document is the whole cost.
    Raster(Raster),
    /// Tier 2: the host drew the request onto its own surface and presented it.
    ///
    /// The viewer holds no pixels, so it cannot repaint on the host's behalf: a tier-2 host is
    /// told what changed by [`crate::Event::Damage`] and re-renders. It keeps its own display
    /// list to do that with — the request it was handed is enough.
    Presented,
    /// The request could not be drawn, and this is why.
    ///
    /// Reported rather than swallowed: a viewer that silently shows the previous page when a
    /// render fails is telling the person something false about the document.
    Failed(String),
}

/// Which page to show.
///
/// Relative and absolute in one enum because they are one question — "which page next" — and a
/// host that had to turn a key press into an index would be doing the clamping itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTarget {
    /// A zero-based index, which is the page tree's numbering and not a reader's.
    Index(usize),
    /// The first page.
    First,
    /// The last page.
    Last,
    /// The next page, or nowhere if this is the last.
    Next,
    /// The previous page, or nowhere if this is the first.
    Previous,
    /// A signed number of pages from here, clamped to the document.
    ///
    /// Clamped rather than wrapping: paging past the end and landing back at page one is
    /// disorienting, and the end of a document is information worth feeling.
    Relative(isize),
}

/// How large the page is drawn.
///
/// Two of these are *modes* rather than numbers — a page fitted to the viewport restays fitted
/// when the window is resized, which is the behaviour a fixed scale cannot express.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Zoom {
    /// The whole page, as large as fits.
    FitPage,
    /// The page's width, as large as fits, with the rest scrolled.
    FitWidth,
    /// A fixed magnification: logical pixels per PDF user space unit, where 1.0 is 72 dpi.
    Scale(f32),
    /// One step larger than whatever is showing now.
    In,
    /// One step smaller.
    Out,
}
