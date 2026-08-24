//! Toolkit-independent application logic.
//!
//! Owns everything the user interface needs but that does not depend on a windowing
//! toolkit: the open-document set, view state (page, zoom, scroll), the render
//! scheduler's bookkeeping, and what a document says about itself.
//!
//! No type from a windowing or graphics library appears in this crate's API. That is
//! what keeps `viewer-ui` replaceable — the project has already changed windowing
//! decisions once, from Qt to winit — and it is what makes the application logic
//! testable without a display.
//!
//! # The shape: two channels, and why messages rather than a trait
//!
//! ```text
//! host toolkit  ──Command──▶  Viewer  ──Event──▶  host
//!                             query(&self, Query) ──▶ Answer
//!                                 │
//!                                 └─Event::NeedsRender─▶ worker ─Command::RenderReady─┘
//! ```
//!
//! [`Viewer::handle`] takes one [`Command`] and returns the [`Event`]s it produced;
//! [`Viewer::query`] answers a [`Query`] from `&self`, synchronously, producing no events at
//! all. Two channels rather than one because **a selection cannot wait for a render round
//! trip**: hit-testing a point and asking for a page's geometry happen at pointer speed, and a
//! host that had to post a command and wait for an event to come back would feel it.
//!
//! Messages rather than a callback trait, for four reasons specific to this project: the state
//! machine is testable with no display, which is what `viewer-core` exists for; the vocabulary
//! survives an FFI boundary unchanged, which `AppKit`, `WinUI` and Qt will each need; it survives a
//! *process* boundary unchanged, which is what principle 3's confined renderer needs; and it
//! needs no runtime, which `CLAUDE.md` forbids.
//!
//! # Five rules
//!
//! 1. **[`pdf_syntax::Document`] is immutable.** What a user does to a document is a log beside
//!    it, never a change to it — the pattern `pdf_model::view::ViewState` already uses for
//!    §12.6.4's actions. Interpretation stays a pure function of the file's bytes and the view
//!    state, which is what makes the reference oracle's comparison of 1665 pages mean anything.
//! 2. **No filesystem.** The host supplies bytes and the host receives bytes. A document naming
//!    a file is a document asking this machine for something, and whether to give it is not a
//!    rendering decision.
//! 3. **No clock.** Anything timed arrives as a command carrying the elapsed milliseconds.
//! 4. **No threads this crate was not handed, and nothing blocking.** Rasterisation happens
//!    wherever the host runs it; this crate only schedules it.
//! 5. **No toolkit or graphics type in the public API.** [`pdf_render::Raster`] is this
//!    project's own row-major RGBA with no row padding — a blit target for GDI, `CGImage`,
//!    `QImage`, `GdkTexture` and cairo without conversion — and it is the widest the interface
//!    goes.
//!
//! # What crosses as pixels, and what does not
//!
//! Page content crosses as pixels, and changes when the page, the zoom or an edit does.
//! Interactive chrome — a selection highlight, a caret, a rubber band — changes at pointer
//! speed and crosses as *geometry*, so that a native host can draw it in macOS's selection
//! colour, KDE's accent or the Windows highlight brush, with its own caret blink and focus
//! ring. That is most of what makes an embedded view feel native, and it is unreachable if a
//! host is handed finished pixels. [`Query`] is that channel.
//!
//! # Who interprets, and who rasterises
//!
//! This crate interprets: [`Event::NeedsRender`] carries a finished [`pdf_render::DisplayList`]
//! and the [`pdf_render::TargetSpec`] to draw it at, and the host rasterises them wherever it
//! likes. Three reasons, in the order they decided it. Interpretation is a pure function of an
//! immutable document, so it is the half that must not be duplicated per host — a host that
//! interpreted for itself would be a second reading of the same file. A display list is
//! resolution-independent, so **zoom and scroll re-rasterise without re-interpreting**, which
//! is what `TargetSpec` exists to allow and is the difference between a smooth zoom and a
//! parse per frame. And it is what leaves the confined-process boundary in one piece:
//! everything that touches PDF bytes stays on this side of the protocol.
//!
//! The cost is that interpretation runs on whichever thread calls [`Viewer::handle`]. That is a
//! real cost — interpretation is the expensive half — and the answer is that a host wanting it
//! elsewhere moves the *whole* `Viewer` to another thread, which the message vocabulary is
//! designed to allow and a callback trait would not.

//! # Nothing here is `#[non_exhaustive]`
//!
//! Deliberately, and it is the same argument as trap 5. A `#[non_exhaustive]` message type
//! forces every host to write a catch-all arm, and a catch-all arm is where a message this crate
//! adds later goes to be ignored in silence. Leaving the enums closed means a new [`Event`]
//! *fails to compile* in every consumer until somebody decides what it should do there — which
//! is the loud failure this project prefers everywhere else. The cost is that adding a variant is
//! a breaking change for out-of-tree consumers, and that cost is paid deliberately until there
//! are any.

#![forbid(unsafe_code)]

mod accessibility;
mod command;
mod event;
mod interact;
mod layout;
mod notes;
mod open;
mod presentation;
mod query;
mod readback;
mod report;
mod search;
mod select;
mod viewer;

pub mod secret;
pub mod transition;

pub use accessibility::{AccessibilityNode, Character, TextLine};
pub use command::{
    Command, Edit, Find, FindDirection, FocusMove, PageTarget, PointerAction, PresentationMode,
    Purpose, Rendered, RestrictionLevel, Selection, Zoom,
};
pub use event::{Event, Extraction, Found, RenderRequest};
/// What [`Edit::SetField`] puts into a field: §12.7.5.3's characters, §12.7.5.4's chosen options,
/// or nothing.
///
/// Re-exported for the reason [`pdf_model::view::Markup`] is named directly in [`Edit::Markup`]:
/// it is a statement about a *document's* field rather than about a view, so it belongs to the
/// crate that reads documents — and a host should not have to name that crate to send an edit.
pub use pdf_model::view::Entered;
pub use query::{
    Answer, FormField, FormWidget, FrameView, Layer, PageGeometry, PageReadback, PageReports,
    PageStructure, PopupWindow, Query, Selected,
};
pub use readback::ReadbackCache;
pub use secret::Secret;
pub use viewer::{DocumentId, MAX_PIXELS, RenderToken, Viewer};
