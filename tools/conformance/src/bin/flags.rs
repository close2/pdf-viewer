//! Every mention of a command-line flag the program named does not accept.
//!
//! ```sh
//! cargo run -p conformance --bin flags
//! ```
//!
//! One line per binary with the flags its own source names, then one entry per mention of a flag
//! none of the binaries asked accepts, with `file:line` and the message itself.
//! [`conformance::flags`] states the three conditions that make a mention a message and what each
//! cost to work out. `tools/state.sh flags` runs this, and `tests/flags.rs` is the gate.

#![expect(
    clippy::print_stdout,
    reason = "a report whose entire output is the finding"
)]

use conformance::flags;

/// Why the sweep could not be run.
#[derive(Debug, thiserror::Error)]
enum Failure {
    /// The sweep refused.
    #[error(transparent)]
    Sweep(#[from] flags::Error),
}

fn main() -> Result<(), Failure> {
    let root = conformance::workspace_root();
    let found = flags::sweep(&root)?;
    print!("{}", flags::report(&found));
    Ok(())
}
