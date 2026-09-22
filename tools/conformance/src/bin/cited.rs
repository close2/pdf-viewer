//! Sweeps the tree's clause citations against the `code` list of the clause each one names.
//!
//! ```sh
//! cargo run --release -p conformance --bin cited
//! ```
//!
//! `doc/todo/01`'s twenty-seventh sweep. [`conformance::cited`] says what the rank is made of,
//! why it ranks rather than filters, and what is counted rather than listed.
//!
//! It prints the rungs closest first, then the pairs the rows already name, so that a clean run
//! says what it was clean over. It exits non-zero only where it cannot read what it needs.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::cited;
use conformance::ledger::Ledger;

fn main() -> ExitCode {
    let root = conformance::workspace_root();
    let ledger = match Ledger::read(&root.join(conformance::LEDGER)) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("cited: {error}");
            return ExitCode::FAILURE;
        }
    };
    match cited::sweep(&root, &ledger) {
        Ok(found) => {
            print!("{}", cited::report(&found));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cited: {error}");
            ExitCode::FAILURE
        }
    }
}
