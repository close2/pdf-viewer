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
