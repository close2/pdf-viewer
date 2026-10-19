//! `tools/wait.sh` is how a round waits for a command longer than the prompt cache's five minutes:
//! the command runs detached in a process group of its own, and every wait for it returns inside a
//! wall, so the agent's next request reads its context from the cache instead of writing it again
//! (ADR 1976). Its `--self-test` runs the cases a round leans on — `--start` returning as soon as
//! the run has started, a locked run queued behind a holder included, so that a start and a wait in
//! one call hold one wall (ADR 2025); a run's status returned, the wall coming first with 75, the
//! run outliving its watcher, `--stop` ending the group, a killed group said at once with 70, a
//! wall past the cache's lifetime refused, and a log the script did not start ending on
//! `tools/bounded.sh`'s closing line.
//!
//! It lives here for the reason `bounded.rs` does: this crate's gates read the repository's own
//! files, and a waiter broken by an edit should fail a round before that round waits on it.

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
fn the_waiters_self_test_holds() {
    let output = Command::new("bash")
        .arg(repository_root().join("tools/wait.sh"))
        .arg("--self-test")
        .current_dir(repository_root())
        .output()
        .expect("bash runs tools/wait.sh");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tools/wait.sh --self-test failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("every case holds"),
        "tools/wait.sh --self-test exited 0 without its closing line:\n{stdout}\n{stderr}"
    );
}
