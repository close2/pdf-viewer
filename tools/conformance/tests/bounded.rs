//! `tools/bounded.sh` is the wrapper every corpus walk, census and build runs under, and the
//! memory ceiling it enforces is what keeps the machine up (`doc/environment.md`'s parallel-round
//! agreements, ADR 0798). Its `--self-test` exercises the half of it that a gate can see without
//! a corpus: the sampler that measures the tree, the ceiling's kill, and the wrapper's own
//! answer to a sampler that stalls.
//!
//! Not a conformance question, and it lives here for the reason `sandbox_gates.rs`,
//! `submodules.rs` and `workspaces.rs` do: this is the crate whose gates read the repository's
//! own files rather than a PDF, and `cargo test -p conformance` is the sequence's last line, so
//! a wrapper broken by an edit fails a round before that round walks anything under it.
//!
//! # What it guards
//!
//! Until the eight-hundred-and-eightieth round the sampler walked the process table with an inner
//! loop over every process for every node of the tree — quadratic, and 6 s a sample over a tree
//! of 8 000 processes — with no guard against visiting a pid twice and no bound on how long the
//! `ps` underneath it might take under exactly the memory pressure the ceiling exists to prevent.
//! Round 874 watched one sample hang for minutes and killed it by pid. **A bound that is not
//! being measured is a bound that is not there**, which is `doc/traps/instruments-and-reports.md`'s
//! trap 18 read from the other side: there the limit destroyed the channel that reports it; here
//! the channel that measures the limit could stop, and nothing said so.
//!
//! The self-test's five cases are the script's own (`tools/bounded.sh --self-test` prints one line
//! each): a synthetic table of a hundred thousand children sampled in a fraction of the interval,
//! a chain, a cycle and a duplicated row walked once each, a live tree that fans out, a child
//! over the ceiling stopped with exit 137, and a sampler that never returns stopping the tree
//! after the stated number of missed samples. This test runs the script and repeats what it said.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot run `bash` over its own tools directory has not \
              found a defect, and reporting that as one would be worse than stopping"
)]

use std::path::Path;
use std::process::Command;

fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. This
    // cannot fail for a crate that is in the workspace, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

#[test]
fn the_bounded_wrappers_self_test_holds() {
    let script = repository_root().join("tools/bounded.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg("--self-test")
        .current_dir(repository_root())
        .output()
        .expect("bash runs tools/bounded.sh");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tools/bounded.sh --self-test failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("every case holds"),
        "tools/bounded.sh --self-test exited 0 without its closing line:\n{stdout}\n{stderr}"
    );
    // A case that could not run says so on standard error rather than passing quietly; a machine
    // without `python3` is the one such case, and CI has it, so here it is a failure to read.
    assert!(
        !stderr.contains("NOT RUN"),
        "a self-test case did not run:\n{stderr}"
    );
}
