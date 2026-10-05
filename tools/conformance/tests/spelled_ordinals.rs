//! No Rust source under `crates` or `tools` spells a round by ordinal.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `CLAUDE.md`'s comment rule ("Where knowledge lives") says a comment carries the current reason
//! and the ADR that argued it, never the round that changed it (ADR 1023), and it names a grep with
//! two halves that measures what is still owed. Its second half is a reading list, because a viewer's,
//! a FUSE mount's and a desktop bus's own nouns share the word; `tools/comment-history.py` sorts it
//! by shape. Its first half is not a reading list. This tree spells a round's ordinal in words, the
//! hundreds joined to the rest by `-and-`, and no identifier, no clause and no noun of the standard
//! contains that string, so every line carrying it is a debt.
//!
//! So the count is held at a ceiling, and the ceiling is zero. A string literal counts as much as a
//! comment does — a sentence a tool prints, or a fixture modelling a ledger note, reaches a reader
//! the same way. The ledger's own notes are not this gate's: they are TOML, and a separate debt.
//!
//! This file is the one source the walk leaves out, so that its own wording can never be the hit.
//!
//! # What it prints
//!
//! The count, and beside it the capitalised digit form (`Session 944`), which `CLAUDE.md`'s grep
//! cannot see because it is case-sensitive. That second figure is printed and not held: it is the
//! next debt, named so that a round choosing work can find it.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// The most lines under `crates` and `tools` that may carry the spelled ordinal, held with `==`
/// as `CONDITION_UNQUOTED_CEILING` is (ADR 1535), so that a fall is written down rather than banked.
const CEILING: usize = 0;

/// The string `CLAUDE.md`'s sweep searches for, assembled so that this file's own text does not
/// carry it even outside the excluded path.
fn spelled_ordinal() -> String {
    ["hundred", "-and-"].concat()
}

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// Every `.rs` file under `directory`, build output and hidden directories left out.
fn rust_sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory).expect("a directory of the tree is readable");
    for entry in entries {
        let path = entry.expect("a directory entry is readable").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if path.is_dir() {
            rust_sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// Whether `line` names a round by a capitalised number, `Session 944`.
fn names_a_round_in_digits(line: &str) -> bool {
    line.match_indices("Session ").any(|(index, word)| {
        line.get(index.saturating_add(word.len())..)
            .and_then(|after| after.chars().next())
            .is_some_and(|first| first.is_ascii_digit())
    })
}

#[test]
fn no_source_spells_a_round_by_ordinal() {
    let root = repository_root();
    let own = Path::new(file!())
        .file_name()
        .expect("this test's own path names a file");
    let mut sources = Vec::new();
    for directory in ["crates", "tools"] {
        rust_sources(&root.join(directory), &mut sources);
    }
    sources.sort();
    let needle = spelled_ordinal();
    let mut spelled = Vec::new();
    let mut numbered = 0usize;
    for path in &sources {
        let is_this_file = path.file_name() == Some(own)
            && path.parent().and_then(Path::file_name) == Some("tests".as_ref())
            && path.starts_with(root.join("tools").join("conformance"));
        if is_this_file {
            continue;
        }
        let text = fs::read_to_string(path).expect("a source file of the tree is readable");
        for (number, line) in text.lines().enumerate() {
            if line.contains(&needle) {
                let relative = path.strip_prefix(root).unwrap_or(path);
                spelled.push(format!(
                    "{}:{}: {}",
                    relative.display(),
                    number + 1,
                    line.trim()
                ));
            }
            if names_a_round_in_digits(line) {
                numbered += 1;
            }
        }
    }
    println!(
        "{} line(s) spell a round by ordinal (ceiling {CEILING}); {numbered} name one as \
         `Session <number>`, printed and not held",
        spelled.len()
    );
    assert_eq!(
        spelled.len(),
        CEILING,
        "a comment carries the current reason and the ADR that argued it, never the round that \
         changed it (ADR 1023); rewrite these as what is:\n{}",
        spelled.join("\n")
    );
}

#[test]
fn a_round_in_digits_is_told_from_the_noun() {
    assert!(names_a_round_in_digits("Session 944 measured it"));
    assert!(!names_a_round_in_digits("the Session bus answers"));
}
