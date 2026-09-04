//! The confined side of RFC 0003 section 6: a generator with no filesystem and no network.
//!
//! The whole of the worker's life: confine itself, say so, take the document the broker hands it,
//! and answer questions until its input closes. It opens nothing, connects to nothing, and —
//! because [`pdf_sandbox::lockdown::Profile::Interpreter`] permits a thread and no new program —
//! starts nothing.
//!
//! # Order matters here more than anywhere else in this crate
//!
//! Lockdown comes first, before a single byte of a document is read. A worker that took its
//! document and *then* confined itself would have parsed untrusted input unconfined, which is the
//! entire failure RFC 0003 section 6 exists to prevent and would look exactly like working code.
//!
//! # The system calls this worker needs beyond the interpreter's, and there are none
//!
//! **[`pdf_sandbox::lockdown::Profile::Interpreter`] unchanged, not a third profile** (ADR 0847).
//! The profile's own documentation says a third one means measuring a third, and it was measured:
//! `strace -f` over this program answering every question in [`crate::worker::Query`] on a real
//! document issues nothing after start-up that is not already on that list. The reason is
//! structural rather than lucky — what this worker does is what `pdf-view-worker` already does:
//!
//! - It **parses** a document handed to it as a descriptor, through the same
//!   `pdf_syntax::FileBytes`, so the same `recvmsg` and `pread64` ADR 0812 admitted, and no
//!   `openat`, `statx`, `lseek` or `fstat`.
//! - It **draws** pages with the same `render-cpu` on the same one-thread `rayon` pool, so the
//!   same `clone3`, `rseq`, `set_robust_list` and `sched_getaffinity`.
//! - It **decodes** §7.4.7's JBIG2 and §7.4.9's JPEG 2000 in-process, because a confined process
//!   cannot spawn `pdf-sandbox-worker` and setting [`pdf_sandbox::Isolation::InProcess`] is what
//!   ADR 0218 already decided for exactly this case.
//! - Everything it *writes* goes into `pdf_transform::MemorySinks`, which is memory: a
//!   transform's file sinks would need `openat` and this worker never constructs one.
//!   `RLIMIT_FSIZE` is 0 in the confinement, so even a sink that tried would be killed rather
//!   than obeyed.
//!
//! The probes in `tests/confined.rs` are what hold that claim: that a confined worker answers
//! every question, and that it is killed for asking the filesystem anything.

use std::io::Write as _;

use confined_transport::link::{Link, ReceivedDescriptor};
use pdf_syntax::FileBytes;
use pdf_transform::{Secret, Source as TransformSource};

use crate::wire::{self, Held};
use crate::worker::{InProcess, Worker as _, WorkerError};

/// The name of the worker program, **without the platform's executable suffix**.
///
/// A separate executable rather than this one re-invoked with a flag, for the reason
/// [`pdf_sandbox::WORKER_PROGRAM`] gives: everything it links is reachable only from a `main`
/// whose first statements give away the ability to do anything but read and derive from a
/// document.
pub const WORKER_PROGRAM: &str = "pdf-vfs-worker";

/// Environment variable naming the worker program explicitly.
///
/// Set it when the executable is not installed alongside its worker — in particular, when running
/// a test binary that Cargo has put in `target/<profile>/deps/`.
pub const WORKER_PATH_VARIABLE: &str = "PDF_VFS_WORKER";

/// How many threads a confined process rasterises with.
///
/// **One, and it is a finding rather than a preference.** `glibc`'s allocator sizes its arena
/// count from `__get_nprocs`, which reads `/sys/devices/system/cpu/online` — so the first
/// allocation in a thread of a many-threaded confined process is an `openat` the filter kills the
/// process for (ADR 0218). It costs speed and not bytes: `render-cpu`'s own property is that a
/// machine with four cores and one with thirty-two draw the same bytes, and `tests/a_face.rs`
/// holds a page out of the mount to what `pdf-transform` itself writes on every core.
const RASTERISING_THREADS: u32 = 1;

/// How many copies of an answer live at once, at the moment the peak is reached.
///
/// **Two.** The transform seam's `MemorySinks` hands back the file it generated, and
/// [`crate::wire::encode_answer`] writes a second copy of it into the frame's payload; the sink's
/// copy is dropped when the answer is, which is after the frame has been written. Nothing else
/// holds a third: the query that asked for it is nine bytes and a page number.
const COPIES_OF_AN_ANSWER: std::num::NonZeroU64 = match std::num::NonZeroU64::new(2) {
    Some(copies) => copies,
    // A literal two is not zero. The arm is written rather than unwrapped because this workspace
    // forbids an `unwrap` outside a test even where the compiler can see through it.
    None => std::num::NonZeroU64::MIN,
};

/// What confining this process settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerLimits {
    /// What confinement was reached, which is what the greeting carries to the broker.
    pub confinement: pdf_sandbox::lockdown::Confinement,
    /// How many strips a page's raster may be cut into. See [`RASTERISING_THREADS`].
    pub strips: u32,
    /// The largest message this process will read or write, in bytes.
    ///
    /// Derived rather than chosen — see [`message_budget`]. [`u64::MAX`] where there is no ceiling
    /// to derive it from, which is every platform `doc/todo/35` covers.
    pub message_budget: u64,
    /// What this process's address space was *before* the confinement, in bytes.
    ///
    /// Kept because the budget is derived again when a document arrives with a ceiling of its
    /// own, and this term cannot be read a second time: `/proc/self/status` is a file, and a
    /// confined process has none.
    pub address_space: u64,
}

/// The largest message a ceiling leaves room for, in bytes.
///
/// **Every term is read from somewhere or measured, and none is picked.** The arithmetic is
/// `confined_transport::ceiling`'s, shared with `viewer-confined` (ADR 0846); the two numbers
/// under it are this worker's:
///
/// - `already` is this process's own address space *before* the confinement. It has to be read
///   there because a confined process has no filesystem.
/// - `reserved` is what one rendered page may claim beside the message. `pdf_transform::Budget`'s
///   own `max_pixels` is the ceiling on a page, and RGBA is four bytes of it — the same
///   arithmetic `INTERPRETER_ADDRESS_SPACE_LIMIT` was itself derived from. The default budget's
///   2²⁸ pixels is a gibibyte.
///
/// **What this bounds is the answer as much as the question**, which is the direction that
/// matters here: a mount will be asked for a 300 dpi render of a large page, and the frame that
/// carries it back is the largest thing on this wire. A worker whose answer is past the budget
/// refuses it *by name* rather than writing a frame the broker will not hold — see
/// [`unaffordable`].
#[must_use]
pub fn message_budget(ceiling: u64, already: u64, max_pixels: u64) -> u64 {
    confined_transport::ceiling::message_budget(
        ceiling,
        already,
        max_pixels.saturating_mul(4),
        COPIES_OF_AN_ANSWER,
    )
}

/// Confines this process for deriving files from a document, and says what that settled.
///
/// **Five steps in this order, and each is where it is for a reason the next one makes true.**
///
/// 1. **Image decoding moves in-process**, because after step 4 nothing can be spawned, and
///    **the machine's fonts are declared unreachable**, because after step 4 there is no
///    filesystem and `pdf_font::substitute` would otherwise walk `/usr/share/fonts` and be
///    *killed* rather than told no (ADR 0870).
/// 2. **How many processors this machine has is asked now, and mostly thrown away.**
///    `std::thread::available_parallelism` reads `/proc/self/cgroup` on Linux, so a confined
///    process asking it is *killed* rather than told no — and this is the one place it can be
///    asked.
/// 3. **How much address space this process already occupies is read**, for the same reason and
///    from the same impossibility: `/proc/self/status` is a file, and after step 4 there are none.
/// 4. **The confinement itself**, with [`pdf_sandbox::lockdown::Profile::Interpreter`] — the
///    viewer's profile, unchanged, because this worker needs nothing the viewer's does not.
/// 5. **`rayon`'s pool, with that number stated**, built *after* the confinement so that its
///    thread inherits both the Landlock domain and the seccomp filter.
///
/// # Errors
///
/// Returns an error if the process could not be confined or the thread pool could not be built. A
/// caller that gets one must not go on to read anything: it is not confined.
pub fn confine() -> Result<WorkerLimits, std::io::Error> {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    // A substitute font is read off the machine, and a confined process cannot read
    // anything: `openat` is not on the seccomp allow-list, whose action is
    // `SECCOMP_RET_KILL_PROCESS`, so the walk over the font directories ends this
    // process instead of returning an `Err` it is already written to shrug off. Stated
    // here for the same reason as the two steps below — this is the last moment it can
    // be stated at all.
    pdf_font::substitute::no_machine_fonts();

    let machine = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let threads = usize::try_from(RASTERISING_THREADS)
        .unwrap_or(1)
        .min(machine);
    let already = confined_transport::ceiling::address_space_in_use();

    let confinement = pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
        .map_err(std::io::Error::other)?;

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .map_err(std::io::Error::other)?;

    Ok(WorkerLimits {
        confinement,
        strips: u32::try_from(threads).unwrap_or(1),
        // The budget the *default* ceilings imply, so that the first frame — the open frame,
        // which carries the document — has a bound before the document's own budget is known.
        // Replaced by the opened document's own once it has been read.
        message_budget: message_budget(
            confinement.address_space_limit,
            already,
            pdf_transform::Budget::default().max_pixels,
        ),
        address_space: already,
    })
}

/// Runs the confined worker to completion.
///
/// Reads messages from standard input and writes answers to standard output, which are the socket
/// and the pipe the broker connected. Returns when the broker closes its end.
///
/// # Errors
///
/// Returns an error if the process could not be confined, or if the socket or the pipe failed. A
/// confinement failure returns *before* the greeting, so the broker sees a worker that never
/// identified itself rather than one it can trust.
pub fn serve() -> Result<(), std::io::Error> {
    let mut limits = confine()?;

    let mut input = Link::stdin(WORKER_PROGRAM);
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    output.write_all(&confined_transport::greeting::encode(
        wire::MAGIC,
        limits.confinement,
    ))?;
    output.flush()?;

    let mut opened: Option<InProcess> = None;
    while let Some(incoming) = read_frame(&mut input, limits.message_budget)? {
        let (kind, response) = match incoming {
            Incoming::Frame {
                kind,
                payload,
                mut descriptors,
            } => answer(&mut opened, &mut limits, kind, &payload, &mut descriptors),
            Incoming::NoRoom { length } => (
                wire::FRAME_REFUSAL,
                wire::encode_refusal(&WorkerError::Refused(unaffordable(length, &limits))),
            ),
        };
        output.write_all(&confined_transport::frame::header(kind, response.len()))?;
        output.write_all(&response)?;
        output.flush()?;
    }
    Ok(())
}

/// The sentence a broker is given when a message is larger than the ceiling leaves room for.
///
/// **A sentence rather than a signal number**: a person whose page is too large for the
/// confinement gets told so, and the worker that told them is still running with the document it
/// had open.
fn unaffordable(length: usize, limits: &WorkerLimits) -> String {
    let ceiling = limits.confinement.address_space_limit;
    if ceiling == 0 {
        // No ceiling means no budget, so getting here at all is the *machine* refusing the
        // allocation rather than the confinement refusing the message. Saying otherwise would be
        // naming a bound that was never installed.
        return format!(
            "a message of {length} bytes could not be held: this worker has no address-space \
             ceiling, so what refused it is the machine"
        );
    }
    format!(
        "a message of {length} bytes is more than this confined worker can hold: its \
         address-space ceiling is {ceiling} bytes, a message costs {COPIES_OF_AN_ANSWER} copies of \
         itself beside a page's pixels, so the largest it will read or write is {budget} bytes",
        budget = limits.message_budget,
    )
}

/// One frame, or the fact that there was no room for one.
#[derive(Debug)]
enum Incoming {
    /// A frame, whole.
    Frame {
        /// Which kind.
        kind: u8,
        /// Its bytes.
        payload: Vec<u8>,
        /// What arrived beside it: the document's open file, where the broker sent one.
        descriptors: Vec<ReceivedDescriptor>,
    },
    /// A frame this process will not find room for.
    ///
    /// Its bytes have been read and thrown away, so the next frame begins where the sender thinks
    /// it does. That is what makes this a *refusal* rather than the end of the conversation.
    NoRoom {
        /// What the header said the payload was.
        length: usize,
    },
}

/// Reads one frame, or `None` at end of input.
fn read_frame(
    input: &mut impl confined_transport::link::Source,
    budget: u64,
) -> Result<Option<Incoming>, std::io::Error> {
    let mut header = [0u8; confined_transport::frame::HEADER_LEN];
    let mut descriptors = Vec::new();
    match input.fill(&mut header, &mut descriptors) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let Some((kind, length)) = confined_transport::frame::parse_header(header) else {
        return Err(std::io::Error::other(
            "a frame whose length this build does not define",
        ));
    };
    if !matches!(kind, wire::FRAME_OPEN | wire::FRAME_QUERY) {
        return Err(std::io::Error::other(
            "a frame whose kind this build does not define",
        ));
    }

    let affordable = u64::try_from(length).is_ok_and(|length| length <= budget);
    let mut payload = Vec::new();
    if !affordable || payload.try_reserve_exact(length).is_err() {
        // Read past it rather than closing: the sender has already written these bytes, and a
        // reader that left them in the socket would read the next frame out of the middle of this
        // one. A descriptor that came with the header is dropped here with the frame — closed —
        // because the document it opens is the one this process just refused.
        drop(descriptors);
        input.skip(length)?;
        return Ok(Some(Incoming::NoRoom { length }));
    }
    payload.resize(length, 0);
    input.fill(&mut payload, &mut descriptors)?;
    Ok(Some(Incoming::Frame {
        kind,
        payload,
        descriptors,
    }))
}

/// Answers one frame, as the kind to write and the payload to write after it.
///
/// A refusal is a *response* and not an error: a broker that asked for a page this document does
/// not have keeps its worker, and only a broken pipe ends one.
///
/// **A descriptor is claimed by the one frame that names it and every other is closed.** A
/// confined process keeping a descriptor nobody asked it to hold would be the one thing the
/// descriptor ceiling exists to notice (ADR 0812).
fn answer(
    opened: &mut Option<InProcess>,
    limits: &mut WorkerLimits,
    kind: u8,
    payload: &[u8],
    descriptors: &mut Vec<ReceivedDescriptor>,
) -> (u8, Vec<u8>) {
    match kind {
        wire::FRAME_OPEN => match open(limits, payload, descriptors) {
            Ok(worker) => {
                *opened = Some(worker);
                (wire::FRAME_READY, Vec::new())
            }
            Err(error) => (wire::FRAME_REFUSAL, wire::encode_refusal(&error)),
        },
        wire::FRAME_QUERY => {
            let Some(worker) = opened.as_ref() else {
                return (
                    wire::FRAME_REFUSAL,
                    wire::encode_refusal(&WorkerError::Refused(String::from(
                        "this worker was asked a question before it was given a document",
                    ))),
                );
            };
            match wire::decode_query(payload) {
                Ok(query) => match worker.ask(&query) {
                    Ok(answer) => {
                        let encoded = wire::encode_answer(&answer);
                        // **The answer is measured against the same budget the question was.**
                        // A frame larger than this process can hold twice is one the broker
                        // cannot hold either, and a sentence is a better answer than an abort in
                        // the middle of writing it.
                        if u64::try_from(encoded.len())
                            .is_ok_and(|len| len <= limits.message_budget)
                        {
                            (wire::FRAME_ANSWER, encoded)
                        } else {
                            let sentence = unaffordable(encoded.len(), limits);
                            (
                                wire::FRAME_REFUSAL,
                                wire::encode_refusal(&WorkerError::Refused(sentence)),
                            )
                        }
                    }
                    Err(error) => (wire::FRAME_REFUSAL, wire::encode_refusal(&error)),
                },
                Err(error) => (
                    wire::FRAME_REFUSAL,
                    wire::encode_refusal(&WorkerError::Refused(error.to_string())),
                ),
            }
        }
        // `read_frame` refuses every other kind before this is reached; the arm is written rather
        // than asserted, because a frame this build does not define is a message and not a fault.
        _ => (
            wire::FRAME_REFUSAL,
            wire::encode_refusal(&WorkerError::Refused(String::from(
                "a frame whose kind this build does not define",
            ))),
        ),
    }
}

/// Takes the document the broker handed over, and the ceilings it set.
fn open(
    limits: &mut WorkerLimits,
    payload: &[u8],
    descriptors: &mut Vec<ReceivedDescriptor>,
) -> Result<InProcess, WorkerError> {
    let opening =
        wire::decode_open(payload).map_err(|error| WorkerError::Refused(error.to_string()))?;
    let bytes = match opening.document {
        Held::Bytes(bytes) => FileBytes::from(bytes),
        Held::OnDisk { length } => {
            if descriptors.is_empty() {
                return Err(WorkerError::Refused(String::from(
                    "the broker said the document was on disk and sent no descriptor with it",
                )));
            }
            // The first, because a frame carries one document and the sender attaches it to the
            // header; anything after it is nobody's and the caller closes it.
            let descriptor = descriptors.remove(0);
            FileBytes::from_handle(std::fs::File::from(descriptor), length).map_err(|_| {
                WorkerError::Refused(String::from(
                    "the document's stated length is more than this process can address",
                ))
            })?
        }
    };
    // The budget is re-derived from *this* document's ceiling rather than the default's, because
    // a broker that raised `max_pixels` raised what one answer may cost and the two numbers have
    // to move together.
    limits.message_budget = message_budget(
        limits.confinement.address_space_limit,
        limits.address_space,
        opening.budget.max_pixels,
    );
    let source = match opening.password {
        Some(password) => TransformSource::with_password(bytes, Secret::from(password)),
        None => TransformSource::new(bytes),
    };
    Ok(InProcess::new(
        source,
        opening.policy,
        opening.budget,
        Some(limits.strips),
    ))
}
