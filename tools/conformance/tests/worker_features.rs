//! The sandbox worker's decoders are built with the features its own manifest asks for, whatever
//! else the build selects.
//!
//! Not a conformance question, and it lives here for the reason `sandbox_gates.rs` does: this is
//! the crate whose gates read the repository's own files and tools rather than a PDF.
//!
//! # What it guards
//!
//! **Cargo unifies a package's features over every package a build selects** (resolver 2 and
//! later), and `cargo build --release --bin …` with no `-p` selects every member of this virtual
//! workspace. So a feature another member turns on in a dependency the worker shares is in the
//! worker's copy too, wherever the worker is built in the same run as the programs. That is not
//! hypothetical: `tools/hayro-compare`'s `hayro` turns on `hayro-syntax`'s `unsafe` feature, which
//! turns on `hayro-jbig2/simd`, and with `hayro-jbig2` taken from crates.io the worker's decoder of
//! untrusted JBIG2 bytes was built with `fearless_simd, simd, std` where its stanza asks for `std`
//! and `default-features = false` — the one `unsafe` that stanza exists to leave out (principle 3,
//! ADR 1714). Taken from a git source it is a different package from the reference renderer's copy,
//! and gets `std`; nothing but this check says so the day a stanza moves.
//!
//! **So the worker is built alone.** The DCT encoder brought `bytemuck` into the worker's graph
//! (ADR 2138), and the windows' graph turns on five of its features — `derive` and
//! `extern_crate_alloc` among them — that no stanza of the worker asks for; no git source makes
//! that a different package. `tools/batch.sh install` and CI's release build therefore build
//! `pdf-sandbox-worker` in a Cargo run that selects `pdf-sandbox` and nothing else, and this file
//! holds both lines, the resolution that run makes, and a census of what a whole-workspace build
//! would add — printed, because it is the reason for the separate run rather than a defect.
//!
//! # The instrument, and why it is not the unit graph
//!
//! ADR 1714 read it from `cargo +nightly build --release --bin pdf-sandbox-worker --unit-graph -Z
//! unstable-options`, which is the build's own answer and needs a nightly toolchain. This reads
//! `cargo tree -e normal -f '{p}|{f}'` instead, on the stable toolchain the tree builds with, so it
//! runs wherever `cargo test -p conformance` does, CI included: `--workspace` is the resolution of a
//! build that selects every member, and `-p pdf-sandbox` the resolution the worker's own manifest
//! makes. The two were read against each other on 2026-10-08 (ADR 1718): the unit graph gave the
//! worker's three `hayro` units `std`, `std` and none, as `--workspace` does, and both gave
//! `fearless_simd, simd, std` to `hayro-jbig2` with `--features hayro-jbig2/simd` planted.
//!
//! **The population is derived** (trap 25): every package reachable from the unconditional
//! `[dependencies]` table of `crates/pdf-sandbox/Cargo.toml` — the codecs, which that manifest keeps
//! apart from the confinement's target-specific tables — not descending into a procedural macro,
//! which runs in the compiler and is no part of the worker. For each, the features the
//! whole-workspace resolution gives it must be the features the worker's own resolution gives it.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot run `cargo tree` over its own workspace has not found a \
              defect, and the census it prints is the point of a run that passes"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// The repository root, two levels above this crate's manifest.
fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`; a workspace member always has two
    // ancestors, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// `cargo tree` over the normal-dependency graph, one `{p}|{f}` line a package, with `selection`
/// choosing which packages the resolution is made for.
fn tree(selection: &[&str], prefix: &str, dedupe: bool) -> String {
    let mut command = Command::new(env!("CARGO"));
    // `--color never`: the lines are parsed, not read, and CI sets `CARGO_TERM_COLOR=always`, under
    // which the `(*)` a repeated node carries arrives wrapped in escapes that `package_and_features`
    // then takes for a feature (ADR 1879 section 5).
    command
        .current_dir(repository_root())
        .args([
            "tree", "--locked", "--color", "never", "-e", "normal", "--prefix", prefix,
        ])
        .args(["-f", "{p}|{f}"])
        .args(selection);
    if !dedupe {
        command.arg("--no-dedupe");
    }
    let output = command.output().expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo tree {selection:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// One `{p}|{f}` line as the package and its features, with the `(*)` a repeated node carries
/// after its format taken off.
fn package_and_features(line: &str) -> Option<(String, BTreeSet<String>)> {
    let (package, features) = line.rsplit_once('|')?;
    let features = features.trim().trim_end_matches("(*)").trim();
    Some((
        package.trim().to_owned(),
        features
            .split(',')
            .map(str::trim)
            .filter(|feature| !feature.is_empty())
            .map(str::to_owned)
            .collect(),
    ))
}

/// The names in the unconditional `[dependencies]` table of a manifest.
fn unconditional_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut inside = false;
    let mut names = BTreeSet::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[dependencies]";
            continue;
        }
        if inside
            && !line.starts_with('#')
            && let Some(name) = line.split(['.', '=', ' ']).next()
            && !name.is_empty()
        {
            names.insert(name.to_owned());
        }
    }
    names
}

/// The packages of a `--prefix depth --no-dedupe` tree reachable from the depth-1 nodes `roots`
/// names, with their features, not descending into a procedural macro: a `--no-dedupe` tree prints
/// a node's dependencies deeper than it, straight after it, so a subtree ends at the next line as
/// shallow as its root.
fn decoder_units(depth_tree: &str, roots: &BTreeSet<String>) -> BTreeMap<String, BTreeSet<String>> {
    let mut units: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut in_root = false;
    let mut in_macro_below: Option<usize> = None;
    for line in depth_tree.lines() {
        let digits = line.chars().take_while(char::is_ascii_digit).count();
        let Ok(depth) = line.get(..digits).unwrap_or_default().parse::<usize>() else {
            continue;
        };
        let Some((package, features)) =
            package_and_features(line.get(digits..).unwrap_or_default())
        else {
            continue;
        };
        if depth <= 1 {
            let name = package.split_whitespace().next().unwrap_or_default();
            in_root = depth == 1 && roots.contains(name);
            in_macro_below = None;
        }
        if in_macro_below.is_some_and(|macro_depth| depth <= macro_depth) {
            in_macro_below = None;
        }
        if !in_root || in_macro_below.is_some() {
            continue;
        }
        if package.contains("(proc-macro)") {
            in_macro_below = Some(depth);
            continue;
        }
        units.entry(package).or_default().extend(features);
    }
    units
}

/// The decoder units whose features in `shipped` go beyond what `own` gives them, one line each.
fn inherited(own: &BTreeMap<String, BTreeSet<String>>, shipped: &str) -> Vec<String> {
    let mut resolved: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in shipped.lines() {
        if let Some((package, features)) = package_and_features(line) {
            resolved.entry(package).or_default().extend(features);
        }
    }
    own.iter()
        .filter_map(|(package, features)| {
            let extra: Vec<&String> = resolved
                .get(package)
                .map(|shipped| shipped.difference(features).collect())
                .unwrap_or_default();
            (!extra.is_empty()).then(|| {
                format!(
                    "{package}: the resolution read turns on {} beyond the worker's own {:?}",
                    extra
                        .iter()
                        .map(|feature| feature.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    features
                )
            })
        })
        .collect()
}

/// The worker's decoder units, as its own resolution makes them, and the roots they were walked
/// from.
fn worker_decoders() -> (BTreeMap<String, BTreeSet<String>>, BTreeSet<String>) {
    let manifest = std::fs::read_to_string(repository_root().join("crates/pdf-sandbox/Cargo.toml"))
        .expect("pdf-sandbox's manifest is in the tree");
    let roots = unconditional_dependencies(&manifest);
    let own = decoder_units(&tree(&["-p", "pdf-sandbox"], "depth", false), &roots);
    (own, roots)
}

/// The readers are the shape they state. Calibrated by planting (trap 13): the walk keeps a
/// root's subtree and drops the confinement's, steps over a macro's subtree and resumes after it,
/// and a repeated node's `(*)` is not a feature.
#[test]
fn the_readers_take_the_codecs_and_leave_the_rest() {
    let manifest = "[package]\nname = \"x\"\n[dependencies]\n# a comment\ncodec.workspace = true\n\
                    errors = { workspace = true }\n[target.'cfg(unix)'.dependencies]\nconfinement.workspace = true\n";
    let roots = unconditional_dependencies(manifest);
    assert_eq!(
        roots.iter().map(String::as_str).collect::<Vec<_>>(),
        ["codec", "errors"]
    );
    let planted = "0x v0.1.0 (/x)|\n1codec v1.0.0|std\n2inner v2.0.0|alloc\n1errors v2.0.0|default,std\n\
                   2errors-impl v2.0.0 (proc-macro)|\n3syn v2.0.0|full\n2after v1.0.0|\n1confinement v1.0.0|net\n\
                   2inner v2.0.0|alloc,net (*)\n";
    let units = decoder_units(planted, &roots);
    assert_eq!(
        units
            .iter()
            .map(|(package, features)| format!(
                "{package}:{}",
                features
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            ))
            .collect::<Vec<_>>(),
        [
            "after v1.0.0:",
            "codec v1.0.0:std",
            "errors v2.0.0:default,std",
            "inner v2.0.0:alloc"
        ],
        "the walk is not the shape it states"
    );
    assert_eq!(
        inherited(&units, "codec v1.0.0|simd,std\ninner v2.0.0|alloc (*)\n"),
        ["codec v1.0.0: the resolution read turns on simd beyond the worker's own {\"std\"}"]
    );
}

/// **The build that ships the worker gives its decoders the worker's own features and no more**
/// (principle 3, ADR 1714). Calibrated by planting through Cargo itself (trap 13): the same check
/// over a resolution with `hayro-jbig2/simd` turned on names that package and those two features.
#[test]
fn the_shipped_worker_inherits_no_feature_another_member_turns_on() {
    let (own, roots) = worker_decoders();
    for (package, features) in &own {
        println!(
            "worker decoder unit: {package} [{}]",
            features
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    assert!(
        own.keys()
            .any(|package| package.starts_with("hayro-jbig2 "))
            && own
                .keys()
                .any(|package| package.starts_with("hayro-jpeg2000 ")),
        "the walk from {roots:?} reached neither JBIG2 nor JPEG 2000 decoder: the population is not \
         the worker's"
    );

    let planted = inherited(
        &own,
        &tree(
            &["-p", "pdf-sandbox", "--features", "hayro-jbig2/simd"],
            "none",
            true,
        ),
    );
    assert!(
        planted.len() == 1
            && planted
                .first()
                .is_some_and(|line| line.starts_with("hayro-jbig2 ")
                    && line.contains("turns on fearless_simd, simd beyond")),
        "a planted `hayro-jbig2/simd` was not named as itself: {planted:?}"
    );

    let shipped = inherited(&own, &tree(&["-p", "pdf-sandbox"], "none", true));
    assert!(
        shipped.is_empty(),
        "the worker built alone carries features its own manifest does not ask for:\n{}",
        shipped.join("\n")
    );
    for line in inherited(&own, &tree(&["--workspace"], "none", true)) {
        println!("a whole-workspace build would add, and the separate run keeps out: {line}");
    }
}

/// **The two builds that ship the worker build it alone** (ADRs 1714, 2138): `tools/batch.sh
/// install` names it in `install_alone`, leaves it out of the programs' one invocation and builds
/// it with `-p pdf-sandbox`, and CI's release build does the same. Read as text, because the
/// property above is the resolution of exactly those lines and holds of no other build.
#[test]
fn the_builds_that_ship_the_worker_select_its_package_alone() {
    let read = |path: &str| {
        std::fs::read_to_string(repository_root().join(path)).expect("the file is in the tree")
    };
    let batch = read("tools/batch.sh");
    let alone = batch
        .lines()
        .find_map(|line| line.strip_prefix("install_alone=\""))
        .and_then(|rest| rest.split_once('"'))
        .map(|(words, _)| words.split_whitespace().collect::<Vec<_>>())
        .unwrap_or_default();
    assert!(
        alone.contains(&"pdf-sandbox-worker:pdf-sandbox"),
        "tools/batch.sh's install_alone does not name the worker and its package: {alone:?}"
    );
    for line in [
        r#"for alone in $install_alone; do [ "$name" = "${alone%%:*}" ] && continue 2; done"#,
        r#"cargo build --release -p "${entry#*:}" --bin "${entry%%:*}""#,
    ] {
        assert!(
            batch.contains(line),
            "tools/batch.sh install no longer builds the worker alone: `{line}` is gone"
        );
    }
    let workflow = read(".github/workflows/ci.yml");
    for line in [
        r#"alone="pdf-sandbox-worker:pdf-sandbox""#,
        r#"[ "$binary" = "${alone%%:*}" ] && continue"#,
        r#"cargo build --release -p "${alone#*:}" --bin "${alone%%:*}""#,
    ] {
        assert!(
            workflow.contains(line),
            "CI's release build no longer builds the worker alone: `{line}` is gone"
        );
    }
}
