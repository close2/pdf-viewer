//! The documents this project writes about itself, and which of them the `§` rule is checked in.
//!
//! # The gap this closes
//!
//! `CLAUDE.md` asks that every clause citation be checkable, and `doc/pdf-a-mitigations.md`
//! states the rule the check rests on in one sentence: a `§` is a clause of ISO 32000-2 and
//! nothing else, so another standard's section is written out in words. [`crate::scan_tree`]
//! holds that rule over every Rust source and `tests/conformance.rs` holds it over the ledger's
//! notes — **and until the one-thousand-and-ninety-sixth session nothing held it over the prose
//! those two are written beside.** The merge of sessions 1086–1091 found sixty-nine sites of
//! `ISO 19005-2 §6.3.3`-shaped text in `doc/*.md` alone, every one of them a section number
//! checked against the wrong standard's clause list, and most of them landing on a clause ISO
//! 32000-2 has.
//!
//! It is trap 25 with the population on the instrument's side: a checker whose denominator is
//! "the Rust sources" reports cleanly over the part of the project it was told about.
//!
//! # Which documents, and why not the others
//!
//! The population is the project's **current instructions** — what a round reads to do its work,
//! and therefore what a wrong reference misleads a round with:
//!
//! - `doc/*.md`, the documents at the head of the tree;
//! - `doc/todo/`, the work list;
//! - `doc/habits/`, the habits;
//! - `doc/questions/Q*.md`, the open questions this project wrote.
//!
//! Four directories are deliberately out, and the reason is not cost:
//!
//! - **`doc/adr/`, `doc/history/` and `doc/reviews/` are records.** `CLAUDE.md` ("Where
//!   knowledge lives") says they keep their chronology and that rewriting one for tidiness is
//!   the single thing a record may not have done to it. A gate whose only remedy is an edit
//!   cannot be pointed at a file that may not be edited.
//! - **`doc/questions/A*.md` are the owner's answers**, written by the project owner in the
//!   owner's own words. They are evidence of what was decided, and correcting somebody else's
//!   sentence would change what the record says they wrote.
//!
//! What is *not* excluded on principle, and simply is not here yet, is `doc/rfc/` and
//! `doc/traps/`: both are current, both would belong, and neither was in this round's contract.
//! Adding a directory to [`INSTRUCTIONS`] is the whole change.
//!
//! # The half this does not gate, measured rather than assumed
//!
//! The Rust sources are held to a second rule as well — every `§` names a clause the standard
//! *has* — and the instruction documents are not, yet. Run over this population it reports **620
//! of 7 134** clause citations naming no clause of ISO 32000-2, across 44 documents: `doc/todo/58`
//! citing RFC 0003's section 5.2 as `§5.2`, `doc/todo/03` citing its own section 30,
//! `doc/verify.md` citing RFC 5280's 4.1.1.2. Every one is somebody's section written without its
//! document's name in front of the sign, and the 620 are only the visible ones: a section number
//! that *is* also a clause of ISO 32000-2 — `§5.2` is — resolves in silence and is counted as a
//! citation of a clause nobody made. Naming the document is the whole fix, and it is 620 edits
//! across documents this round did not own; the measurement is here so that the round that does
//! own them knows the size before it starts.

use std::path::{Path, PathBuf};

use crate::citation::{self, Scan};

/// Where this project's current instructions are, relative to the tree's root.
///
/// A directory, and the prefix a file's name must carry within it — `""` where every Markdown
/// file in it counts. The walk is one level deep: every one of these directories is flat, and a
/// deeper walk would reach `doc/md/`'s copy of the standard and `doc/corpora`'s submodule.
pub const INSTRUCTIONS: [(&str, &str); 4] = [
    ("doc", ""),
    ("doc/todo", ""),
    ("doc/habits", ""),
    ("doc/questions", "Q"),
];

/// Every instruction document under `root`, sorted, with paths relative to `root`.
///
/// # Errors
///
/// If a directory cannot be read. A checker that skipped what it could not open would report a
/// clean tree for a tree it had not looked at.
pub fn instructions(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for (directory, prefix) in INSTRUCTIONS {
        let path = root.join(directory);
        if !path.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&path)? {
            let file = entry?.path();
            if !file.is_file() || file.extension().is_none_or(|suffix| suffix != "md") {
                continue;
            }
            let named = file
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix));
            if named {
                found.push(file.strip_prefix(root).unwrap_or(&file).to_path_buf());
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Scans every instruction document's prose for what it says about the standard.
///
/// Only the `§` populations of the returned [`Scan`] are meaningful here. `Table N` references
/// are read by the same scanner and deliberately not checked: these documents name ISO 32000-1's
/// tables, ISO 19005's and ETSI's as freely as this standard's, and a table designation carries
/// no marker saying which — that is a second population and a second reading, not a line in the
/// gate this module exists for.
///
/// # Errors
///
/// If a document cannot be read.
pub fn scan(root: &Path) -> std::io::Result<Vec<(PathBuf, Scan)>> {
    let mut scanned = Vec::new();
    for relative in instructions(root)? {
        let text = std::fs::read_to_string(root.join(&relative))?;
        scanned.push((relative, citation::scan_prose(&text)));
    }
    Ok(scanned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_population_is_the_four_directories_and_reaches_every_one_of_them() {
        let root = crate::workspace_root();
        let found = instructions(&root).expect("the instruction documents");
        for (directory, _) in INSTRUCTIONS {
            assert!(
                found
                    .iter()
                    .any(|path| path.parent().is_some_and(|at| at == Path::new(directory))),
                "no document found under {directory}: the walk is not reaching it"
            );
        }
    }

    /// The owner's answers are the population's own boundary, and an off-by-one in the prefix
    /// test would take them in without anything else noticing.
    #[test]
    fn the_owners_answers_are_not_in_the_population() {
        let root = crate::workspace_root();
        let found = instructions(&root).expect("the instruction documents");
        let answers: Vec<&PathBuf> = found
            .iter()
            .filter(|path| {
                path.parent() == Some(Path::new("doc/questions"))
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with('A'))
            })
            .collect();
        assert!(answers.is_empty(), "{answers:?} are the owner's, not ours");
    }

    /// A record may not be rewritten, so a gate may not be pointed at one.
    #[test]
    fn no_record_is_in_the_population() {
        let root = crate::workspace_root();
        let found = instructions(&root).expect("the instruction documents");
        for record in ["doc/adr", "doc/history", "doc/reviews"] {
            assert!(
                !found.iter().any(|path| path.starts_with(record)),
                "{record} is a record and is in the checked population"
            );
        }
    }
}
