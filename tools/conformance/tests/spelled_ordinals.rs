//! No Rust source under `crates` or `tools` names a round, by ordinal or by number.
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
//! # The second form
//!
//! The capitalised digit form (`Session 944`) is held at zero beside it. `CLAUDE.md`'s grep cannot
//! see it, because the grep is case-sensitive, and a viewer's, an FFI's and a desktop bus's
//! `Session` are nouns of their own — so the form held is the word followed by a digit, which none
//! of those nouns is.
//!
//! # The todo files, held by a ratchet
//!
//! `doc/todo/`'s files carry the same ordinal in their prose, and ADR 1547's argument reaches them
//! as it reaches a ledger note: a todo file states what is owed and why as of now, and
//! `doc/history/` and `doc/adr/` keep the chronology (ADR 1576). Their lines are counted per file,
//! printed, and held on ADR 1576 section 3's path: a figure written here that the count may only
//! fall to, and `==` once the figure is zero, so that a fall is written down rather than banked
//! (ADR 1637). The digit form is held beside it — the word `session` or `round` followed by a
//! number, in either case, since a todo file writes the word in lower case — by the same ratchet
//! at its own figure.

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

/// The most lines under `crates` and `tools` that may name a round as `Session <number>`, held
/// with `==` for the same reason.
const NUMBERED_CEILING: usize = 0;

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
    let mut numbered = Vec::new();
    for path in &sources {
        let is_this_file = path.file_name() == Some(own)
            && path.parent().and_then(Path::file_name) == Some("tests".as_ref())
            && path.starts_with(root.join("tools").join("conformance"));
        if is_this_file {
            continue;
        }
        let text = fs::read_to_string(path).expect("a source file of the tree is readable");
        let relative = path.strip_prefix(root).unwrap_or(path);
        for (number, line) in text.lines().enumerate() {
            let located = || format!("{}:{}: {}", relative.display(), number + 1, line.trim());
            if line.contains(&needle) {
                spelled.push(located());
            }
            if names_a_round_in_digits(line) {
                numbered.push(located());
            }
        }
    }
    println!(
        "{} line(s) spell a round by ordinal (ceiling {CEILING}); {} name one as \
         `Session <number>` (ceiling {NUMBERED_CEILING})",
        spelled.len(),
        numbered.len()
    );
    assert_eq!(
        spelled.len(),
        CEILING,
        "a comment carries the current reason and the ADR that argued it, never the round that \
         changed it (ADR 1023); rewrite these as what is:\n{}",
        spelled.join("\n")
    );
    assert_eq!(
        numbered.len(),
        NUMBERED_CEILING,
        "a comment carries the current reason and the ADR that argued it, never the round that \
         changed it (ADR 1023); rewrite these as what is:\n{}",
        numbered.join("\n")
    );
}

#[test]
fn a_round_in_digits_is_told_from_the_noun() {
    assert!(names_a_round_in_digits("Session 944 measured it"));
    assert!(!names_a_round_in_digits("the Session bus answers"));
}

/// The most lines under `doc/todo` that may spell a round by ordinal: none, so the ratchet's `<=`
/// is `==` (ADR 1637).
const TODO_CEILING: usize = 0;

/// The most lines under `doc/todo` that may name a round as `session <number>` or
/// `round <number>`, held by the same ratchet (ADR 1637). The three are `doc/todo/56`'s: its
/// §12.11.5 row and the paragraph under it, and the commissioning of RFC 0008 section 6.
const TODO_NUMBERED_CEILING: usize = 0;

/// Whether `line` names a round by number in a todo file's spelling: `session` or `round`, either
/// case, singular or plural, at the start of a word and followed by a space and a digit.
fn names_a_round_by_number(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ["session", "round"].iter().any(|word| {
        lower.match_indices(word).any(|(index, found)| {
            let starts_a_word = lower
                .get(..index)
                .and_then(|before| before.chars().next_back())
                .is_none_or(|previous| !previous.is_ascii_alphanumeric());
            let rest = lower
                .get(index.saturating_add(found.len())..)
                .unwrap_or_default();
            let rest = rest.strip_prefix('s').unwrap_or(rest);
            starts_a_word
                && rest
                    .strip_prefix(' ')
                    .and_then(|after| after.chars().next())
                    .is_some_and(|first| first.is_ascii_digit())
        })
    })
}

/// Every `doc/todo/*.md` file's count of lines matching `matches`, largest first, with the total.
fn todo_counts(matches: impl Fn(&str) -> bool) -> (usize, Vec<(usize, String)>) {
    let directory = repository_root().join("doc").join("todo");
    let mut counts: Vec<(usize, String)> = Vec::new();
    for entry in fs::read_dir(&directory).expect("doc/todo is readable") {
        let path = entry.expect("a directory entry is readable").path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a todo file is readable");
        let lines = text.lines().filter(|line| matches(line)).count();
        if lines > 0 {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            counts.push((lines, name));
        }
    }
    counts.sort_by(|a, b| b.cmp(a));
    let total = counts.iter().map(|(lines, _)| lines).sum();
    (total, counts)
}

/// Holds `total` to `ceiling` on ADR 1576 section 3's path: `<=` while the figure is above zero,
/// `==` at zero.
fn hold(what: &str, total: usize, ceiling: usize, counts: &[(usize, String)]) {
    println!("{total} line(s) under doc/todo {what} (ceiling {ceiling}, ADR 1637):");
    for (lines, name) in counts {
        println!("  {lines:>5}  doc/todo/{name}");
    }
    assert!(
        total <= ceiling,
        "{total} line(s) under doc/todo {what}, above the ceiling of {ceiling}: a todo file states \
         what is owed and why as of now, and doc/history/ and doc/adr/ keep the chronology \
         (ADR 1576)"
    );
}

/// The `doc/todo` files' spelled ordinals and their digit form, each held to its figure.
#[test]
fn the_todo_files_name_no_round() {
    let needle = spelled_ordinal();
    let (spelled, counts) = todo_counts(|line| line.contains(&needle));
    hold("spell a round by ordinal", spelled, TODO_CEILING, &counts);
    let (numbered, counts) = todo_counts(names_a_round_by_number);
    hold(
        "name a round as `session <number>` or `round <number>`",
        numbered,
        TODO_NUMBERED_CEILING,
        &counts,
    );
}

#[test]
fn a_round_by_number_is_told_from_other_words() {
    assert!(names_a_round_by_number("measured in session 995"));
    assert!(names_a_round_by_number("Round 899's deadlock"));
    assert!(names_a_round_by_number("rounds 1365 and 1369"));
    assert!(!names_a_round_by_number("around 3 times"));
    assert!(!names_a_round_by_number("rounded 4 values"));
    assert!(!names_a_round_by_number("a session bus"));
}
