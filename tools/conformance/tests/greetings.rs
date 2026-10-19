//! A document that names a worker's greeting names the greeting the code sends.
//!
//! Each confined worker opens its wire with an eight-byte magic, six letters and a two-digit
//! version, and the version moves whenever something new crosses (`viewer-confined`'s
//! `protocol.rs` says when, ADR 0626). `doc/ui-boundary.md` states the confined greeting beside the
//! list of what crossed since it last moved, so the sentence is true only while the two agree — and
//! nothing but this check notices the day the constant moves and the document does not.
//!
//! It lives here for the reason `wait.rs` does: this crate's gates read the repository's own files.
//!
//! **The population is derived** (trap 25): every `const MAGIC: &[u8; 8] = b"…";` under `crates/`
//! and `raster/crates/`, so a worker that gains a greeting is held too. The documents are the
//! boundary's own and the four that are written as *what is* (CLAUDE.md, "Where knowledge lives");
//! the records under `doc/adr/`, `doc/history/` and `doc/reviews/`, and the todo files that say
//! what a greeting moved *from*, keep their chronology and are not read.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// The documents read, each written as what is rather than as what was.
const DOCUMENTS: [&str; 5] = [
    "doc/ui-boundary.md",
    "doc/PLAN.md",
    "doc/crate-map.md",
    "doc/state-of-play.md",
    "doc/HANDOVER.md",
];

/// The line shape a greeting is declared in, up to the opening quote of its bytes.
const DECLARATION: &str = "const MAGIC: &[u8; 8] = b\"";

/// The repository root, two levels above this crate's manifest.
fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`; a workspace member always has two
    // ancestors, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// One worker's greeting, and the source file that declares it.
struct Greeting {
    file: PathBuf,
    magic: String,
}

impl Greeting {
    /// The six letters every version of this greeting shares.
    fn family(&self) -> &str {
        &self.magic[..6]
    }
}

/// Every `.rs` file under `directory`, skipping `target` directories.
fn rust_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "target") {
                rust_files(&path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The greeting a line declares: eight bytes, six ASCII capitals then two ASCII digits.
fn declared(line: &str) -> Option<String> {
    let (_, bytes) = line.split_once(DECLARATION)?;
    let magic = bytes.get(..8)?;
    let shaped = magic.bytes().take(6).all(|b| b.is_ascii_uppercase())
        && magic.bytes().skip(6).all(|b| b.is_ascii_digit())
        && bytes.get(8..).is_some_and(|after| after.starts_with('"'));
    shaped.then(|| magic.to_owned())
}

/// Every greeting the tree declares.
fn greetings(root: &Path) -> Vec<Greeting> {
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    rust_files(&root.join("raster/crates"), &mut files);
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("a source file of the tree is readable");
        for magic in text.lines().filter_map(declared) {
            found.push(Greeting {
                file: file.clone(),
                magic,
            });
        }
    }
    found
}

/// Every version of `family` that `text` names: its six letters followed by exactly two digits,
/// which is the eight bytes of a greeting and nothing longer.
fn named_versions<'t>(text: &'t str, family: &str) -> Vec<&'t str> {
    text.match_indices(family)
        .filter_map(|(start, _)| {
            let rest = text.get(start..)?;
            let named = rest.get(..8)?;
            let versioned = named.bytes().skip(6).all(|b| b.is_ascii_digit());
            let longer = rest
                .get(8..)
                .and_then(|after| after.bytes().next())
                .is_some_and(|b| b.is_ascii_digit());
            (versioned && !longer).then_some(named)
        })
        .collect()
}

#[test]
fn every_greeting_a_document_names_is_the_one_the_code_sends() {
    let root = repository_root();
    let greetings = greetings(root);
    // Calibration: the confined worker's greeting is the one this check exists for, so a parse
    // that no longer finds it has stopped looking, whatever else it found.
    assert!(
        greetings.iter().any(|greeting| greeting
            .file
            .ends_with("crates/viewer-confined/src/protocol.rs")),
        "no greeting found in crates/viewer-confined/src/protocol.rs: the declaration's shape \
         `{DECLARATION}…\"` no longer matches"
    );
    let mut stale = Vec::new();
    for document in DOCUMENTS {
        let text = fs::read_to_string(root.join(document))
            .expect("each of the documents read exists and is readable");
        for greeting in &greetings {
            for named in named_versions(&text, greeting.family()) {
                if named != greeting.magic {
                    stale.push(format!(
                        "{document} names `{named}` where {} sends `{}`",
                        greeting
                            .file
                            .strip_prefix(root)
                            .unwrap_or(&greeting.file)
                            .display(),
                        greeting.magic
                    ));
                }
            }
        }
    }
    assert!(
        stale.is_empty(),
        "a document names a greeting the code no longer sends:\n{}",
        stale.join("\n")
    );
}

#[test]
fn the_boundary_document_names_the_confined_greeting() {
    let root = repository_root();
    let confined = greetings(root)
        .into_iter()
        .find(|greeting| {
            greeting
                .file
                .ends_with("crates/viewer-confined/src/protocol.rs")
        })
        .expect("the confined worker declares a greeting");
    let text = fs::read_to_string(root.join("doc/ui-boundary.md"))
        .expect("doc/ui-boundary.md is readable");
    assert!(
        named_versions(&text, confined.family()).contains(&confined.magic.as_str()),
        "doc/ui-boundary.md no longer names the confined greeting `{}`: the sentence that says what \
         crossed since it last moved has gone, or names it in another shape",
        confined.magic
    );
}
