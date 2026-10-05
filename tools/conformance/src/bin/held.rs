//! Prints the oracle's pages held by name — per verdict the count, the groups by size, and the
//! candidates for the next page to take — read from `crates/pdf-model/tests/oracle.rs`'s own
//! constants without running the walk.
//!
//! ```sh
//! cargo run -q -p conformance --bin held
//! ```
//!
//! The walk's verdicts are the gate's (`tools/state.sh oracle`); this is the standing fact a
//! robustness round reads first, and [`conformance::held`] says what it can and cannot see. It
//! writes nothing, and exits non-zero only where the file cannot be read or holds no group the
//! parse recognises. ADR 1512.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match conformance::held::read(&conformance::workspace_root()) {
        Ok(held) => {
            print!("{}", conformance::held::report(&held));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("held: {error}");
            ExitCode::FAILURE
        }
    }
}
