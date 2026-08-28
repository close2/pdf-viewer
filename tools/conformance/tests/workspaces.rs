//! Every Cargo workspace in this tree is named by `doc/todo/02` §2's sequence — formatted by one
//! of its `cargo fmt` lines and compiled by one of its `cargo clippy`, `cargo check` or
//! `cargo build` lines.
//!
//! Not a conformance question, and it lives here for the reason `sandbox_gates.rs` and
//! `submodules.rs` do: this is the crate whose gates read the repository's own files rather than a
//! PDF.
//!
//! # What it guards
//!
//! `--workspace` and `--all` mean *every package in **this** workspace*, and a manifest that
//! declares its own `[workspace]` table is not in it. `fuzz/Cargo.toml` declares one deliberately
//! — `cargo-fuzz` builds that crate with its own sanitiser and profile settings, and a member
//! would apply those to the whole tree — so **`cargo fmt --all` and `cargo clippy --workspace` do
//! not read one line of `fuzz/fuzz_targets/`**. Neither command says so. Both exit 0.
//!
//! §2 already knew this for compiling: the sequence carries a
//! `cargo check --manifest-path fuzz/Cargo.toml --bins` line, added after fourteen rounds in which
//! the targets did not compile against the tree they fuzz. The formatting line above it never got
//! the same treatment, and the eight-hundred-and-seventh round found two rustfmt diffs sitting in
//! `fuzz/fuzz_targets/` that every round since had reported `cargo fmt --all --check` clean over.
//!
//! The general shape is the one this project cares most about: **an instrument that reports
//! success without having done its job.** A formatting gate blind to files in the tree is not a
//! weaker gate, it is a gate with a hole, and the hole is invisible from the gate's own output.
//!
//! # Why the sequence is the population, and cargo the authority
//!
//! `doc/todo/02-every-round.md` §2 **owns** the gate sequence and nothing else states it, so this
//! gate reads that block rather than a list of its own — the same argument `sandbox_gates.rs`
//! makes, and the drift ADR 0232 §4 is about.
//!
//! The other side is asked of cargo rather than derived here. `cargo locate-project --workspace`
//! answers, for one manifest, which workspace root governs it; running it over every tracked
//! `Cargo.toml` yields the tree's workspaces exactly, by the same rule cargo itself applies. A
//! `[workspace]`-table grep would agree today and would be this gate's own reading of cargo's
//! rules rather than cargo's.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a gate that cannot run `git` or `cargo` in its own repository has not \
              found a defect, and reporting that as one would be worse than stopping"
)]

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Where the repository root is, relative to this crate's manifest.
///
/// Compiled in, which is trap 15 — and correct here, because what this gate reads is the tree it
/// was built from. `cargo test -p conformance` is how it runs and that cannot pick another tree.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// Every `Cargo.toml` the index tracks, as a path relative to the repository root.
///
/// Taken from git rather than from a directory walk because a worktree round replaces its
/// submodules and its corpus cache with symlinks into the primary checkout, and a walk would
/// follow them out of this tree entirely.
fn tracked_manifests(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "*Cargo.toml"])
        .output()
        .expect("git is on the path wherever this workspace builds");

    assert!(
        output.status.success(),
        "the index could not be listed, so this gate cannot say anything"
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Which workspace root governs `manifest`, as cargo resolves it, relative to the repository root.
fn workspace_root_of(root: &Path, manifest: &str) -> String {
    let output = Command::new("cargo")
        .args([
            "locate-project",
            "--workspace",
            "--message-format",
            "plain",
            "--manifest-path",
        ])
        .arg(manifest)
        .current_dir(root)
        .output()
        .expect("cargo is on the path wherever this workspace builds");

    assert!(
        output.status.success(),
        "`cargo locate-project` refused {manifest}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    let located = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let relative = Path::new(&located).strip_prefix(root).unwrap_or_else(|_| {
        panic!("cargo placed {manifest}'s workspace at {located}, which is outside this tree")
    });
    relative.to_string_lossy().into_owned()
}

/// Every `cargo` invocation `doc/todo/02` §2 states, split into words after the `cargo`, with the
/// leading environment assignment and the trailing comment removed.
///
/// Narrowed to §2 rather than read from the whole file, which is where this differs from
/// `sandbox_gates.rs`: that gate's pattern — `cargo test … --test …` — appears nowhere else in the
/// document, and `cargo build` appears here in §5 as well, over the binaries a person runs. A gate
/// that counted those would let a §5 line answer for a §2 hole. The section not being found is a
/// loud failure, by the emptiness assertions in the test below.
///
/// **The environment prefix is not a detail.** Two of the sequence's lines begin
/// `RUSTFLAGS="-D warnings"` rather than `cargo`, and one of those is the line that compiles
/// `fuzz/` — so a parser anchored on `cargo` at the start of the line reports the tree's second
/// workspace uncompiled while the sequence compiles it. This gate said exactly that the first time
/// it ran, and the quoted value's own space is why splitting into words first is not enough.
fn cargo_invocations(root: &Path) -> Vec<Vec<String>> {
    let path = root.join("doc/todo/02-every-round.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{} is this gate's population: {why}", path.display()));

    let mut found = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            in_section = heading.starts_with("2. ");
            continue;
        }
        if !in_section {
            continue;
        }
        // A trailing `# …` is a comment on the command rather than an argument of it.
        let command = line.split_once('#').map_or(line, |(before, _)| before);
        let words: Vec<&str> = command.split_whitespace().collect();
        let Some(at) = words.iter().position(|word| *word == "cargo") else {
            continue;
        };
        // Only a shell environment assignment may stand in front of the command; anything else
        // means the word was prose about cargo rather than an invocation of it.
        let assignment = words[..at]
            .first()
            .is_some_and(|word| word.split_once('=').is_some_and(|(key, _)| !key.is_empty()));
        if at == 0 || assignment {
            found.push(
                words[at.saturating_add(1)..]
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect(),
            );
        }
    }
    found
}

/// The subcommand an invocation names — `fmt`, `clippy`, `check` and so on.
fn subcommand(invocation: &[String]) -> Option<&str> {
    invocation.first().map(String::as_str)
}

/// Whether `line` passes `--manifest-path manifest`.
fn names_manifest(line: &[String], manifest: &str) -> bool {
    line.windows(2)
        .any(|pair| pair[0] == "--manifest-path" && pair[1] == manifest)
}

/// Whether `line` names no manifest at all, and so acts on the workspace it is run from.
fn names_no_manifest(line: &[String]) -> bool {
    !line.iter().any(|word| word == "--manifest-path")
}

/// The subcommands that read a workspace's sources and would report a defect in them.
///
/// `build` is here beside `check` and `clippy` because §2 uses it for `pdf-sandbox`'s worker, and a
/// workspace whose only compiling line were a `build` would still be compiled.
const COMPILING: [&str; 3] = ["clippy", "check", "build"];

#[test]
fn every_workspace_in_the_tree_is_formatted_and_compiled_by_the_sequence() {
    let root = repository_root();

    let manifests = tracked_manifests(root);
    assert!(
        manifests.len() > 5,
        "the index yielded {} `Cargo.toml` files, which means this gate is measuring nothing — \
         either the `git ls-files` invocation stopped working or this is not the tree",
        manifests.len()
    );

    let roots: BTreeSet<String> = manifests
        .iter()
        .map(|manifest| workspace_root_of(root, manifest))
        .collect();
    assert!(
        roots.contains("Cargo.toml"),
        "no tracked manifest resolved to the tree's own root workspace, so the paths this gate \
         compares are not the ones §2 writes: {roots:?}"
    );

    let lines = cargo_invocations(root);
    let formatting: Vec<&Vec<String>> = lines
        .iter()
        .filter(|line| subcommand(line) == Some("fmt"))
        .collect();
    let compiling: Vec<&Vec<String>> = lines
        .iter()
        .filter(|line| subcommand(line).is_some_and(|name| COMPILING.contains(&name)))
        .collect();
    assert!(
        !formatting.is_empty() && !compiling.is_empty(),
        "doc/todo/02 §2 yielded {} formatting and {} compiling lines, which means this gate is \
         measuring nothing — either the sequence moved out of that file or the parse stopped \
         working",
        formatting.len(),
        compiling.len()
    );

    let mut unformatted = Vec::new();
    let mut uncompiled = Vec::new();
    for workspace in &roots {
        // The tree's own root workspace is the one a bare invocation acts on, because §2's
        // sequence is run from the repository root.
        let covered = |line: &&Vec<String>| {
            if workspace == "Cargo.toml" {
                names_no_manifest(line)
            } else {
                names_manifest(line, workspace)
            }
        };
        if !formatting.iter().any(covered) {
            unformatted.push(workspace.clone());
        }
        if !compiling.iter().any(covered) {
            uncompiled.push(workspace.clone());
        }
    }

    assert!(
        unformatted.is_empty(),
        "these workspaces are in the tree and no `cargo fmt` line in doc/todo/02 §2 reads them, \
         so a rustfmt diff under one of them passes the formatting gate silently: {unformatted:?}\n\
         \n\
         `--all` means every package in *this* workspace, and a manifest with its own \
         `[workspace]` table is not in it. Add \
         `cargo fmt --manifest-path <manifest> --check` to the sequence."
    );

    assert!(
        uncompiled.is_empty(),
        "these workspaces are in the tree and no `cargo clippy`, `cargo check` or `cargo build` \
         line in doc/todo/02 §2 reads them, so they can stop compiling against the tree they are \
         part of without any gate here saying so: {uncompiled:?}"
    );
}
