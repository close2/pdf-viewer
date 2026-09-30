//! The fuzz workspace resolves to the tree's own dependency versions, and every fuzz target arrives
//! with the two things a round needs to run it.
//!
//! Not a conformance question; it lives here for `workspaces.rs`'s reason — this is the crate whose
//! gates read the repository's own files rather than a PDF.
//!
//! # The lock
//!
//! `fuzz/Cargo.toml` is a workspace of its own (`workspaces.rs` says why), so it has a lock of its
//! own, and cargo resolves the two independently. Where the two resolutions pick different versions
//! of one package the fuzz targets are compiled against code the tree's tests never ran: a lock
//! that pinned `hybrid-array` 0.4.13 while `pdf-signature` needs 0.4.15 left the whole fuzz
//! workspace failing to build, and nothing said so until a campaign tried to start. So
//! `fuzz/Cargo.lock` is tracked, and this gate holds every version it pins of a package the root
//! lock also holds to one the root lock pins (ADR 1439). A package the root lock holds at two
//! versions may appear in the fuzz lock at either; one the root lock does not hold at all —
//! `libfuzzer-sys` and what it brings — is the fuzz workspace's alone and is not asked about.
//!
//! The lock files are cargo's own TOML, read here by line: a `[[package]]` header and the `name`
//! and `version` keys under it are the whole of what is compared, and the conformance crate takes
//! no TOML dependency for them (its `Cargo.toml` says why).
//!
//! # The targets
//!
//! `tools/fuzz.sh` takes a target's invocation from its `cargo +nightly fuzz run <target>` line in
//! `doc/verify.md` and refuses a target that has none; `fuzz/seeds.sh` builds a target's corpus
//! from the recipe in its `case` arm, and a target with none fuzzes from nothing, which from
//! nothing reaches almost none of what it exists for (ADR 0742). Both refusals used to be found by
//! the round that tried to run the target. This gate finds them the day the target is written:
//! every file under `fuzz/fuzz_targets/` is a `[[bin]]` of `fuzz/Cargo.toml`, has its line, and has
//! its arm.
//!
//! Each property has a calibration test that plants a defect into the function doing the finding
//! and fails unless it is found (trap 13), planting into the functions rather than into a file for
//! `names.rs`'s reason.

#![expect(
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators and its findings, so a failure and a \
              clean run are read the same way"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Every package a `Cargo.lock` holds, by name, with the versions it pins.
fn locked(lock: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut packages: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut name: Option<String> = None;
    for line in lock.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            name = None;
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let value = value.trim_matches('"').to_owned();
        match key {
            "name" => name = Some(value),
            "version" => {
                if let Some(name) = name.take() {
                    packages.entry(name).or_default().insert(value);
                }
            }
            _ => {}
        }
    }
    packages
}

/// Each `name version` the fuzz lock pins that the root lock holds under another version only.
fn disagreements(
    root: &BTreeMap<String, BTreeSet<String>>,
    fuzz: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let mut found = Vec::new();
    for (name, versions) in fuzz {
        let Some(rooted) = root.get(name) else {
            continue;
        };
        for version in versions.difference(rooted) {
            let pinned: Vec<&str> = rooted.iter().map(String::as_str).collect();
            found.push(format!(
                "{name} {version} (the workspace pins {})",
                pinned.join(", ")
            ));
        }
    }
    found
}

/// The file names under `fuzz/fuzz_targets/`, less `.rs`.
fn targets(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join("fuzz/fuzz_targets"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .strip_suffix(".rs")
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The `name` of every `[[bin]]` a manifest declares.
fn bins(manifest: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut in_bin = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
            continue;
        }
        if in_bin && let Some(value) = line.strip_prefix("name = ") {
            found.insert(value.trim_matches('"').to_owned());
            in_bin = false;
        }
    }
    found
}

/// Whether `verify` holds the `cargo +nightly fuzz run <target>` line `tools/fuzz.sh` reads.
fn has_invocation(verify: &str, target: &str) -> bool {
    verify.lines().any(|line| {
        line.split_once("cargo +nightly fuzz run ")
            .is_some_and(|(_, rest)| rest.split_whitespace().next() == Some(target))
    })
}

/// Whether `seeds` holds a `case` arm for `target`: a line whose text up to its first `)` is a
/// `|`-separated list of bare words, one of them `target`.
fn has_recipe(seeds: &str, target: &str) -> bool {
    seeds.lines().any(|line| {
        line.split_once(')').is_some_and(|(pattern, _)| {
            let words: Vec<&str> = pattern.split('|').map(str::trim).collect();
            words.iter().all(|word| {
                !word.is_empty()
                    && word
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            }) && words.contains(&target)
        })
    })
}

/// What each target lacks, one line per lack.
fn lacks(targets: &[String], manifest: &str, verify: &str, seeds: &str) -> Vec<String> {
    let declared = bins(manifest);
    let mut found = Vec::new();
    for target in targets {
        if !declared.contains(target) {
            found.push(format!("{target}: no [[bin]] in fuzz/Cargo.toml"));
        }
        if !has_invocation(verify, target) {
            found.push(format!(
                "{target}: no `cargo +nightly fuzz run {target}` line in doc/verify.md"
            ));
        }
        if !has_recipe(seeds, target) {
            found.push(format!("{target}: no `{target})` arm in fuzz/seeds.sh"));
        }
    }
    found
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap_or_default()
}

#[test]
fn the_fuzz_lock_is_tracked_and_pins_only_versions_the_workspace_lock_pins() {
    let root = conformance::workspace_root();
    let ignored = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["check-ignore", "-q", "fuzz/Cargo.lock"])
        .status()
        .is_ok_and(|status| status.success());
    assert!(
        !ignored,
        "fuzz/Cargo.lock is gitignored, so every checkout resolves the fuzz workspace on its own \
         and this gate has nothing to hold (ADR 1439)"
    );
    let workspace = locked(&read(&root, "Cargo.lock"));
    let fuzz = locked(&read(&root, "fuzz/Cargo.lock"));
    let shared = fuzz
        .keys()
        .filter(|name| workspace.contains_key(*name))
        .count();
    println!(
        "{} package(s) in Cargo.lock, {} in fuzz/Cargo.lock, {shared} in both",
        workspace.len(),
        fuzz.len()
    );
    assert!(
        !workspace.is_empty() && shared > 0,
        "the sweep read nothing: {} in the workspace lock, {shared} shared — a clean answer over \
         an empty population is a sentence about the sweep",
        workspace.len()
    );
    let found = disagreements(&workspace, &fuzz);
    assert!(
        found.is_empty(),
        "fuzz/Cargo.lock pins a version of a shared package the workspace does not, so the fuzz \
         targets build against code the tree's tests never ran. In fuzz/, `cargo update -p <name> \
         --precise <the workspace's version>`:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_lock_comparison_names_a_planted_disagreement() {
    let workspace = locked(
        "version = 4\n\n[[package]]\nname = \"hybrid-array\"\nversion = \"0.4.15\"\n\
         source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\n\
         name = \"bitflags\"\nversion = \"1.3.2\"\n\n[[package]]\nname = \"bitflags\"\n\
         version = \"2.13.1\"\n",
    );
    let fuzz = locked(
        "[[package]]\nname = \"hybrid-array\"\nversion = \"0.4.13\"\ndependencies = [\n \
         \"typenum\",\n]\n\n[[package]]\nname = \"bitflags\"\nversion = \"2.13.1\"\n\n\
         [[package]]\nname = \"libfuzzer-sys\"\nversion = \"0.4.10\"\n",
    );
    assert_eq!(
        workspace.get("bitflags").map(BTreeSet::len),
        Some(2),
        "two versions of one package are both held"
    );
    assert_eq!(
        disagreements(&workspace, &fuzz),
        ["hybrid-array 0.4.13 (the workspace pins 0.4.15)"],
        "one of two versions is agreement, a package the workspace lacks is not asked about"
    );
}

#[test]
fn every_fuzz_target_has_its_bin_its_invocation_and_its_seed_recipe() {
    let root = conformance::workspace_root();
    let all = targets(&root);
    println!("{} fuzz target(s) under fuzz/fuzz_targets/", all.len());
    assert!(
        !all.is_empty(),
        "the sweep read no fuzz target — a clean answer over an empty population is a sentence \
         about the sweep"
    );
    let found = lacks(
        &all,
        &read(&root, "fuzz/Cargo.toml"),
        &read(&root, "doc/verify.md"),
        &read(&root, "fuzz/seeds.sh"),
    );
    assert!(
        found.is_empty(),
        "a fuzz target is missing what `tools/fuzz.sh` refuses it without, or what seeds it:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_target_sweep_names_each_planted_lack() {
    let manifest = "[[bin]]\nname = \"lexer\"\npath = \"fuzz_targets/lexer.rs\"\n\n\
                    [[bin]]\nname = \"page\"\n";
    let verify = "cd fuzz && cargo +nightly fuzz run lexer -- -runs=50000\n\
                  cd fuzz && cargo +nightly fuzz run pages -- -runs=1\n\
                  cd fuzz && cargo +nightly fuzz run ccitt -- -runs=1\n";
    let seeds =
        "case $t in\n    lexer)\n        : ;;\n    page | object) : ;;\n    # ccitt)\nesac\n";
    let targets = ["ccitt", "lexer", "object", "page"].map(str::to_owned);
    assert_eq!(
        lacks(&targets, manifest, verify, seeds),
        [
            "ccitt: no [[bin]] in fuzz/Cargo.toml",
            "ccitt: no `ccitt)` arm in fuzz/seeds.sh",
            "object: no [[bin]] in fuzz/Cargo.toml",
            "object: no `cargo +nightly fuzz run object` line in doc/verify.md",
            "page: no `cargo +nightly fuzz run page` line in doc/verify.md",
        ],
        "a prefix is not a line, a comment is not an arm, and one arm may name two targets"
    );
}
