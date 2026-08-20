//! The viewer's document, interpretation and rasterisation in a confined process.
//!
//! `pdf-sandbox` confines three image decoders. This confines everything above them: the
//! document, the content interpreter and the rasteriser — which is where a PDF's bytes actually
//! go, and by far the larger attack surface. `CLAUDE.md` principle 3 asks for it, and
//! `doc/ui-boundary.md` says why it costs one protocol rather than two: if the boundary
//! is `Command`/`Event` with `Raster` payloads, the confined process owns document,
//! interpretation and rasterisation, and the host receives pixels and events. The design question
//! that used to be recorded there — whether a display list would have to cross — dissolves,
//! because it never leaves.
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
//! pixels.** That is the whole design, and it is why nothing in `viewer-core` had to change:
//! rules 2, 3 and 4 already forbid it a filesystem, a clock and threads it was not handed, so a
//! process with none of those three is exactly the environment it was written for.
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
//! - the document arrives as bytes in [`viewer_core::Command::Open`];
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
//! # What this is not, yet
//!
//! It is not on the viewer's launch path and nothing in `viewer-ui` uses it — deliberately, so
//! that a transport cannot cost the first frame anything before the decision to spend it has
//! been argued. `doc/todo/34` holds what is left; ADR 0218 holds the argument.

#![forbid(unsafe_code)]
// Nothing here needs it. The confinement is `pdf-sandbox`'s, which reaches seccomp-BPF and
// Landlock through safe wrappers, and everything else in this crate is bytes and pipes.

mod protocol;
mod worker;

pub use protocol::{ProtocolError, Uncarried};
pub use worker::{confine, serve};

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
    use viewer_core::{Command, Event, Query};

    use crate::{ProtocolError, Reply, protocol};

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
}

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command as OsCommand, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use pdf_render::Raster;
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
    /// A [`Canceller`] ended the worker, so this viewer has nothing left to ask.
    ///
    /// Distinct from [`Self::WorkerDied`] on purpose: a worker that died is a fault to report,
    /// and a worker the host itself ended is not. Once this is returned it is what every later
    /// call returns — the worker is gone, and with it the document, so there is nothing to
    /// resume. A host that wants to go on starts another one.
    #[error("the confined viewer was cancelled, and its worker ended with it")]
    Cancelled,
}

/// What a host holds so that it can end a confined viewer from another thread.
///
/// # Why a cancel is a kill, and not a message
///
/// **The confined process is running a hostile document; a cancel it has to agree to is a cancel
/// the document can decline.** A cooperative cancel — a flag the interpreter polls, a message a
/// second thread inside the confinement reads — bounds only the work that reaches a check. A
/// content stream that expands into a hundred million marks, a form nested to its depth limit and
/// branching at every level, a filter chain that inflates for a minute: each of those reaches the
/// next check when it reaches it, and "when it reaches it" is the number the attacker chooses.
/// So the only cancel worth the name is the one the kernel enforces, and that is `SIGKILL`.
///
/// What follows from that is a cost rather than a caveat, and it is in the type: the worker's
/// document, its edits and its frame go when it does. [`ConfinedError::Cancelled`] is what every
/// later call returns, and a host that wants to carry on starts a new [`Confined`].
///
/// # What it does *not* replace
///
/// Not a deadline. A page's cost is bounded by the document and the magnification together, so
/// any fixed number refuses work a viewer permits; what is offered here is the *ability* to
/// decide, on whatever grounds a host has — a person pressing escape, a wall clock the host owns,
/// a second document becoming the one in front. `Confined` deliberately imposes none of them.
///
/// # Shape
///
/// Cheap to clone, and every clone cancels the same worker. It may be made *before* the worker
/// is — [`Canceller::new`] then [`Confined::start_with`] — because [`Confined::start`] itself
/// blocks reading the worker's greeting, and a blocking call whose canceller does not exist yet
/// is exactly the hole this exists to close.
#[derive(Debug, Clone)]
pub struct Canceller(Arc<Cancellation>);

impl Canceller {
    /// A canceller for a worker that has not been started yet.
    ///
    /// Hand it to [`Confined::start_with`]. Cancelling one that never gets a worker is not an
    /// error: it makes that `start_with` fail with [`ConfinedError::Cancelled`] instead of
    /// spawning anything.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Cancellation {
            cancelled: AtomicBool::new(false),
            worker: Mutex::new(None),
        }))
    }

    /// Ends the worker, now.
    ///
    /// Idempotent, callable from any thread, and it never blocks on the work being cancelled —
    /// only on the brief moment [`Confined`] spends reaping a worker that has already gone.
    /// Returns as soon as the signal is delivered; the host thread learns of it where it was
    /// blocked, as [`ConfinedError::Cancelled`].
    pub fn cancel(&self) {
        self.0.cancel();
    }

    /// Whether [`Self::cancel`] has been called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

impl Default for Canceller {
    fn default() -> Self {
        Self::new()
    }
}

/// The worker handle and the cancelled flag, shared by a [`Confined`] and its [`Canceller`]s.
///
/// The child lives here rather than in [`Confined`] for one reason: `Child::kill` needs `&mut`,
/// and the thread that would call it is not the thread that owns the [`Confined`] — it is the
/// one that is *not* blocked in a read. A mutex is what lets both reach it without this crate
/// reaching for a raw process identifier and a signal, which would cost the `unsafe` it forbids.
#[derive(Debug)]
struct Cancellation {
    /// Set once by [`Canceller::cancel`] and never cleared.
    cancelled: AtomicBool,
    /// The worker, from the moment it is spawned until it has been waited for.
    worker: Mutex<Option<Child>>,
}

impl Cancellation {
    /// Takes the lock, recovering it from a panic rather than propagating one.
    ///
    /// Nothing under this lock can panic — it holds a `Child` and calls `kill`, `wait` and
    /// `try_wait` on it — so a poisoned lock would mean a panic somewhere that cannot poison it.
    /// Recovering the guard is therefore the honest reading, and it is spelled out rather than
    /// hidden behind an `unwrap` this workspace forbids.
    fn worker(&self) -> std::sync::MutexGuard<'_, Option<Child>> {
        self.worker.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether a cancel has been asked for.
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Marks the viewer cancelled and ends its worker if there is one.
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.kill();
    }

    /// Signals the worker, if one is still held here.
    fn kill(&self) {
        if let Some(child) = self.worker().as_mut() {
            // A worker that has already gone is the outcome asked for, so a failure here is not
            // one: `kill` on a child that has exited is `ESRCH` and means the same thing.
            let _ = child.kill();
        }
    }

    /// Waits for the worker and says how it ended, leaving nothing behind to wait for twice.
    ///
    /// Called only where the worker's output has closed or it has been signalled, so the wait is
    /// the moment it takes a dead process to be reaped rather than the length of a render. That
    /// matters because the lock is held across it, and [`Self::kill`] wants the same lock.
    fn reap(&self) -> String {
        let mut worker = self.worker();
        match worker.take() {
            Some(mut child) => match child.wait() {
                Ok(status) => describe_exit(status),
                Err(error) => format!("and its status could not be read: {error}"),
            },
            None => "and it had already been waited for".to_owned(),
        }
    }

    /// Whether the worker has ended, without waiting for it to.
    fn ended(&self) -> Option<String> {
        let mut worker = self.worker();
        let child = worker.as_mut()?;
        match child.try_wait() {
            Ok(Some(status)) => Some(describe_exit(status)),
            Ok(None) => None,
            Err(error) => Some(format!("and its status could not be read: {error}")),
        }
    }
}

/// One page's pixels as they crossed the confinement, and where they belong on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Framed {
    /// Which page these pixels are of.
    pub page: usize,
    /// Row-major RGBA, no padding.
    pub raster: Raster,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin: (f32, f32),
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
    /// **The pixels**, drawn in the confined process — one entry per page on the screen.
    ///
    /// The payload the whole boundary exists for: the page was interpreted and rasterised behind
    /// the seccomp filter, and what crossed is a raster. A list since Table 29's `/PageLayout`
    /// was obeyed, because `OneColumn` puts several pages in one window.
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

/// A confined viewer: a worker process, the pipes to it, and what it reported about itself.
///
/// Dropping this closes the worker's input, which is how it learns to leave; the process is
/// waited for rather than left behind.
///
/// **Every call here blocks for as long as the document takes**, and that is deliberate — see
/// [`Canceller`] for why a page has no deadline and what a host holds instead.
#[derive(Debug)]
pub struct Confined {
    cancellation: Arc<Cancellation>,
    to_worker: ChildStdin,
    from_worker: ChildStdout,
    confinement: Confinement,
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
        let cancellation = Arc::clone(&canceller.0);
        if cancellation.is_cancelled() {
            return Err(ConfinedError::Cancelled);
        }

        let program = worker_program()?;
        let mut child = OsCommand::new(&program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Inherited, so that a worker that dies says so where the operator can see it. It is
            // the one descriptor it keeps that points outside itself, and it is write-only.
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(ConfinedError::Spawn)?;

        let (Some(to_worker), Some(from_worker)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ConfinedError::Spawn(std::io::Error::other(
                "the worker was started without pipes",
            )));
        };

        // Published before the blocking read below, so that a cancel arriving during the greeting
        // has something to signal — and re-checked after publishing, because one arriving between
        // the check above and this line would otherwise have found an empty slot and been lost.
        *cancellation.worker() = Some(child);
        if cancellation.is_cancelled() {
            cancellation.kill();
        }

        // Read before anything is constructed, so that the confinement this holds is the one the
        // worker reported rather than a value that stood in for it: until the greeting arrives,
        // nothing is known about the process on the other end — including whether it is the
        // worker at all.
        let mut greeting = [0u8; protocol::HANDSHAKE_LEN];
        let mut from_worker = from_worker;
        let confinement = match from_worker.read_exact(&mut greeting) {
            Ok(()) => protocol::parse_handshake(&greeting),
            Err(_) => None,
        };
        let Some(confinement) = confinement else {
            let detail = cancellation.reap();
            return Err(if cancellation.is_cancelled() {
                ConfinedError::Cancelled
            } else {
                ConfinedError::WorkerDied { detail }
            });
        };

        Ok(Self {
            cancellation,
            to_worker,
            from_worker,
            confinement,
        })
    }

    /// A handle another thread can end this viewer with.
    ///
    /// Cheap, and any number of them may exist. See [`Canceller`] for why the only cancel this
    /// offers is one the confined process cannot decline.
    #[must_use]
    pub fn canceller(&self) -> Canceller {
        Canceller(Arc::clone(&self.cancellation))
    }

    /// Whether this viewer has been cancelled, in which case every call returns
    /// [`ConfinedError::Cancelled`].
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// What confinement the worker reached.
    ///
    /// [`Confinement::shortfall`] is the sentence to print when it is not everything: a host that
    /// believed itself confined and was not is the failure this whole arrangement exists to
    /// prevent.
    #[must_use]
    pub fn confinement(&self) -> Confinement {
        self.confinement
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
        self.still_running()?;
        let payload = protocol::encode_command(command)?;
        self.write_frame(protocol::FRAME_COMMAND, &payload)?;
        let (kind, payload) = self.read_frame()?;
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
        self.still_running()?;
        let payload = protocol::encode_query(query)?;
        self.write_frame(protocol::FRAME_QUERY, &payload)?;
        let (kind, payload) = self.read_frame()?;
        match kind {
            protocol::FRAME_ANSWER => protocol::decode_answer(&payload).map_err(Into::into),
            protocol::FRAME_REFUSAL => Err(refusal(&payload)),
            _ => Err(ConfinedError::UnrecognisedFrame),
        }
    }

    /// Writes one frame and flushes it.
    ///
    /// **Header and payload in two calls, never concatenated.** A `Command::Open` payload is the
    /// whole document — 19.2 MB for `doc/ISO_32000-2_sponsored_EC3.pdf` — and putting nine bytes
    /// in front of it by building a third buffer cost a whole pass over it plus the page faults
    /// for a fresh allocation that size. The pipe itself moves those bytes in about 4 ms;
    /// everything around it cost ten times that, which is what ADR 0241 measured and what this
    /// takes a quarter of back.
    fn write_frame(&mut self, kind: u8, payload: &[u8]) -> Result<(), ConfinedError> {
        let header = protocol::header(kind, payload.len());
        self.to_worker
            .write_all(&header)
            .and_then(|()| self.to_worker.write_all(payload))
            .and_then(|()| self.to_worker.flush())
            .map_err(|error| self.explain(error))
    }

    /// Reads one whole frame.
    fn read_frame(&mut self) -> Result<(u8, Vec<u8>), ConfinedError> {
        let mut header = [0u8; protocol::FRAME_HEADER_LEN];
        self.read_exactly(&mut header)?;
        let (kind, length) =
            protocol::parse_frame_header(header).ok_or(ConfinedError::UnrecognisedFrame)?;
        let mut payload = vec![0u8; length];
        self.read_exactly(&mut payload)?;
        Ok((kind, payload))
    }

    /// Fills `buffer` from the worker.
    ///
    /// **There is no deadline here, and that is a decision rather than an omission.** A decode
    /// has a budget because one image's cost is bounded by its own dimensions;
    /// interpreting and rasterising a page is bounded by the document *and* the magnification,
    /// so a fixed number would refuse work a viewer permits. What a host has instead is a
    /// [`Canceller`], which ends the worker from another thread — and ending it is what makes
    /// the read below return, because the pipe's other end closes with the process.
    fn read_exactly(&mut self, buffer: &mut [u8]) -> Result<(), ConfinedError> {
        match self.from_worker.read_exact(buffer) {
            Ok(()) => Ok(()),
            // The worker closed its output, which it only does on the way out — so its status is
            // worth waiting for rather than reporting the read.
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Err(self.died()),
            Err(error) => Err(self.explain(error)),
        }
    }

    /// Refuses a call on a viewer whose worker the host has already ended.
    ///
    /// Checked before anything is written, so that a cancel arriving between two commands is the
    /// same answer as one arriving during a render rather than a pipe error about a dead process.
    fn still_running(&self) -> Result<(), ConfinedError> {
        if self.cancellation.is_cancelled() {
            return Err(ConfinedError::Cancelled);
        }
        Ok(())
    }

    /// Waits for a worker whose output has closed, and says how it ended.
    ///
    /// Blocking, for the reason `pdf_sandbox`'s own version gives: a status sampled the instant a
    /// pipe closes is usually not there yet, and a diagnosis that depends on scheduling is worse
    /// than none because it is believed.
    fn died(&mut self) -> ConfinedError {
        let detail = self.cancellation.reap();
        if self.cancellation.is_cancelled() {
            return ConfinedError::Cancelled;
        }
        ConfinedError::WorkerDied { detail }
    }

    /// Turns a pipe failure into the reason the worker is gone, when it is.
    fn explain(&mut self, error: std::io::Error) -> ConfinedError {
        if self.cancellation.is_cancelled() {
            return ConfinedError::Cancelled;
        }
        match self.cancellation.ended() {
            Some(detail) => ConfinedError::WorkerDied { detail },
            None => ConfinedError::Connection(error),
        }
    }
}

impl Drop for Confined {
    /// Ends the worker.
    ///
    /// Killed rather than asked: the worker leaves when its input closes, which dropping the
    /// pipe does — but a worker in the middle of a render would not notice for as long as that
    /// render takes, and a host that has dropped its handle is not waiting for one.
    ///
    /// This is [`Canceller::cancel`] without the flag, and it is the same one mechanism: dropping
    /// a `Confined` is a host giving up on the work, which is what a cancel is. The [`Canceller`]s
    /// that outlive it find an empty slot and do nothing.
    fn drop(&mut self) {
        self.cancellation.kill();
        let _ = self.cancellation.reap();
    }
}

/// Reads a refusal frame into an error.
fn refusal(payload: &[u8]) -> ConfinedError {
    ConfinedError::Refused {
        detail: String::from_utf8_lossy(payload).into_owned(),
    }
}

/// Describes how a worker ended, naming the signal where the platform has one.
///
/// `SIGSYS` is the interesting one and it is the seccomp filter firing: the confined viewer
/// attempted something no page needs. A platform with no filter cannot produce that diagnosis and
/// does not pretend to.
fn describe_exit(status: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    if let Some(signal) = {
        use std::os::unix::process::ExitStatusExt as _;
        status.signal()
    } {
        // 31 is `SIGSYS` on every Linux architecture this builds for, and naming it costs no
        // dependency; a platform whose numbering differs simply reports the number.
        let name = if cfg!(target_os = "linux") && signal == 31 {
            " (SIGSYS: a system call the confinement forbids)"
        } else {
            ""
        };
        return format!("killed by signal {signal}{name}");
    }
    match status.code() {
        Some(code) => format!("exited with status {code}"),
        None => "stopped for an unknown reason".to_owned(),
    }
}

/// Finds the worker program.
///
/// Searched next to the running executable, then one directory up, because Cargo puts test
/// binaries in `target/<profile>/deps/` while it puts programs in `target/<profile>/`. The
/// environment variable overrides both.
fn worker_program() -> Result<PathBuf, ConfinedError> {
    if let Some(named) = std::env::var_os(WORKER_PATH_VARIABLE) {
        return Ok(PathBuf::from(named));
    }

    let executable = std::env::current_exe().map_err(ConfinedError::Spawn)?;
    let directory = executable.parent().unwrap_or(&executable);
    let name = format!("{WORKER_PROGRAM}{}", std::env::consts::EXE_SUFFIX);
    let beside = directory.join(&name);
    if beside.is_file() {
        return Ok(beside);
    }
    if let Some(parent) = directory.parent() {
        let above = parent.join(&name);
        if above.is_file() {
            return Ok(above);
        }
    }
    Err(ConfinedError::WorkerMissing { executable })
}
