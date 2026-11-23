//! RFC 0008 section 3's census population, as the `script_corpus` gates walk it.
//!
//! One list of roots for the two walks over it — Tier 0's in `pdf-model`'s `tests/script_corpus.rs`
//! and Tier 1's in `pdf-script`'s, reached through a `#[path]` — so that the engine is measured on
//! exactly the documents the library is (trap 25: a population is derived, never written down
//! twice). A file that includes this one includes `corpus_passwords` beside it.

use std::path::{Path, PathBuf};

/// Files larger than this are counted and not read, the census's own bound (RFC 0008 section 3.1).
pub(crate) const MAX_FILE_BYTES: u64 = 128 << 20;

/// The census population's roots, relative to the repository, and whether each is walked whole.
const ROOTS: [(&str, bool); 5] = [
    ("doc/pdf.js/test/pdfs", false),
    ("doc/corpora", true),
    ("corpus-cache/openpreserve", true),
    ("corpus-cache/tika-issue-tracker", true),
    ("corpus-cache/safedocs", true),
];

/// The repository root, from the including crate's manifest directory.
pub(crate) fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.pdf` under one directory, recursively where `whole` says so.
fn collect(path: &Path, whole: bool, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if whole {
                collect(&path, whole, into);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// The published password for a corpus file, or the empty default §7.6.4.1 starts with.
pub(crate) fn password_for(path: &Path) -> &'static str {
    path.file_name()
        .and_then(|name| super::corpus_passwords::corpus_password(&name.to_string_lossy()))
        .map_or("", |known| known.password)
}

/// The environment variable that narrows both walks to the documents Tier 0's held file names.
///
/// Set to `1`, a walk reads only those documents — a few minutes rather than hours, because the
/// census population is mostly documents with no field script, each opened to find that out — for a
/// round that changes how a field's value is read, set or committed (ADR 3471).
pub(crate) const HELD_ONLY_VARIABLE: &str = "PDFVIEWER_SCRIPT_CORPUS_HELD";

/// Tier 0's held file, relative to the repository: its first column names every document either
/// walk has found a field script in.
const HELD_FILE: &str = "crates/pdf-model/tests/script_corpus.tsv";

/// Whether [`HELD_ONLY_VARIABLE`] narrows this walk.
pub(crate) fn held_only() -> bool {
    std::env::var(HELD_ONLY_VARIABLE).as_deref() == Ok("1")
}

/// What this walk reads: [`population`], or under [`HELD_ONLY_VARIABLE`] the documents of the held
/// file that are on this machine.
pub(crate) fn walked(root: &Path) -> Vec<PathBuf> {
    if !held_only() {
        return population(root);
    }
    let held = std::fs::read_to_string(root.join(HELD_FILE)).unwrap_or_default();
    let named: Vec<PathBuf> = held
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split('\t').next())
        .filter(|key| !key.is_empty())
        .map(|key| root.join(key))
        .collect();
    let mut files: Vec<PathBuf> = named
        .iter()
        .filter(|path| path.is_file())
        .cloned()
        .collect();
    println!(
        "{HELD_ONLY_VARIABLE}=1: {} document(s) held, {} on this machine",
        named.len(),
        files.len()
    );
    files.sort();
    files.dedup();
    files
}

/// Every PDF of the census population on this machine, sorted, each root's count printed.
pub(crate) fn population(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for (relative, whole) in ROOTS {
        let directory = root.join(relative);
        if directory.is_dir() {
            let before = files.len();
            collect(&directory, whole, &mut files);
            println!("{relative}: {} PDF(s)", files.len().saturating_sub(before));
        } else {
            println!("{relative}: not on this machine");
        }
    }
    files.sort();
    files.dedup();
    files
}
