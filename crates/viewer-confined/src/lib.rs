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
//! [`viewer_core::Event::NeedsRender`] and [`viewer_core::Command::RenderReady`]. Everything else
//! a host can say crosses, and every question it can ask either crosses or is refused **by name**
//! — see [`Uncarried`].
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
/// the pipe — so what arrives is the same answer with its parts owned. One variant per carried
/// question; the eleven questions this transport does not carry have no variant here, which is
/// how "the host cannot ask for what it cannot be told" is said in the type system rather than in
/// a comment.
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
    /// See [`ConfinedError`]. [`ConfinedError::Uncarried`] for the eleven questions whose answers
    /// are document-model types this transport does not encode yet.
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
