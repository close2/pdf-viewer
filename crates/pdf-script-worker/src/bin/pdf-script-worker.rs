//! The confined script worker.
//!
//! Not meant to be run by hand: it is started by `pdf_script_worker::ScriptWorker` at a document's
//! first trigger, confines itself before it reads anything, and answers runs over its standard
//! input and output. Run without a parent it waits for a run that never arrives.
//!
//! A program of its own rather than a mode of the viewer, so that the engine cannot be reached in
//! the viewer's process by accident: everything it links is reachable only from a `main` whose
//! first statement gives away every ability but computing.

#![deny(unsafe_code)]

fn main() -> std::process::ExitCode {
    if let Err(error) = keep_only_the_standard_descriptors() {
        // Before the confinement, which would otherwise hold whatever was leaked to it.
        eprintln!(
            "{}: cannot close the descriptors it inherited: {error}",
            pdf_script_worker::WORKER_PROGRAM
        );
        return std::process::ExitCode::FAILURE;
    }
    match pdf_script_worker::serve() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Standard error is a pipe the host reads, and a worker that could not confine
            // itself, or whose pipe failed, says so there.
            eprintln!("{}: {error}", pdf_script_worker::WORKER_PROGRAM);
            std::process::ExitCode::FAILURE
        }
    }
}

/// Closes every descriptor this process inherited past standard input, output and error (ADR 1945).
///
/// **The worker closes them because only the worker can.** A library that spawns in the parent
/// beside this worker's start — `GLib`'s `dbus-launch`, whose pipes are made without close-on-exec —
/// hands every such descriptor to a child started meanwhile, and a write end held here is an end
/// of file the library waits for and never gets (ADR 1907). The parent passes exactly the three,
/// so whatever else is open at `main` is somebody else's leak, and it is closed before a byte of
/// input is read. The defect itself is the library's, and the owner wants it fixed there too
/// (`doc/questions/A430`).
#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "closes by number the descriptors no object in this process owns (ADR 1945)"
)]
fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
    // SAFETY: `main`'s first statement, before anything in this process has opened a descriptor
    // or made an object that owns one past the standard three, so no owner is left to use or close
    // what this closes; and `close_range` reads and writes none of this process's memory.
    let closed = unsafe { libc::close_range(3, u32::MAX, 0) };
    if closed == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Nothing to close on the two platforms this worker is not yet confined on (`doc/todo/35`):
/// `close_range` is Linux's, and each of those owes its own answer with its confinement.
#[cfg(not(target_os = "linux"))]
fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
    Ok(())
}
