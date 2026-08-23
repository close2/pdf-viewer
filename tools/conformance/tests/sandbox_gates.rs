//! Every gate in `doc/todo/02` §2's sequence either requires the sandboxed image decoder or says
//! why it does not.
//!
//! Not a conformance question, and it lives here for the reason `submodules.rs` does: this is the
//! crate whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by `pdf-sandbox`'s separate worker
//! program, and **Cargo does not build another package's binaries when it tests this one** — trap
//! 10. A gate line run on its own therefore measures a build that draws every other image and none
//! of those three. Nothing fails: the documents open, the pages draw, and a count comes out that is
//! about the build rather than about the tree.
//!
//! That is not hypothetical and it is not small. **One binary, run twice in one directory, reads
//! nine structure elements differently** depending on whether the worker is beside it: the nine are
//! `issue5481.pdf`'s, whose `JPXDecode` image drew nothing, so §14.8.3.3 could derive no rectangle
//! for them — and the accessibility census's ratchet passes on one of those readings and fails on
//! the other. Four rounds read that difference as a build directory, as staleness, and as Cargo
//! feature unification before the seven-hundredth ran the two conditions against the same binary
//! (ADR 0557, and `doc/traps/instruments-and-reports.md` traps 10 and 16).
//!
//! # Why the sequence is the population
//!
//! `doc/todo/02-every-round.md` §2 **owns** the gate sequence and nothing else states it, which
//! makes its command block the one list that cannot drift from what a round runs. So this gate
//! reads that block rather than a list of its own: a line added there is a gate this check demands
//! an answer from, and a gate deleted there stops being asked. A list here would be a second copy
//! of the sequence, which is the drift ADR 0232 §4 is about.
//!
//! # The exemption, and why it is a sentence rather than a silence
//!
//! A gate that never decodes an image does not need the worker, and `dates` and `xmp` are two. They
//! say so in their own file, in a line beginning [`EXEMPTION`], with the reason after it — because
//! the failure this gate exists to catch is a gate that *forgot*, and a forgetting and a decision
//! look identical from outside the file. Writing the reason down is what tells them apart.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::path::{Path, PathBuf};

/// The marker a gate carries to say it refuses to measure without the decoder.
///
/// The name of the function every one of them defines. A literal rather than a parsed call graph:
/// what is being checked is that somebody thought about it, and a function with this name that is
/// never called would be caught by the calibration this gate was written against — `doc/adr/0557`
/// records planting exactly that.
const REQUIREMENT: &str = "require_the_sandbox";

/// The line a gate that needs no decoder carries instead, with its reason after the colon.
const EXEMPTION: &str = "// no sandbox worker:";

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

/// One `cargo test … -p <package> --test <target>` line of the sequence.
#[derive(Debug, PartialEq, Eq)]
struct Gate {
    /// The package the line names.
    package: String,
    /// The integration-test target the line names.
    target: String,
}

/// Every gate line in `doc/todo/02` §2's command block.
///
/// Parsed from the whole file rather than from the block alone: `cargo test … --test …` appears
/// nowhere else in it, so narrowing to the fenced block would buy nothing and would add a second
/// thing that can break when the document is edited.
fn gates(root: &Path) -> Vec<Gate> {
    let path = root.join("doc/todo/02-every-round.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{} is this gate's population: {why}", path.display()));

    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("cargo test") {
            continue;
        }
        let mut fields = line.split_whitespace();
        let mut package = None;
        let mut target = None;
        while let Some(field) = fields.next() {
            match field {
                "-p" => package = fields.next().map(str::to_owned),
                "--test" => target = fields.next().map(str::to_owned),
                _ => {}
            }
        }
        if let (Some(package), Some(target)) = (package, target) {
            let gate = Gate { package, target };
            if !found.contains(&gate) {
                found.push(gate);
            }
        }
    }
    found
}

/// Where a package's integration test lives, given the workspace's two member directories.
fn source_of(root: &Path, gate: &Gate) -> PathBuf {
    for members in ["crates", "tools"] {
        let path = root
            .join(members)
            .join(&gate.package)
            .join("tests")
            .join(format!("{}.rs", gate.target));
        if path.is_file() {
            return path;
        }
    }
    panic!(
        "doc/todo/02 §2 names `-p {} --test {}` and neither crates/ nor tools/ holds that file — \
         either the gate moved or the sequence is stale",
        gate.package, gate.target
    );
}

#[test]
fn every_gate_in_the_sequence_answers_for_the_sandboxed_decoder() {
    let root = repository_root();
    let gates = gates(root);

    assert!(
        gates.len() > 5,
        "doc/todo/02 §2 yielded {} gate lines, which means this check is measuring nothing — \
         either the sequence moved out of that file or the parse stopped working",
        gates.len()
    );

    let mut silent = Vec::new();
    let mut requiring = 0_usize;
    for gate in &gates {
        let path = source_of(root, gate);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|why| panic!("{} could not be read: {why}", path.display()));
        if text.contains(REQUIREMENT) {
            requiring += 1;
        } else if !text.contains(EXEMPTION) {
            silent.push(format!("-p {} --test {}", gate.package, gate.target));
        }
    }

    assert!(
        requiring > 0,
        "no gate in the sequence carries `{REQUIREMENT}`, so this check would pass a tree in \
         which every one of them had lost it — the marker's name has probably changed"
    );

    assert!(
        silent.is_empty(),
        "these gates neither require the sandboxed decoder nor say why they need none — a run \
         without `pdf-sandbox-worker` measures a build that decodes no CCITT, JBIG2 or JPEG 2000 \
         image, and the numbers move without failing (trap 10, trap 16, ADR 0557). Call \
         `{REQUIREMENT}()` at the top of the gate, or put a line beginning `{EXEMPTION}` in the \
         file with the reason: {silent:?}"
    );
}
