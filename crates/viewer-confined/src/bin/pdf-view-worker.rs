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

fn main() -> std::process::ExitCode {
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
