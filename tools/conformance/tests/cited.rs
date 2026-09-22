//! The sweep that reads a clause citation against that clause's own `code` list is calibrated.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # Why only the calibration is a gate
//!
//! [`conformance::cited`] ranks rather than filters: it cannot tell a citation that implements a
//! clause from one that points at a neighbour, so a build failing on a hit would teach rounds to
//! drop the citation `CLAUDE.md` principle 5 asks for. What *can* be gated is that the instrument
//! still works — trap 13's rule, that a sweep coming back clean is a sentence about the sweep
//! until the defect has been planted back.
//!
//! The plant is a row losing one of its own citing files, made in memory against the real tree,
//! and the pair is chosen by the sweep rather than written here: a hand-written name rots into a
//! calibration that has quietly stopped calibrating (trap 25).
//!
//! ADR 1274.

#![expect(
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators, so a failure and a clean run are read \
              the same way"
)]

use conformance::cited;
use conformance::ledger::Ledger;

#[test]
fn the_sweep_names_a_row_that_has_lost_one_of_its_own_citing_files() {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER)).expect("the conformance ledger");
    let found = cited::sweep(&root, &ledger).expect("the tree's citations against the ledger");
    assert!(
        found.files_read > 0 && !found.citing.is_empty(),
        "the sweep read nothing: {} file(s), {} pair(s) — a clean answer over an empty population \
         is a sentence about the sweep",
        found.files_read,
        found.citing.len(),
    );
    let subject = cited::calibrate(&root, &ledger)
        .expect("the tree's citations against a planted ledger")
        .expect(
            "no row names a library file that reads its clause three times or more, so the \
             calibration has nothing to pluck and this sweep cannot be trusted",
        );
    println!(
        "calibrated on {}: plucked {:?}, intact {:?}",
        subject.named, subject.gapped, subject.intact
    );
    assert!(
        subject.gapped.is_some(),
        "{} was taken out of its row's `code` and the sweep did not name it",
        subject.named
    );
    assert_eq!(
        subject.intact, None,
        "{} is named by its own row and the sweep reported it anyway",
        subject.named
    );
}
