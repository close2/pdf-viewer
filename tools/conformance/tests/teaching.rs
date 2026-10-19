//! `doc/teaching/how-a-page-lands-on-the-screen.md` names only files, symbols and records that exist.
//!
//! The document is a walk a student follows from a content stream's bytes to a pixel, and every
//! stage of it names the crate, the module and the decision that stands there. A walk that names a
//! file the tree no longer has teaches the wrong tree, and nothing else would notice: the prose is
//! read by people and by no compiler. This gate is the floor under it — it holds *names* true. It
//! cannot hold a sentence true, which is why each diagram in the document ends with a maintenance
//! note saying which files it depends on and the document's last section says what a round that
//! changes one of them owes.
//!
//! What is checked, in the order the checks run:
//!
//! 1. every word that starts with `crates/`, `raster/`, `doc/` or `tools/`, and every word that
//!    contains a `/` and ends in a file extension this tree writes, names a file or a directory;
//! 2. every `path::name` pair names a file that contains `name` as a whole word, which is what
//!    keeps a function the document points at from being renamed away unseen;
//! 3. every `ADR NNNN` names a record in `doc/adr/`, and `raster ADR NNNN` one in `raster/doc/adr/`;
//! 4. every Mermaid diagram is followed, before the next heading, by a maintenance note;
//! 5. `doc/HANDOVER.md` and `doc/crate-map.md` each point at the document.
//!
//! The checks are a function of a text and a root, so that the calibration at the bottom can run
//! them over a text with four planted faults and see all four reported.

use std::fs;
use std::path::{Path, PathBuf};

/// The document under test, relative to the repository root.
const DOCUMENT: &str = "doc/teaching/how-a-page-lands-on-the-screen.md";

/// How many diagrams the document owes: one for the whole walk and one for each of the seven
/// stages. A document with fewer has dropped a stage's diagram or merged two stages.
const DIAGRAMS_OWED: usize = 8;

/// The prefixes that make a word a path from the repository root.
const ROOT_PREFIXES: [&str; 4] = ["crates/", "raster/", "doc/", "tools/"];

/// The extensions that make a word containing a `/` a file the document claims exists.
const FILE_EXTENSIONS: [&str; 4] = [".rs", ".md", ".wgsl", ".toml"];

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. A crate
    // in the workspace always has two ancestors; falling back to the manifest directory itself
    // would only make every check below report a missing file, which is the honest failure.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// The words of `text` that could name a path: runs of the characters a path, a `::` pair or an
/// ADR-free file name is written in, with sentence punctuation trimmed off the ends.
fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '/' | ':' | '-'))
    })
    .map(|word| word.trim_matches(|character: char| matches!(character, '.' | ':' | '-' | ',')))
    .filter(|word| !word.is_empty())
    .collect()
}

/// Whether `word` is a claim about a file or a directory rather than prose.
fn claims_a_path(word: &str) -> bool {
    let path = word.split("::").next().unwrap_or(word);
    ROOT_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
        || (path.contains('/') && FILE_EXTENSIONS.iter().any(|ext| path.ends_with(ext)))
}

/// The place `path` is, trying the repository root and then the two directories whose crates the
/// document names without their parent (`pdf-syntax/src/file.rs` in a diagram label).
fn located(root: &Path, path: &str) -> Option<PathBuf> {
    [
        root.join(path),
        root.join("crates").join(path),
        root.join("raster/crates").join(path),
    ]
    .into_iter()
    .find(|candidate| candidate.exists())
}

/// Whether `name` occurs in `source` as a whole word, so that `fill` is not found inside `fills`.
fn contains_word(source: &str, name: &str) -> bool {
    source.match_indices(name).any(|(at, _)| {
        let before = source[..at].chars().next_back();
        let after = source[at.saturating_add(name.len())..].chars().next();
        let part_of_a_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !part_of_a_word(before) && !part_of_a_word(after)
    })
}

/// Faults in the paths and the `path::name` pairs.
fn path_faults(root: &Path, text: &str) -> Vec<String> {
    let mut faults = Vec::new();
    for word in words(text).into_iter().filter(|word| claims_a_path(word)) {
        let (path, name) = match word.split_once("::") {
            Some((path, name)) => (path, Some(name)),
            None => (word, None),
        };
        let Some(found) = located(root, path) else {
            faults.push(format!("`{path}` names no file or directory"));
            continue;
        };
        let Some(name) = name.filter(|name| !name.is_empty()) else {
            continue;
        };
        match fs::read_to_string(&found) {
            Ok(source) if contains_word(&source, name) => {}
            Ok(_) => faults.push(format!("`{path}` does not contain `{name}`")),
            Err(problem) => faults.push(format!(
                "`{path}` is named with `{name}` but cannot be read as a file: {problem}"
            )),
        }
    }
    faults
}

/// Faults in the ADR numbers: one that names no record.
fn record_faults(root: &Path, text: &str) -> Vec<String> {
    let mut faults = Vec::new();
    for (at, _) in text.match_indices("ADR ") {
        let digits: String = text[at.saturating_add(4)..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if digits.len() != 4 {
            continue;
        }
        let raster = text[..at].ends_with("raster ");
        let directory = if raster { "raster/doc/adr" } else { "doc/adr" };
        let prefix = format!("{digits}-");
        let found = fs::read_dir(root.join(directory)).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        });
        if !found {
            let which = if raster { "raster ADR" } else { "ADR" };
            faults.push(format!("{which} {digits} has no record in {directory}/"));
        }
    }
    faults
}

/// Faults in the diagrams: a count below what the document owes, and a diagram with no note.
fn diagram_faults(text: &str, owed: usize) -> Vec<String> {
    let mut faults = Vec::new();
    let opens: Vec<usize> = text.match_indices("```mermaid").map(|(at, _)| at).collect();
    if opens.len() < owed {
        faults.push(format!(
            "{} diagrams, and the walk owes {owed}: one for the whole and one per stage",
            opens.len()
        ));
    }
    for (number, open) in opens.iter().enumerate() {
        let body = &text[open.saturating_add("```mermaid".len())..];
        let Some(close) = body.find("\n```") else {
            faults.push(format!(
                "diagram {} is never closed",
                number.saturating_add(1)
            ));
            continue;
        };
        let after = &body[close.saturating_add(4)..];
        let bound = [after.find("```mermaid"), after.find("\n## ")]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(after.len());
        if !after[..bound].contains("**Maintenance note.**") {
            faults.push(format!(
                "diagram {} has no **Maintenance note.** before the next diagram or heading",
                number.saturating_add(1)
            ));
        }
    }
    faults
}

/// Everything wrong with `text` as a teaching document under `root`.
fn faults(root: &Path, text: &str, owed: usize) -> Vec<String> {
    let mut all = path_faults(root, text);
    all.extend(record_faults(root, text));
    all.extend(diagram_faults(text, owed));
    all
}

fn read(relative: &str) -> std::io::Result<String> {
    fs::read_to_string(repository_root().join(relative))
}

#[test]
fn every_name_the_walk_makes_exists() {
    let found = faults(
        &repository_root(),
        &read(DOCUMENT).expect("the teaching document's text"),
        DIAGRAMS_OWED,
    );
    assert!(
        found.is_empty(),
        "{DOCUMENT} names what the tree does not have — redraw the stage, then the sentence \
         beside it:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_index_and_the_crate_map_point_at_the_walk() {
    for pointer in ["doc/HANDOVER.md", "doc/crate-map.md"] {
        assert!(
            read(pointer)
                .expect("the pointing document's text")
                .contains(DOCUMENT),
            "{pointer} does not point at {DOCUMENT}"
        );
    }
}

/// The gate's own calibration: a text with one fault of each kind is reported four times, and the
/// same text with the faults mended is reported none, so a pass above means something.
#[test]
fn the_gate_sees_a_planted_fault_of_each_kind() {
    let root = repository_root();
    let planted = "\
`crates/pdf-syntax/src/no_such_file.rs` is a path that is not there.\n\
`crates/pdf-syntax/src/xref.rs::no_such_function_anywhere` is a name the file lacks.\n\
ADR 9999 is a record nobody wrote.\n\
```mermaid\nflowchart LR\n    A --> B\n```\n\
No note follows that diagram.\n";
    let found = faults(&root, planted, 1);
    assert_eq!(
        found.len(),
        4,
        "the four planted faults, and only those: {found:?}"
    );

    let mended = "\
`crates/pdf-syntax/src/xref.rs` is there, and `crates/pdf-syntax/src/xref.rs::rebuild` too.\n\
ADR 0395 is a record.\n\
```mermaid\nflowchart LR\n    A --> B\n```\n\
**Maintenance note.** Depends on nothing.\n";
    let found = faults(&root, mended, 1);
    assert!(found.is_empty(), "the mended text is clean: {found:?}");
}
