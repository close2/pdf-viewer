//! The privileged side of the boundary: start a worker, exchange frames with it, end it.

use std::io::{Read as _, Write as _};
use std::path::Path;
use std::process::{ChildStdout, Command as OsCommand, Stdio};

use pdf_sandbox::lockdown::Confinement;

use crate::supervision::Canceller;
use crate::{Channel, SentDescriptor, frame, greeting};

/// Why an exchange with a confined worker could not happen.
///
/// Every variant is a *refusal*, and none of them is a reason to carry on as though the work had
/// happened somewhere else. There is deliberately no path in this crate that does the work in the
/// calling process when a worker cannot be started, which is the same rule `pdf-sandbox` states
/// and for the same reason.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// The worker program could not be started.
    #[error("starting the confined worker failed: {0}")]
    Spawn(#[source] std::io::Error),
    /// The worker stopped before answering.
    #[error("the confined worker stopped without answering ({detail})")]
    WorkerDied {
        /// How it stopped, as far as the host can tell.
        detail: String,
    },
    /// The socket to the worker, or the pipe from it, failed.
    #[error("the connection to the confined worker failed: {0}")]
    Connection(#[source] std::io::Error),
    /// The worker sent a frame whose header this build cannot read.
    #[error("the confined worker sent a frame whose header this build cannot read")]
    UnrecognisedFrame,
    /// The worker sent a message this process could not find room for.
    ///
    /// **The mirror of the worker's own refusal, and it is here because the worker is the
    /// untrusted side of this boundary**: a length it states is a claim, and a host that believed
    /// a two-gibibyte claim would abort on the allocation instead of refusing it. The frame's
    /// bytes are read and thrown away, so the worker and whatever it holds survive the refusal.
    #[error("the confined worker sent {bytes} bytes and this process could not find room for them")]
    NoRoom {
        /// What the frame header claimed.
        bytes: usize,
    },
    /// A [`Canceller`] ended the worker, so there is nothing left to ask.
    ///
    /// Distinct from [`Self::WorkerDied`] on purpose: a worker that died is a fault to report, and
    /// a worker the host itself ended is not.
    #[error("the confined worker was cancelled, and it ended with it")]
    Cancelled,
}

/// A running confined worker, and the frames a host exchanges with it.
///
/// **Every call here blocks for as long as the work takes**, and that is deliberate — see
/// [`Canceller`] for why the work has no deadline and what a host holds instead.
#[derive(Debug)]
pub struct Host {
    /// The cancel flag and the child.
    canceller: Canceller,
    /// The worker's standard input: a socket where a descriptor can cross.
    to_worker: Channel,
    /// The worker's standard output: bytes only.
    from_worker: ChildStdout,
    /// What the worker reported it reached.
    confinement: Confinement,
}

impl Host {
    /// Starts `program` as a confined worker and reads its greeting under `magic`.
    ///
    /// The greeting is what the worker reached, not what it asked for: a kernel can refuse what a
    /// build offers, so [`Self::confinement`] is a report rather than a promise.
    ///
    /// The canceller is armed before the worker is spawned, which closes the one gap a handle
    /// obtained afterwards cannot: this call blocks reading the greeting, and a handle taken from
    /// its result does not exist while it is blocked.
    ///
    /// # Errors
    ///
    /// See [`TransportError`]. A worker that cannot confine itself never sends a greeting, so it
    /// arrives here as [`TransportError::WorkerDied`] — never as a worker that quietly runs
    /// unconfined.
    pub fn start(
        program: &Path,
        magic: &[u8; 8],
        canceller: &Canceller,
    ) -> Result<Self, TransportError> {
        if canceller.is_cancelled() {
            return Err(TransportError::Cancelled);
        }

        let (to_worker, worker_end) = Channel::pair().map_err(TransportError::Spawn)?;
        let mut child = OsCommand::new(program)
            // **One allocator arena, and it is a finding rather than a tuning knob.** `glibc`
            // creates a per-thread arena at a thread's *first* allocation, and sizes the number
            // of arenas from `__get_nprocs`, which reads `/sys/devices/system/cpu/online` — an
            // `openat` every confined profile here kills the process for. So a worker that
            // dispatches any work onto a second thread dies on that thread's first `malloc`,
            // whatever the work was: `pdf-vfs-worker` extracting the two images on page 60 of
            // `doc/Tagged-PDF-Best-Practice-Guide.pdf` was killed with `SIGSYS`, `syscall=257`
            // in the kernel's own audit line, while every page with one image was fine —
            // `rayon` runs a single item on the calling thread and hands two to the pool
            // (round 911).
            //
            // `MALLOC_ARENA_MAX` is read by `glibc` at start-up, so it has to be set *here*,
            // by whoever spawns: the worker itself is already past it by `main`, and the
            // confinement cannot be moved after the pool because the Landlock domain is
            // per-thread. One arena is also what these workers want on their own account —
            // both run their rasterisation on one thread by `pdf_vfs::serve`'s own finding.
            .env("MALLOC_ARENA_MAX", "1")
            .stdin(worker_end)
            .stdout(Stdio::piped())
            // A pipe rather than the inherited descriptor: `RLIMIT_FSIZE` is 0 in the confinement,
            // so a worker whose standard error is a *file* — every logged deployment — is killed
            // by `SIGXFSZ` the moment it tries to explain itself. It is still written on to this
            // process's standard error, so an operator sees what they saw before.
            .stderr(Stdio::piped())
            .spawn()
            .map_err(TransportError::Spawn)?;

        let (Some(to_worker), Some(from_worker)) =
            (to_worker.attach(&mut child), child.stdout.take())
        else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(TransportError::Spawn(std::io::Error::other(
                "the worker was started without pipes",
            )));
        };

        // Published before the blocking read below, so that a cancel arriving during the greeting
        // has something to signal — and re-checked after publishing, because one arriving between
        // the check above and this line would otherwise have found an empty slot and been lost.
        let name = program.file_name().map_or_else(
            || String::from("the confined worker"),
            |name| name.to_string_lossy().into_owned(),
        );
        canceller.adopt(&name, child);
        if canceller.is_cancelled() {
            canceller.kill();
        }

        // Read before anything is constructed, so that the confinement this holds is the one the
        // worker reported rather than a value that stood in for it: until the greeting arrives,
        // nothing is known about the process on the other end — including whether it is the
        // worker at all.
        let mut said = [0u8; greeting::LEN];
        let mut from_worker = from_worker;
        let confinement = match from_worker.read_exact(&mut said) {
            Ok(()) => greeting::parse(magic, &said),
            Err(_) => None,
        };
        let Some(confinement) = confinement else {
            let detail = canceller.reap();
            return Err(if canceller.is_cancelled() {
                TransportError::Cancelled
            } else {
                TransportError::WorkerDied { detail }
            });
        };

        Ok(Self {
            canceller: canceller.clone(),
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

    /// A handle another thread can end this worker with.
    #[must_use]
    pub fn canceller(&self) -> Canceller {
        self.canceller.clone()
    }

    /// Whether this worker has been cancelled, in which case every call refuses.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.canceller.is_cancelled()
    }

    /// Writes one frame and reads the one that answers it.
    ///
    /// # Errors
    ///
    /// See [`TransportError`].
    pub fn exchange(
        &mut self,
        kind: u8,
        payload: &[u8],
        descriptor: Option<SentDescriptor<'_>>,
    ) -> Result<(u8, Vec<u8>), TransportError> {
        self.still_running()?;
        self.write_frame(kind, payload, descriptor)?;
        self.read_frame()
    }

    /// Writes one frame and flushes it.
    ///
    /// **Header and payload in two calls, never concatenated.** A document is 19.2 MB, and putting
    /// nine bytes in front of it by building a third buffer costs a whole pass over it plus the
    /// page faults for a fresh allocation that size. The socket itself moves those bytes in about
    /// 4 ms; everything around it cost ten times that (ADR 0241).
    ///
    /// **A descriptor rides beside the header** (ADR 0812): the nine header bytes go out with
    /// `sendmsg` and the descriptor as `SCM_RIGHTS`, so the worker's `recvmsg` of the header is
    /// what delivers it, and the payload follows as bytes. The kernel duplicates the descriptor
    /// into the worker at that moment; what this side holds stays this side's.
    fn write_frame(
        &mut self,
        kind: u8,
        payload: &[u8],
        descriptor: Option<SentDescriptor<'_>>,
    ) -> Result<(), TransportError> {
        let header = frame::header(kind, payload.len());
        self.to_worker
            .send_header(&header, descriptor)
            .and_then(|()| self.to_worker.write_all(payload))
            .and_then(|()| self.to_worker.flush())
            .map_err(|error| self.explain(error))
    }

    /// Reads one whole frame.
    ///
    /// **The buffer is asked for rather than demanded**, because its size is a number the *worker*
    /// wrote into the header and the worker is the untrusted side here.
    fn read_frame(&mut self) -> Result<(u8, Vec<u8>), TransportError> {
        let mut header = [0u8; frame::HEADER_LEN];
        self.read_exactly(&mut header)?;
        let (kind, length) =
            frame::parse_header(header).ok_or(TransportError::UnrecognisedFrame)?;
        let mut payload = Vec::new();
        if payload.try_reserve_exact(length).is_err() {
            // Read past it rather than giving up on the pipe: the worker has written these bytes
            // already, and leaving them there would make the next frame start in the middle of
            // this one. Refusing this way costs the answer and keeps the worker.
            self.discard(length)?;
            return Err(TransportError::NoRoom { bytes: length });
        }
        payload.resize(length, 0);
        self.read_exactly(&mut payload)?;
        Ok((kind, payload))
    }

    /// Reads and throws away a stated number of bytes, in a buffer whose size is this side's.
    fn discard(&mut self, mut bytes: usize) -> Result<(), TransportError> {
        /// Enough to empty a pipe quickly and small enough to be nobody's memory problem.
        const SCRATCH: usize = 64 * 1024;

        let mut scratch = vec![0u8; SCRATCH.min(bytes.max(1))];
        while bytes > 0 {
            let take = bytes.min(scratch.len());
            let Some(slice) = scratch.get_mut(..take) else {
                break;
            };
            self.read_exactly(slice)?;
            bytes = bytes.saturating_sub(take);
        }
        Ok(())
    }

    /// Fills `buffer` from the worker.
    ///
    /// **There is no deadline here, and that is a decision rather than an omission.** What a
    /// confined worker is asked to do is bounded by the document *and* by what was asked, so a
    /// fixed number would refuse work the caller permits. What a host has instead is a
    /// [`Canceller`], which ends the worker from another thread — and ending it is what makes the
    /// read below return, because the pipe's other end closes with the process.
    fn read_exactly(&mut self, buffer: &mut [u8]) -> Result<(), TransportError> {
        match self.from_worker.read_exact(buffer) {
            Ok(()) => Ok(()),
            // The worker closed its output, which it only does on the way out — so its status is
            // worth waiting for rather than reporting the read.
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Err(self.died()),
            Err(error) => Err(self.explain(error)),
        }
    }

    /// Refuses a call on a worker the host has already ended.
    ///
    /// Checked before anything is written, so that a cancel arriving between two exchanges is the
    /// same answer as one arriving during the work rather than a pipe error about a dead process.
    fn still_running(&self) -> Result<(), TransportError> {
        if self.canceller.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        Ok(())
    }

    /// Waits for a worker whose output has closed, and says how it ended.
    ///
    /// Blocking, for the reason `pdf_sandbox`'s own version gives: a status sampled the instant a
    /// pipe closes is usually not there yet, and a diagnosis that depends on scheduling is worse
    /// than none because it is believed.
    fn died(&mut self) -> TransportError {
        let detail = self.canceller.reap();
        if self.canceller.is_cancelled() {
            return TransportError::Cancelled;
        }
        TransportError::WorkerDied { detail }
    }

    /// Turns a pipe failure into the reason the worker is gone, when it is.
    fn explain(&mut self, error: std::io::Error) -> TransportError {
        if self.canceller.is_cancelled() {
            return TransportError::Cancelled;
        }
        match self.canceller.ended() {
            Some(detail) => TransportError::WorkerDied { detail },
            None => TransportError::Connection(error),
        }
    }
}

impl Drop for Host {
    /// Ends the worker.
    ///
    /// Killed rather than asked: the worker leaves when its input closes, which dropping the
    /// socket does — but a worker in the middle of the work would not notice for as long as that
    /// takes, and a host that has dropped its handle is not waiting for one.
    fn drop(&mut self) {
        self.canceller.kill();
        let _ = self.canceller.reap();
    }
}
