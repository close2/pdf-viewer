//! `tools/cache-census.py` is how a round answers whether the prompt cache held: it reads the
//! agents' transcripts and names every request that wrote again what the one before had left
//! cached. Its `--self-test` plants one transcript at each lifetime with each account of a miss in
//! it — a first request, a compaction, a gap over five minutes while a command ran, a gap over the
//! hour after a turn's end, a prefix change named by its attachment, and one no account explains —
//! together with a run of keep-warm re-issues of `tools/wait.sh`, a request sent with nothing
//! between it and the one before, and a main session's bookkeeping record, and asserts that the
//! census names each — the instrument is shown to find what was planted before its clean answer
//! is read (trap 13; ADR 1976 is the census's argument).
//!
//! It lives here for the reason `wait.rs` does: this crate's gates read the repository's own files,
//! and a census broken by an edit should fail a round before that round reports its figures.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot run `python3` over its own tools directory has not \
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
fn the_cache_census_names_every_planted_miss() {
    let output = Command::new("python3")
        .args(["-I", "-B"])
        .arg(repository_root().join("tools/cache-census.py"))
        .arg("--self-test")
        .current_dir(repository_root())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("python3 runs tools/cache-census.py");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tools/cache-census.py --self-test failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("every case holds"),
        "tools/cache-census.py --self-test exited 0 without its closing line:\n{stdout}\n{stderr}"
    );
}
