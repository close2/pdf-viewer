//! The viewer's document, interpretation and rasterisation in a confined process.
//!
//! `pdf-sandbox` confines three image decoders. This confines everything above them: the
//! document, the content interpreter and the rasteriser — which is where a PDF's bytes actually
//! go, and by far the larger attack surface. `CLAUDE.md` principle 3 asks for it, and
//! `doc/ui-boundary.md` says why it costs one protocol rather than two: the boundary is
//! `Command`/`Event`, and the confined process owns document, interpretation and rasterisation.
//!
//! **What a page crosses as is a measurement rather than a tier**, since ADR 0607 and the
//! seven-hundred-and-thirty-sixth session wired it in: [`Payload::Raster`] where the pixels are
//! smaller, [`Payload::List`] where the marks are — which is almost every page, and which is what
//! a host holding the graphics device needs, because a process holding one cannot be confined at
//! all. It is still one protocol: `Rendered::{Raster, Presented}` was already a payload choice on
//! this boundary, and this makes the choice a comparison of two byte counts.
//!
//! **And a page on the marks arm is not drawn here at all**, since ADR 0640: the worker answers
//! [`viewer_core::Rendered::Listed`], which says *the host took this request's own list* about
//! one page rather than about the viewer, so `viewer_core::MAX_PIXELS` goes on bounding every
//! request this process makes and `Query::Frame` goes on answering for the pages that must still
//! cross as pixels. One consequence is worth stating rather than discovering: **the cancel is
//! about the work this process does**, so on the marks arm it covers the interpretation and there
//! is no rasterisation of ours for it to stop. Drawing the marks is the host's, as it always was
//! — the worker was drawing a second copy and throwing it away.
//!
//! # The shape of it
//!
//! ```text
//!   host                            pdf-view-worker (confined)
//!   ────                            ─────────────────────────
//!   Command  ──────────────────────▶ viewer_core::Viewer
//!                                     │  NeedsRender
//!                                     ▼
//!                                    render_cpu::CpuRasterizer
//!                                     │  RenderReady
//!   Event    ◀──────────────────────  ┘
//!   Query    ──────────────────────▶ Viewer::query
//!   Reply    ◀──────────────────────
//! ```
//!
//! **The confined process is a host of `viewer-core`, and this crate's caller is a host of
//! pixels.** That is the whole design, and it is why so little in `viewer-core` had to change:
//! rules 2, 3 and 4 already forbid it a filesystem, a clock and threads it was not handed, so a
//! process with none of those three is exactly the environment it was written for. **One
//! outcome** in all of it — [`viewer_core::Rendered::Listed`], ADR 0640 — and it was needed
//! because this host is the first that keeps *some* pages' marks and hands *others* back as
//! pixels, which nothing in that vocabulary could say.
//!
//! Two messages therefore never cross, and both because the confined side answers them itself:
//! [`viewer_core::Event::NeedsRender`] and [`viewer_core::Command::RenderReady`]. **Everything
//! else crosses**, including all twenty-nine questions — the eleven a panel is made of since the
//! three-hundred-and-eighty-sixth session (ADR 0223), and four more since: `Offset` and
//! `FieldSelection` (ADR 0225), `Fields` (ADR 0235) and `FreeTextAt` (ADR 0238). What is left of
//! [`Uncarried`] is those two and three contents an *answer* can hold that this build cannot name;
//! each is refused **by name**, which is the difference between a boundary that is incomplete and
//! one that is quietly wrong.
//!
//! # What the host still owns
//!
//! Rule 2's division survives the process boundary unchanged, which is the second reason this
//! was cheap. A confined process has no filesystem *at all*, and it does not need one:
//!
//! - the document arrives in [`viewer_core::Command::Open`] — as bytes where the host holds
//!   it in memory, and **as the open file's descriptor where the host opened it on disk**
//!   (ADR 0812): the host sends the descriptor beside the frame with `SCM_RIGHTS`, and the
//!   worker holds the same open file and reads it where the document's offsets point, through
//!   the one system call the filter admits on it. A descriptor to one file is not a file
//!   system: the worker can still name no path, and a 6 GB document costs it its trailer,
//!   its table and page one rather than 6 GB down a pipe and twice that in its address space;
//! - a file the document asks for arrives in [`viewer_core::Command::Supply`], after
//!   [`viewer_core::Event::NeedsFile`] asked the host for it;
//! - a saved file and an extracted attachment leave as bytes, for the host to write.
//!
//! `Command::Save`'s own documentation predicted this before there was a confined process to
//! prove it: the host writes the bytes, "which is also what lets a confined process with none
//! still produce a saved file".
//!
//! # What bounds a hostile document
//!
//! Not a deadline, and not the confined process's cooperation. A page's cost is bounded by the
//! document *and* the magnification together, so any fixed number refuses work a viewer permits;
//! and a check the interpreter polls is a check a document decides when to reach. What is here
//! instead is [`Canceller`] — a handle another thread holds, whose `cancel` ends the worker with
//! a signal it cannot decline — beside the address-space ceiling the confinement already
//! installs. ADR 0241 argues both halves.
//!
//! # And what a host does once the worker is gone
//!
//! A worker dies for reasons the document chose — the ceiling refusing an allocation, the filter
//! firing — and until [`Resuming`] existed the only host on this boundary read every one of them
//! as the end of the document. **It is the end of one worker.** The confinement is what makes
//! another one cheap and safe: the process held nothing but the document's bytes, which are the
//! host's by rule 2, so a host starts a second worker, opens the file again, goes back to the page
//! the reader was on, and does not send the command that killed the first. `Resuming` owns the
//! part two confined hosts must not disagree about — which errors are worth another worker, and
//! how many in a row are enough — and nothing else, because the file and the window are a host's.
//!
//! # What this is not, yet
//!
//! It is not on the flagship's launch path: `pdf-viewer`, `pdf-viewer-gtk` and `pdf-viewer-qt`
//! still hold their viewer in process, so a transport cannot cost their first frame anything
//! before the decision to spend it has been argued. **What uses it since the
//! seven-hundred-and-seventy-fifth session is a window of its own** — `viewer-ui`'s
//! `pdf-viewer-confined`, deliberately the smallest complete host on this boundary (ADR 0713).
//! `doc/todo/34` holds what is left; ADR 0218 holds the argument.

#![forbid(unsafe_code)]
// Nothing here needs it. The confinement is `pdf-sandbox`'s, which reaches seccomp-BPF and
// Landlock through safe wrappers, and everything else in this crate is bytes and pipes.

mod protocol;
mod resume;
mod worker;

pub use protocol::display_list::{Crossing, RasterReason, Uncodable};
pub use protocol::{ProtocolError, Uncarried};
pub use resume::{RESTARTS, Reopen, Resume, Resuming};
pub use worker::{WorkerLimits, confine, serve};

/// What a host holds so that it can end a confined viewer from another thread.
///
/// `confined_transport`'s, because a cancel is a `SIGKILL` and a `SIGKILL` knows nothing about
/// what the process was doing (ADR 0846). Everything the type's own documentation says about why
/// a cancel is a kill rather than a message applies here unchanged.
pub use confined_transport::Canceller;

/// The decoders on their own, without the process that produced the bytes.
///
/// [`Confined`] spawns a worker and holds both ends of the pipe, which is what a host normally
/// wants. This is the *reading* half by itself, and it is public for one reason said twice:
///
/// - **The confined side is the untrusted side of this boundary.** A worker interprets hostile
///   documents by design, so what it writes back is untrusted input in exactly the sense a content
///   stream is — and this project's rule is that a parser is fuzzed. A fuzz target lives outside
///   this crate and cannot reach a private module, which is why [`answer`](wire::answer) and
///   [`events`](wire::events) are here: `fuzz/fuzz_targets/confined_wire.rs`.
/// - **`pdf-view-worker` is a program**, so its standard input is whatever was piped into it
///   rather than necessarily a host of this crate's making. [`command`](wire::command) and
///   [`query`](wire::query) are the decoders it runs on those bytes, and they are fuzzed for the
///   same reason.
///
/// A host that connects a worker over something other than a pipe reads its frames with the same
/// four functions.
pub mod wire {
    use pdf_render::DisplayList;
    use viewer_core::{Command, Event, Query};

    use crate::{Crossing, ProtocolError, Reply, Uncodable, protocol};

    /// A question read from the wire, holding whatever its answer needs to outlive the message.
    ///
    /// [`Query::Find`] borrows the string a host already has, and there is no such string on the
    /// receiving side — it arrived in the message. So this owns it, and [`Self::as_query`] lends
    /// it back for as long as this value lives.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Question(protocol::OwnedQuery);

    impl Question {
        /// The question, borrowed for as long as this value lives.
        #[must_use]
        pub fn as_query(&self) -> Query<'_> {
            self.0.as_query()
        }
    }

    /// Reads everything one command caused, as a worker writes it.
    ///
    /// # Errors
    ///
    /// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
    /// a length is larger than the message that states it, or bytes are left over.
    pub fn events(bytes: &[u8]) -> Result<Vec<Event>, ProtocolError> {
        protocol::decode_events(bytes)
    }

    /// Reads one answer, as a worker writes it.
    ///
    /// # Errors
    ///
    /// See [`events`].
    pub fn answer(bytes: &[u8]) -> Result<Reply, ProtocolError> {
        protocol::decode_answer(bytes)
    }

    /// Reads one command, as a confined worker reads it.
    ///
    /// # Errors
    ///
    /// See [`events`].
    pub fn command(bytes: &[u8]) -> Result<Command, ProtocolError> {
        protocol::decode_command(bytes)
    }

    /// Reads one question, as a confined worker reads it.
    ///
    /// # Errors
    ///
    /// See [`events`].
    pub fn query(bytes: &[u8]) -> Result<Question, ProtocolError> {
        protocol::decode_query(bytes).map(Question)
    }

    /// Reads one page's resolved marks, as a confined worker writes them (ADR 0607).
    ///
    /// **The fifth decoder, and the one whose input is a whole page of geometry.** It is public
    /// for this module's own reason — `fuzz/fuzz_targets/display_list.rs` cannot reach a private
    /// module — and it is the one a host runs on the payload a confined process chose to send as
    /// a list rather than as pixels.
    ///
    /// # Errors
    ///
    /// See [`events`], and [`ProtocolError::OutOfTable`] and [`ProtocolError::Unbuildable`],
    /// which are this decoder's own.
    pub fn display_list(bytes: &[u8]) -> Result<DisplayList, ProtocolError> {
        protocol::display_list::decode(bytes)
    }

    /// Writes one page's resolved marks down, as a confined worker does.
    ///
    /// # Errors
    ///
    /// [`Uncodable`] where the list holds one of the two deferred producers ADR 0607 leaves to
    /// the raster arm, or nests past what a backend composites. [`crossing`] is the caller that
    /// turns those into the payload choice.
    pub fn encode_display_list(list: &DisplayList) -> Result<Vec<u8>, Uncodable> {
        protocol::display_list::encode(list)
    }

    /// Which payload one page crosses as, given what its raster would cost (ADR 0607).
    #[must_use]
    pub fn crossing(list: &DisplayList, raster_bytes: u64) -> Crossing {
        protocol::display_list::crossing(list, raster_bytes)
    }
}

use std::path::PathBuf;
use std::sync::Arc;

use confined_transport::{Host, TransportError};
use pdf_render::{DisplayList, Raster};
use pdf_sandbox::lockdown::Confinement;
use pdf_syntax::ObjectId;
use viewer_core::{Command, DocumentId, Event, PageGeometry, Query};

/// The name of the worker program, **without the platform's executable suffix**.
///
/// A separate executable rather than this one re-invoked with a flag, for the reason
/// [`pdf_sandbox::WORKER_PROGRAM`] gives: everything it links is reachable only from a `main`
/// whose first statement gives away the ability to do anything but interpret a page.
pub const WORKER_PROGRAM: &str = "pdf-view-worker";

/// Environment variable naming the worker program explicitly.
///
/// Set it when the executable is not installed alongside its worker — in particular, when
/// running a test binary that Cargo has put in `target/<profile>/deps/`.
pub const WORKER_PATH_VARIABLE: &str = "PDF_VIEW_WORKER";

/// Why a confined viewer could not do what was asked.
///
/// Every variant is a *refusal*, and none of them is a reason to carry on as though the work had
/// happened somewhere else: there is deliberately no path in this crate that interprets a
/// document in the calling process when the worker cannot be started, which is the same rule
/// `pdf-sandbox` states and for the same reason.
///
/// # Why this one *is* `#[non_exhaustive]`, beside a vocabulary where nothing is
///
/// `doc/ui-boundary.md` says of the messages this transport carries: *"Nothing is
/// `#[non_exhaustive]`, deliberately: it forces a catch-all arm on every host, and a catch-all arm
/// is where a message added later goes to be ignored in silence."* ADR 0734 recorded the tension
/// and left it; the line between the two is where a value comes from, and it is worth stating
/// rather than leaving as an inconsistency somebody trips over.
///
/// **That rule binds what crosses inside a message.** A [`viewer_core::Event`] is a *vocabulary*:
/// its population is this project's own, every member of it means something a host has to decide
/// about, and one added later that a host silently ignores is a feature that never reached a
/// person. So the enum stays closed, the host fails to compile, and somebody decides. The rule
/// reaches types this crate does not declare for the same reason — `pdf_render::RasterFormat`
/// crosses *inside* [`Reply::Frame`] and stopped being `#[non_exhaustive]` for it.
///
/// **This is not a vocabulary; it is a failure population, and it is the kernel's.** A seccomp
/// filter, a Landlock ruleset, a pipe and an address-space ceiling can fail in ways this crate
/// will learn about after a host has shipped, and a host recompiled by every one of them would be
/// paying for a fact about somebody's kernel. What a host must *decide* about a refusal is not
/// which one it is: it is whether another worker is worth starting, and that question has a closed
/// answer — [`Resume`], two arms, matched exhaustively, with [`Resuming::after`] holding the
/// wildcard-free match over every variant here **inside this crate**, where `#[non_exhaustive]`
/// does not apply. So the decision a host takes is protected by exactly the mechanism the rule
/// asks for, and the attribute costs nothing: a variant added here stops *this* crate's build
/// until somebody has said which of the two answers it is, and a host that only prints the
/// sentence goes on printing it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfinedError {
    /// The worker program could not be found.
    #[error(
        "the confined viewer `{WORKER_PROGRAM}` was not found next to {executable} — build it \
         with `cargo build -p viewer-confined --bins`, or name it with ${WORKER_PATH_VARIABLE}"
    )]
    WorkerMissing {
        /// The executable whose directory was searched.
        executable: PathBuf,
    },
    /// The worker program could not be started.
    #[error("starting the confined viewer failed: {0}")]
    Spawn(#[source] std::io::Error),
    /// The worker stopped before answering.
    #[error("the confined viewer stopped without answering ({detail})")]
    WorkerDied {
        /// How it stopped, as far as the host can tell.
        detail: String,
    },
    /// The pipe to or from the worker failed.
    #[error("the connection to the confined viewer failed: {0}")]
    Connection(#[source] std::io::Error),
    /// The worker sent something that is not a well-formed message.
    #[error("the confined viewer sent a malformed message: {0}")]
    Malformed(#[from] ProtocolError),
    /// The worker sent a frame this build does not define, or one too large to be honest.
    #[error("the confined viewer sent an unrecognised frame")]
    UnrecognisedFrame,
    /// The message does not cross this boundary.
    #[error(transparent)]
    Uncarried(#[from] Uncarried),
    /// The worker refused the message, in its own words.
    ///
    /// What a host gets when it asks the *worker* for something the worker will not do — the
    /// mirror of [`Self::Uncarried`], which is the same refusal caught on this side before
    /// anything was sent.
    #[error("the confined viewer refused: {detail}")]
    Refused {
        /// The worker's own sentence.
        detail: String,
    },
    /// The worker sent a message this process could not find room for.
    ///
    /// **The mirror of the worker's own refusal, and it is here because the worker is the
    /// untrusted side of this boundary**: a length it states is a claim, and a host that believed
    /// a two-gibibyte claim would abort on the allocation instead of refusing it. The frame's
    /// bytes are read and thrown away, so the worker and its document survive the refusal —
    /// which is the same reason its side reads past a message it will not hold.
    #[error("the confined viewer sent {bytes} bytes and this process could not find room for them")]
    NoRoom {
        /// What the frame header claimed.
        bytes: usize,
    },
    /// A [`Canceller`] ended the worker, so this viewer has nothing left to ask.
    ///
    /// Distinct from [`Self::WorkerDied`] on purpose: a worker that died is a fault to report,
    /// and a worker the host itself ended is not. Once this is returned it is what every later
    /// call returns — the worker is gone, and with it the document, so there is nothing to
    /// resume. A host that wants to go on starts another one.
    #[error("the confined viewer was cancelled, and its worker ended with it")]
    Cancelled,
}

impl From<TransportError> for ConfinedError {
    /// The transport's failure population, in this crate's words.
    ///
    /// **A wildcard-free match**, which is what keeps ADR 0846's split honest: a variant added to
    /// the shared transport stops this crate's build until somebody has said which refusal it is
    /// here, exactly as `doc/ui-boundary.md` asks of the vocabulary.
    fn from(error: TransportError) -> Self {
        match error {
            TransportError::Spawn(error) => Self::Spawn(error),
            TransportError::WorkerDied { detail } => Self::WorkerDied { detail },
            TransportError::Connection(error) => Self::Connection(error),
            TransportError::UnrecognisedFrame => Self::UnrecognisedFrame,
            TransportError::NoRoom { bytes } => Self::NoRoom { bytes },
            TransportError::Cancelled => Self::Cancelled,
            // `TransportError` is `#[non_exhaustive]` for the reason this enum is: a kernel can
            // fail in ways this tree learns about after a host has shipped. A refusal nobody has
            // named yet is still a refusal, and it arrives with its own sentence.
            other => Self::WorkerDied {
                detail: other.to_string(),
            },
        }
    }
}

/// One page as it crossed the confinement, and where it belongs on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Framed {
    /// Which page this is of.
    pub page: usize,
    /// The pixels, or the marks that make them — see [`Payload`].
    pub payload: Payload,
    /// Where the page's top-left corner sits in the viewport, in device pixels.
    ///
    /// The same number for either payload: it is where the *page* goes, and a host that
    /// rasterises a list for itself puts the result exactly where it would have put pixels.
    pub origin: (f32, f32),
}

/// What one page crossed the confinement as (ADR 0607).
///
/// **Chosen per page, by size, in the confined process**, which is the whole of ADR 0607's
/// decision: a display list is scale-invariant and a raster is quadratic in the scale, so the
/// list is a small fraction of the pixels for almost every page — and larger for exactly one
/// population, a scan, whose decoded samples *are* its display list. Two of `doc/pdf.js`'s first
/// pages carry a producer this format defers ([`Uncodable`]) and cross as pixels for that reason
/// instead.
///
/// **Nothing is `#[non_exhaustive]`**, for `doc/ui-boundary.md`'s reason: a third payload should
/// fail to compile in every host rather than fall into a catch-all arm.
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    /// The pixels, drawn in the confined process on the processor.
    ///
    /// Row-major RGBA, no padding. What every page crossed as before ADR 0607, and what a page
    /// still crosses as when its pixels are the smaller of the two.
    Raster(Raster),
    /// The page's marks, for a host that holds the graphics device to draw for itself.
    ///
    /// **The device is the host's by necessity and not by preference**: a process holding one
    /// dies on its first `ioctl` under this confinement, measured over four orderings (ADR 0607),
    /// so the process that interprets a hostile document cannot be the process that draws it on a
    /// device. What crosses instead is what is left after the standard has been read.
    List {
        /// The marks, resolution-independent.
        ///
        /// Shared because a host keeps it: a zoom or a scroll is a new [`pdf_render::TargetSpec`]
        /// over the same list, which is the difference between re-rasterising and asking the
        /// confined process for another frame. **The identity holds across frames, not only
        /// inside one**: [`Confined`] hands back the same `Arc` while a page's encoded bytes are
        /// unchanged (ADR 0725) — for the whole of this crate's life before that, every
        /// [`Query::Frame`] decoded a fresh `Arc` and everything a host keyed by this identity
        /// missed on every scroll.
        list: Arc<DisplayList>,
        /// What the confined process would have drawn into, and the transform to it.
        ///
        /// Carried rather than rebuilt from the list's page size and a scale, because a target
        /// carries the y flip and any tile offset besides — and `doc/traps/the-interactive-loop.md`'s
        /// trap 12a is a doc comment that claimed the display list's space *was* the raster's.
        target: pdf_render::TargetSpec,
    },
}

/// One page's sentences about what it could not draw, as they crossed the confinement.
///
/// [`viewer_core::PageReports`] with the notes owned, for the reason [`Reply`] gives: there is no
/// viewer on this side of the pipe to borrow them from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    /// Which page these are about, zero-based.
    pub page: usize,
    /// What that page could not draw, already worded.
    pub notes: Vec<String>,
}

/// One page's counts of what could not be *read*, as they crossed the confinement.
///
/// [`viewer_core::PageReadback`], and a count rather than a report for the reason that type gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadShort {
    /// Which page these counts are about, zero-based.
    pub page: usize,
    /// What that page's text cost the reader.
    pub shortfall: pdf_model::content::Shortfall,
}

/// One page's §14.7 structure, as it crossed the confinement.
///
/// [`viewer_core::PageStructure`], with that type's rule about the indices: a node's parent and
/// its headers index **this page's** list and no other.
#[derive(Debug, Clone, PartialEq)]
pub struct Structured {
    /// Which page this tree is of, zero-based.
    pub page: usize,
    /// §14.7's structure for it, parent-first, in §14.8.2.5's logical order.
    pub nodes: Vec<viewer_core::AccessibilityNode>,
}

/// The answer to a [`Query`], owned.
///
/// [`viewer_core::Answer`] borrows the viewer's own state, and there is no viewer on this side of
/// the pipe — so what arrives is the same answer with its parts owned. **One variant per question
/// [`viewer_core::Query`] states, since the three-hundred-and-eighty-sixth session**: the eleven
/// that used to have none here were the eleven a panel is made of, and a host on this boundary
/// therefore had no panels. ADR 0223.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// Nothing to answer with: no document is focused, or the question named a page that is not
    /// showing.
    None,
    /// A count of pages.
    Count(usize),
    /// Where the reader is looking: the page, the magnification and the scroll.
    ///
    /// The one reply a host hands straight back, as [`viewer_core::Command::View`] — which is
    /// what puts a reader where they were after [`Resuming`] has started another worker. It
    /// carries [`viewer_core::Viewing`] itself rather than an owned copy of it, because unlike
    /// every borrowed answer beside it there is nothing here to own: three numbers and a mode.
    View(viewer_core::Viewing),
    /// Which page is showing.
    Page {
        /// Which document.
        document: DocumentId,
        /// The zero-based index.
        index: usize,
        /// §12.4.2's label, where the document states one.
        label: Option<String>,
        /// How many pages there are.
        of: usize,
    },
    /// Where the page sits and how large it is drawn.
    Geometry(PageGeometry),
    /// §12.4.2's label for the page asked about.
    Label(String),
    /// Whether a link is under the point asked about.
    Link(bool),
    /// What is selected, and the shapes covering it in device pixels.
    Selected {
        /// The selected text, as the page reads back.
        text: String,
        /// One quadrilateral per run of a line.
        quads: Vec<[f32; 8]>,
    },
    /// What the field at a point is called, and what it says now.
    Field {
        /// §12.7.4.2's fully qualified name.
        qualified: String,
        /// Table 226's `/TU`, where the field states one.
        alternative: Option<String>,
        /// The value as §12.7.4.3 would lay it out, or `None` where it is not text.
        ///
        /// Carries Table 231 bit 14's `obscured` beside the characters, because
        /// [`viewer_core::Answer::Field`] does and a reply that dropped it would be a host on
        /// this side of the pipe learning less than a host on the other (ADR 0247).
        value: Option<pdf_model::view::ShownValue>,
    },
    /// Where the caret is, in device pixels of the viewport.
    Caret {
        /// The end on the descent side of the baseline.
        from: (f32, f32),
        /// The end on the ascent side.
        to: (f32, f32),
    },
    /// How far into a field's value a point is, in bytes — the caret's inverse.
    Offset(usize),
    /// The shapes covering a range of a field's value, one per line it touches.
    FieldSelection(Vec<[f32; 8]>),
    /// §12.5.6.6's annotation at a point, and Table 166's `/Contents` as it now stands.
    FreeText {
        /// The object, which [`viewer_core::Edit::SetFreeText`] names it by.
        annotation: ObjectId,
        /// What it says.
        text: String,
    },
    /// Where a string occurs, one entry per occurrence.
    Found(Vec<Vec<[f32; 8]>>),
    /// Annex O's highlighted rectangles on the page being shown.
    Highlighted(Vec<[f32; 8]>),
    /// Whether anything has been edited.
    Dirty(bool),
    /// The focused annotation and the quadrilateral covering it.
    Focus {
        /// The annotation itself.
        object: ObjectId,
        /// Its `/Rect` on the screen.
        quad: [f32; 8],
    },
    /// §14.8.2.5's logical content order for the selection.
    LogicalSelection(String),
    /// **The page**, interpreted behind the seccomp filter — one entry per page on the screen.
    ///
    /// The payload the whole boundary exists for. What each entry carries is [`Payload`]'s
    /// subject and is chosen per page by size: the pixels the confined process drew, or the marks
    /// for a host to draw itself. A *list* of entries since Table 29's `/PageLayout` was obeyed,
    /// because `OneColumn` puts several pages in one window.
    Frame(Vec<Framed>),
    /// What the pages on the screen could not draw, one entry per page.
    ///
    /// A list since the six-hundred-and-tenth session, for the reason [`Reply::Frame`] is one:
    /// a column shows several pages and a status bar carrying the current page's sentences for
    /// four of them would be silent about three.
    Reports(Vec<Reported>),
    /// What the pages on the screen could not be read as: the per-code counts, never a report.
    ///
    /// See [`viewer_core::Query::Readback`] for why counting is the answer here and reporting is
    /// not.
    Readback(Vec<ReadShort>),
    /// §12.3.3's outline, whole.
    Outline(pdf_model::outline::Outline),
    /// §8.11.4.3's layers, in `/Order`.
    Layers(Vec<viewer_core::Layer>),
    /// §7.11.4's embedded files, listed.
    ///
    /// [`Attachment`] rather than [`pdf_model::attachment::Attachment`], and the difference is the
    /// stream: see that type.
    Attachments(Vec<Attachment>),
    /// §12.3.5's portable collection, read, and the document it opens on.
    ///
    /// Boxed because it is Table 153, Table 155's columns, Table 156's sort, Table 158's split,
    /// Table 159's folder tree and Table 160's navigator together, and an enumeration is as large
    /// as its largest variant — a `Reply::Count` would otherwise cost what a collection costs.
    Collection {
        /// Table 153, whole.
        collection: Box<pdf_model::collection::Collection>,
        /// §12.3.5.1's `/D`, resolved against the `/EmbeddedFiles` name tree the confined process
        /// holds and this one does not. See [`viewer_core::Answer::Collection`].
        initial: pdf_model::collection::Initial,
    },
    /// §12.4.3's article threads, in the `/Threads` array's own order.
    Articles(Vec<pdf_model::article::Thread>),
    /// §12.3.4's thumbnail for the page asked about, decoded.
    Thumbnail(pdf_model::thumbnail::Thumbnail),
    /// §14.3.3's Table 349, and §14.3.2's metadata stream beside it.
    Properties {
        /// What the trailer's `/Info` says. Boxed for [`Self::Collection`]'s reason.
        information: Box<pdf_model::metadata::Information>,
        /// The catalog's `/Metadata`, read — `None` where the document names none, and
        /// `Some(Err(_))` where it names one this reader refused.
        metadata: Option<Result<pdf_model::xmp::Xmp, pdf_model::xmp::XmpError>>,
    },
    /// Table 29's `/PageMode` and `/PageLayout`.
    Opening(pdf_model::viewer_preferences::Opening),
    /// §12.2's Table 147, whole. Boxed for [`Self::Collection`]'s reason.
    Preferences(Box<pdf_model::viewer_preferences::ViewerPreferences>),
    /// §12.5.6.14's open popup windows, in the `/Annots` array's order.
    Popups(Vec<viewer_core::PopupWindow>),
    /// §12.7's form fields with a widget on the page being shown, in `/Annots` order.
    ///
    /// The twelfth answer a host builds an interface out of, and the one that makes a *form*
    /// possible on this boundary: without it a confined host could place native controls over
    /// every other kind of chrome and would have had to take fields as pixels. ADR 0235.
    Fields(Vec<viewer_core::FormField>),
    /// §14.7's structure for the page being shown, in §14.8.2.5's logical order, parent-first.
    Accessibility(Vec<Structured>),
}

/// One of §7.11.4's embedded files, as a panel lists them.
///
/// [`pdf_model::attachment::Attachment`] without its stream, and **the absence is in the type
/// rather than in a comment**. A panel showing five attachments shows five names, five
/// descriptions and five sizes; making it also pull five payloads across the pipe would be paying
/// a document's whole weight to draw a list. The bytes have their own channel and always did —
/// [`viewer_core::Command::Extract`] names one file and [`viewer_core::Event::Extracted`] brings
/// it back — and it is the channel a host uses when a person clicks *save*.
///
/// Everything else Table 43, Table 44 and Table 45 state crosses, field for field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// The name the `/EmbeddedFiles` tree filed it under, or the file specification's own name.
    pub name: String,
    /// Table 43's `/UF`, or `/F` where the file states no Unicode form.
    pub file_name: Option<String>,
    /// Table 43's `/Desc`.
    pub description: Option<String>,
    /// Table 44's `/Subtype`: the embedded file's MIME media type, as the file spells it.
    pub media_type: Option<String>,
    /// Table 45's `/Size`, which is the document's claim rather than a measurement.
    pub size: Option<i64>,
    /// Table 45's `/CreationDate`, as the §7.9.4 date string the file wrote.
    pub created: Option<String>,
    /// Table 45's `/ModDate`, likewise.
    pub modified: Option<String>,
    /// Table 45's `/CheckSum`, carried rather than checked — checking means inflating the stream,
    /// and the stream is on the other side of this boundary.
    pub checksum: Option<Vec<u8>>,
    /// Table 43's `/AFRelationship`, §14.13's own subject.
    pub relationship: pdf_model::attachment::Relationship,
}

impl Attachment {
    /// Whether the bytes [`viewer_core::Event::Extracted`] brought back are the ones Table 45's
    /// `/CheckSum` describes.
    ///
    /// The half of §7.11.4.1 this boundary would otherwise have taken away. `pdf-model` answers
    /// it on an attachment that has its stream; a host on this boundary has the checksum in one
    /// message and the payload in another, and asking would have meant reimplementing the clause
    /// or not asking. [`pdf_model::attachment::checksum_matches`] is the one implementation.
    ///
    /// `None` where the file states none, which is most of them. The clause is explicit about
    /// what an answer is worth: it "is strictly a checksum, and is not used for security
    /// purposes", so a mismatch is a producer's mistake worth reporting rather than a reason to
    /// withhold the bytes.
    #[must_use]
    pub fn checksum_matches(&self, bytes: &[u8]) -> Option<bool> {
        pdf_model::attachment::checksum_matches(self.checksum.as_deref(), bytes)
    }
}

/// A confined viewer: a `viewer-core` host in a process with no filesystem and no network.
///
/// **Every call here blocks for as long as the document takes**, and that is deliberate — see
/// [`Canceller`] for why a page has no deadline and what a host holds instead.
#[derive(Debug)]
pub struct Confined {
    /// The wire: the child, the socket its frames go down, and the pipe they come back up.
    host: Host,
    /// The lists the last frame handed this host, so an unchanged page's bytes come back as the
    /// same `Arc` on the next one — what makes [`Payload::List`]'s sharing promise true across
    /// two [`Query::Frame`]s rather than only inside one (ADR 0725).
    held: protocol::HeldLists,
}

impl Confined {
    /// Starts a confined viewer and reads its greeting.
    ///
    /// The greeting is what the worker reached, not what it asked for: a kernel can refuse what a
    /// build offers, so [`Self::confinement`] is a report rather than a promise.
    ///
    /// This blocks until the worker has confined itself and said so. Use [`Self::start_with`]
    /// where a host wants to be able to give up on that too.
    ///
    /// # Errors
    ///
    /// See [`ConfinedError`]. A worker that cannot confine itself never sends a greeting, so it
    /// arrives here as [`ConfinedError::WorkerDied`] — never as a viewer that quietly runs
    /// unconfined.
    pub fn start() -> Result<Self, ConfinedError> {
        Self::start_with(&Canceller::new())
    }

    /// Starts a confined viewer that the given [`Canceller`] can end, including while it starts.
    ///
    /// The canceller is armed before the worker is spawned, which closes the one gap
    /// [`Self::canceller`] cannot: `start` blocks reading the greeting, and a handle obtained from
    /// the value `start` returns does not exist while it is blocked.
    ///
    /// # Errors
    ///
    /// See [`ConfinedError`], and [`ConfinedError::Cancelled`] where the canceller fired — before
    /// the spawn, in which case nothing was started, or during the greeting, in which case what
    /// was started has been ended and waited for.
    pub fn start_with(canceller: &Canceller) -> Result<Self, ConfinedError> {
        let program = worker_program()?;
        Ok(Self {
            host: Host::start(&program, protocol::MAGIC, canceller)?,
            held: protocol::HeldLists::default(),
        })
    }

    /// A handle another thread can end this viewer with.
    ///
    /// Cheap, and any number of them may exist. See [`Canceller`] for why the only cancel this
    /// offers is one the confined process cannot decline.
    #[must_use]
    pub fn canceller(&self) -> Canceller {
        self.host.canceller()
    }

    /// Whether this viewer has been cancelled, in which case every call returns
    /// [`ConfinedError::Cancelled`].
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.host.is_cancelled()
    }

    /// What confinement the worker reached.
    ///
    /// [`Confinement::shortfall`] is the sentence to print when it is not everything: a host that
    /// believed itself confined and was not is the failure this whole arrangement exists to
    /// prevent.
    #[must_use]
    pub fn confinement(&self) -> Confinement {
        self.host.confinement()
    }

    /// Performs one command in the confined process and returns everything it caused.
    ///
    /// The render round trip happens on the other side, so a command that changes the page comes
    /// back with the page already drawn: [`Query::Frame`] has the pixels.
    ///
    /// # Errors
    ///
    /// See [`ConfinedError`]. [`ConfinedError::Uncarried`] for
    /// [`viewer_core::Command::RenderReady`], which the confined process answers itself.
    pub fn handle(&mut self, command: &Command) -> Result<Vec<Event>, ConfinedError> {
        let payload = protocol::encode_command(command)?;
        let (kind, payload) = self.host.exchange(
            protocol::FRAME_COMMAND,
            &payload,
            protocol::command_descriptor(command),
        )?;
        match kind {
            protocol::FRAME_EVENTS => protocol::decode_events(&payload).map_err(Into::into),
            protocol::FRAME_REFUSAL => Err(refusal(&payload)),
            _ => Err(ConfinedError::UnrecognisedFrame),
        }
    }

    /// Asks the confined viewer a question.
    ///
    /// # Errors
    ///
    /// See [`ConfinedError`]. **Every question crosses**; what can still come back is
    /// [`ConfinedError::Refused`], where the *answer* held something this build cannot name — a
    /// raster in a second pixel layout, a §7.11.6 collection value outside Table 47's three
    /// kinds, or a metadata failure `pdf_model::xmp` grew after this build. Each says which.
    pub fn query(&mut self, query: Query<'_>) -> Result<Reply, ConfinedError> {
        let payload = protocol::encode_query(query)?;
        let (kind, payload) = self.host.exchange(protocol::FRAME_QUERY, &payload, None)?;
        match kind {
            protocol::FRAME_ANSWER => {
                protocol::decode_answer_reusing(&payload, &mut self.held).map_err(Into::into)
            }
            protocol::FRAME_REFUSAL => Err(refusal(&payload)),
            _ => Err(ConfinedError::UnrecognisedFrame),
        }
    }
}

/// Reads a refusal frame into an error.
fn refusal(payload: &[u8]) -> ConfinedError {
    ConfinedError::Refused {
        detail: String::from_utf8_lossy(payload).into_owned(),
    }
}

/// Finds the worker program.
///
/// Searched next to the running executable, then one directory up, because Cargo puts test
/// binaries in `target/<profile>/deps/` while it puts programs in `target/<profile>/`. The
/// environment variable overrides both. The search is `confined_transport`'s; the *sentence* a
/// missing worker produces is this crate's, because it names this crate's build command.
fn worker_program() -> Result<PathBuf, ConfinedError> {
    confined_transport::program_beside_executable(WORKER_PROGRAM, WORKER_PATH_VARIABLE).map_err(
        |missing| match missing {
            confined_transport::ProgramMissing::NotBeside { executable } => {
                ConfinedError::WorkerMissing { executable }
            }
            confined_transport::ProgramMissing::Unlocatable(error) => ConfinedError::Spawn(error),
        },
    )
}
