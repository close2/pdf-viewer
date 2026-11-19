//! Every file directly under a crate's `tests/` is compiled, where the crate has turned off
//! Cargo's discovery of test targets.
//!
//! It lives here for the reason `workspaces.rs` does: this is the crate whose gates read the
//! repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! Cargo makes each `.rs` file directly under `tests/` a test binary of its own, and a crate whose
//! tests are grouped into a few binaries by subject (`pdf-model`, ADR 3086) has to stop that:
//! `autotests = false` in its `[package]`, every binary a `[[test]]`, and each grouped file a
//! `mod` line in one of them. From then on **a file nobody names is compiled by nothing and run by
//! nothing, and no command says so** — `cargo test`, `cargo nextest run` and `cargo clippy
//! --all-targets` all exit 0 over it. That is an instrument reporting success without having done
//! its job (`doc/traps/instruments-and-reports.md`, trap 23), so this check asks the question the
//! build no longer asks: is every `tests/*.rs` file of such a crate a declared `[[test]]` root, or
//! the module of one?
//!
//! **The population is derived** (trap 25): every workspace member whose manifest states
//! `autotests = false`, so a crate that groups its tests later is held without an edit here.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The repository root, two levels above this crate's manifest.
fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`; a workspace member always has two
    // ancestors, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// Whether the manifest's `[package]` table turns Cargo's test discovery off.
fn discovery_is_off(manifest: &str) -> bool {
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[package]";
            continue;
        }
        if inside
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "autotests"
        {
            return value.trim() == "false";
        }
    }
    false
}

/// The source file of every `[[test]]` the manifest declares: its `path`, or Cargo's default
/// `tests/<name>.rs` when it states none.
fn declared_roots(manifest: &str, crate_dir: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<(Option<String>, Option<String>)> = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[[test]]";
            if inside {
                roots.push((None, None));
            }
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_owned();
        if let Some(last) = roots.last_mut() {
            match key.trim() {
                "name" => last.0 = Some(value),
                "path" => last.1 = Some(value),
                _ => {}
            }
        }
    }
    roots
        .into_iter()
        .filter_map(|(name, path)| match (name, path) {
            (_, Some(path)) => Some(crate_dir.join(path)),
            (Some(name), None) => Some(crate_dir.join("tests").join(format!("{name}.rs"))),
            (None, None) => None,
        })
        .collect()
}

/// The files a root's out-of-line `mod` items name, resolved as rustc resolves them from a
/// crate root: `#[path = "…"]` relative to the root's directory, otherwise `<name>.rs` beside it.
fn modules_of(root: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(root).expect("a declared test root is readable");
    let directory = root.parent().expect("a test root lies in a directory");
    let mut path_attribute: Option<String> = None;
    let mut found = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("#[path = \"") {
            path_attribute = rest.strip_suffix("\"]").map(str::to_owned);
            continue;
        }
        let item = line.strip_prefix("pub ").unwrap_or(line);
        if let Some(name) = item.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
            let file = path_attribute
                .take()
                .unwrap_or_else(|| format!("{}.rs", name.trim()));
            found.push(directory.join(file));
        } else if !line.starts_with("#[") && !line.starts_with("//") {
            path_attribute = None;
        }
    }
    found
}

/// The `.rs` files directly under `directory`.
fn files_directly_under(directory: &Path) -> BTreeSet<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "rs"))
        .collect()
}

#[test]
fn every_test_file_of_a_crate_without_discovery_is_compiled() {
    let root = repository_root();
    let members = conformance::roots::members(root).expect("the workspace manifest states members");
    let mut held = 0_usize;
    let mut orphans = Vec::new();
    for member in members {
        let crate_dir = root.join(&member);
        let manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap_or_default();
        if !discovery_is_off(&manifest) {
            continue;
        }
        held = held.saturating_add(1);
        let roots = declared_roots(&manifest, &crate_dir);
        let mut compiled: BTreeSet<PathBuf> = roots.iter().cloned().collect();
        for test_root in &roots {
            compiled.extend(modules_of(test_root));
        }
        let compiled: BTreeSet<PathBuf> = compiled
            .into_iter()
            .filter_map(|path| path.canonicalize().ok())
            .collect();
        for file in files_directly_under(&crate_dir.join("tests")) {
            let canonical = file.canonicalize().expect("a listed file canonicalises");
            if !compiled.contains(&canonical) {
                orphans.push(file);
            }
        }
    }
    assert!(
        held > 0,
        "no workspace member states `autotests = false`, so this check holds nothing; ADR 3086 \
         says which crate it was written for"
    );
    assert!(
        orphans.is_empty(),
        "these test files are compiled by no target, because their crate turned Cargo's test \
         discovery off and no `[[test]]` root names them by a `mod` line (ADR 3086); add each \
         to the binary of its subject:\n{}",
        orphans
            .iter()
            .map(|path| format!("  {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The calls that change a whole process for good: each is made once, by a process about to be
/// confined, and has no way back by design (`pdf_font::substitute::no_machine_fonts`,
/// `pdf_font::provider::faces_come_from`, ADR 0870).
const PROCESS_WIDE_SWITCHES: [&str; 2] = ["no_machine_fonts(", "faces_come_from("];

/// A module of a grouped suite throws no process-wide switch.
///
/// Under `cargo test` — CI's runner — a suite's modules share one process, so a module that turns
/// the machine's fonts off takes them from every test scheduled after it, and which tests those
/// are depends on the harness's threads; nextest, which every local gate runs, gives each test a
/// process of its own and never sees it. Two such files grouped into `pdf-model`'s `text_suite`
/// failed CI twice over while passing here (ADR 3130). A file that throws one is a `[[test]]` of
/// its own, as ADR 3086 provides for a walk.
#[test]
fn no_module_of_a_suite_throws_a_process_wide_switch() {
    let root = repository_root();
    let members = conformance::roots::members(root).expect("the workspace manifest states members");
    let mut offenders = Vec::new();
    for member in members {
        let crate_dir = root.join(&member);
        let manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap_or_default();
        if !discovery_is_off(&manifest) {
            continue;
        }
        for test_root in declared_roots(&manifest, &crate_dir) {
            for module in modules_of(&test_root) {
                let text = fs::read_to_string(&module).unwrap_or_default();
                for switch in PROCESS_WIDE_SWITCHES {
                    if text.contains(switch) {
                        offenders.push(format!("  {} calls {switch}", module.display()));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these modules share a process with their suite's other tests under `cargo test` and \
         change it for good; make each a `[[test]]` of its own (ADR 3130):\n{}",
        offenders.join("\n")
    );
}
