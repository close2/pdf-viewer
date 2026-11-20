//! Where the close is, and where it is called: read off the sources (ADRs 1945, 3361).
//!
//! [`inherited_descriptors::keep_only_the_standard_descriptors`] is a safe function whose soundness
//! rests on a position — `main`'s first statement, before anything owns a descriptor past the
//! three — which no type can state. So this file states it, as `viewer-qt`'s `unsafe_position.rs`
//! states its own crate's position: the crate lifts the denial once, on the close, whose one
//! `unsafe` is the one call; every source in the tree that names the crate is a helper program
//! whose root forbids `unsafe` and whose `main` calls it first; and the helpers are the ones named.
//!
//! What the close *does* is asked of a running worker by `pdf-sandbox`'s
//! `tests/inherited_descriptors.rs`.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// The helpers that close what they inherited, each started with exactly standard input, output and
/// error: the image decoder, the confined viewer, the script engine, the media decoder and the sound
/// helper.
const CALLERS: [&str; 5] = [
    "pdf-media-worker/src/bin/pdf-media-worker.rs",
    "pdf-sandbox/src/bin/pdf-sandbox-worker.rs",
    "pdf-script-worker/src/bin/pdf-script-worker.rs",
    "quorra-sound/src/bin/quorra-sound.rs",
    "viewer-confined/src/bin/pdf-view-worker.rs",
];

/// The tree's crates directory.
fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A line with its comment removed and the lint's own name taken out, so that only a use of the
/// permission is left to find: the attributes that name `unsafe_code` state the rule.
fn code_only(line: &str) -> String {
    let code = line.find("//").map_or(line, |at| &line[..at]);
    code.replace("unsafe_code", "")
}

/// Every Rust source under a crate's `src/`, as a path relative to the crates directory.
fn sources() -> Vec<String> {
    let root = crates();
    let mut found = Vec::new();
    // An unreadable directory finds no caller, which the assertion on `CALLERS` then fails.
    let Ok(entries) = std::fs::read_dir(&root) else {
        return found;
    };
    for entry in entries.flatten() {
        let mut stack = vec![entry.path().join("src")];
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                let Ok(inner) = std::fs::read_dir(&path) else {
                    continue;
                };
                stack.extend(inner.flatten().map(|one| one.path()));
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && let Ok(relative) = path.strip_prefix(&root)
            {
                found.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn the_crate_lifts_the_denial_once_and_its_one_unsafe_is_the_close() {
    let path = crates().join("inherited-descriptors/src/lib.rs");
    let source = std::fs::read_to_string(&path).expect("the crate's root");
    assert_eq!(
        source.matches("#![deny(unsafe_code)]").count(),
        1,
        "the root denies it"
    );
    let lines: Vec<&str> = source.lines().collect();
    let lifts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(at, line)| {
            line.trim() == "#[expect("
                && lines
                    .get(at.saturating_add(1))
                    .is_some_and(|next| next.trim() == "unsafe_code,")
        })
        .map(|(at, _)| at)
        .collect();
    let [lift] = lifts.as_slice() else {
        panic!("the denial is lifted exactly once, not at {lifts:?}");
    };
    let lifted = lines
        .iter()
        .skip(lift.saturating_add(1))
        .map(|line| line.trim())
        .find(|line| line.starts_with("pub fn "));
    assert_eq!(
        lifted,
        Some("pub fn keep_only_the_standard_descriptors() -> std::io::Result<()> {"),
        "and it is lifted on the close"
    );
    let uses: Vec<String> = lines
        .iter()
        .map(|line| code_only(line))
        .filter(|line| line.contains("unsafe"))
        .map(|line| line.trim().to_owned())
        .collect();
    assert_eq!(
        uses,
        vec!["let closed = unsafe { libc::close_range(3, u32::MAX, 0) };".to_owned()],
        "the one `unsafe` is the one call"
    );
}

#[test]
fn every_caller_is_a_helper_whose_main_calls_it_first() {
    let root = crates();
    let naming: Vec<String> = sources()
        .into_iter()
        .filter(|path| !path.starts_with("inherited-descriptors/"))
        .filter(|path| {
            std::fs::read_to_string(root.join(path))
                .is_ok_and(|text| text.contains("inherited_descriptors"))
        })
        .collect();
    assert_eq!(
        naming, CALLERS,
        "the sources that name the crate are the five helpers' programs, and a new caller is a new \
         helper to name here (ADR 3361)"
    );
    for path in CALLERS {
        let source = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("{path} is a helper: {error}"));
        assert_eq!(
            source.matches("#![forbid(unsafe_code)]").count(),
            1,
            "{path}: the program forbids `unsafe`, because the close is not its own"
        );
        assert_eq!(
            source.matches("inherited_descriptors::").count(),
            1,
            "{path}: it calls the close once"
        );
        let first = source
            .lines()
            .skip_while(|line| !line.starts_with("fn main()"))
            .nth(1)
            .map(str::trim);
        assert_eq!(
            first,
            Some(
                "if let Err(error) = inherited_descriptors::keep_only_the_standard_descriptors() {"
            ),
            "{path}: and the call is `main`'s first statement"
        );
    }
}
