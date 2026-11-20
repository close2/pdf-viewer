//! The confined media worker.
//!
//! Not meant to be run by hand: it is started by `pdf_media_worker::MediaWorker` at the first sound
//! played, confines itself before it reads anything, and answers requests over its standard
//! input and output. Run without a parent it waits for a request that never arrives.
//!
//! A program of its own rather than a mode of the viewer, so that a sound's bytes are read only in a
//! process that gave away every ability but computing before it read one (ADR 3297).

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    if let Err(error) = inherited_descriptors::keep_only_the_standard_descriptors() {
        // Before the confinement, which would otherwise hold whatever was leaked to it.
        eprintln!(
            "{}: cannot close the descriptors it inherited: {error}",
            pdf_media_worker::WORKER_PROGRAM
        );
        return std::process::ExitCode::FAILURE;
    }
    match pdf_media_worker::serve() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Standard error is a pipe the host reads, and a worker that could not confine
            // itself, or whose pipe failed, says so there.
            eprintln!("{}: {error}", pdf_media_worker::WORKER_PROGRAM);
            std::process::ExitCode::FAILURE
        }
    }
}
