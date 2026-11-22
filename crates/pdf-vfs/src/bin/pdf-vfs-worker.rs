//! The confined generator behind a document-as-a-directory.
//!
//! Not meant to be run by hand: it is started by `pdf_vfs::ConfinedWorkers`, speaks a private
//! protocol over its standard input and output, and confines itself before reading either. Run
//! without a parent it will simply wait for a document that never arrives.
//!
//! This program exists as a separate executable rather than as a flag on a face so that the
//! reading path cannot be reached in-process by accident. Everything it links is reachable only
//! from a `main` whose first statements close what it inherited and give away the ability to do anything but read a document
//! it was handed and derive files from it.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    if let Err(error) = inherited_descriptors::keep_only_the_standard_descriptors() {
        // Before the confinement, which would otherwise hold whatever was leaked to it: a host
        // that loads the vfs library beside GLib hands its spawn GLib's pipes (ADRs 1907, 3378).
        eprintln!("pdf-vfs-worker: cannot close the descriptors it inherited: {error}");
        return std::process::ExitCode::FAILURE;
    }
    match pdf_vfs::serve() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Standard error is the one descriptor pointing outside this process, and this is the
            // only thing written to it: a worker that could not confine itself, or whose pipe
            // failed, must say so somewhere an operator will see.
            eprintln!("pdf-vfs-worker: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
