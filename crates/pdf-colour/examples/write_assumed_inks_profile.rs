//! Writes `data/icc/quorra-assumed-inks.icc`, the profile of this renderer's assumed process inks
//! (ADR 2700), to the path given as the only argument.
//!
//! The profile is checked in rather than generated at build time, and
//! `the_checked_in_profile_is_what_the_generator_writes_every_time` fails until this has been run
//! after a change to the ink cube or to the generator:
//!
//! ```text
//! cargo run -p pdf-colour --example write_assumed_inks_profile -- data/icc/quorra-assumed-inks.icc
//! ```

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage: write_assumed_inks_profile <path>");
        return ExitCode::from(2);
    };
    match std::fs::write(&path, pdf_colour::colour::assumed_inks_profile()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: {error}", path.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
