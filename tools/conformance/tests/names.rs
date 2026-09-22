//! A Rust path a doc comment names is a path this tree declares.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `Edit::ChooseFile`'s doc comment named `viewer_host::policy::may_choose_file`, which nobody had
//! written. [`conformance::names`] says why nothing in this project could see that, and what
//! resolution means here.
//!
//! # Why a zero
//!
//! There is no version of this tree in which a doc comment naming an item nobody declares is
//! acceptable: `CLAUDE.md`'s comment rule keeps retired names out of comments, and a name a round
//! is about to add is a name that round is adding. So every finding fails, and the failure prints
//! the file, the line and the sentence, which is all a reader needs to correct it.
//!
//! # And the calibration, which is not optional here
//!
//! A sweep that comes back clean is a sentence about the sweep (trap 13). The second test plants
//! the defect — a path under a module of this crate that nothing declares — into the sweep's own
//! machinery and fails unless the sweep names it. It plants into the functions rather than into a
//! file, because a plant written into the tree is a plant somebody has to remember to remove.
//!
//! ADR 1273.

#![expect(
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators and its findings, so a failure and a \
              clean run are read the same way"
)]

use conformance::names::{self, Reach};

#[test]
fn every_rust_path_a_doc_comment_names_is_one_this_tree_declares() {
    let root = conformance::workspace_root();
    let found = names::sweep(&root).expect("the tree's sources and the names they declare");
    print!("{}", names::report(&found));
    assert!(
        found.index.files_read > 0 && found.comment_lines > 0 && !found.mentions.is_empty(),
        "the sweep read nothing: {} file(s), {} comment line(s), {} path(s) — a clean answer over \
         an empty population is a sentence about the sweep",
        found.index.files_read,
        found.comment_lines,
        found.mentions.len(),
    );
    let absent: Vec<String> = found
        .on(Reach::Absent)
        .iter()
        .map(|mention| {
            format!(
                "{}:{} {}",
                mention.path.display(),
                mention.line,
                mention.named
            )
        })
        .collect();
    assert!(
        absent.is_empty(),
        "a doc comment names an item this workspace does not declare, which reads exactly like a \
         reference that works. Either the item exists under that name or the sentence names one \
         that does:\n  {}",
        absent.join("\n  ")
    );
}

#[test]
fn the_sweep_names_a_planted_path_nothing_declares() {
    let root = conformance::workspace_root();
    let plant = names::calibrate(&root).expect("the tree's sources and the names they declare");
    assert_eq!(
        plant.len(),
        1,
        "the plant is one path in one comment, and the sweep read {plant:?}"
    );
    assert_eq!(
        plant[0],
        (
            "ledger::no_function_of_this_name_is_declared".to_owned(),
            Reach::Absent
        ),
        "the plant names a module this crate declares and a function nothing declares, so the \
         sweep owes it the finding rung"
    );
}
