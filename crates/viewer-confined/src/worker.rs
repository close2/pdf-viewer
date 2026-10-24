//! The confined side of the boundary: a `viewer-core` host with no filesystem.
//!
//! The whole of the worker's life: confine itself, say so, then answer commands and questions
//! until its input closes. It opens nothing, connects to nothing, and — because
//! [`pdf_sandbox::lockdown::Profile::Interpreter`] permits a thread and no new program — starts
//! nothing.
//!
//! # Order matters here more than anywhere else in this crate
//!
//! Lockdown comes first, before a single byte of a document is read. A worker that opened its
//! first document and *then* confined itself would have parsed untrusted input unconfined, which
//! is the entire failure this crate exists to prevent and would look exactly like working code.
//!
//! # Three things this does before it confines itself, and why each is safe
//!
//! - **Image decoding is moved in-process.** `pdf-model` decodes JBIG2, JPEG 2000 and CCITT
//!   through `pdf-sandbox`, which *spawns* a worker — and a confined process cannot spawn
//!   anything, by design. So this one sets [`pdf_sandbox::Isolation::InProcess`] and decodes
//!   them here. That is not the in-process fallback `pdf-sandbox` refuses to have: it is the
//!   same code running inside a process that is confined at least as tightly as the decoder's
//!   own worker — seccomp, Landlock and an address-space ceiling. What it costs is the panic
//!   containment `pdf-sandbox`'s second reason names: a decoder panic here takes this process
//!   rather than one image, so it costs the page instead of the viewer. ADR 0218 argues it.
//! - **Its own address space is read**, once, from `/proc/self/status`. A confined process has no
//!   filesystem, so this is the only moment the question can be asked — and the answer is what
//!   turns the ceiling into a message budget instead of a number nobody compares anything against.
//!   See [`message_budget`].
//! - **Nothing else.** In particular no thread pool is warmed: `rayon` builds its pool on first
//!   use, which is inside a render, which is after the confinement — so every rasterising thread
//!   inherits both the Landlock domain and the seccomp filter. Warming one first would leave
//!   threads outside the domain, which `pdf_sandbox::lockdown_linux`'s header states as the rule
//!   for every caller.

use std::io::Write as _;

use confined_transport::link::{Link, ReceivedDescriptor, Source};
use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_core::{Command, Event, Rendered, Viewer};

use crate::protocol;

/// The viewport the worker starts with, in device pixels.
///
/// A viewer with no viewport renders nothing, and `Viewer::new` takes the size precisely so that
/// a host cannot forget to say it — but a *process* is started before its host has said anything.
/// So this is the size in force until the first [`Command::Resize`], and a host that sends one
/// before it opens a document never sees it. Chosen as a page of A4 at 96 dpi, which is a size
/// something sensible happens at rather than a size anything depends on.
const INITIAL_VIEWPORT: (u32, u32) = (794, 1123);

/// The variable whose being set is what makes a pool wider than one thread safe to build here.
///
/// **`glibc`'s allocator would otherwise kill a wide pool.** It creates a per-thread arena at a
/// thread's first allocation and, past a count of its own, sizes the number of arenas from
/// `__get_nprocs`, which reads `/sys/devices/system/cpu/online` — an `openat` the filter kills the
/// process for. Found with `strace -k` on the twenty-fourth rayon worker of a page that had
/// otherwise drawn (ADR 0218). Where `MALLOC_ARENA_MAX` is set, `glibc` takes the limit from it
/// and never asks; it is read at the process's start, so it is the spawner's to set, and
/// `confined_transport::Host::start` sets it to one for every worker it starts. A process started
/// any other way — a test calling [`confine`] in its own process — draws on one thread, which
/// cannot reach the question at all (ADR 1554).
const ARENA_LIMIT_VARIABLE: &str = "MALLOC_ARENA_MAX";

/// The stack each rasterising thread is given, stated rather than defaulted.
///
/// The standard library's own default for a spawned thread, written here because it is what the
/// pool costs the address-space ceiling — [`message_budget`] takes it off what a message may be.
/// A strip's rasterisation recurses nowhere deep; the interpretation, which does, runs on the
/// process's main thread.
const RASTERISING_STACK: usize = 2 << 20;

/// How many copies of a message live at once, at the moment the peak is reached.
///
/// **Two, measured rather than assumed** (ADR 0597). A frame's payload is read into a buffer,
/// [`crate::protocol::decode_command`] copies the document out of it, and `pdf_syntax` copies that
/// into an `Arc<[u8]>` — three copies, and `VmPeak` was the worker's start-up size plus exactly
/// three times the document's length, to the kilobyte, over four sizes. The payload is now dropped
/// the moment the message is decoded, which takes the live set to two: the copy the command holds
/// and the copy the document holds.
///
/// **The remaining one is not ours to make fallible.** `impl From<Vec<T>> for Arc<[T]>` copies and
/// forgets — an `Arc` needs a header the `Vec` has no room for — and there is no stable fallible
/// form of it. So the last copy cannot be a `try_reserve`, which is why this factor exists at all:
/// a message the ceiling cannot hold twice is refused *before* the first byte is read, rather than
/// aborted at the third allocation.
const COPIES_OF_A_MESSAGE: std::num::NonZeroU64 = match std::num::NonZeroU64::new(2) {
    Some(copies) => copies,
    // A literal two is not zero. The arm is written rather than unwrapped because this workspace
    // forbids an `unwrap` outside a test even where the compiler can see through it.
    None => std::num::NonZeroU64::MIN,
};

/// What confining this process settled.
///
/// Three facts a worker needs and cannot ask for afterwards: what the kernel granted, how many
/// strips a raster may be cut into, and how large a message the ceiling leaves room for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerLimits {
    /// What confinement was reached, which is what the greeting carries to the host.
    pub confinement: pdf_sandbox::lockdown::Confinement,
    /// How many strips a page's raster may be cut into: the pool's width (ADR 1554).
    pub strips: u32,
    /// The largest message this process will read, in bytes.
    ///
    /// Derived rather than chosen — see `message_budget()`. [`u64::MAX`] where there is no
    /// ceiling to derive it from, which is every platform `doc/todo/35` covers.
    pub message_budget: u64,
}

/// The largest message a ceiling leaves room for, in bytes.
///
/// **Every term is read from somewhere or measured, and none is picked.** The arithmetic is
/// `confined_transport::ceiling`'s, because it is the same arithmetic `pdf-vfs`'s worker does
/// (ADR 0846); what is *this* worker's is the two numbers it supplies.
///
/// - `ceiling` is `pdf_sandbox`'s `INTERPRETER_ADDRESS_SPACE_LIMIT`, as the kernel installed it
///   and as the greeting reports it. Zero means no ceiling, and then there is no budget either.
/// - `already` is this process's own address space *before* the confinement, read once from
///   `/proc/self/status`. It has to be read there because a confined process has no filesystem:
///   `openat` is not on the allow-list, so this is the only moment the question can be asked.
/// - [`viewer_core::MAX_PIXELS`] × 4 is what a page's pixels may claim, in RGBA. It is subtracted
///   because the document is still held when the raster is allocated, and it is the same
///   arithmetic `INTERPRETER_ADDRESS_SPACE_LIMIT` was itself derived from.
/// - `threads` × [`RASTERISING_STACK`] is what the pool's stacks claim. The pool is built after
///   `already` was read, since it has to be built inside the confinement, so its stacks are not
///   in that figure; the allocator adds no arena for them, because `MALLOC_ARENA_MAX` is one
///   ([`ARENA_LIMIT_VARIABLE`]).
/// - [`COPIES_OF_A_MESSAGE`] is how many copies of it live at once at the peak.
fn message_budget(ceiling: u64, already: u64, threads: usize) -> u64 {
    let stacks = u64::try_from(threads.saturating_mul(RASTERISING_STACK)).unwrap_or(u64::MAX);
    confined_transport::ceiling::message_budget(
        ceiling,
        already,
        viewer_core::MAX_PIXELS
            .saturating_mul(4)
            .saturating_add(stacks),
        COPIES_OF_A_MESSAGE,
    )
}

/// Confines this process for interpreting and drawing pages, and says what that settled.
///
/// **Five steps in this order, and each is where it is for a reason the next one makes true.**
///
/// 1. **Image decoding moves in-process**, because after step 4 nothing can be spawned, and
///    **the machine's fonts are declared unreachable**, because after step 4 there is no
///    filesystem and `pdf_font::substitute` would otherwise walk `/usr/share/fonts` and be
///    *killed* rather than told no (ADR 0870).
/// 2. **How many processors this machine has is asked now**, because
///    `std::thread::available_parallelism` reads `/proc/self/cgroup` on Linux, so a confined
///    process asking it is *killed* rather than told no — and this is the one place it can be
///    asked. The pool is that wide where the spawner limited the allocator's arenas, and one
///    thread where it did not (`ARENA_LIMIT_VARIABLE`, ADR 1554).
/// 3. **How much address space this process already occupies is read**, for the same reason and
///    from the same impossibility: `/proc/self/status` is a file, and after step 4 there are none.
/// 4. **The confinement itself.**
/// 5. **`rayon`'s pool, with that number stated**, built *after* the confinement so that every
///    thread of it inherits both the Landlock domain, which `landlock_restrict_self` gives only to
///    the calling thread and its future children, and the seccomp filter. Stated rather than
///    defaulted because rayon's own default asks the machine, at step 2's syscall.
///
/// # Errors
///
/// Returns an error if the process could not be confined or the thread pool could not be built.
/// A caller that gets one must not go on to interpret anything: it is not confined.
pub fn confine() -> Result<WorkerLimits, std::io::Error> {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    // A substitute font is read off the machine, and a confined process cannot read
    // anything: `openat` is not on the seccomp allow-list, whose action is
    // `SECCOMP_RET_KILL_PROCESS`, so the walk over the font directories ends this
    // process instead of returning an `Err` it is already written to shrug off. Stated
    // here for the same reason as the two steps below — this is the last moment it can
    // be stated at all.
    pdf_font::substitute::no_machine_fonts();
    // ...and the port that gives them back, which is a *port* rather than a permission: the
    // allow-list below is unchanged, this process still cannot name a path, and what it gains is
    // that a description it sends — a family, a weight, the characters a script needs — may come
    // back as a descriptor its broker opened (ADR 0880, `doc/todo/59`). Armed here for the same
    // reason as the line above: after the lockdown nothing can be decided. A broker that offers
    // nothing answers *nothing offered*, which is where this layer defaults to and is the posture
    // ADR 0870 left.
    pdf_font::provider::faces_come_from(confined_transport::link::ask_the_host);
    confined_transport::link::requests_go_to_the_host(crate::WORKER_PROGRAM);

    // **As wide as the machine, and the measurement is why** (ADR 1554): a page that crosses as
    // pixels is rasterised here while the host waits for it, and on `bug1815476.pdf` the round
    // trip of the first `Resize` fell from 0.039 s to 0.011 s at the median of fifteen launches.
    // A strip edge can move a pixel by at most two levels (ADR 0219's finding, ADR 1742's bound),
    // and nothing compares a window's bytes for identity, so the width is the measurement's to set.
    let arenas_limited = std::env::var(ARENA_LIMIT_VARIABLE)
        .ok()
        .and_then(|limit| limit.parse::<u32>().ok())
        .is_some_and(|limit| limit > 0);
    let threads = if arenas_limited {
        std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
    } else {
        1
    };
    let already = confined_transport::ceiling::address_space_in_use();

    let confinement = pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
        .map_err(std::io::Error::other)?;

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .stack_size(RASTERISING_STACK)
        .build_global()
        .map_err(std::io::Error::other)?;

    Ok(WorkerLimits {
        confinement,
        strips: u32::try_from(threads).unwrap_or(1),
        message_budget: message_budget(confinement.address_space_limit, already, threads),
    })
}

/// Runs the confined viewer to completion.
///
/// Reads messages from standard input and writes answers to standard output, which are the pipes
/// the host connected. Returns when the host closes its end.
///
/// # Errors
///
/// Returns an error if the process could not be confined, or if a pipe failed. A confinement
/// failure returns *before* the greeting, so the host sees a worker that never identified itself
/// rather than one it can trust.
pub fn serve() -> Result<(), std::io::Error> {
    let limits = confine()?;

    let mut input = Link::stdin(crate::WORKER_PROGRAM);
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    output.write_all(&protocol::encode_handshake(limits.confinement))?;
    output.flush()?;

    let mut viewer = Viewer::new(INITIAL_VIEWPORT.0, INITIAL_VIEWPORT.1, 1.0);
    let mut rasterizer = CpuRasterizer::new().with_strips(limits.strips);
    // **How far a page has got, told to the host as it goes** (ADR 1811): every command is
    // performed under a figure that relays a reading every `RELAY_EVERY` operators as a notice the
    // host reads inside the exchange it is waiting on. A host that does not listen reads past them.
    let progress = pdf_model::progress::Progress::relaying(Box::new(|reading| {
        // A figure nobody saw is a figure nobody needed: the work goes on either way.
        let _ = confined_transport::link::tell_the_host(&crate::notice::encode(reading));
    }));
    let mut marks = protocol::Marks::default();
    let mut preparing = Preparing::default();
    let mut turning = Turning::default();

    while let Some(incoming) = read_frame(&mut input, limits.message_budget)? {
        // Header and payload in two calls rather than one concatenated buffer: a raster is 4.1 MB
        // and a saved file is a document, so putting nine bytes in front of either by copying it
        // is a pass over megabytes that buys nothing. The host writes the same way (`write_frame`),
        // and ADR 0241 has what the two of them were costing.
        let (kind, response) = match incoming {
            Incoming::Frame {
                kind,
                payload,
                descriptors,
            } => progress.watch(|| {
                answer(
                    &mut viewer,
                    &mut rasterizer,
                    (&mut marks, &mut preparing, &mut turning),
                    (kind, payload, descriptors),
                )
            }),
            Incoming::NoRoom { length } => refuse(&unaffordable(length, &limits)),
        };
        output.write_all(&protocol::header(kind, response.len()))?;
        output.write_all(&response)?;
        output.flush()?;
    }
    Ok(())
}

/// The sentence a host is given when a message is larger than the ceiling leaves room for.
///
/// **A sentence rather than a signal number**, which is the whole of `doc/todo/15`'s second
/// defect: a person whose document is too large for the confinement gets told so, and the worker
/// that told them is still running with whatever it had open before.
fn unaffordable(length: usize, limits: &WorkerLimits) -> String {
    let ceiling = limits.confinement.address_space_limit;
    if ceiling == 0 {
        // No ceiling means no budget, so getting here at all is the *machine* refusing the
        // allocation rather than the confinement refusing the message. Saying otherwise would be
        // naming a bound that was never installed.
        return format!(
            "a message of {length} bytes could not be held: this viewer has no address-space \
             ceiling, so what refused it is the machine"
        );
    }
    format!(
        "a message of {length} bytes is more than this confined viewer can hold: its \
         address-space ceiling is {ceiling} bytes, a message costs {COPIES_OF_A_MESSAGE} copies of \
         itself beside a page's pixels, so the largest it will read is {budget} bytes",
        budget = limits.message_budget,
    )
}

/// One frame, or the fact that there was no room for one.
#[derive(Debug)]
enum Incoming {
    /// A frame, whole.
    Frame {
        /// Which kind of message it is.
        kind: u8,
        /// Its bytes.
        payload: Vec<u8>,
        /// What arrived beside it: a document's open file, where the frame opens one on disk.
        descriptors: Vec<ReceivedDescriptor>,
    },
    /// A frame this process will not find room for.
    ///
    /// Its bytes have been read and thrown away, so the next frame begins where the sender thinks
    /// it does. That is what makes this a *refusal* rather than the end of the conversation: the
    /// alternative — stopping — would cost the host the document it already had open, which is
    /// exactly the dead window this variant exists to prevent.
    NoRoom {
        /// What the header said the payload was.
        length: usize,
    },
}

/// Reads one frame, or `None` at end of input.
///
/// **The budget is checked before the buffer is asked for**, because the allocation this bound
/// exists to prevent is not the first one — see [`COPIES_OF_A_MESSAGE`]. `try_reserve` after the
/// check is belt to that braces: it catches a machine that cannot find the room for a reason the
/// ceiling knows nothing about.
fn read_frame(input: &mut impl Source, budget: u64) -> Result<Option<Incoming>, std::io::Error> {
    let mut header = [0u8; protocol::FRAME_HEADER_LEN];
    let mut descriptors = Vec::new();
    match input.fill(&mut header, &mut descriptors) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let Some((kind, length)) = protocol::parse_frame_header(header) else {
        return Err(std::io::Error::other(
            "a frame whose kind or length this build does not define",
        ));
    };

    let affordable = u64::try_from(length).is_ok_and(|length| length <= budget);
    let mut payload = Vec::new();
    if !affordable || payload.try_reserve_exact(length).is_err() {
        // Read past it rather than closing: the sender has already written these bytes, and a
        // reader that left them in the pipe would read the next frame out of the middle of this
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
/// A refusal is a *response* and not an error, for the reason `pdf_sandbox::worker` gives: a host
/// that asked for something this does not carry keeps its worker, and only a broken pipe ends
/// one.
///
/// **The payload is taken by value so that it can be dropped before the work starts**, and that
/// is the largest single saving on this boundary rather than a tidiness: everything a decoded
/// message holds is already its own — `Command::Open` owns the document's bytes and
/// `OwnedQuery` owns its string — so the frame buffer is dead the moment the decode returns.
/// Held across the work instead, it was a whole third copy of the document alive at the peak,
/// and `VmPeak` was the worker's start-up size plus exactly three times the document's length
/// (ADR 0597).
///
/// **A descriptor is claimed by the one command that names it and every other is closed.** The
/// decoder takes the document's descriptor out of `descriptors`; what it leaves — a second one
/// somebody sent, or one beside a query — is dropped at the end of this function, which closes
/// it. A confined process keeping a descriptor nobody asked it to hold would be the one thing
/// the descriptor ceiling exists to notice (ADR 0812).
fn answer(
    viewer: &mut Viewer,
    rasterizer: &mut CpuRasterizer,
    (marks, preparing, turning): (&mut protocol::Marks, &mut Preparing, &mut Turning),
    (kind, payload, mut descriptors): (u8, Vec<u8>, Vec<ReceivedDescriptor>),
) -> (u8, Vec<u8>) {
    preparing.look(viewer);
    match kind {
        protocol::FRAME_COMMAND => {
            let decoded = protocol::decode_command_holding(&payload, &mut descriptors);
            drop(payload);
            drop(descriptors);
            match decoded {
                Ok(command) => perform(viewer, rasterizer, (marks, preparing, turning), command),
                Err(error) => refuse(&error.to_string()),
            }
        }
        protocol::FRAME_QUERY => {
            let decoded = protocol::decode_query(&payload);
            drop(payload);
            match decoded {
                Ok(query) => {
                    // The outline is what the thread is reading: waiting for it costs less than
                    // reading it a second time here, since that thread started first.
                    if matches!(query.as_query(), viewer_core::Query::Outline) {
                        preparing.wait(viewer);
                    }
                    match protocol::encode_answer(&viewer.query(query.as_query()), marks) {
                        Ok(encoded) => (protocol::FRAME_ANSWER, encoded),
                        Err(uncarried) => refuse(&uncarried.to_string()),
                    }
                }
                Err(error) => refuse(&error.to_string()),
            }
        }
        // How the host asks a page past the bound to cross, which is recorded and nothing else:
        // the next command moves it, so this answers at once whatever the page costs (ADR 2017).
        protocol::FRAME_ROUTE => match protocol::decode_route(&payload) {
            Ok((page, route)) => {
                marks.route(page, route);
                match protocol::encode_events(&[]) {
                    Ok(encoded) => (protocol::FRAME_EVENTS, encoded),
                    Err(uncarried) => refuse(&uncarried.to_string()),
                }
            }
            Err(error) => refuse(&error.to_string()),
        },
        // A frame a host has no business sending: the three the *worker* produces.
        _ => refuse("that frame is one this side sends, not one it reads"),
    }
}

/// Performs one command, drawing whatever it asked for.
///
/// # What this decides about each page, and why it is decided here
///
/// ADR 0607's payload choice is per page and by size, and this is the one place both sizes are
/// known: the display list is in the request, and the raster's byte count is the target's pixel
/// count times four — arithmetic, not a rasteriser. [`protocol::Marks`] carries the answer to the
/// moment a host asks for a frame.
///
/// **A page whose marks are what crosses is not drawn at all**, which is ADR 0640 and which this
/// comment recorded as a cost for four rounds. What made the render unskippable was a vocabulary
/// question rather than a rasteriser one: the only outcome meaning *no pixels here* was
/// [`Rendered::Presented`], a statement about the **viewer** — it holds for every page at once,
/// silences `Query::Frame` about the pages that must still cross as pixels, and takes
/// `viewer_core::MAX_PIXELS` off what this process is asked to draw, where an unbounded raster is
/// the kill ADR 0597 spent a round turning back into a sentence. [`Rendered::Listed`] says the
/// same thing about **one page**, so the budget stays, the neighbours stay answerable, and the
/// render goes.
fn perform(
    viewer: &mut Viewer,
    rasterizer: &mut CpuRasterizer,
    (marks, preparing, turning): (&mut protocol::Marks, &mut Preparing, &mut Turning),
    command: Command,
) -> (u8, Vec<u8>) {
    // What the preparation said since the last events frame goes first: it is about the page as
    // it stood before this command.
    let mut outgoing = std::mem::take(&mut preparing.owed);
    // **An open is followed by page one's interpretation, viewport or not** (ADR 1539). A host
    // that opens before its window exists tells this process there is no viewport yet, so nothing
    // is arranged and nothing drawn; `Viewer::anticipate` interprets the page the document stands
    // at all the same, while the host's device comes up, and the first `Resize` finds it done.
    // Behind a viewport the arrangement has already interpreted it, and the call does nothing.
    let opens = matches!(command, Command::Open { .. });
    turning.told(&command);
    let mut pending: Vec<Command> = vec![command];

    // A render's answer can itself produce events — a page that finished drawing damages the
    // viewport — so this is a loop rather than one pass. It terminates because `RenderReady` is
    // the only command it adds and a viewer that has been given the frame it asked for does not
    // ask again for the same one.
    while let Some(command) = pending.pop() {
        // A page turn's page is drawn while it is read (ADR 1871); every other command is
        // handled whole, as it always was.
        let events: Vec<Event> = if matches!(command, Command::GoTo(_)) {
            turning.turn(viewer, rasterizer, command)
        } else {
            viewer.handle(command).collect()
        };
        for event in events {
            match event {
                Event::NeedsRender(request) => {
                    let ahead = turning.take_for(&request);
                    let origin = placement(viewer, request.page);
                    let mut declined = Vec::new();
                    let rendered = match marks.decide(&request, origin) {
                        // **The render that does not happen.** The host is being handed this
                        // page's own display list, so pixels of it would be drawn, held and
                        // thrown away — or, for a page past the bound the host asked for as
                        // windows, its window is drawn after this command (ADR 2017).
                        protocol::Carries::Marks | protocol::Carries::Window => Rendered::Listed,
                        // A page past the bound whose marks cannot cross is drawn here as the
                        // window shows it, after this command, and crosses as that window: its
                        // raster would be the allocation the ceiling kills (ADR 1986).
                        protocol::Carries::Pixels if past_the_bound(request.target) => {
                            if let Some(origin) = origin {
                                marks.hold_window(&request, origin);
                                Rendered::Listed
                            } else {
                                // A page the viewer does not place has no window to show.
                                Rendered::Failed(format!(
                                    "this page cannot be drawn at this size: {}",
                                    pdf_render::BackendError::TargetTooLarge {
                                        requested: pixels(request.target),
                                        limit: viewer_core::MAX_PIXELS,
                                    }
                                ))
                            }
                        }
                        // And a page under it is drawn here as the window shows it where the
                        // toolkit windows would draw it so, for a host that places a window
                        // (ADRs 1994, 2016); every other page is drawn whole.
                        protocol::Carries::Pixels => {
                            if let Some(origin) =
                                origin.filter(|_| turning.draws_a_window_of(&request))
                            {
                                marks.hold_window(&request, origin);
                                Rendered::Listed
                            } else {
                                let (rendered, said) = drawn_whole(rasterizer, &request, ahead);
                                declined = said;
                                rendered
                            }
                        }
                    };
                    // Popped after the answer, so the page is told of its draw once it holds it;
                    // and its frames say it, so the host's title names the page (ADR 2316).
                    if !declined.is_empty() {
                        marks.declined_here(request.page);
                        pending.push(Command::Declined {
                            document: request.document,
                            page: request.page,
                            declined,
                        });
                    }
                    pending.push(Command::RenderReady {
                        token: request.token,
                        rendered,
                    });
                }
                other => outgoing.push(other),
            }
        }
    }

    if opens {
        outgoing.extend(viewer.anticipate());
        preparing.start(viewer);
    }

    // **Where the pages this store holds sit now, and which of them are still on the screen** —
    // asked of the viewer rather than deduced, because it is the only thing that knows. A scroll
    // moves a page without redrawing it, and a page the viewer will not place has left Table 29's
    // arrangement: forgetting it is what keeps the store bounded by the screen rather than by
    // everything a reader has ever scrolled past, inside the one process that must not leak.
    //
    // It used to ask `Query::Frame` for the same list. That answer is now silent about exactly
    // the pages this store holds, because those are the ones the viewer keeps no pixels of.
    // Both questions are a borrow of state the viewer already has and cost no interpretation.
    marks.place(|page| placement(viewer, page));
    // A page the host asked for as windows since the last command leaves the marks here, and its
    // first window is among those drawn next (ADR 2017).
    marks.take_routed();
    // **And the windows the viewport now owes**, drawn here after the command that moved it — a
    // zoom's first, a scroll's past the margin — so that the frame the host asks for next is of
    // the view as it stands (ADR 1986).
    if let Some((viewport, _)) = turning.viewport {
        draw_windows(viewer, marks, rasterizer, viewport, &mut outgoing);
    }

    match protocol::encode_events(&outgoing) {
        Ok(encoded) => (protocol::FRAME_EVENTS, encoded),
        Err(uncarried) => refuse(&uncarried.to_string()),
    }
}

/// A page drawn whole: the page drawn while it was read, finished from where its read ended, or a
/// drawing that cannot become the page drawn from the start — with what the draw declined
/// (ADR 2290).
fn drawn_whole(
    rasterizer: &mut CpuRasterizer,
    request: &viewer_core::RenderRequest,
    ahead: Option<render_cpu::Underway>,
) -> (Rendered, Vec<viewer_core::Declined>) {
    if let Some(finished) = ahead.and_then(|underway| underway.finish(&request.list).ok()) {
        return (Rendered::Raster(finished.raster), said(&finished.declined));
    }
    match rasterizer.rasterize(&request.list, request.target) {
        Ok(raster) => (Rendered::Raster(raster), said(rasterizer.declined())),
        // Named rather than swallowed: a viewer that silently showed the previous page when a
        // render failed would be telling a person something false.
        Err(error) => (Rendered::Failed(error.to_string()), Vec::new()),
    }
}

/// What the marks a draw left to the library's converter are called where the viewer is told of
/// them — `viewer_host::drawing::said`'s words, stated here because this process links no host crate (ADR
/// 1871).
fn said(declined: &[render_cpu::UnionDecline]) -> Vec<viewer_core::Declined> {
    declined
        .iter()
        .map(|decline| viewer_core::Declined::new(decline.condition, decline))
        .collect()
}

/// Draws every window `marks` owes at a viewport `viewport` across, from the marks it holds.
///
/// A window the rasteriser refuses lets its page go and is said in the rasteriser's words, as a
/// page's refusal is (trap 5): the viewer was told the host took that render, so the sentence is
/// the only thing left to say it was not drawn. What a window's draw declined is told to the viewer,
/// which says it as it says a whole page's (ADR 2290).
fn draw_windows(
    viewer: &mut Viewer,
    marks: &mut protocol::Marks,
    rasterizer: &mut CpuRasterizer,
    viewport: (u32, u32),
    outgoing: &mut Vec<Event>,
) {
    for (page, list, target, window) in marks.windows_owed(viewport) {
        // On the page's own grid, so the window is the page's pixels there (ADR 2132).
        let shown = render_cpu::Shown {
            left: window.x,
            top: window.y,
            width: window.width,
            height: window.height,
        };
        match rasterizer.rasterize_window(&list, target, shown) {
            Ok(raster) => {
                let declined = said(rasterizer.declined());
                if let Some(document) = marks.drew(page, window, raster)
                    && !declined.is_empty()
                {
                    marks.declined_here(page);
                    outgoing.extend(viewer.handle(Command::Declined {
                        document,
                        page,
                        declined,
                    }));
                }
            }
            Err(error) => {
                if let Some(document) = marks.refused_window(page) {
                    outgoing.push(Event::Reported {
                        document,
                        page: Some(page),
                        notes: vec![format!("this page's window could not be drawn: {error}")],
                    });
                }
            }
        }
    }
}

/// A page turn's page, drawn on a thread of this process's own while it is read (ADR 1871).
///
/// **The in-process windows' arrangement (ADR 1855), in the one process that links no host
/// crate**: `viewer_host::Ahead` brings the host's HTTP client and its TLS with it, which is
/// exactly what a confined process is not to hold, so the few steps it takes are taken here with
/// the two types they are made of — `pdf_model::progress::Parts` and `render_cpu::Underway`.
/// The read stays on the worker's thread, where every read of a page has been (trap 154); the
/// drawer is a scoped thread beside it, which `Profile::Interpreter` permits.
///
/// **It starts drawing at the first part the read hands over rather than after a wait**, because
/// this process reads no clock it was not handed (ADR 1811): a page under
/// `pdf_model::progress::FIRST_PART` commands hands nothing over and is drawn whole as before, so
/// what an ordinary turn pays is the drawer's spawn.
#[derive(Debug, Default)]
struct Turning {
    /// The viewport the last `Command::Resize` stated, in device pixels, and its scale.
    viewport: Option<((u32, u32), f32)>,
    /// Whether the host said it places a window of a page (`Command::Oversize`'s `Listed`), which
    /// is what admits a window of a page under the bound on its side of the pipe (ADR 2016).
    places_windows: bool,
    /// The page drawn while it was read, with the hand-over that names the list it was drawn from.
    drawn: Option<(pdf_model::progress::Parts, render_cpu::Underway)>,
}

impl Turning {
    /// Takes in what a command says about the window: a resize's viewport, and whether the host
    /// places a window of a page.
    fn told(&mut self, command: &Command) {
        match command {
            Command::Resize {
                width,
                height,
                scale,
            } => self.viewport = Some(((*width, *height), *scale)),
            Command::Oversize(oversize) => {
                self.places_windows = matches!(oversize, viewer_core::Oversize::Listed);
            }
            _ => {}
        }
    }

    /// Whether `request`, a page under the bound that crosses as pixels, is drawn here as the
    /// window shows it: for a host that places a window, behind a viewport, where the toolkit
    /// windows' rule holds ([`protocol::windowed::worth_a_window`], ADR 2016). The question is
    /// asked once for the request, at the viewport it was asked under, as theirs is.
    fn draws_a_window_of(&self, request: &viewer_core::RenderRequest) -> bool {
        self.places_windows
            && self.viewport.is_some_and(|((width, height), _)| {
                width > 0
                    && height > 0
                    && protocol::windowed::worth_a_window(
                        &request.list,
                        request.target,
                        (width, height),
                    )
            })
    }

    /// Handles a page turn with the page it brings on read on this thread and drawn beside it,
    /// answering what [`Viewer::handle`] would have — and keeping the drawing for the request it
    /// can become ([`Self::take_for`]).
    fn turn(
        &mut self,
        viewer: &mut Viewer,
        rasterizer: &CpuRasterizer,
        command: Command,
    ) -> Vec<Event> {
        self.drawn = None;
        let Some((viewport, scale)) = self
            .viewport
            .filter(|&((width, height), _)| width > 0 && height > 0)
        else {
            return viewer.handle(command).collect();
        };
        let mut events: Vec<Event> = viewer.handle_unread(command).collect();
        let read_and_drawn = viewer.anticipating().map(|anticipating| {
            let parts = pdf_model::progress::Parts::new();
            // A page past the bound is not drawn here at all, while it is read or after: its marks
            // cross or it is refused (ADR 1944).
            let target = anticipating
                .foresee(viewport, scale)
                .map(|foreseen| foreseen.target)
                .filter(|target| !past_the_bound(*target));
            let (read, drawn) = std::thread::scope(|scope| {
                let handing = &parts;
                // The pool's own stack: the drawer draws what a strip draws. Unnamed, because
                // naming a thread is a `prctl` the filter kills this process for, which
                // `tests/confined.rs`'s page turn found. A spawn this process is refused is a
                // turn drawn whole, as it was before there was a drawer.
                let drawer = target.and_then(|target| {
                    std::thread::Builder::new()
                        .stack_size(RASTERISING_STACK)
                        .spawn_scoped(scope, move || draw_beside(rasterizer, target, handing))
                        .ok()
                });
                let read = anticipating.reader().read(&parts);
                let drawn = drawer.and_then(|drawer| match drawer.join() {
                    Ok(drawn) => drawn,
                    Err(panic) => std::panic::resume_unwind(panic),
                });
                (read, drawn)
            });
            (read, parts, drawn)
        });
        match read_and_drawn {
            Some((read, parts, drawn)) => {
                events.extend(viewer.anticipated(read, Some(&parts)));
                self.drawn = drawn.map(|underway| (parts, underway));
            }
            None => events.extend(viewer.anticipate()),
        }
        events.extend(viewer.read_the_rest());
        events
    }

    /// The page drawn while it was read, for the request that asks for its list at the target it
    /// was drawn at — and nothing for any other, after which a request for that list at another
    /// target has made the drawing a picture of nothing anybody will ask for.
    fn take_for(&mut self, request: &viewer_core::RenderRequest) -> Option<render_cpu::Underway> {
        let (parts, underway) = self.drawn.as_ref()?;
        if !parts.is(&request.list) {
            return None;
        }
        let matches = underway.target() == request.target;
        let (_, underway) = self.drawn.take()?;
        matches.then_some(underway)
    }
}

/// The drawer's side of [`Turning::turn`]: every part the read hands over, drawn into one target as
/// it arrives — `None` where nothing was handed over, or what was drawn cannot be the page.
///
/// It ends when the read's hand-over does: `Parts::during` concludes it however the read ends.
fn draw_beside(
    rasterizer: &CpuRasterizer,
    target: pdf_render::TargetSpec,
    parts: &pdf_model::progress::Parts,
) -> Option<render_cpu::Underway> {
    use pdf_model::progress::Handed;

    // How long a take waits before it asks again; the hand-over wakes it the moment a part or the
    // end arrives, so this bounds nothing but a wake that was missed.
    const POLL: std::time::Duration = std::time::Duration::from_millis(5);
    let mut underway = None;
    loop {
        match parts.take(POLL) {
            Some(Handed::Part(part)) => {
                let drawing = match underway.as_mut() {
                    Some(drawing) => drawing,
                    None => underway.insert(render_cpu::Underway::new(rasterizer, target)?),
                };
                if !drawing.take(part) {
                    return None;
                }
            }
            Some(Handed::Ended(Ok(_))) => return underway,
            Some(Handed::Ended(Err(_))) => return None,
            None => {}
        }
    }
}

/// What a document reads after page one, read on a thread of this process's own (ADRs 1543, 1553).
///
/// **The worker is the host here**, so it is the one that hands the core a thread: `viewer-core`'s
/// rule 4 binds the core, which still spawns nothing and waits for nothing. The thread is started
/// after an open's anticipation, inside the confinement, so it inherits the Landlock domain and the
/// seccomp filter as the rasterising pool does; it holds a handle to the immutable file and none of
/// the viewer's state. Nothing crosses to the host unasked — every frame is an answer — so what
/// [`Viewer::prepared`] says is owed to the next events frame, where it is the caption's section.
#[derive(Debug, Default)]
struct Preparing {
    /// The thread's answer, until it has been taken.
    answer: Option<std::sync::mpsc::Receiver<viewer_core::Prepared>>,
    /// What taking it raised, waiting for the next events frame.
    owed: Vec<Event>,
}

impl Preparing {
    /// Starts the thread for the document now in front, where it has anything left to read.
    ///
    /// An earlier document's answer still out is dropped with its receiver: the viewer would drop
    /// it anyway, because a `Prepared` names its file.
    fn start(&mut self, viewer: &Viewer) {
        self.answer = viewer.preparation().map(|preparation| {
            let (send, answer) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // A worker whose loop has ended has nobody to give the answer to.
                let _ = send.send(preparation.run());
            });
            answer
        });
    }

    /// Takes the answer if it has arrived.
    fn look(&mut self, viewer: &mut Viewer) {
        let Some(answer) = self.answer.as_ref() else {
            return;
        };
        match answer.try_recv() {
            Ok(prepared) => {
                self.answer = None;
                self.owed.extend(viewer.prepared(prepared));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            // The thread ended without an answer; the first use reads it instead.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.answer = None,
        }
    }

    /// Waits for the answer, for a question whose answer is what the thread is reading.
    fn wait(&mut self, viewer: &mut Viewer) {
        if let Some(answer) = self.answer.take()
            && let Ok(prepared) = answer.recv()
        {
            self.owed.extend(viewer.prepared(prepared));
        }
    }
}

/// Where the viewer places one page in the viewport, or nothing where it places it nowhere.
///
/// `Answer::None` here is Table 29's arrangement not showing the page, or showing it before
/// anything has interpreted it — and both mean the same thing to a store of encoded marks: there
/// is no frame to describe.
fn placement(viewer: &Viewer, page: usize) -> Option<(f32, f32)> {
    match viewer.query(viewer_core::Query::PageGeometry(page)) {
        viewer_core::Answer::Geometry(geometry) => Some(geometry.origin),
        _ => None,
    }
}

/// How many pixels a target holds.
fn pixels(target: pdf_render::TargetSpec) -> u64 {
    u64::from(target.width).saturating_mul(u64::from(target.height))
}

/// Whether a target's whole raster would be past [`viewer_core::MAX_PIXELS`]: a page the viewer
/// asks this process for only because the window draws it from its marks (ADR 1944).
fn past_the_bound(target: pdf_render::TargetSpec) -> bool {
    pixels(target) > viewer_core::MAX_PIXELS
}

/// A refusal frame carrying one sentence.
fn refuse(detail: &str) -> (u8, Vec<u8>) {
    (protocol::FRAME_REFUSAL, detail.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use confined_transport::ceiling::SETTLING_ALLOWANCE;

    use super::{COPIES_OF_A_MESSAGE, Incoming, RASTERISING_STACK, message_budget, read_frame};
    use crate::protocol;

    /// A ceiling of four gibibytes and a worker eighty mebibytes into it.
    ///
    /// The figures this machine actually reports: `INTERPRETER_ADDRESS_SPACE_LIMIT` is `4 << 30`,
    /// and `/proc/self/status` said `VmSize: 82040 kB` for a `pdf-view-worker` about to confine
    /// itself (ADR 0597).
    const CEILING: u64 = 4 << 30;
    const ALREADY: u64 = 82_040 * 1024;

    /// The budget is the ceiling less what is spent, a page and the pool's stacks, halved.
    ///
    /// Written as the arithmetic rather than as a number, because a number here would be a second
    /// copy of the derivation and the two would drift. What the test pins is that all five terms
    /// take part: change any one of them and this fails. Twenty-four threads is this machine's
    /// pool (ADR 1554).
    #[test]
    fn a_message_budget_leaves_room_for_two_copies_a_page_and_the_pools_stacks() {
        let raster = viewer_core::MAX_PIXELS * 4;
        let stacks = 24 * RASTERISING_STACK as u64;
        assert_eq!(
            message_budget(CEILING, ALREADY, 24),
            (CEILING - ALREADY - SETTLING_ALLOWANCE - raster - stacks) / COPIES_OF_A_MESSAGE
        );
        assert!(message_budget(CEILING, ALREADY, 24) < message_budget(CEILING, ALREADY, 1));
        assert!(message_budget(CEILING, ALREADY, 24) < confined_transport::frame::MAX_MESSAGE);
    }

    /// No ceiling is no budget, which is what every platform `doc/todo/35` covers gets.
    #[test]
    fn a_worker_with_no_ceiling_has_no_message_budget() {
        assert_eq!(message_budget(0, ALREADY, 24), u64::MAX);
    }

    /// A ceiling large enough stops at what the protocol carries anyway.
    #[test]
    fn a_generous_ceiling_stops_at_what_the_format_carries() {
        assert_eq!(
            message_budget(u64::MAX, 0, 24),
            confined_transport::frame::MAX_MESSAGE
        );
    }

    /// A ceiling that cannot hold a page's pixels admits no message at all.
    ///
    /// Saturating rather than wrapping, and it matters: the arithmetic subtracts two quantities
    /// from the ceiling and a decoder profile's gibibyte is smaller than one of them.
    #[test]
    fn a_ceiling_below_a_pages_pixels_admits_nothing() {
        assert_eq!(message_budget(1 << 30, ALREADY, 1), 0);
    }

    /// **The discriminating one**: a frame over the budget is read past, not left in the pipe.
    ///
    /// Two frames back to back and a budget that admits only the second. A reader that refused the
    /// first without consuming it would find the next header inside the first payload — which is
    /// the failure a refusal is meant to prevent, and it is invisible to a test that sends one
    /// frame.
    #[test]
    fn a_frame_over_the_budget_is_read_past_rather_than_allocated() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&protocol::header(protocol::FRAME_COMMAND, 64));
        wire.extend_from_slice(&[7u8; 64]);
        wire.extend_from_slice(&protocol::header(protocol::FRAME_QUERY, 3));
        wire.extend_from_slice(b"abc");

        let mut input = wire.as_slice();
        assert!(matches!(
            read_frame(&mut input, 8).expect("a frame is read"),
            Some(Incoming::NoRoom { length: 64 })
        ));
        match read_frame(&mut input, 8).expect("a frame is read") {
            Some(Incoming::Frame {
                kind,
                payload,
                descriptors,
            }) => {
                assert_eq!(kind, protocol::FRAME_QUERY);
                assert_eq!(payload, b"abc");
                assert!(descriptors.is_empty());
            }
            other => panic!("the second frame was {other:?}"),
        }
        assert!(read_frame(&mut input, 8).expect("end of input").is_none());
    }

    /// A frame inside the budget is read whole, which is the control for the test above.
    #[test]
    fn a_frame_inside_the_budget_is_read_whole() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&protocol::header(protocol::FRAME_COMMAND, 4));
        wire.extend_from_slice(b"1234");
        let mut input = wire.as_slice();
        match read_frame(&mut input, 4).expect("a frame is read") {
            Some(Incoming::Frame {
                kind,
                payload,
                descriptors,
            }) => {
                assert_eq!(kind, protocol::FRAME_COMMAND);
                assert_eq!(payload, b"1234");
                assert!(descriptors.is_empty());
            }
            other => panic!("the frame was {other:?}"),
        }
    }

    /// A document of two pages, the second of many marks — more than a read hands over in one
    /// part — so that a turn to it is drawn while it is read.
    fn a_turn_to_many_marks() -> Vec<u8> {
        use std::fmt::Write as _;

        let mut marks = String::new();
        for y in (0..120_u32).step_by(2) {
            for x in (0..200_u32).step_by(2) {
                // Writing to a `String` cannot fail.
                let _ = writeln!(marks, "{x} {y} 0.5 0.5 re f");
            }
        }
        let page_two = format!("<< /Length {} >>\nstream\n{marks}endstream", marks.len());
        let objects: [&[u8]; 5] = [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 5 0 R >>",
            page_two.as_bytes(),
        ];
        let mut out = b"%PDF-2.0\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        out
    }

    /// A turn's page drawn while it is read is the page drawn whole, byte for byte, and the
    /// drawing is held for the one request it can become (ADR 1871).
    #[test]
    fn a_turn_drawn_while_it_is_read_is_the_page_drawn_whole() {
        use pdf_render::Rasterizer as _;
        use viewer_core::{Command, DocumentId, Event, PageTarget, Viewer};

        let mut viewer = Viewer::new(0, 0, 1.0);
        let mut turning = super::Turning::default();
        for command in [
            Command::Resize {
                width: 300,
                height: 300,
                scale: 1.0,
            },
            Command::Open {
                id: DocumentId(1),
                bytes: a_turn_to_many_marks().into(),
                password: None,
                fragment: None,
            },
        ] {
            turning.told(&command);
            viewer.handle(command).for_each(drop);
        }
        let mut rasterizer = render_cpu::CpuRasterizer::new();
        let events = turning.turn(&mut viewer, &rasterizer, Command::GoTo(PageTarget::Next));
        let requests: Vec<_> = events
            .into_iter()
            .filter_map(|event| match event {
                Event::NeedsRender(request) => Some(request),
                _ => None,
            })
            .collect();
        let [request] = requests.as_slice() else {
            panic!(
                "the turn asks for page two once, and asked {}",
                requests.len()
            );
        };
        assert_eq!(request.page, 1);
        let underway = turning
            .take_for(request)
            .expect("page two was drawn while it was read");
        assert!(turning.take_for(request).is_none(), "and is taken once");
        let finished = underway
            .finish(&request.list)
            .expect("it becomes the page")
            .raster;
        let whole = rasterizer
            .rasterize(&request.list, request.target)
            .expect("the page draws");
        assert_eq!(
            (finished.width, finished.height),
            (whole.width, whole.height)
        );
        assert!(finished.data == whole.data, "the same bytes");
    }
}
