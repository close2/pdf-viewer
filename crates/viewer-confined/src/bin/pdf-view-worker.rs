//! The confined viewer.
//!
//! Not meant to be run by hand: it is started by [`viewer_confined::Confined`], speaks a private
//! protocol over its standard input and output, and confines itself before reading either. Run
//! without a parent it will simply wait for a command that never arrives.
//!
//! This program exists as a separate executable rather than as a flag on the viewer so that the
//! interpreting path cannot be reached in-process by accident. Everything it links is reachable
//! only from a `main` whose first statements give away the ability to do anything but interpret
//! and draw a page.

#![deny(unsafe_code)]

fn main() -> std::process::ExitCode {
    if let Err(error) = keep_only_the_standard_descriptors() {
        // Before the confinement, which would otherwise hold whatever was leaked to it.
        eprintln!("pdf-view-worker: cannot close the descriptors it inherited: {error}");
        return std::process::ExitCode::FAILURE;
    }
    match viewer_confined::serve() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Standard error is the one descriptor pointing outside this process, and this is the
            // only thing written to it: a worker that could not confine itself, or whose pipe
            // failed, must say so somewhere an operator will see.
            eprintln!("pdf-view-worker: {error}");
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
