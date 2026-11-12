//! `tools/xfa-census.py` counts, in each XFA resource `xfa_dynamic_census --dump` wrote, the
//! template grammar laying out a dynamic form's page one would run: page areas and content areas,
//! flowed subforms, `occur` bounds, subform sets, breaks, overflow, `draw` boilerplate, data
//! binding and the scripts that change an instance count (XFA 3.3 chapters 8 and 9;
//! `doc/todo/78`). Its `--self-test` plants one resource holding each, one holding none and one
//! that is not XML, and asserts every count — the instrument is shown to find what was planted
//! before its corpus answer is read (trap 13).
//!
//! It lives here for the reason `cache_census.rs` does: this crate's gates read the repository's
//! own files, and a census broken by an edit should fail a round before that round reports it.

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
fn the_xfa_census_counts_every_planted_element() {
    let output = Command::new("python3")
        .args(["-I", "-B"])
        .arg(repository_root().join("tools/xfa-census.py"))
        .arg("--self-test")
        .current_dir(repository_root())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("python3 runs tools/xfa-census.py");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tools/xfa-census.py --self-test failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("every case holds"),
        "tools/xfa-census.py --self-test exited 0 without its closing line:\n{stdout}\n{stderr}"
    );
}
