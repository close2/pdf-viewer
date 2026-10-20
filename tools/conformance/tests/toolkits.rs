//! ADR 2048's rule: the reference window and the C ABI depend on no widget toolkit.
//!
//! Not a conformance question, and it lives here for the reason `worker_features.rs` does: this is
//! the crate whose gates read the repository's own manifests and tools rather than a PDF.
//!
//! # What it guards
//!
//! `viewer-ui` is the reference window, on winit, whose initialisation the project owns end to end
//! (principle 2's page-one rule is stated and measured there), and `viewer-ffi` is the C ABI a host
//! outside Cargo — the .NET one of RFC 0010, a Swift one later — builds on. A toolkit belongs to
//! the one host that binds it: GTK 4 to `viewer-gtk` (ADR 0244), Qt 6 through `cxx` to `viewer-qt`
//! (ADR 0246). ADR 2048 states the command, `cargo tree -p <crate> -e normal,build -i <toolkit>`,
//! which must match no package for either crate. This reads each crate's whole graph once and
//! intersects it with the toolkits, which is the same answer as one `-i` per toolkit: a package is
//! in the graph exactly when `-i` would print a path to it.
//!
//! `--target all`, because a binding a manifest takes for one platform alone —
//! `[target.'cfg(windows)'.dependencies]` — is the likeliest way for a toolkit to arrive in a crate
//! whose gates run on Linux. `-e normal,build`, because a build script that finds and links Qt is a
//! dependency on Qt (it is how `viewer-qt` takes it), and a dev-dependency ships in no binary.
//!
//! # The population is derived (trap 25)
//!
//! **A toolkit is an external package a toolkit host names directly that none of that host's own
//! workspace dependencies reaches**, which takes `gtk4`, `pango`, `cxx` and `cxx-qt-build` from the
//! two manifests and leaves `thiserror`, which every crate here names; **and every `-sys` package
//! in the toolkit hosts' graphs that those workspace dependencies do not reach**, which is the C
//! libraries themselves (`gtk4-sys`, `glib-sys`, `cairo-sys-rs` and the rest), so a binding that
//! skipped the safe crate is caught too. `-sys` is Cargo's own convention for a package that links
//! a native library (the `links` key); it is a choice of this test, not a rule of Cargo's.
//!
//! The whole of the hosts' private graph is **not** the population, because it holds `memchr`,
//! `pkg-config` and `unicode-width`, which the reference reaches for reasons of its own. And one
//! name is added by hand: `cxx-qt`, the runtime crate the workspace manifest refuses by name, so
//! that taking it anywhere is a finding. It is in no lock file today, which this test prints
//! rather than counting it a tick.
//!
//! # Calibrated
//!
//! An instrument that cannot fail proves nothing (trap 13), so the same check is run over the two
//! hosts that do bind a toolkit, and must name `gtk4` in one and `cxx` in the other.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot run `cargo tree` over its own workspace has not found a \
              defect, and the census it prints is the point of a run that passes"
)]

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// The two crates ADR 2048 keeps free of every toolkit.
const GUARDED: [&str; 2] = ["viewer-ui", "viewer-ffi"];

/// The hosts that bind a toolkit, whose manifests the population is read from.
const TOOLKIT_HOSTS: [&str; 2] = ["viewer-gtk", "viewer-qt"];

/// Named by hand, for the reason the module comment gives.
const REFUSED_BY_NAME: &str = "cxx-qt";

/// The repository root, two levels above this crate's manifest.
fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`; a workspace member always has two
    // ancestors, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// One package of a `cargo tree` listing.
#[derive(Debug)]
struct Package {
    /// How far from the selected package it is; 0 is the package itself.
    depth: usize,
    name: String,
    /// Whether it is this repository's own, which `{p}` says by printing its path.
    local: bool,
}

/// `cargo tree` over the normal and build edges for every target, one package a line.
fn tree(selection: &[&str], depth: Option<&str>) -> Vec<Package> {
    let mut command = Command::new(env!("CARGO"));
    // `--color never`: the lines are parsed, and CI sets `CARGO_TERM_COLOR=always`.
    command
        .current_dir(repository_root())
        .args(["tree", "--locked", "--color", "never", "-e", "normal,build"])
        .args(["--target", "all", "--prefix", "depth", "-f", "{p}"])
        .args(selection);
    if let Some(depth) = depth {
        command.args(["--depth", depth]);
    }
    let output = command.output().expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo tree {selection:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let root = repository_root().to_string_lossy().into_owned();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let digits = line.chars().take_while(char::is_ascii_digit).count();
            let depth = line.get(..digits)?.parse().ok()?;
            let rest = line.get(digits..)?;
            let name = rest.split(' ').next()?.to_owned();
            Some(Package {
                depth,
                name,
                local: rest.contains(&format!("({root}")),
            })
        })
        .collect()
}

/// Every package's name in the graphs of `crates`, the crates themselves included.
fn graph(crates: &[&str]) -> BTreeSet<String> {
    let selection: Vec<&str> = crates.iter().flat_map(|name| ["-p", name]).collect();
    tree(&selection, None)
        .into_iter()
        .map(|package| package.name)
        .collect()
}

/// The toolkits, derived as the module comment says.
fn toolkits() -> BTreeSet<String> {
    let mut named = BTreeSet::new();
    let mut own = BTreeSet::new();
    for host in TOOLKIT_HOSTS {
        for package in tree(&["-p", host], Some("1")) {
            if package.depth != 1 {
                continue;
            }
            if package.local {
                own.insert(package.name);
            } else {
                named.insert(package.name);
            }
        }
    }
    let own: Vec<&str> = own.iter().map(String::as_str).collect();
    let shared = graph(&own);
    let mut found: BTreeSet<String> = named.difference(&shared).cloned().collect();
    found.extend(
        graph(&TOOLKIT_HOSTS)
            .difference(&shared)
            .filter(|name| name.ends_with("-sys") || name.ends_with("-sys-rs"))
            .cloned(),
    );
    found.insert(REFUSED_BY_NAME.to_owned());
    found
}

/// The toolkits `crate_name`'s graph reaches.
fn reached(crate_name: &str, toolkits: &BTreeSet<String>) -> BTreeSet<String> {
    graph(&[crate_name])
        .intersection(toolkits)
        .cloned()
        .collect()
}

#[test]
fn the_reference_window_and_the_c_abi_depend_on_no_widget_toolkit() {
    let toolkits = toolkits();
    // The derivation must have found what the two manifests visibly name; finding less is a broken
    // reader, and the check below it would then pass by reading nothing.
    for expected in ["gtk4", "cxx", "cxx-qt-build"] {
        assert!(
            toolkits.contains(expected),
            "the toolkit population lost {expected}: {toolkits:?}"
        );
    }
    let locked = graph(&TOOLKIT_HOSTS);
    let absent: Vec<&String> = toolkits
        .iter()
        .filter(|name| !locked.contains(*name))
        .collect();
    println!(
        "{} toolkit packages: {toolkits:?}; in no toolkit host's graph today: {absent:?}",
        toolkits.len()
    );

    // Calibration: the check run over a crate that does bind a toolkit names it.
    assert!(
        reached("viewer-gtk", &toolkits).contains("gtk4"),
        "the check does not see gtk4 in viewer-gtk, so it would not see it anywhere"
    );
    assert!(
        reached("viewer-qt", &toolkits).contains("cxx"),
        "the check does not see cxx in viewer-qt, so it would not see it anywhere"
    );

    for guarded in GUARDED {
        let found = reached(guarded, &toolkits);
        assert!(
            found.is_empty(),
            "{guarded} depends on a widget toolkit, which ADR 2048 gives to the one host that binds \
             it: {found:?} (`cargo tree -p {guarded} -e normal,build --target all -i <name>` says \
             how)"
        );
        println!("{guarded}: no toolkit in its graph");
    }
}
