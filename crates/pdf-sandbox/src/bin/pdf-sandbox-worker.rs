//! The sandboxed image decoder.
//!
//! Not meant to be run by hand: it is started by [`pdf_sandbox::Sandbox`], speaks a private
//! protocol over its standard input and output, and confines itself before reading either.
//! Run without a parent it will simply wait for a request that never arrives.
//!
//! This program exists as a separate executable rather than as a flag on the viewer so that
//! the decoding path cannot be reached in-process by accident. Everything it links is
//! reachable only from a `main` whose first statement gives away the ability to do anything
//! but decode.

fn main() -> std::process::ExitCode {
    match pdf_sandbox::serve() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Standard error is the one descriptor pointing outside this process, and this
            // is the only thing written to it: a worker that could not confine itself, or
            // whose pipe failed, must say so somewhere an operator will see.
            eprintln!("pdf-sandbox-worker: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
