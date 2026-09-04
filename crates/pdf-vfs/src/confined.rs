//! The broker's side of RFC 0003 section 6: a [`crate::worker::Worker`] that is another process.
//!
//! # What this is, and what it is not
//!
//! [`crate::InProcessWorkers`] answers in the calling process. **A face may not ship on it**, and
//! `doc/todo/58` §4 says why in one sentence: a mount is entered by anything that touches a
//! folder, so a file manager will open a document nobody chose to open, with the user's full
//! privileges. This module is the other implementation — and it is a *transport* change and
//! nothing else, which is the property `crate::worker`'s seam was shaped to make true:
//!
//! - [`crate::worker::Query`] and [`crate::worker::Answer`] are plain data with no borrow, no
//!   path and no descriptor in them, so a question is a message.
//! - A worker is created once per generation, which is exactly the moment a broker opens the file
//!   and passes the descriptor across with `SCM_RIGHTS` (ADR 0812).
//! - [`crate::worker::WorkerError`] is the same population from both, so a face cannot behave
//!   differently depending on which one answered it.
//!
//! # The document crosses as an open file, not as bytes
//!
//! `pdf_syntax::FileBytes::on_disk` is what `crate::FileBacking` produces, and its descriptor
//! rides beside the open frame's header. The worker reads it with `pread64` where the file's
//! offsets point and can ask the filesystem nothing about it — not even its length, which crosses
//! in the frame. A document held in memory (a test's `crate::MemoryBacking`, a face that has
//! already read the bytes) crosses whole instead, which is the same choice `viewer-confined` makes
//! and for the same reason: there is no descriptor to send.
//!
//! # A dead worker is an error, never a hang
//!
//! The worker is killable by design — by its own seccomp filter, by the address-space ceiling, by
//! a panic under `panic = "abort"`. Every one of those closes its output, which is what makes the
//! broker's blocking read return, and it arrives as [`crate::worker::WorkerError::Transport`] with
//! the worker's own last words in it. [`Confined::is_alive`] then answers `false` for ever, and
//! `crate::Vfs` throws the generation away rather than asking a corpse — so the *next* operation
//! on the mount starts a fresh worker over the same file.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use confined_transport::{Canceller, Host, TransportError};
use pdf_syntax::FileBytes;
use pdf_transform::{Budget, Policy, Secret};

use crate::serve::{WORKER_PATH_VARIABLE, WORKER_PROGRAM};
use crate::wire::{self, Document};
use crate::worker::{Answer, Query, Worker, WorkerError, Workers};

/// Workers that answer in a confined process of their own.
///
/// One per generation of one document, started by [`Workers::spawn`] and ended when the
/// [`Confined`] it produced is dropped.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConfinedWorkers;

impl ConfinedWorkers {
    /// Starts one, and hands back the worker itself rather than the trait object.
    ///
    /// [`Workers::spawn`] is this boxed. The concrete form is what a face wants when it needs
    /// [`Confined::confinement`] — the sentence to print where the kernel granted less than the
    /// build asked for — or [`Confined::canceller`].
    ///
    /// # Errors
    ///
    /// [`WorkerError::Refused`] where the program is not beside this one,
    /// [`WorkerError::Transport`] where it could not be started or never greeted, and whatever
    /// the document itself is refused for.
    pub fn start(
        bytes: &FileBytes,
        password: Option<&Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Confined, WorkerError> {
        let program = confined_transport::program_beside_executable(
            WORKER_PROGRAM,
            WORKER_PATH_VARIABLE,
        )
        .map_err(|missing| {
            WorkerError::Refused(format!(
                "the confined generator `{WORKER_PROGRAM}` {missing} — build it with `cargo build \
                 -p pdf-vfs --bins`, or name it with ${WORKER_PATH_VARIABLE}"
            ))
        })?;
        let mut host = Host::start(&program, wire::MAGIC, &Canceller::new())?;

        // The document, once, at spawn. `descriptor()` answers `Some` only for a file this
        // process opened on disk, which is what makes the two arms below exhaustive rather than a
        // preference: there is nothing else a `FileBytes` can be.
        let descriptor = bytes.descriptor();
        // In memory this borrows; on disk it would read the whole file, which is exactly why the
        // descriptor arm exists and is taken first. `Cow::Borrowed` either way for the memory
        // case, so nothing is copied on the way into the frame.
        let whole = if descriptor.is_some() {
            std::borrow::Cow::Borrowed(&[][..])
        } else {
            bytes.read(0..bytes.len())
        };
        let payload = wire::encode_open(
            policy,
            &budget,
            password.map(Secret::reveal),
            match descriptor {
                Some(_) => Document::OnDisk {
                    length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                },
                None => Document::Bytes(&whole),
            },
        );
        let (kind, answer) = host.exchange(wire::FRAME_OPEN, &payload, descriptor)?;
        drop(payload);
        drop(whole);
        match kind {
            wire::FRAME_READY => Ok(Confined {
                host: Mutex::new(host),
                alive: AtomicBool::new(true),
            }),
            wire::FRAME_REFUSAL => Err(refusal(&answer)),
            _ => Err(WorkerError::Mismatched {
                got: "an unrecognised frame",
                wanted: "a worker holding the document",
            }),
        }
    }
}

impl Workers for ConfinedWorkers {
    fn spawn(
        &self,
        bytes: FileBytes,
        password: Option<Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Box<dyn Worker>, WorkerError> {
        Ok(Box::new(Self::start(
            &bytes,
            password.as_ref(),
            policy,
            budget,
        )?))
    }
}

/// One document, answered by a confined process.
#[derive(Debug)]
pub struct Confined {
    /// The wire. Behind a mutex because [`Worker::ask`] takes `&self` — a `Vfs` is shared between
    /// a face's threads — and one worker answers one question at a time.
    host: Mutex<Host>,
    /// Whether it can still be asked anything. Set once, never cleared.
    alive: AtomicBool,
}

impl Confined {
    /// What confinement the worker reached.
    ///
    /// A **report** rather than a promise: a kernel can refuse what a build offers, so a face that
    /// has to say what it got asks the worker rather than the build.
    /// `pdf_sandbox::lockdown::Confinement::shortfall` is the sentence to print when it is not
    /// everything.
    #[must_use]
    pub fn confinement(&self) -> pdf_sandbox::lockdown::Confinement {
        self.host
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .confinement()
    }

    /// A handle another thread can end this worker with.
    ///
    /// See [`Canceller`] for why the only cancel on this boundary is one the confined process
    /// cannot decline. A face holds one so that a person who navigated away from a folder is not
    /// waiting on a render nobody wants any more; ending it makes the next operation on the mount
    /// start a fresh worker.
    #[must_use]
    pub fn canceller(&self) -> Canceller {
        self.host
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .canceller()
    }
}

impl Worker for Confined {
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.exchange(&wire::encode_query(query))
    }

    /// The question with a person's *yes* behind it, as the one extra byte `Query::Consented`
    /// is on the wire.
    ///
    /// Encoded rather than *built*: wrapping would mean owning the query, and an insertion
    /// carries a whole document. `wire::encode_consented` writes the tag and then the same bytes
    /// `encode_query` writes, so a copy of that document is not the price of a consent.
    fn ask_consented(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.exchange(&wire::encode_consented(query))
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }
}

impl Confined {
    /// One encoded question across the wire, and the answer or the refusal that came back.
    fn exchange(&self, payload: &[u8]) -> Result<Answer, WorkerError> {
        if !self.alive.load(Ordering::SeqCst) {
            return Err(WorkerError::Transport(TransportError::WorkerDied {
                detail: String::from("it had already stopped when this question was asked"),
            }));
        }
        let exchanged = {
            let mut host = self
                .host
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            host.exchange(wire::FRAME_QUERY, payload, None)
        };
        let (kind, answer) = match exchanged {
            Ok(answered) => answered,
            Err(error) => {
                // Every transport failure ends this worker for good. A worker that answered a
                // pipe error and was asked again would produce a second, stranger error about a
                // closed descriptor; a face that showed the first and tried again gets a *new*
                // worker instead, which is `Vfs`'s doing and this flag's.
                self.alive.store(false, Ordering::SeqCst);
                return Err(error.into());
            }
        };
        match kind {
            wire::FRAME_ANSWER => wire::decode_answer(&answer)
                .map_err(|error| WorkerError::Refused(error.to_string())),
            wire::FRAME_REFUSAL => Err(refusal(&answer)),
            _ => Err(WorkerError::Mismatched {
                got: "an unrecognised frame",
                wanted: "an answer or a refusal",
            }),
        }
    }
}

/// Reads a refusal frame into the error it is.
///
/// A refusal this build cannot decode is still a refusal — the worker said *something* and the
/// frame said it was a refusal — so it comes back as one rather than as a decoding fault nobody
/// can act on.
fn refusal(payload: &[u8]) -> WorkerError {
    wire::decode_refusal(payload).unwrap_or_else(|error| {
        WorkerError::Refused(format!(
            "the confined generator refused, in words this build cannot read: {error}"
        ))
    })
}
