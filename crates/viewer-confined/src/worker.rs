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

/// How many threads a confined process rasterises with.
///
/// **One, and it is a finding rather than a preference.** `glibc`'s allocator sizes its arena
/// count from `__get_nprocs`, which reads `/sys/devices/system/cpu/online` — so the first
/// allocation in a thread of a many-threaded confined process is an `openat` the filter kills the
/// process for. Found with `strace -k` on the twenty-fourth rayon worker of a page that had
/// otherwise drawn (ADR 0218). It is `narenas > mp_.arena_test` that decides when the allocator
/// asks, and a bound derived from another library's internal constant is not a bound — so this is
/// one, which cannot reach it, rather than some number below it.
///
/// **It costs speed and not pixels.** ADR 0139's property is that "a machine with four cores and
/// one with thirty-two draw the same bytes", which `tests/confined.rs` re-checks from the other
/// side: what this one thread draws is byte-identical to what the unconfined viewer draws on
/// every core. Making it more than one is `doc/todo/34`'s, with two candidate answers written
/// there.
const RASTERISING_THREADS: u32 = 1;

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
    /// How many strips a page's raster may be cut into. See [`RASTERISING_THREADS`].
    pub strips: u32,
    /// The largest message this process will read, in bytes.
    ///
    /// Derived rather than chosen — see [`message_budget`]. [`u64::MAX`] where there is no
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
/// - [`COPIES_OF_A_MESSAGE`] is how many copies of it live at once at the peak.
fn message_budget(ceiling: u64, already: u64) -> u64 {
    confined_transport::ceiling::message_budget(
        ceiling,
        already,
        viewer_core::MAX_PIXELS.saturating_mul(4),
        COPIES_OF_A_MESSAGE,
    )
}

/// Confines this process for interpreting and drawing pages, and says what that settled.
///
/// **Five steps in this order, and each is where it is for a reason the next one makes true.**
///
/// 1. **Image decoding moves in-process**, because after step 4 nothing can be spawned.
/// 2. **How many processors this machine has is asked now, and the answer is thrown away.**
///    `std::thread::available_parallelism` reads `/proc/self/cgroup` on Linux, so a confined
///    process asking it is *killed* rather than told no — and this is the one place it can be
///    asked. What is done with the answer is [`RASTERISING_THREADS`]'s subject: today the pool is
///    one thread whatever the machine has, and the call stays because the *number* is what
///    `doc/todo/34` is about and the place to ask for it is here.
/// 3. **How much address space this process already occupies is read**, for the same reason and
///    from the same impossibility: `/proc/self/status` is a file, and after step 4 there are none.
/// 4. **The confinement itself.**
/// 5. **`rayon`'s pool, with that number stated**, built *after* the confinement so that its
///    thread inherits both the Landlock domain and the seccomp filter. Stated rather than
///    defaulted because rayon's own default asks the machine, at step 2's syscall.
///
/// # Errors
///
/// Returns an error if the process could not be confined or the thread pool could not be built.
/// A caller that gets one must not go on to interpret anything: it is not confined.
pub fn confine() -> Result<WorkerLimits, std::io::Error> {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);

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
        message_budget: message_budget(confinement.address_space_limit, already),
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
    let mut marks = protocol::Marks::default();

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
            } => answer(
                &mut viewer,
                &mut rasterizer,
                &mut marks,
                kind,
                payload,
                descriptors,
            ),
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
    marks: &mut protocol::Marks,
    kind: u8,
    payload: Vec<u8>,
    mut descriptors: Vec<ReceivedDescriptor>,
) -> (u8, Vec<u8>) {
    match kind {
        protocol::FRAME_COMMAND => {
            let decoded = protocol::decode_command_holding(&payload, &mut descriptors);
            drop(payload);
            drop(descriptors);
            match decoded {
                Ok(command) => perform(viewer, rasterizer, marks, command),
                Err(error) => refuse(&error.to_string()),
            }
        }
        protocol::FRAME_QUERY => {
            let decoded = protocol::decode_query(&payload);
            drop(payload);
            match decoded {
                Ok(query) => {
                    match protocol::encode_answer(&viewer.query(query.as_query()), marks) {
                        Ok(encoded) => (protocol::FRAME_ANSWER, encoded),
                        Err(uncarried) => refuse(&uncarried.to_string()),
                    }
                }
                Err(error) => refuse(&error.to_string()),
            }
        }
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
    marks: &mut protocol::Marks,
    command: Command,
) -> (u8, Vec<u8>) {
    let mut outgoing = Vec::new();
    let mut pending: Vec<Command> = vec![command];

    // A render's answer can itself produce events — a page that finished drawing damages the
    // viewport — so this is a loop rather than one pass. It terminates because `RenderReady` is
    // the only command it adds and a viewer that has been given the frame it asked for does not
    // ask again for the same one.
    while let Some(command) = pending.pop() {
        for event in viewer.handle(command) {
            match event {
                Event::NeedsRender(request) => {
                    let rendered = match marks.decide(&request, placement(viewer, request.page)) {
                        // **The render that does not happen.** The host is being handed this
                        // page's own display list, so pixels of it would be drawn, held and
                        // thrown away.
                        protocol::Carries::Marks => Rendered::Listed,
                        protocol::Carries::Pixels => {
                            match rasterizer.rasterize(&request.list, request.target) {
                                Ok(raster) => Rendered::Raster(raster),
                                // Named rather than swallowed: a viewer that silently showed the
                                // previous page when a render failed would be telling a person
                                // something false.
                                Err(error) => Rendered::Failed(error.to_string()),
                            }
                        }
                    };
                    pending.push(Command::RenderReady {
                        token: request.token,
                        rendered,
                    });
                }
                other => outgoing.push(other),
            }
        }
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

    match protocol::encode_events(&outgoing) {
        Ok(encoded) => (protocol::FRAME_EVENTS, encoded),
        Err(uncarried) => refuse(&uncarried.to_string()),
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

/// A refusal frame carrying one sentence.
fn refuse(detail: &str) -> (u8, Vec<u8>) {
    (protocol::FRAME_REFUSAL, detail.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use confined_transport::ceiling::SETTLING_ALLOWANCE;

    use super::{COPIES_OF_A_MESSAGE, Incoming, message_budget, read_frame};
    use crate::protocol;

    /// A ceiling of four gibibytes and a worker eighty mebibytes into it.
    ///
    /// The figures this machine actually reports: `INTERPRETER_ADDRESS_SPACE_LIMIT` is `4 << 30`,
    /// and `/proc/self/status` said `VmSize: 82040 kB` for a `pdf-view-worker` about to confine
    /// itself (ADR 0597).
    const CEILING: u64 = 4 << 30;
    const ALREADY: u64 = 82_040 * 1024;

    /// The budget is the ceiling less what is spent and less a page, halved.
    ///
    /// Written as the arithmetic rather than as a number, because a number here would be a second
    /// copy of the derivation and the two would drift. What the test pins is that all four terms
    /// take part: change any one of them and this fails.
    #[test]
    fn a_message_budget_leaves_room_for_two_copies_and_a_page() {
        let raster = viewer_core::MAX_PIXELS * 4;
        assert_eq!(
            message_budget(CEILING, ALREADY),
            (CEILING - ALREADY - SETTLING_ALLOWANCE - raster) / COPIES_OF_A_MESSAGE
        );
        assert!(message_budget(CEILING, ALREADY) < confined_transport::frame::MAX_MESSAGE);
    }

    /// No ceiling is no budget, which is what every platform `doc/todo/35` covers gets.
    #[test]
    fn a_worker_with_no_ceiling_has_no_message_budget() {
        assert_eq!(message_budget(0, ALREADY), u64::MAX);
    }

    /// A ceiling large enough stops at what the protocol carries anyway.
    #[test]
    fn a_generous_ceiling_stops_at_what_the_format_carries() {
        assert_eq!(
            message_budget(u64::MAX, 0),
            confined_transport::frame::MAX_MESSAGE
        );
    }

    /// A ceiling that cannot hold a page's pixels admits no message at all.
    ///
    /// Saturating rather than wrapping, and it matters: the arithmetic subtracts two quantities
    /// from the ceiling and a decoder profile's gibibyte is smaller than one of them.
    #[test]
    fn a_ceiling_below_a_pages_pixels_admits_nothing() {
        assert_eq!(message_budget(1 << 30, ALREADY), 0);
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
}
