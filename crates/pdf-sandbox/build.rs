//! Stamps this crate with an identity of the tree it was built from.
//!
//! The parent and the worker are two processes running two copies of this crate, and the
//! greeting in `protocol.rs` has always proved they speak the same *wire format*. It could
//! not prove they were the same *build*, and that turned out to be the difference that
//! matters: a worker whose decoders are older answers every request perfectly well, with
//! older answers, and a decoder's refusal from last week's binary is word for word a
//! decoder's refusal from this one's. ADR 0458 has the session that cost.
//!
//! So the greeting carries this number too. It is a hash of what decides the worker's
//! answers, as far as a build script can see it:
//!
//! - the workspace `Cargo.lock`, which pins every decoder — `hayro-jbig2`, `hayro-ccitt` and
//!   `hayro-jpeg2000` are external crates and a git revision is where a fix to one of them
//!   arrives;
//! - every `.rs` file of this crate, which is the code on both ends of the pipe.
//!
//! **What it does not cover is stated rather than implied**: the compiler, its version, and
//! the profile a binary was built with. Two builds of the same sources by different
//! compilers agree here, which is the right trade — they also agree about every image — and
//! the failure this exists to catch is a *stale binary*, which always differs in one of the
//! two inputs above.
//!
//! **It is not a security control and must not be read as one.** The worker is the untrusted
//! side of that pipe; a subverted worker can send any sixteen bytes it likes. What this
//! detects is a mistake.

// A build script's job is to abort the build when its input is malformed, and the panic
// message is the diagnostic a developer reads — the same argument `pdf-font/build.rs` makes.
#![expect(
    clippy::panic,
    reason = "aborting the build is the intended and only useful failure mode here"
)]

use std::path::{Path, PathBuf};

/// FNV-1a's 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a's 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Folds bytes into an FNV-1a hash.
///
/// FNV rather than a real digest because this is an *accident* detector and a build script
/// may not buy a dependency for one: any hash whose collisions are not chosen by an adversary
/// is enough to tell two builds apart, and the crate documentation above says why nobody is
/// choosing them.
fn fold(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Every `.rs` file under `root`, relative to it, sorted.
///
/// Sorted because a directory listing's order is the filesystem's, and an identity that
/// depended on it would differ between two checkouts of the same commit.
fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", directory.display()));
        for entry in entries {
            let entry =
                entry.unwrap_or_else(|error| panic!("{} is readable: {error}", root.display()));
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest.join("src");
    let lock = manifest.join("../../Cargo.lock");
    println!("cargo::rerun-if-changed={}", source_root.display());
    println!("cargo::rerun-if-changed={}", lock.display());

    // The lockfile is absent in a vendored or `cargo package`d build, where there is no
    // second process to disagree with in the first place. Its absence is folded in as
    // itself rather than ignored, so that "built without a lockfile" is its own identity
    // and not accidentally equal to some tree that had one.
    let mut hash = match std::fs::read(&lock) {
        Ok(bytes) => fold(fold(FNV_OFFSET, b"Cargo.lock\0"), &bytes),
        Err(_) => fold(FNV_OFFSET, b"no Cargo.lock\0"),
    };

    let files = sources(&source_root);
    assert!(
        !files.is_empty(),
        "crates/pdf-sandbox/src holds no Rust source, so the identity would describe nothing"
    );
    for file in files {
        let relative = file.strip_prefix(&manifest).unwrap_or(&file);
        // The path goes in as well as the contents: moving a file between two names is a
        // change, and hashing contents alone would call the two trees identical.
        hash = fold(hash, relative.to_string_lossy().as_bytes());
        hash = fold(hash, b"\0");
        let bytes = std::fs::read(&file)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", file.display()));
        hash = fold(hash, &bytes);
    }

    // Sixteen lowercase hex digits, a width `protocol.rs` relies on: the greeting is a
    // fixed-length record and this field is copied into it without being parsed.
    println!("cargo::rustc-env=PDF_SANDBOX_BUILD={hash:016x}");
}
