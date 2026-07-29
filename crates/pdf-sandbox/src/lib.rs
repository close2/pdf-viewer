//! Process isolation for untrusted image decoding.
//!
//! Confines the code that decodes JBIG2 and JPEG 2000 to an unprivileged process with no
//! filesystem and no network, using seccomp-BPF to restrict system calls and Landlock to
//! restrict filesystem reachability. The privileged process holds the document; the worker
//! receives one image's bytes at a time over a pipe and returns samples over another. It
//! never learns which document they came from, and it cannot ask.
//!
//! # Why this is not redundant with Rust
//!
//! It would be, if memory corruption were the only thing that goes wrong. The decoders this
//! isolates are written here, in Rust, with `#![forbid(unsafe_code)]`, so the class of bug
//! that made JBIG2 famous — FORCEDENTRY was an integer overflow into a heap write — cannot
//! occur in them. Three things remain, and each is a reason the boundary exists:
//!
//! - **Resource exhaustion.** Rust does not bound memory or time. A JBIG2 symbol dictionary
//!   can name a hundred thousand symbols in a few hundred bytes, and a JPEG 2000 codestream
//!   can declare a tile grid whose product overflows any sane working set. Refusing those by
//!   hand at every allocation site is a discipline; an address-space limit on a separate
//!   process is a fact.
//! - **Panics.** This workspace builds release binaries with `panic = "abort"`. A slice
//!   index this crate's authors got wrong on one malformed file would take the viewer down
//!   with the document open. In a worker it takes the worker down, and the image reports as
//!   undecodable while the rest of the page draws.
//! - **What comes later.** If a codec this project cannot reasonably write in Rust ever has
//!   to be linked — JPEG 2000's more exotic extensions, a future format — the boundary that
//!   contains it has to already exist, with the protocol already narrow. A sandbox retrofitted
//!   around a C library is a sandbox designed by that library's needs.
//!
//! # The shape of it
//!
//! [`Sandbox::shared`] hands out a lazily-started worker: nothing is spawned until the first
//! image that needs one, because startup latency is a first-class requirement and most
//! documents contain neither codec. The worker is a separate program, [`WORKER_PROGRAM`],
//! found next to the running executable. It applies [`lockdown::apply`] before reading a
//! single byte of anything, and answers requests until its input closes.
//!
//! A worker that dies — killed by its own seccomp filter, by the address-space limit, or by
//! a panic — is *reported*, not retried in-process. Falling back to decoding in the parent
//! "just this once" would defeat the whole arrangement, so there is no code path that does
//! it. The next request starts a fresh worker.
//!
//! # The isolation is a choice, and its default is the safe one
//!
//! [`set_isolation`] turns the worker off, and [`Isolation::Sandboxed`] is what you get if
//! nobody calls it. A reader whose documents are their own — a scanner's output, a build
//! system's invoices — is paying a process spawn and a pipe round trip for a threat they do
//! not have, and it is not this crate's place to insist. What is this crate's place is to
//! make the choice explicit, default it to confinement, and write down what turning it off
//! costs:
//!
//! - a decoder panic aborts the viewer with the document open, rather than costing one image;
//! - the address-space limit is gone, so a decompression bomb is bounded only by the
//!   machine;
//! - the no-filesystem, no-network property is gone with it.
//!
//! What it does not cost is memory safety: both decoders are `#![forbid(unsafe_code)]` with
//! no unsafe dependency, in process or out of it. That is why this can be a flag at all.

#![forbid(unsafe_code)]
// `landlock`, `seccompiler` and `rustix` each wrap the raw system calls in safe interfaces,
// and `libc` is used here only for its system-call *numbers*, which are constants. So the
// crate that exists to touch the most dangerous interfaces in the tree needs no `unsafe` of
// its own, and says so in the same way every other crate here does.
#![cfg_attr(
    not(target_os = "linux"),
    expect(
        unused_imports,
        reason = "the platform check below is the real diagnostic"
    )
)]

#[cfg(not(target_os = "linux"))]
compile_error!(
    "pdf-sandbox confines processes with seccomp-BPF and Landlock, which are Linux \
     interfaces. There is deliberately no fallback: a sandbox that silently does nothing \
     on another platform is worse than no sandbox, because the code above it would keep \
     handing untrusted input to a decoder while believing it was contained."
);

mod decode;
pub mod lockdown;
mod protocol;
mod worker;

pub use protocol::{Bilevel, CcittParameters, Colour, Raster, Request};
pub use worker::serve;

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use lockdown::Confinement;

/// Where image decoding happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Isolation {
    /// In a confined worker process. The default.
    #[default]
    Sandboxed,
    /// In this process, with no confinement and no bound but the machine's.
    ///
    /// Faster by a process spawn and a pipe round trip, and appropriate for documents whose
    /// origin the reader trusts. See the crate documentation for what it gives up.
    InProcess,
}

/// The process-wide isolation policy.
///
/// A `static` because this is a property of the *process*, decided once by whoever started
/// it, in the same category as a umask — not a property of a document, a page or an image.
/// Threading it through `Document`, `Page` and the content interpreter to reach the one
/// function that reads it would put a security posture in four API signatures that have no
/// opinion about security, which is a worse arrangement than this one, not a better one.
static ISOLATION: AtomicU8 = AtomicU8::new(SANDBOXED);

/// [`Isolation::Sandboxed`] as stored in [`ISOLATION`].
const SANDBOXED: u8 = 0;
/// [`Isolation::InProcess`] as stored in [`ISOLATION`].
const IN_PROCESS: u8 = 1;

/// Sets where image decoding happens, for this process.
///
/// Call it from `main`, before a document is opened. Calling it later is not an error and
/// not a race — each decode reads the current value — but it means two images in one
/// document were decoded under different rules, which is a thing nobody wants to debug.
pub fn set_isolation(isolation: Isolation) {
    let value = match isolation {
        Isolation::Sandboxed => SANDBOXED,
        Isolation::InProcess => IN_PROCESS,
    };
    ISOLATION.store(value, Ordering::Relaxed);
}

/// Returns where image decoding happens.
#[must_use]
pub fn isolation() -> Isolation {
    match ISOLATION.load(Ordering::Relaxed) {
        IN_PROCESS => Isolation::InProcess,
        // Anything else is the default, and the default is the confined one. A policy byte
        // that has somehow become a value this build does not know must not read as
        // "unconfined".
        _ => Isolation::Sandboxed,
    }
}

/// Decodes an image under the process's isolation policy.
///
/// This is what callers should use. [`Sandbox::shared`] is the confined path named
/// explicitly, which is what the tests of the confinement itself want.
///
/// # Errors
///
/// See [`SandboxError`]. Every variant is reportable as "this image could not be drawn";
/// none is a reason to stop rendering the page.
pub fn decode(request: &Request<'_>) -> Result<Decoded, SandboxError> {
    match isolation() {
        Isolation::Sandboxed => Sandbox::shared().decode(request),
        Isolation::InProcess => decode::here(request),
    }
}

/// The name of the worker program.
///
/// It is a separate executable rather than this one re-invoked with a flag. Both work; this
/// way round means the decoding path cannot be reached in-process by accident, because the
/// only code that calls it lives in a program whose whole `main` is to confine itself
/// first.
pub const WORKER_PROGRAM: &str = "pdf-sandbox-worker";

/// Environment variable naming the worker program explicitly.
///
/// Set it when the executable is not installed alongside its worker — in particular, when
/// running a test binary that Cargo has put in `target/<profile>/deps/`.
pub const WORKER_PATH_VARIABLE: &str = "PDF_SANDBOX_WORKER";

/// How long a single decode may take before the worker is killed.
///
/// Wall clock, not processor time, and enforced by the parent: the worker cannot be trusted
/// to time itself, and a cumulative processor-time limit would eventually kill a healthy
/// long-lived worker for the crime of having decoded a lot of images. Thirty seconds is the
/// same budget the corpus gate gives an entire document to a reference renderer, so an
/// image that reaches it is not slow, it is not returning.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Largest response this will accept from a worker, in bytes.
///
/// The worker is the untrusted side of this boundary. Its answers are bounded and checked
/// here for the same reason the document's own numbers are: a length is a claim, and a
/// claim from a process that may have been subverted is worth less than one from a file.
const MAX_RESPONSE: usize = 1 << 28;

/// Why an image could not be decoded in the sandbox.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SandboxError {
    /// The worker program could not be found.
    #[error(
        "the sandbox worker `{WORKER_PROGRAM}` was not found next to {executable} — build it \
         with `cargo build -p pdf-sandbox --bins`, or name it with ${WORKER_PATH_VARIABLE}"
    )]
    WorkerMissing {
        /// The executable whose directory was searched.
        executable: PathBuf,
    },
    /// The worker program could not be started.
    #[error("starting the sandbox worker failed: {0}")]
    Spawn(#[source] std::io::Error),
    /// The worker stopped before answering.
    ///
    /// This is the expected outcome for an image that trips a bound: the address-space limit
    /// aborts it, and the seccomp filter kills it outright. Either way the page reports an
    /// undecodable image and the viewer keeps running.
    #[error("the sandbox worker stopped without answering ({detail})")]
    WorkerDied {
        /// How it stopped, as far as the parent can tell.
        detail: String,
    },
    /// The worker did not answer within [`REQUEST_TIMEOUT`].
    #[error("the sandbox worker did not answer within {}s", REQUEST_TIMEOUT.as_secs())]
    TimedOut,
    /// The pipe to or from the worker failed.
    #[error("the sandbox worker connection failed: {0}")]
    Connection(#[source] std::io::Error),
    /// The worker answered with something that is not a well-formed response.
    #[error("the sandbox worker sent a malformed response: {detail}")]
    Malformed {
        /// What was wrong with it.
        detail: String,
    },
    /// The worker decoded nothing and said why.
    ///
    /// This is the ordinary failure: a malformed or unsupported image. It carries the
    /// decoder's own words so that the page can name what it could not draw.
    #[error("{detail}")]
    Undecodable {
        /// The decoder's description of the problem.
        detail: String,
    },
}

/// A worker process and the pipes to it.
#[derive(Debug)]
struct Connection {
    child: Child,
    to_worker: ChildStdin,
    from_worker: ChildStdout,
    confinement: Confinement,
}

/// A handle to the sandboxed decoder.
#[derive(Debug)]
pub struct Sandbox {
    /// `None` until the first request, and again after a worker dies.
    ///
    /// One worker serves every caller, so requests serialise. That is the right first
    /// answer: a request is a pipe write and a decode, images are a small minority of what a
    /// page costs, and a pool would have to be sized by measurement nobody has taken. When
    /// a profile says otherwise, this is the field that grows.
    connection: Mutex<Option<Connection>>,
}

impl Sandbox {
    /// Returns the process-wide sandbox, without starting anything.
    ///
    /// The worker starts on the first request, not here. Documents needing neither codec are
    /// the common case and must not pay for one.
    #[must_use]
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<Sandbox> = OnceLock::new();
        SHARED.get_or_init(|| Self {
            connection: Mutex::new(None),
        })
    }

    /// Decodes one image in the worker.
    ///
    /// # Errors
    ///
    /// See [`SandboxError`]. A failure here is always reportable as "this image could not be
    /// drawn"; none of them is a reason to stop rendering the page.
    pub fn decode(&self, request: &Request<'_>) -> Result<Decoded, SandboxError> {
        let mut guard = match self.connection.lock() {
            Ok(guard) => guard,
            // A panic while another thread held the lock cannot have corrupted anything the
            // parent owns — the worker's state lives in another process, and a poisoned
            // connection is replaced below anyway.
            Err(poisoned) => poisoned.into_inner(),
        };

        if guard.is_none() {
            *guard = Some(Connection::start()?);
        }
        let Some(connection) = guard.as_mut() else {
            unreachable!("the connection was just established")
        };

        let outcome = connection.exchange(request);
        if outcome.is_err() {
            // Any failure leaves the pipe at an unknown offset, so the worker is not reusable
            // even when it is still alive. Killing it is how the next request gets a clean
            // one; a worker already dead ignores this.
            let _ = connection.child.kill();
            let _ = connection.child.wait();
            *guard = None;
        }
        outcome
    }

    /// Returns what confinement the running worker reported, starting one if needed.
    ///
    /// # Errors
    ///
    /// See [`SandboxError`].
    pub fn confinement(&self) -> Result<Confinement, SandboxError> {
        let mut guard = match self.connection.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_none() {
            *guard = Some(Connection::start()?);
        }
        guard
            .as_ref()
            .map(|connection| connection.confinement)
            .ok_or_else(|| SandboxError::Malformed {
                detail: "no connection".to_owned(),
            })
    }
}

/// What a worker returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    /// One bit per pixel: JBIG2.
    Bilevel(Bilevel),
    /// Eight bits per component: JPEG 2000.
    Raster(Raster),
}

impl Connection {
    /// Starts a worker and reads its handshake.
    fn start() -> Result<Self, SandboxError> {
        let program = worker_program()?;
        let mut child = Command::new(&program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Inherited, so that a worker that dies says so where the operator can see it.
            // It is the one descriptor it keeps that points outside itself, and it is
            // write-only.
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(SandboxError::Spawn)?;

        let (Some(to_worker), Some(from_worker)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            return Err(SandboxError::Spawn(std::io::Error::other(
                "the worker was started without pipes",
            )));
        };

        let mut connection = Self {
            child,
            to_worker,
            from_worker,
            confinement: Confinement {
                landlock: lockdown::LandlockLevel::Unavailable,
                address_space_limit: 0,
            },
        };
        connection.confinement = connection.read_handshake()?;
        Ok(connection)
    }

    /// Reads the greeting the worker sends once it is confined.
    ///
    /// Until this arrives, nothing is known about the process on the other end — including
    /// whether it is the worker at all. A program that ignored its arguments and started an
    /// interactive viewer would fail here rather than being fed image data.
    fn read_handshake(&mut self) -> Result<Confinement, SandboxError> {
        let mut greeting = [0u8; protocol::HANDSHAKE_LEN];
        self.read_exactly(&mut greeting, deadline())?;
        protocol::parse_handshake(&greeting).ok_or_else(|| SandboxError::Malformed {
            detail: "the greeting was not this protocol's".to_owned(),
        })
    }

    /// Sends one request and reads its response.
    fn exchange(&mut self, request: &Request<'_>) -> Result<Decoded, SandboxError> {
        let deadline = deadline();

        let encoded = protocol::encode_request(request);
        self.to_worker
            .write_all(&encoded)
            .and_then(|()| self.to_worker.flush())
            .map_err(|error| self.explain(error))?;

        let mut header = [0u8; protocol::RESPONSE_HEADER_LEN];
        self.read_exactly(&mut header, deadline)?;
        let shape =
            protocol::parse_response_header(header).ok_or_else(|| SandboxError::Malformed {
                detail: "unrecognised response header".to_owned(),
            })?;
        if shape.payload_len > MAX_RESPONSE {
            return Err(SandboxError::Malformed {
                detail: format!(
                    "a {}-byte response exceeds the {MAX_RESPONSE}-byte limit",
                    shape.payload_len
                ),
            });
        }

        let mut payload = vec![0u8; shape.payload_len];
        self.read_exactly(&mut payload, deadline)?;
        protocol::parse_response(shape, &payload)
    }

    /// Fills `buffer`, giving up at `deadline`.
    ///
    /// A blocking read cannot be interrupted, so each read is preceded by a poll bounded by
    /// what is left of the budget. That is what makes [`SandboxError::TimedOut`] possible at
    /// all: without it, a worker that stopped answering would hang whatever thread asked.
    fn read_exactly(&mut self, buffer: &mut [u8], deadline: Instant) -> Result<(), SandboxError> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};

        let mut filled = 0;
        while filled < buffer.len() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SandboxError::TimedOut);
            }
            let timeout = Timespec {
                tv_sec: remaining.as_secs().try_into().unwrap_or(i64::MAX),
                tv_nsec: remaining.subsec_nanos().into(),
            };
            let mut fds = [PollFd::new(&self.from_worker, PollFlags::IN)];
            match poll(&mut fds, Some(&timeout)) {
                Ok(0) => return Err(SandboxError::TimedOut),
                Ok(_) => {}
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(SandboxError::Connection(error.into())),
            }

            let Some(rest) = buffer.get_mut(filled..) else {
                return Err(SandboxError::Malformed {
                    detail: "read past the end of the buffer".to_owned(),
                });
            };
            match self.from_worker.read(rest) {
                // End of file: the worker closed its output, which it only does on the way
                // out. Its status is therefore worth waiting for rather than sampling.
                Ok(0) => return Err(self.died()),
                Ok(count) => filled = filled.saturating_add(count),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(self.explain(error)),
            }
        }
        Ok(())
    }

    /// Waits for a worker whose output has closed, and says how it ended.
    ///
    /// Blocking, and deliberately: a status sampled with `try_wait` the instant the pipe
    /// closes is usually not there yet, so the same failed allocation reported itself as
    /// `SIGABRT` on one run and as an anonymous closed pipe on the next. A diagnosis that
    /// depends on scheduling is worse than no diagnosis, because it is believed. The wait is
    /// bounded by the fact that a process which has closed its standard output is on its way
    /// out already.
    fn died(&mut self) -> SandboxError {
        let detail = match self.child.wait() {
            Ok(status) => describe_exit(status),
            Err(error) => format!("and its status could not be read: {error}"),
        };
        SandboxError::WorkerDied { detail }
    }

    /// Turns a pipe failure into the reason the worker is gone, when it is.
    ///
    /// A closed pipe on its own says nothing useful. The exit status says whether the worker
    /// aborted on a failed allocation, was killed by its own seccomp filter, or simply
    /// exited — three very different events that all arrive here as `EPIPE`.
    fn explain(&mut self, error: std::io::Error) -> SandboxError {
        match self.child.try_wait() {
            Ok(Some(status)) => SandboxError::WorkerDied {
                detail: describe_exit(status),
            },
            // Still running, so the pipe failure is what it says it is.
            Ok(None) => SandboxError::Connection(error),
            Err(wait_error) => SandboxError::WorkerDied {
                detail: format!("and its status could not be read: {wait_error}"),
            },
        }
    }
}

/// When the current request runs out of time.
///
/// `checked_add` rather than `+` because a deadline is arithmetic on untrusted-adjacent
/// input everywhere else in this tree and the habit is worth keeping. An `Instant` that
/// cannot represent thirty seconds from now is not reachable on any real clock; if it were,
/// the safe reading is "already out of time", not "never".
fn deadline() -> Instant {
    let now = Instant::now();
    now.checked_add(REQUEST_TIMEOUT).unwrap_or(now)
}

/// Describes how a worker ended, naming the signal when one killed it.
fn describe_exit(status: std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt as _;

    if let Some(signal) = status.signal() {
        // `SIGSYS` is the seccomp filter firing, and it is worth naming: it means the worker
        // attempted something no decode needs, which is a finding rather than a mishap.
        let name = match signal {
            libc::SIGSYS => " (SIGSYS: a system call the sandbox forbids)",
            libc::SIGABRT => " (SIGABRT: it aborted, typically a failed allocation)",
            libc::SIGKILL => " (SIGKILL)",
            libc::SIGSEGV => " (SIGSEGV)",
            _ => "",
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
/// environment variable overrides both and is what a deployment with a different layout
/// should set.
fn worker_program() -> Result<PathBuf, SandboxError> {
    if let Some(named) = std::env::var_os(WORKER_PATH_VARIABLE) {
        return Ok(PathBuf::from(named));
    }

    let executable = std::env::current_exe().map_err(SandboxError::Spawn)?;
    let directory = executable.parent().unwrap_or(&executable);
    let beside = directory.join(WORKER_PROGRAM);
    if beside.is_file() {
        return Ok(beside);
    }
    if let Some(parent) = directory.parent() {
        let above = parent.join(WORKER_PROGRAM);
        if above.is_file() {
            return Ok(above);
        }
    }
    Err(SandboxError::WorkerMissing { executable })
}
