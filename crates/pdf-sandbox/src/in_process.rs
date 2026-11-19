//! Decoding in this process, and the deadline a caller of it is held to.
//!
//! [`crate::Isolation::InProcess`] runs the worker's own filters on a thread of this process. The
//! worker's deadline is a *kill*: the parent stops reading at `REQUEST_TIMEOUT` and ends the
//! process, and whatever the decoder was doing ends with it. A thread cannot be killed, and
//! neither codec offers a way to be asked to stop — `hayro_jbig2::Image::decode` and
//! `hayro_jpeg2000`'s decode each run to their end in one call. So the bound here is on the
//! **caller's wait**, not on the work: the decode runs on a decoding thread this module keeps, the
//! caller waits while the thread has processor time left of its budget, and a decode that spends
//! it is *abandoned* — left to finish on its thread, holding what it has allocated until it does,
//! and reported as [`SandboxError::Exhausted`], the image's refusal.
//!
//! **The budget is the thread's processor time, as the worker's is the worker's** (ADR 3133, ADR
//! 3150): the thread names itself through `/proc/thread-self` when it takes a request and reads
//! its own time then, and the caller reads the time under that name while it waits
//! (the `clock` module), so a thread a loaded machine starves is not abandoned for the machine's
//! load. A wall-clock ceiling of ten times the budget stays behind it, the worker's own,
//! refused as [`SandboxError::Overran`], the machine's refusal; where `/proc` cannot be read the
//! wall clock is the whole budget, refused the same way.
//!
//! **An abandoned decode may never finish.** ITU-T T.88 section E.3.4 has the arithmetic decoder
//! fed 1-bits once its data is exhausted, for as long as decoding asks, and a symbol dictionary
//! whose coded data ends inside its height-class loop can ask for ever: section 6.5.5's loop ends
//! only when its count of new symbols is reached, and a run of empty height classes whose delta
//! height is zero never reaches it (ADR 1447). So abandoned decodes are counted, and at
//! [`MAX_ABANDONED`] still running this path refuses further decodes with a sentence rather than
//! spend a further core on each — the one thing a process that cannot stop a thread can do
//! about it. [`crate::Isolation::Sandboxed`] has none of this cost, which is part of why it is
//! the default.

use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::clock::{self, Allowance, Ended, Whose};
use crate::protocol::{self, Request};
use crate::{Decoded, SandboxError, decode};

/// How many abandoned decodes may still be running before this path refuses to start another.
///
/// This program's choice, not a derived number: each abandoned decode holds a core until it ends,
/// and it may not end. Two is enough that one slow but finite image does not stop a document's
/// other images from decoding, and few enough that a document of hostile images costs two cores
/// rather than one per image.
pub const MAX_ABANDONED: usize = 2;

/// Abandoned decodes whose threads have not yet returned, process-wide.
static ABANDONED: AtomicUsize = AtomicUsize::new(0);

/// A decode's thread has not returned and its caller is still waiting.
const RUNNING: u8 = 0;
/// A decode's caller stopped waiting before its thread returned.
const LEFT: u8 = 1;
/// A decode's thread has returned.
const RETURNED: u8 = 2;

/// Decodes on the calling thread, with no deadline.
///
/// The path for a caller that bounds time itself. A fuzzer, whose own per-input timeout is the
/// bound, and which [`decode_here_within`] would serve badly: it abandons a slow input at its
/// deadline and, once [`MAX_ABANDONED`] run on, refuses every input after them. And a process
/// behind its own system-call filter, which [`crate::decode()`] sends here: starting a thread asks
/// for `prctl`, which the filter kills for, and such a process is bounded from outside by whoever
/// can kill it (ADR 1447). It is the same function the worker calls, so what it decodes is what
/// the worker would.
///
/// # Errors
///
/// [`SandboxError::Undecodable`], with the filter's own sentence.
pub fn decode_here(request: &Request<'_>) -> Result<Decoded, SandboxError> {
    decode::here(request)
}

/// Decodes in this process, waiting while the decoding thread has spent less than `timeout` of
/// processor time on it, and at most ten times `timeout` of wall clock, the worker's ceiling.
///
/// What [`crate::decode()`] does under [`crate::Isolation::InProcess`], with the worker's own
/// `REQUEST_TIMEOUT`; named so that a caller can state a shorter one. The request is copied into
/// the form the worker reads from its pipe, because the thread that decodes it may outlive the
/// borrow the caller holds.
///
/// # Errors
///
/// - [`SandboxError::Exhausted`] when the decode spent `timeout` of processor time unfinished;
/// - [`SandboxError::Overran`] when it neither finished nor spent that before the wall-clock
///   ceiling, or, where its thread's processor time cannot be read, did not finish within
///   `timeout` of wall clock;
/// - [`SandboxError::TooManyOverran`] when [`MAX_ABANDONED`] decodes that overran are still
///   running, so this one is not started;
/// - [`SandboxError::Thread`] when no thread could be started to decode on;
/// - [`SandboxError::Undecodable`], with the filter's own sentence.
pub fn decode_here_within(
    request: &Request<'_>,
    timeout: Duration,
) -> Result<Decoded, SandboxError> {
    crate::counted(within(request, timeout))
}

/// [`decode_here_within`] before its wall-clock refusals are counted.
fn within(request: &Request<'_>, timeout: Duration) -> Result<Decoded, SandboxError> {
    let still_running = ABANDONED.load(Ordering::Acquire);
    if still_running >= MAX_ABANDONED {
        return Err(SandboxError::TooManyOverran {
            running: still_running,
        });
    }

    let wire = protocol::read_request(&mut protocol::encode_request(request).as_slice())
        .ok()
        .flatten()
        .ok_or_else(|| SandboxError::Malformed {
            detail: "a request did not survive its own framing".to_owned(),
        })?;
    let state = Arc::new(AtomicU8::new(RUNNING));
    let (answer, answered) = mpsc::channel();
    let began = Instant::now();
    dispatch(Job {
        wire,
        answer,
        state: Arc::clone(&state),
    })?;

    // Until the thread has said who it is, the wall clock from `began` is all there is to wait
    // under: `timeout` of it, the bound where no reading is possible.
    let mut allowance = Allowance::since(None, timeout, began);
    loop {
        let wait = match allowance.next_wait() {
            Ok(wait) => wait,
            Err(ended) => return abandon(&state, &answered, ended, timeout),
        };
        match answered.recv_timeout(wait) {
            Ok(Message::Taken { whose, at_start }) => {
                allowance = Allowance::since(whose.zip(at_start), timeout, began);
            }
            Ok(Message::Answer(decoded)) => return decoded,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(SandboxError::Malformed {
                    detail: "the decoding thread ended without answering".to_owned(),
                });
            }
        }
    }
}

/// Leaves a decode its allowance has `ended` running on its thread, and says why — unless it
/// answered between the allowance's end and the claim, in which case its answer is the result.
fn abandon(
    state: &AtomicU8,
    answered: &mpsc::Receiver<Message>,
    ended: Ended,
    budget: Duration,
) -> Result<Decoded, SandboxError> {
    let refusal = match ended {
        Ended::Spent => SandboxError::Exhausted { budget },
        Ended::Ceiling { after } => SandboxError::Overran { after },
    };
    // Counted before it is claimed, so that a thread returning in between never subtracts what
    // was not yet added.
    ABANDONED.fetch_add(1, Ordering::AcqRel);
    if state
        .compare_exchange(RUNNING, LEFT, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        return Err(refusal);
    }
    // It returned between the end and the claim, so its answer is waiting behind any `Taken`.
    ABANDONED.fetch_sub(1, Ordering::AcqRel);
    while let Ok(message) = answered.recv() {
        if let Message::Answer(decoded) = message {
            return decoded;
        }
    }
    Err(refusal)
}

/// What a decoding thread tells the caller waiting on one request, in this order.
enum Message {
    /// The thread has taken the request: its name under `/proc`, and its processor time as it
    /// began, each `None` where `/proc` could not be read.
    Taken {
        /// The thread, as [`clock::this_thread`] names it.
        whose: Option<Whose>,
        /// Its processor time before the decode.
        at_start: Option<Duration>,
    },
    /// The decode's outcome.
    Answer(Result<Decoded, SandboxError>),
}

/// One request for a decoding thread: what to decode, where to answer, and whether anyone is
/// still waiting.
struct Job {
    /// The request, as the worker would have read it from its pipe.
    wire: protocol::Wire,
    /// Where the answer goes; a receiver that stopped waiting has been dropped.
    answer: mpsc::Sender<Message>,
    /// [`RUNNING`], [`LEFT`] or [`RETURNED`].
    state: Arc<AtomicU8>,
}

/// Decoding threads with nothing to do, each named by the sender that reaches it.
///
/// **Kept rather than started per decode, and measured to be worth it.** A thread started for
/// each decode costs its start and, the larger part, a cold allocator arena: over 60 decodes of
/// corpus streams on this machine, the median of a 137 KB stream went from 22.1 ms on the calling
/// thread to 30.1 ms on a new thread each time, and of a 1.2 KB stream from 207 to 367 µs; on a
/// kept thread they are 21.5 ms against 21.3 and 206 µs against 199 (ADR 1447). How many are kept needs no bound of its own: a thread
/// joins this list only after finishing a decode it was given, so the list never holds more than
/// the most decodes that were ever running at once.
static IDLE: Mutex<Vec<mpsc::Sender<Job>>> = Mutex::new(Vec::new());

/// Gives `job` to a kept thread, or to a new one when none is idle.
fn dispatch(mut job: Job) -> Result<(), SandboxError> {
    while let Some(idle) = lock(&IDLE).pop() {
        match idle.send(job) {
            Ok(()) => return Ok(()),
            // Its thread is gone — a decoder panic unwound it — so the job comes back.
            Err(mpsc::SendError(back)) => job = back,
        }
    }
    let (sender, jobs) = mpsc::channel();
    let own = sender.clone();
    std::thread::Builder::new()
        .name("pdf-sandbox decode".to_owned())
        .spawn(move || serve(&jobs, &own))
        .map_err(SandboxError::Thread)?;
    sender.send(job).map_err(|_| SandboxError::Malformed {
        detail: "a decoding thread ended before its first request".to_owned(),
    })
}

/// A decoding thread's life: decode what arrives, answer, and wait in [`IDLE`] for the next.
fn serve(jobs: &mpsc::Receiver<Job>, own: &mpsc::Sender<Job>) {
    // A thread's name does not change, so it is read once, on the thread it names.
    let whose = clock::this_thread();
    while let Ok(job) = jobs.recv() {
        let returned = Returned(job.state);
        // A caller already gone has nobody to tell; the same holds for the answer below.
        let _ = job.answer.send(Message::Taken {
            whose,
            at_start: clock::this_threads_time(),
        });
        let decoded = protocol::typed_request(&job.wire).map_or_else(
            || {
                Err(SandboxError::Malformed {
                    detail: "a request named no filter this build implements".to_owned(),
                })
            },
            |request| decode::here(&request),
        );
        // A caller that stopped waiting has dropped the receiver, and then there is nobody to
        // tell: the decode was already reported as `Exhausted` or `Overran`.
        let _ = job.answer.send(Message::Answer(decoded));
        drop(returned);
        lock(&IDLE).push(own.clone());
    }
}

/// The idle list, whatever a panicking holder left it as: it holds senders, and a sender whose
/// thread is gone is found out and dropped by [`dispatch`].
fn lock(idle: &Mutex<Vec<mpsc::Sender<Job>>>) -> std::sync::MutexGuard<'_, Vec<mpsc::Sender<Job>>> {
    match idle.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Marks a decode's thread as returned when it ends, however it ends, and gives back the slot
/// an abandoned one held.
struct Returned(Arc<AtomicU8>);

impl Drop for Returned {
    fn drop(&mut self) {
        if self.0.swap(RETURNED, Ordering::AcqRel) == LEFT {
            ABANDONED.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

/// How many decodes that overran their deadline are still running in this process.
#[must_use]
pub fn overran_still_running() -> usize {
    ABANDONED.load(Ordering::Acquire)
}
