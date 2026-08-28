//! Every Cargo workspace in this tree is named by `doc/todo/02` §2's sequence — formatted by one
//! of its `cargo fmt` lines, compiled by one of its `cargo clippy`, `cargo check` or `cargo build`
//! lines, and **linted** by one of its `cargo clippy` lines under `RUSTFLAGS="-D warnings"` — and
//! states the same lint levels the tree's own root workspace does.
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
//! §2 already knew this for compiling: the sequence carried a
//! `cargo check --manifest-path fuzz/Cargo.toml --bins` line, added after fourteen rounds in which
//! the targets did not compile against the tree they fuzz — a `clippy` line since ADR 0742, for
//! the reason the next section gives. The formatting line above it never got
//! the same treatment, and the eight-hundred-and-seventh round found two rustfmt diffs sitting in
//! `fuzz/fuzz_targets/` that every round since had reported `cargo fmt --all --check` clean over.
//!
//! The general shape is the one this project cares most about: **an instrument that reports
//! success without having done its job.** A formatting gate blind to files in the tree is not a
//! weaker gate, it is a gate with a hole, and the hole is invisible from the gate's own output.
//! `doc/traps/instruments-and-reports.md`'s trap 23 is the lesson; ADR 0739 is the argument.
//!
//! # The lint half, and why it needs two questions rather than one
//!
//! ADR 0739 stopped at compiling, deliberately and in writing: a `cargo check` line answers *do
//! the targets still build*, and putting `fuzz/` under the tree's lint levels was a larger
//! decision because that crate took no `[lints] workspace = true` at all. The
//! eight-hundred-and-tenth session took it (ADR 0742), and closing the hole took two properties
//! rather than one, because a lint level travels by a different road from a command:
//!
//! - **The command.** `cargo clippy --workspace` stops at the workspace boundary exactly as
//!   `cargo fmt --all` does, so every root owes a `cargo clippy` line naming its manifest — under
//!   `RUSTFLAGS="-D warnings"`, which ADR 0450 established is what makes a lint run *the* gate
//!   rather than a weaker one than CI's.
//! - **The levels.** `[lints] workspace = true` resolves against the workspace the package is in,
//!   and cargo offers no way to point one workspace's packages at another's table. So a second
//!   workspace's lint levels are a **copy**, and a copy is what this project's own rule about
//!   two documents stating one thing predicts the fate of. The gate therefore compares the two
//!   `[workspace.lints.*]` tables and fails on any level the two state differently — which is the
//!   drift the copy invites, said out loud on the day it happens.
//!
//! A run under lint levels the tree does not share would be an instrument reporting success
//! against a different standard, which is the same failure wearing the other half of the costume.
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
/// it ran, and the quoted value's own space is why splitting into words first is not enough. The
/// prefix is *kept* rather than only skipped, because the lint arm below asks what is in it.
fn cargo_invocations(root: &Path) -> Vec<Invocation> {
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
            found.push(Invocation {
                // The whole line up to `cargo`, so that a `RUSTFLAGS` value containing a space is
                // matched as it is written rather than word by word.
                environment: command
                    .split("cargo")
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
                words: words[at.saturating_add(1)..]
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect(),
            });
        }
    }
    found
}

/// One `cargo` line of the sequence: what stands in front of the command, and what follows it.
struct Invocation {
    /// The shell environment assignments the line is prefixed with, verbatim and trimmed.
    environment: String,
    /// The words after `cargo`, so `words[0]` is the subcommand.
    words: Vec<String>,
}

impl Invocation {
    /// The subcommand this line names — `fmt`, `clippy`, `check` and so on.
    fn subcommand(&self) -> Option<&str> {
        self.words.first().map(String::as_str)
    }

    /// Whether this line passes `--manifest-path manifest`.
    fn names_manifest(&self, manifest: &str) -> bool {
        self.words
            .windows(2)
            .any(|pair| pair[0] == "--manifest-path" && pair[1] == manifest)
    }

    /// Whether this line names no manifest at all, and so acts on the workspace it is run from.
    fn names_no_manifest(&self) -> bool {
        !self.words.iter().any(|word| word == "--manifest-path")
    }

    /// Whether this line turns the workspace's `warn` levels into errors, as CI does.
    ///
    /// ADR 0450: the lint levels are `warn` so that an ordinary build stays usable, so a `clippy`
    /// run without this is a *weaker* gate than the one that gates a push — and a workspace linted
    /// only by such a line is not linted to this tree's standard.
    fn denies_warnings(&self) -> bool {
        self.environment.contains(r#"RUSTFLAGS="-D warnings""#)
    }
}

/// Every `[workspace.lints.…]` entry a manifest states, normalised so that only the levels differ.
///
/// A textual read rather than a parse: `tools/conformance` depends on nothing but `thiserror`, and
/// pulling a TOML reader in to compare two hand-maintained tables would cost more than it buys.
/// Whitespace is removed so that a reflow is not a finding, comments and blank lines are dropped,
/// and each entry is keyed by its table so that `clippy`'s `pedantic` and `rust`'s cannot be
/// confused. The failure mode of reading it this way is a *spurious* failure naming the entry,
/// which is loud; a parse that silently agreed would be the shape this gate exists against.
fn lint_levels(manifest: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(manifest)
        .unwrap_or_else(|why| panic!("{} is this gate's population: {why}", manifest.display()));

    let mut levels = BTreeSet::new();
    let mut table: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(heading) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            table = heading
                .strip_prefix("workspace.lints.")
                .map(str::to_owned)
                .filter(|name| !name.is_empty());
            continue;
        }
        let Some(table) = table.as_deref() else {
            continue;
        };
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let entry: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        levels.insert(format!("{table}.{entry}"));
    }
    levels
}

/// The subcommands that read a workspace's sources and would report a defect in them.
///
/// `build` is here beside `check` and `clippy` because §2 uses it for `pdf-sandbox`'s worker, and a
/// workspace whose only compiling line were a `build` would still be compiled.
const COMPILING: [&str; 3] = ["clippy", "check", "build"];

/// Every workspace root in this tree, as cargo resolves them, relative to the repository root.
///
/// Both assertions are about the *instrument*: a population that came back empty or without the
/// tree's own root would let every arm below pass while comparing nothing.
fn workspace_roots(root: &Path) -> BTreeSet<String> {
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
    roots
}

/// Every lint level one of this tree's other workspaces states and the tree's own root does not,
/// or the other way round, each named by the manifest that differs.
fn divergent_levels(root: &Path, roots: &BTreeSet<String>) -> Vec<String> {
    let tree = lint_levels(&root.join("Cargo.toml"));
    assert!(
        !tree.is_empty(),
        "the tree's own root manifest states no `[workspace.lints.…]` entry this gate can read, so \
         it has nothing to compare a second workspace against"
    );

    let mut divergent = Vec::new();
    for workspace in roots.iter().filter(|name| *name != "Cargo.toml") {
        let theirs = lint_levels(&root.join(workspace));
        divergent.extend(
            tree.symmetric_difference(&theirs)
                .map(|entry| format!("{workspace}: {entry}")),
        );
    }
    divergent
}

#[test]
fn every_workspace_in_the_tree_is_formatted_compiled_and_linted_by_the_sequence() {
    let root = repository_root();
    let roots = workspace_roots(root);

    let lines = cargo_invocations(root);
    let formatting: Vec<&Invocation> = lines
        .iter()
        .filter(|line| line.subcommand() == Some("fmt"))
        .collect();
    let compiling: Vec<&Invocation> = lines
        .iter()
        .filter(|line| {
            line.subcommand()
                .is_some_and(|name| COMPILING.contains(&name))
        })
        .collect();
    let linting: Vec<&Invocation> = lines
        .iter()
        .filter(|line| line.subcommand() == Some("clippy") && line.denies_warnings())
        .collect();
    assert!(
        !formatting.is_empty() && !compiling.is_empty() && !linting.is_empty(),
        "doc/todo/02 §2 yielded {} formatting, {} compiling and {} linting lines, which means this \
         gate is measuring nothing — either the sequence moved out of that file or the parse \
         stopped working",
        formatting.len(),
        compiling.len(),
        linting.len()
    );

    let divergent = divergent_levels(root, &roots);

    let mut unformatted = Vec::new();
    let mut uncompiled = Vec::new();
    let mut unlinted = Vec::new();
    for workspace in &roots {
        // The tree's own root workspace is the one a bare invocation acts on, because §2's
        // sequence is run from the repository root.
        let covered = |line: &&Invocation| {
            if workspace == "Cargo.toml" {
                line.names_no_manifest()
            } else {
                line.names_manifest(workspace)
            }
        };
        if !formatting.iter().any(covered) {
            unformatted.push(workspace.clone());
        }
        if !compiling.iter().any(covered) {
            uncompiled.push(workspace.clone());
        }
        if !linting.iter().any(covered) {
            unlinted.push(workspace.clone());
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

    assert!(
        unlinted.is_empty(),
        "these workspaces are in the tree and no `cargo clippy` line in doc/todo/02 §2 reads them \
         under `RUSTFLAGS=\"-D warnings\"`, so `clippy::pedantic` and the rest of CLAUDE.md \
         principle 1's levels are not enforced over them at all: {unlinted:?}\n\
         \n\
         Add `RUSTFLAGS=\"-D warnings\" cargo clippy --manifest-path <manifest> --all-targets` to \
         the sequence. The flag is part of the requirement, not decoration — ADR 0450."
    );

    assert!(
        divergent.is_empty(),
        "these lint levels are stated by one of this tree's workspaces and not by the other, so \
         the two are linted to different standards and the gate that runs over both cannot say \
         so: {divergent:?}\n\
         \n\
         `[lints] workspace = true` resolves against the workspace a package is in and cargo \
         offers no way to inherit another's, so a second workspace's `[workspace.lints.…]` tables \
         are a copy of the root's and must stay identical to it. ADR 0742."
    );
}
