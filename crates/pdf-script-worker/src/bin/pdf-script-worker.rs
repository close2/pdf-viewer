//! The confined script worker.
//!
//! Not meant to be run by hand: it is started by `pdf_script_worker::ScriptWorker` at a document's
//! first trigger, confines itself before it reads anything, and answers runs over its standard
//! input and output. Run without a parent it waits for a run that never arrives.
//!
//! A program of its own rather than a mode of the viewer, so that the engine cannot be reached in
//! the viewer's process by accident: everything it links is reachable only from a `main` whose
//! first statement gives away every ability but computing.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    if let Err(error) = inherited_descriptors::keep_only_the_standard_descriptors() {
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
