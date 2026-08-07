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
//! # Two things this does before it confines itself, and why each is safe
//!
//! - **Image decoding is moved in-process.** `pdf-model` decodes JBIG2, JPEG 2000 and CCITT
//!   through `pdf-sandbox`, which *spawns* a worker — and a confined process cannot spawn
//!   anything, by design. So this one sets [`pdf_sandbox::Isolation::InProcess`] and decodes
//!   them here. That is not the in-process fallback `pdf-sandbox` refuses to have: it is the
//!   same code running inside a process that is confined at least as tightly as the decoder's
//!   own worker — seccomp, Landlock and an address-space ceiling. What it costs is the panic
//!   containment `pdf-sandbox`'s second reason names: a decoder panic here takes this process
//!   rather than one image, so it costs the page instead of the viewer. ADR 0218 argues it.
//! - **Nothing else.** In particular no thread pool is warmed: `rayon` builds its pool on first
//!   use, which is inside a render, which is after the confinement — so every rasterising thread
//!   inherits both the Landlock domain and the seccomp filter. Warming one first would leave
//!   threads outside the domain, which `pdf_sandbox::lockdown_linux`'s header states as the rule
//!   for every caller.

use std::io::{Read, Write as _};

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

/// Confines this process for interpreting and drawing pages, and says how many strips a raster
/// may be cut into once it is.
///
/// **Four steps in this order, and each is where it is for a reason the next one makes true.**
///
/// 1. **Image decoding moves in-process**, because after step 3 nothing can be spawned.
/// 2. **How many processors this machine has is asked now, and the answer is thrown away.**
///    `std::thread::available_parallelism` reads `/proc/self/cgroup` on Linux, so a confined
///    process asking it is *killed* rather than told no — and this is the one place it can be
///    asked. What is done with the answer is [`RASTERISING_THREADS`]'s subject: today the pool is
///    one thread whatever the machine has, and the call stays because the *number* is what
///    `doc/todo/34` is about and the place to ask for it is here.
/// 3. **The confinement itself.**
/// 4. **`rayon`'s pool, with that number stated**, built *after* the confinement so that its
///    thread inherits both the Landlock domain and the seccomp filter. Stated rather than
///    defaulted because rayon's own default asks the machine, at step 2's syscall.
///
/// # Errors
///
/// Returns an error if the process could not be confined or the thread pool could not be built.
/// A caller that gets one must not go on to interpret anything: it is not confined.
pub fn confine() -> Result<(pdf_sandbox::lockdown::Confinement, u32), std::io::Error> {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);

    let machine = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let threads = usize::try_from(RASTERISING_THREADS)
        .unwrap_or(1)
        .min(machine);

    let confinement = pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
        .map_err(std::io::Error::other)?;

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .map_err(std::io::Error::other)?;

    Ok((confinement, u32::try_from(threads).unwrap_or(1)))
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
    let (confinement, strips) = confine()?;

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    output.write_all(&protocol::encode_handshake(confinement))?;
    output.flush()?;

    let mut viewer = Viewer::new(INITIAL_VIEWPORT.0, INITIAL_VIEWPORT.1, 1.0);
    let mut rasterizer = CpuRasterizer::new().with_strips(strips);

    while let Some((kind, payload)) = read_frame(&mut input)? {
        let response = answer(&mut viewer, &mut rasterizer, kind, &payload);
        output.write_all(&response)?;
        output.flush()?;
    }
    Ok(())
}

/// Reads one frame, or `None` at end of input.
fn read_frame(input: &mut impl Read) -> Result<Option<(u8, Vec<u8>)>, std::io::Error> {
    let mut header = [0u8; protocol::FRAME_HEADER_LEN];
    match input.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let Some((kind, length)) = protocol::parse_frame_header(header) else {
        return Err(std::io::Error::other(
            "a frame whose kind or length this build does not define",
        ));
    };
    let mut payload = vec![0u8; length];
    input.read_exact(&mut payload)?;
    Ok(Some((kind, payload)))
}

/// Answers one frame.
///
/// A refusal is a *response* and not an error, for the reason `pdf_sandbox::worker` gives: a host
/// that asked for something this does not carry keeps its worker, and only a broken pipe ends
/// one.
fn answer(
    viewer: &mut Viewer,
    rasterizer: &mut CpuRasterizer,
    kind: u8,
    payload: &[u8],
) -> Vec<u8> {
    match kind {
        protocol::FRAME_COMMAND => match protocol::decode_command(payload) {
            Ok(command) => perform(viewer, rasterizer, command),
            Err(error) => refuse(&error.to_string()),
        },
        protocol::FRAME_QUERY => match protocol::decode_query(payload) {
            Ok(query) => match protocol::encode_answer(&viewer.query(query.as_query())) {
                Ok(encoded) => protocol::frame(protocol::FRAME_ANSWER, &encoded),
                Err(uncarried) => refuse(&uncarried.to_string()),
            },
            Err(error) => refuse(&error.to_string()),
        },
        // A frame a host has no business sending: the three the *worker* produces.
        _ => refuse("that frame is one this side sends, not one it reads"),
    }
}

/// Performs one command, drawing whatever it asked for.
fn perform(viewer: &mut Viewer, rasterizer: &mut CpuRasterizer, command: Command) -> Vec<u8> {
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
                    let rendered = match rasterizer.rasterize(&request.list, request.target) {
                        Ok(raster) => Rendered::Raster(raster),
                        // Named rather than swallowed: a viewer that silently showed the previous
                        // page when a render failed would be telling a person something false.
                        Err(error) => Rendered::Failed(error.to_string()),
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

    match protocol::encode_events(&outgoing) {
        Ok(encoded) => protocol::frame(protocol::FRAME_EVENTS, &encoded),
        Err(uncarried) => refuse(&uncarried.to_string()),
    }
}

/// A refusal frame carrying one sentence.
fn refuse(detail: &str) -> Vec<u8> {
    protocol::frame(protocol::FRAME_REFUSAL, detail.as_bytes())
}
