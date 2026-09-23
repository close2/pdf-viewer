//! `tools/batch.sh commit` stages the whole population, and `tools/batch.sh close` refuses a
//! worktree holding work nobody committed.
//!
//! Not a conformance question, and it lives here for `bounded.rs`'s reason: this is the crate
//! whose gates read the repository's own tools, and a guard broken by an edit should fail the
//! round that broke it rather than the merge that trusts it.
//!
//! # What it guards
//!
//! The merge of batch thirty-five chained `git add`, `git commit`, `git merge --ff-only` and
//! `tools/batch.sh close` on one line. `git add` refused a pathspec — a file already staged as
//! deleted, which matches nothing on disk — the `;` let the commit run with that one deletion, the
//! fast-forward took it, and `close`, which removes the worktree with `--force`, deleted 128
//! uncommitted files. ADR 1313 records the incident and the two guards; these tests hold both
//! against a throwaway repository, never against the batch's own worktree.
//!
//! - `close` refuses a worktree with anything uncommitted outside `scratchpad/`, names what is
//!   there, and removes nothing; a spelling of `--force` is refused rather than read as a branch.
//! - `commit` stages by name the same population `close` refuses on, counts it against the index,
//!   and commits the incident's own shape — a staged deletion beside new and modified files —
//!   whole, leaving `scratchpad/` out.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot build a throwaway repository has not found a defect, \
              and reporting that as one would be worse than stopping"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. This
    // cannot fail for a crate that is in the workspace, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// A repository with `main`, one tracked file, and a copy of `tools/batch.sh` — so that the
/// script's own root, which it derives from where it lives, is this repository and not the tree.
struct Sandbox {
    base: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let base =
            std::env::temp_dir().join(format!("batch-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(base.join("repo/tools")).expect("a temporary directory");
        let sandbox = Self { base };
        std::fs::copy(
            repository_root().join("tools/batch.sh"),
            sandbox.repo().join("tools/batch.sh"),
        )
        .expect("tools/batch.sh copies");
        std::fs::write(sandbox.repo().join("a.txt"), "a\n").expect("a tracked file");
        sandbox.git(&sandbox.repo(), &["init", "-q", "-b", "main"]);
        sandbox.git(&sandbox.repo(), &["add", "a.txt", "tools/batch.sh"]);
        sandbox.git(&sandbox.repo(), &["commit", "-q", "-m", "base"]);
        let opened = sandbox.batch(&["open", "batch-test"]);
        assert!(opened.status.success(), "open failed: {}", text(&opened));
        sandbox
    }

    fn repo(&self) -> PathBuf {
        self.base.join("repo")
    }

    fn worktree(&self) -> PathBuf {
        self.base.join("wt")
    }

    fn command(&self, program: &str, directory: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(directory)
            .env("BATCH_WORKTREE", self.worktree())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@invalid")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@invalid");
        command
    }

    fn git(&self, directory: &Path, arguments: &[&str]) -> Output {
        let output = self
            .command("git", directory)
            .args(arguments)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            text(&output)
        );
        output
    }

    /// The script, run from the repository — outside the worktree, as `close` requires.
    fn batch(&self, arguments: &[&str]) -> Output {
        self.command("bash", &self.repo())
            .arg(self.repo().join("tools/batch.sh"))
            .args(arguments)
            .output()
            .expect("bash runs tools/batch.sh")
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.worktree().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a directory in the worktree");
        }
        std::fs::write(path, contents).expect("a file in the worktree");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn close_refuses_a_worktree_with_uncommitted_work_and_removes_nothing() {
    let sandbox = Sandbox::new("close");
    sandbox.write("new.txt", "new\n");
    sandbox.write("a.txt", "changed\n");

    let refused = sandbox.batch(&["close", "batch-test"]);
    let said = text(&refused);
    assert!(
        !refused.status.success(),
        "close removed a dirty worktree: {said}"
    );
    assert!(
        said.contains("2 uncommitted path(s)"),
        "the refusal does not count: {said}"
    );
    assert!(
        said.contains("new.txt") && said.contains("a.txt"),
        "the refusal does not name what is there: {said}"
    );
    assert!(
        sandbox.worktree().join("new.txt").exists(),
        "the refusal deleted the work"
    );

    let forced = sandbox.batch(&["close", "--force"]);
    assert!(
        !forced.status.success(),
        "close accepted --force: {}",
        text(&forced)
    );
    assert!(
        sandbox.worktree().join("new.txt").exists(),
        "--force deleted the work"
    );

    let misnamed = sandbox.batch(&["close", "batch-other"]);
    assert!(
        !misnamed.status.success(),
        "close accepted a branch the worktree is not on"
    );
}

#[test]
fn commit_stages_the_whole_population_and_close_then_needs_the_fast_forward() {
    let sandbox = Sandbox::new("commit");
    // The incident's own shape: a deletion already staged, beside new and changed files.
    sandbox.git(&sandbox.worktree(), &["rm", "-q", "a.txt"]);
    sandbox.write("b.txt", "b\n");
    sandbox.write("c/d.txt", "d\n");
    sandbox.write("scratchpad/r1/notes.txt", "never committed\n");
    sandbox.write("scratchpad/message", "the batch\n");

    let message = sandbox.worktree().join("scratchpad/message");
    let message = message.to_str().expect("a UTF-8 temporary path");
    let committed = sandbox.batch(&["commit", message]);
    let said = text(&committed);
    assert!(committed.status.success(), "commit failed: {said}");
    assert!(
        said.contains("population 3 path(s), staged 3 path(s)"),
        "the count is not printed: {said}"
    );
    assert!(
        said.contains("now 0 (must be 0)"),
        "work was left behind: {said}"
    );
    assert!(
        !said.contains("merge --ff-only batch-test;"),
        "commit chained the fast-forward: {said}"
    );

    let names = sandbox.git(
        &sandbox.worktree(),
        &["show", "--no-renames", "--name-status", "--format=", "HEAD"],
    );
    let names = String::from_utf8_lossy(&names.stdout);
    for line in ["D\ta.txt", "A\tb.txt", "A\tc/d.txt"] {
        assert!(
            names.lines().any(|name| name == line),
            "{line} is not in the commit:\n{names}"
        );
    }
    assert!(
        !names.contains("scratchpad/"),
        "scratchpad/ was committed:\n{names}"
    );

    let early = sandbox.batch(&["close", "batch-test"]);
    assert!(
        !early.status.success(),
        "close ran before the fast-forward: {}",
        text(&early)
    );
    assert!(
        sandbox.worktree().exists(),
        "the early close removed the tree"
    );

    sandbox.git(&sandbox.repo(), &["merge", "-q", "--ff-only", "batch-test"]);
    let closed = sandbox.batch(&["close", "batch-test"]);
    assert!(
        closed.status.success(),
        "close refused a finished batch: {}",
        text(&closed)
    );
    assert!(!sandbox.worktree().exists(), "close left the worktree");
}

#[test]
fn commit_refuses_an_empty_population_and_a_message_it_would_commit() {
    let sandbox = Sandbox::new("empty");
    sandbox.write("scratchpad/message", "the batch\n");
    let message = sandbox.worktree().join("scratchpad/message");
    let nothing = sandbox.batch(&["commit", message.to_str().expect("a UTF-8 temporary path")]);
    assert!(
        !nothing.status.success(),
        "commit made an empty commit: {}",
        text(&nothing)
    );

    sandbox.write("message.txt", "the batch\n");
    let inside = sandbox.worktree().join("message.txt");
    let inside = sandbox.batch(&["commit", inside.to_str().expect("a UTF-8 temporary path")]);
    assert!(
        !inside.status.success(),
        "commit committed its own message file: {}",
        text(&inside)
    );
}
