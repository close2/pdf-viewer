//! A message that names a command-line flag names one the program accepts.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! Five refusal messages told a user to supply a missing font with `--font` while the program
//! refused that flag as a usage error. The two halves are written in different crates — the
//! accepted set in the binary's argument parser, the message in a library's error text — so
//! nothing brought them together until [`conformance::flags`] did.
//!
//! # Why it may fail rather than ratchet
//!
//! The population is **zero** and a bound is not needed to say so: a message naming a flag the
//! program refuses is a defect whatever else is true, and there is no version of this tree in
//! which one is acceptable. What a failure prints is the file, the line, the flag and the message,
//! which is everything a round needs to fix it or to accept the flag.
//!
//! # And the calibration, which is not optional here
//!
//! A sweep that comes back clean is a sentence about the sweep (trap 13). The second test plants
//! the defect — a message naming `--frobnicate`, which no program of this tree accepts — into the
//! sweep's own machinery and fails unless the sweep names it. It plants into the functions rather
//! than into a crate's file, because a plant written into the tree is a plant somebody has to
//! remember to remove.
//!
//! ADR 1213.

#![expect(
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators and its findings, so a failure and a \
              clean run are read the same way"
)]

use conformance::flags;

#[test]
fn every_flag_a_message_names_is_one_the_program_accepts() {
    let root = conformance::workspace_root();
    let found = flags::sweep(&root).expect("the tree's binaries and their flags");
    print!("{}", flags::report(&found));
    assert!(
        !found.binaries.is_empty() && found.sources_read > 0 && found.documents_read > 0,
        "the sweep read nothing: {} binary(ies), {} source(s), {} document(s) — a clean answer \
         over an empty population is a sentence about the sweep",
        found.binaries.len(),
        found.sources_read,
        found.documents_read,
    );
    let named: Vec<String> = found
        .unaccepted
        .iter()
        .map(|mention| {
            format!(
                "{}:{} names {} to {} — {}",
                mention.path.display(),
                mention.line,
                mention.flag,
                mention.binaries.join(", "),
                mention.text,
            )
        })
        .collect();
    assert!(
        named.is_empty(),
        "a message names a flag its program does not accept, which spends the reader's next \
         attempt and reads exactly like a message that works. Either the program accepts the \
         flag or the message names one it does:\n  {}",
        named.join("\n  ")
    );
}

#[test]
fn the_sweep_names_a_planted_flag_no_program_accepts() {
    let root = conformance::workspace_root();
    let plant = flags::calibrate(&root).expect("the tree's binaries and their flags");
    assert_eq!(
        plant.len(),
        2,
        "the plant is one message and one document line, and the sweep named {plant:?}"
    );
    for mention in &plant {
        assert_eq!(mention.flag, "--frobnicate", "{mention:?}");
    }
}
