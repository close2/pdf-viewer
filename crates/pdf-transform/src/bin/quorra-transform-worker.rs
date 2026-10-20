//! The confined half of `quorra-transform`: the whole program, with no filesystem and no network
//! (ADR 2140, `doc/questions/A02`).
//!
//! The whole of the worker's life: confine itself, say so, take the arguments its broker hands it,
//! and run the program `quorra-transform` runs — the same file, taken by path — over a world that
//! is a frame to the broker for every file, document, descriptor, output and line of text. It opens
//! nothing, connects to nothing, and starts nothing.
//!
//! # Order matters here more than anywhere else in the suite
//!
//! Lockdown comes first, before a single argument is read. A worker that took its arguments and
//! *then* confined itself would already have done everything a subverted parser needs to do
//! nothing about; the arguments are the person's, but the order is the rule (`pdf-vfs`'s worker
//! states it the same way).
//!
//! # The system calls this worker needs beyond the interpreter's, and there are none
//!
//! [`pdf_sandbox::lockdown::Profile::Interpreter`] unchanged, as `pdf-vfs-worker` runs under
//! (ADR 0847): this worker parses a document handed to it as a descriptor, draws pages on a
//! `rayon` pool, decodes the three confined codecs in-process (ADR 0218), and writes every output
//! as frames to its broker — `write` and `recvmsg` on the two descriptors it was started with.
//! `getrandom`, which §7.6.3.3's random bytes of an encrypted output come from, is on that list
//! already. `RLIMIT_FSIZE` is 0, so an output that tried to become a file here would be killed
//! rather than obeyed.

#![forbid(unsafe_code)]

/// The program, taken by path from the binary that runs it in-process: one program, two worlds.
///
/// **Its `main`, its route and its machine world are linked here and never reached**, which is
/// what the lint says: they are `quorra-transform`'s half, and the only entry into this file from
/// below is `program`, called with the forwarded world.
#[path = "quorra-transform.rs"]
#[expect(
    dead_code,
    reason = "the broker's main, route and machine world are the other binary's, unreachable here"
)]
mod program;

/// The stack each rasterising thread is given, stated rather than defaulted, as `pdf-vfs-worker`
/// states it.
#[cfg(unix)]
const RASTERISING_STACK: usize = 2 << 20;

fn main() -> std::process::ExitCode {
    match worker() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Before the greeting this is the only channel there is, and the broker reads the
            // worker's standard error when it explains a worker that never identified itself.
            #[expect(
                clippy::print_stderr,
                reason = "a confined worker's own failure, before or after its broker is gone"
            )]
            {
                eprintln!("{}: {error}", pdf_transform::confined::WORKER_PROGRAM);
            }
            std::process::ExitCode::from(2)
        }
    }
}

/// Confines this process, greets the broker, and runs the program it is handed.
#[cfg(unix)]
fn worker() -> Result<(), std::io::Error> {
    use std::io::Write as _;

    use pdf_transform::confined::{Forwarded, MAGIC, WORKER_PROGRAM};

    let (confinement, most) = confine()?;
    let stdout = std::io::stdout();
    let mut greeting = stdout.lock();
    greeting.write_all(&confined_transport::greeting::encode(MAGIC, confinement))?;
    greeting.flush()?;
    drop(greeting);

    let mut input = confined_transport::link::Link::stdin(WORKER_PROGRAM);
    let given = Forwarded::arguments(&mut input, most)?;
    let mut world = Forwarded::new(most);
    let code = program::program(&mut world, given);
    pdf_transform::confined::exit(code)
}

/// Off Unix this suite has no confined worker, and its broker never starts one (`doc/todo/35`).
#[cfg(not(unix))]
fn worker() -> Result<(), std::io::Error> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

/// Confines this process for running the transform program, and says what that settled and the
/// largest answer it will hold.
///
/// **`pdf-vfs-worker`'s five steps, in its order, for its reasons** (`pdf_vfs::serve::confine`):
/// the codecs move in-process and the machine's fonts are declared unreachable, then given back
/// through the transport's port; the processor count and the address space in use are read while
/// there is a filesystem to read them from; the confinement; and then `rayon`'s pool, so that every
/// thread of it inherits the Landlock domain and the filter.
#[cfg(unix)]
fn confine() -> Result<(pdf_sandbox::lockdown::Confinement, usize), std::io::Error> {
    use pdf_transform::confined::WORKER_PROGRAM;

    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    pdf_font::substitute::no_machine_fonts();
    pdf_font::provider::faces_come_from(confined_transport::link::ask_the_host);
    confined_transport::link::requests_go_to_the_host(WORKER_PROGRAM);

    // **`RAYON_NUM_THREADS` where the caller set it**, because the in-process route's pool is
    // `rayon`'s default and honours it, and a run under the heavy-walk lock is held to four threads
    // through it (ADR 1766): a worker wider than the program it replaced would be a measurement of
    // another program. Otherwise as wide as the machine where the spawner limited the allocator's
    // arenas, and one thread where it did not: `glibc` sizes its arenas from a file a confined
    // process cannot open (ADR 1804).
    let stated = std::env::var("RAYON_NUM_THREADS")
        .ok()
        .and_then(|threads| threads.parse::<usize>().ok())
        .filter(|threads| *threads > 0);
    let arenas_limited = std::env::var("MALLOC_ARENA_MAX")
        .ok()
        .and_then(|limit| limit.parse::<u32>().ok())
        .is_some_and(|limit| limit > 0);
    let threads = match stated {
        Some(threads) if arenas_limited => threads,
        _ if arenas_limited => {
            std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
        }
        _ => 1,
    };
    let already = confined_transport::ceiling::address_space_in_use();

    let confinement = pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
        .map_err(std::io::Error::other)?;

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .stack_size(RASTERISING_STACK)
        .build_global()
        .map_err(std::io::Error::other)?;

    // What one answer may cost: a page's raster beside it, at the default budget's 2^28 pixels in
    // RGBA, and the pool's stacks, which were built after `already` was read; two copies of the
    // answer live at once, the frame's and the reader's (`pdf_vfs::serve::message_budget`).
    let stacks = u64::try_from(threads.saturating_mul(RASTERISING_STACK)).unwrap_or(u64::MAX);
    let reserved = pdf_transform::Budget::default()
        .max_pixels
        .saturating_mul(4)
        .saturating_add(stacks);
    let most = confined_transport::ceiling::message_budget(
        confinement.address_space_limit,
        already,
        reserved,
        std::num::NonZeroU64::MIN.saturating_add(1),
    );
    Ok((confinement, usize::try_from(most).unwrap_or(usize::MAX)))
}
