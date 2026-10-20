//! `tools/rustdoc-brackets.py` is how a crate joins the docs gate: it reads rustdoc's own error
//! log and makes ADR 1938's two mechanical answers — a letter a quotation lowered escaped
//! (`"\[t]he`), a link from a public page to a private item made a code span — at the bracket
//! rustdoc's column points into, and lists every other error for a hand. Its `--self-test`
//! plants a file and a log with one of each class: two letters on one line (edited from the
//! right, so the first edit does not move the second's column), a private link, a link with an
//! explicit target, a backticked name that does not resolve, a labelled link and its reference
//! definition, a sentence's second line that only looks like a definition, the same location
//! reported twice, a column one character off the bracket and a bracket in code. The last two
//! must stop the run with nothing written (habit 135); the instrument is shown to answer what was
//! planted before its answer over a crate is read (trap 13).
//!
//! It lives here for the reason `cache_census.rs` does: this crate's gates read the repository's
//! own files, and a script broken by an edit should fail a round before the round runs it.

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
fn the_bracket_script_answers_every_planted_error() {
    let output = Command::new("python3")
        .args(["-I", "-B"])
        .arg(repository_root().join("tools/rustdoc-brackets.py"))
        .arg("--self-test")
        .current_dir(repository_root())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("python3 runs tools/rustdoc-brackets.py");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tools/rustdoc-brackets.py --self-test failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("every case holds"),
        "tools/rustdoc-brackets.py --self-test exited 0 without its closing line:\n{stdout}\n{stderr}"
    );
}
