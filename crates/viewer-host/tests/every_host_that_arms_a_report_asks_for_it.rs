//! Every host that arms the document's report also asks for it.
//!
//! **A report deferred into never being produced is a silent regression**, and this is the sweep
//! that stops it. `viewer_core::notes::about` left the open path in the one-thousand-and-twenty-
//! seventh session (ADR 1044) because §12.8's answer digests the signed part of the file and
//! `CLAUDE.md` principle 2 keeps such work off a launch. What replaced it is a host asking —
//! `viewer_host::report::Due::opened` when a document opens, `Due::after_a_frame` once the reader
//! has their page — and a host that armed the first and forgot the second would go quiet about
//! every signature in every file, with nothing failing.
//!
//! **The population is derived rather than written down** (trap 25): it is whichever crates of
//! this workspace call the first of those two, read off the tree. A host added later joins the
//! sweep by existing.

#![expect(
    clippy::expect_used,
    reason = "test code: a source tree that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `directory`.
fn sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|kind| kind == "rs") {
            into.push(path);
        }
    }
}

/// The crates whose sources contain `needle`, by directory name.
fn crates_saying(needle: &str) -> BTreeSet<String> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("this crate sits in crates/")
        .to_owned();
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(&crates)
        .expect("the workspace has a crates/ directory")
        .flatten()
    {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        sources(&directory.join("src"), &mut files);
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            if text.contains(needle) {
                found.insert(
                    directory
                        .file_name()
                        .expect("a directory has a name")
                        .to_string_lossy()
                        .into_owned(),
                );
                break;
            }
        }
    }
    found
}

/// Arming the report and asking for it are the same set of crates.
///
/// Not a count and not a list of names: the two sets are compared with each other, so a host
/// added, renamed or removed moves both sides together and only a host that does one without the
/// other fails. Calibrated by deleting one `after_a_frame` call and watching this name the crate
/// it was deleted from (trap 13).
#[test]
fn a_host_that_arms_the_documents_report_also_asks_for_it() {
    let arming = crates_saying("report_due.opened()");
    let asking = crates_saying("report_due.after_a_frame()");
    assert!(
        !arming.is_empty(),
        "some host arms the report, or this sweep is measuring nothing"
    );
    assert_eq!(
        arming, asking,
        "every host that arms the document's report asks for it once the reader has their page \
         — a host in the first set and not the second says nothing about any signature in any \
         file, and nothing else would fail"
    );
}

/// The rule itself: armed by an open, spent by one frame, and not by the next.
#[test]
fn the_report_is_owed_once_per_opened_document() {
    let mut due = viewer_host::report::Due::default();
    assert!(
        !due.after_a_frame(),
        "a host that has opened nothing owes no report"
    );
    due.opened();
    assert!(due.after_a_frame(), "the first frame is when it is asked");
    assert!(
        !due.after_a_frame(),
        "and the second frame is not: the sentences do not change while the file is open"
    );
    // Annex O's `ef` and §12.6.4.4's embedded go-to both put a second document in front of the
    // reader without restarting the program, and what that one says about itself is its own.
    due.opened();
    assert!(due.after_a_frame(), "a second document owes its own report");
}
