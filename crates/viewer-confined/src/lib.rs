//! The viewer's document, interpretation and rasterisation in a confined process.
//!
//! `pdf-sandbox` confines three image decoders. This confines everything above them: the
//! document, the content interpreter and the rasteriser — which is where a PDF's bytes actually
//! go, and by far the larger attack surface. `CLAUDE.md` principle 3 asks for it, and
//! `doc/HANDOVER.md`'s section 0 says why it costs one protocol rather than two: if the boundary
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
//! else crosses**, including all twenty-five questions — the eleven a panel is made of since the
//! three-hundred-and-eighty-sixth session (ADR 0223). What is left of [`Uncarried`] is those two
//! and three contents an *answer* can hold that this build cannot name; each is refused **by
//! name**, which is the difference between a boundary that is incomplete and one that is quietly
//! wrong.
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
        value: Option<String>,
    },
    /// Where the caret is, in device pixels of the viewport.
    Caret {
        /// The end on the descent side of the baseline.
        from: (f32, f32),
        /// The end on the ascent side.
        to: (f32, f32),
    },
    /// Where a string occurs, one entry per occurrence.
    Found(Vec<Vec<[f32; 8]>>),
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
    /// **The pixels**, drawn in the confined process.
    ///
    /// The payload the whole boundary exists for: the page was interpreted and rasterised behind
    /// the seccomp filter, and what crossed is a raster.
    Frame {
        /// Which page these pixels are of.
        page: usize,
        /// Row-major RGBA, no padding.
        raster: Raster,
        /// Where the raster's top-left corner sits in the viewport, in device pixels.
        origin: (f32, f32),
    },
    /// What the current page could not draw.
    Reports(Vec<String>),
    /// §12.3.3's outline, whole.
    Outline(pdf_model::outline::Outline),
    /// §8.11.4.3's layers, in `/Order`.
    Layers(Vec<viewer_core::Layer>),
    /// §7.11.4's embedded files, listed.
    ///
    /// [`Attachment`] rather than [`pdf_model::attachment::Attachment`], and the difference is the
    /// stream: see that type.
    Attachments(Vec<Attachment>),
    /// §12.3.5's portable collection, read.
    ///
    /// Boxed because it is Table 153, Table 155's columns, Table 156's sort, Table 158's split,
    /// Table 159's folder tree and Table 160's navigator together, and an enumeration is as large
    /// as its largest variant — a `Reply::Count` would otherwise cost what a collection costs.
    Collection(Box<pdf_model::collection::Collection>),
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
    /// §14.7's structure for the page being shown, in §14.8.2.5's logical order, parent-first.
    Accessibility(Vec<viewer_core::AccessibilityNode>),
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
#[derive(Debug)]
pub struct Confined {
    child: Child,
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
    /// # Errors
    ///
    /// See [`ConfinedError`]. A worker that cannot confine itself never sends a greeting, so it
    /// arrives here as [`ConfinedError::WorkerDied`] — never as a viewer that quietly runs
    /// unconfined.
    pub fn start() -> Result<Self, ConfinedError> {
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
            return Err(ConfinedError::Spawn(std::io::Error::other(
                "the worker was started without pipes",
            )));
        };

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
            let detail = match child.wait() {
                Ok(status) => describe_exit(status),
                Err(error) => format!("and its status could not be read: {error}"),
            };
            return Err(ConfinedError::WorkerDied { detail });
        };

        Ok(Self {
            child,
            to_worker,
            from_worker,
            confinement,
        })
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
    fn write_frame(&mut self, kind: u8, payload: &[u8]) -> Result<(), ConfinedError> {
        let framed = protocol::frame(kind, payload);
        self.to_worker
            .write_all(&framed)
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
    /// so a fixed number would refuse work a viewer permits. What bounds a hostile document is
    /// therefore the address-space ceiling and the host's ability to kill the process, and what
    /// is missing is a cancel the host can send — which needs a host with a second thread.
    /// `doc/todo/34` records it.
    fn read_exactly(&mut self, buffer: &mut [u8]) -> Result<(), ConfinedError> {
        match self.from_worker.read_exact(buffer) {
            Ok(()) => Ok(()),
            // The worker closed its output, which it only does on the way out — so its status is
            // worth waiting for rather than reporting the read.
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Err(self.died()),
            Err(error) => Err(self.explain(error)),
        }
    }

    /// Waits for a worker whose output has closed, and says how it ended.
    ///
    /// Blocking, for the reason `pdf_sandbox`'s own version gives: a status sampled the instant a
    /// pipe closes is usually not there yet, and a diagnosis that depends on scheduling is worse
    /// than none because it is believed.
    fn died(&mut self) -> ConfinedError {
        let detail = match self.child.wait() {
            Ok(status) => describe_exit(status),
            Err(error) => format!("and its status could not be read: {error}"),
        };
        ConfinedError::WorkerDied { detail }
    }

    /// Turns a pipe failure into the reason the worker is gone, when it is.
    fn explain(&mut self, error: std::io::Error) -> ConfinedError {
        match self.child.try_wait() {
            Ok(Some(status)) => ConfinedError::WorkerDied {
                detail: describe_exit(status),
            },
            Ok(None) => ConfinedError::Connection(error),
            Err(wait_error) => ConfinedError::WorkerDied {
                detail: format!("and its status could not be read: {wait_error}"),
            },
        }
    }
}

impl Drop for Confined {
    /// Ends the worker.
    ///
    /// Killed rather than asked: the worker leaves when its input closes, which dropping the
    /// pipe does — but a worker in the middle of a render would not notice for as long as that
    /// render takes, and a host that has dropped its handle is not waiting for one.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
