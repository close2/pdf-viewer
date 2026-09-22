//! Sweeps this tree's doc comments for a Rust path naming an item nobody declares.
//!
//! ```sh
//! cargo run --release -p conformance --bin names
//! ```
//!
//! `doc/todo/01`'s twenty-sixth sweep. [`conformance::names`] says what resolution means here,
//! why it is deliberately loose, and which three shapes are counted rather than reported.
//!
//! It prints the findings first — file, line, the path and the comment line it is written on —
//! then the count on every rung, so that a clean run says what it was clean over. It exits
//! non-zero only where it cannot read what it needs; `cargo test -p conformance --test names` is
//! the gate that fails on a finding.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::names;

fn main() -> ExitCode {
    let root = conformance::workspace_root();
    match names::sweep(&root) {
        Ok(found) => {
            print!("{}", names::report(&found));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("names: {error}");
            ExitCode::FAILURE
        }
    }
}
