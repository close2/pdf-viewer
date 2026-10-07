//! `tools/batch.sh commit` stages the whole population, `tools/batch.sh close` refuses a worktree
//! holding work nobody committed, and `tools/batch.sh install` puts what a person runs into the
//! main checkout's `target/` and nowhere else.
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
//! - `check` reads that same population for its findings about added files, so a path git would
//!   print quoted is placed by its real directory: under `scratchpad/` it is no finding, and
//!   anywhere else it is named as itself.
//! - Every `cargo test … --test` line of `gates()` and of `doc/todo/02` names a file whose
//!   `#[ignore]` attributes match the flag the line gives it, so no gate is green having run zero
//!   tests (ADR 1392).
//! - Every subcommand runs under the agent's task budget, the figure `tools/bounded.sh` writes once,
//!   and `check` prints the user's task count and the limit the calling shell held on one line, so a
//!   merge made without the bound is legible (trap 116, ADR 1612).
//! - `install` refuses a worktree with uncommitted work and a batch `main` has not been
//!   fast-forwarded to, then builds the programs and libraries a person runs, installs them into
//!   the main checkout's `target/` — the one path outside the worktree this script writes — and
//!   writes the commit they were built from beside them (ADR 1511). Its names are every program of
//!   a package under `crates/`, `quorra-retrieve`, and every C library, held against the
//!   workspace's own manifests.
//! - `arms` exports HEAD's six corpus arms only from a worktree holding nothing uncommitted, into
//!   the directory the branch's first session names, and never over another commit's export
//!   (ADR 1650).
//! - `open` returns while the warm build and the arms export it started still run, each in a
//!   session of its own holding none of the caller's descriptors; the export's hold of the
//!   heavy-walk lock is a line of the lock's log; and `gates` reads its two clocks off the
//!   wrapper's last line, which says them (ADR 1662).

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
    /// `BATCH_DEBUG_RULE_KIB`, where a test plants a `debug` tree over a rule of its own.
    debug_rule_kib: Option<u64>,
    /// What `open` printed.
    opened: String,
    /// Whether `open` finds a workspace and starts its two detached jobs, with a stand-in `cargo`
    /// on the path that only sleeps and a heavy-walk lock of the sandbox's own.
    detaching: bool,
    /// The process groups `open` detached, stopped by pid when the sandbox goes.
    detached: Vec<String>,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        Self::with_files(name, &[])
    }

    /// As [`Sandbox::new`], with `files` committed on `main` before the batch opens.
    fn with_files(name: &str, files: &[(&str, &str)]) -> Self {
        Self::build(name, files, None, false)
    }

    /// As [`Sandbox::new`], on a branch that names its first session, with a workspace manifest
    /// committed, so that `open` starts the warm build and the arms export; both run the stand-in
    /// `cargo`, which sleeps for [`STAND_IN_SECONDS`].
    fn detaching(name: &str) -> Self {
        Self::build(name, &[("Cargo.toml", "[workspace]\n")], None, true)
    }

    /// As [`Sandbox::new`], with `planted_kib` of real bytes in the build directory's `debug`
    /// before the batch opens and `rule_kib` as the rule it is read against.
    fn with_debug_tree(name: &str, planted_kib: usize, rule_kib: u64) -> Self {
        Self::build(name, &[], Some((planted_kib, rule_kib)), false)
    }

    fn build(
        name: &str,
        files: &[(&str, &str)],
        debug: Option<(usize, u64)>,
        detaching: bool,
    ) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let base =
            std::env::temp_dir().join(format!("batch-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(base.join("repo/tools")).expect("a temporary directory");
        let mut sandbox = Self {
            base,
            debug_rule_kib: debug.map(|(_, rule)| rule),
            opened: String::new(),
            detaching,
            detached: Vec::new(),
        };
        if detaching {
            std::fs::create_dir_all(sandbox.base.join("bin"))
                .expect("a directory for the stand-in");
            let cargo = sandbox.base.join("bin/cargo");
            std::fs::write(
                &cargo,
                format!("#!/bin/sh\nexec sleep {STAND_IN_SECONDS}\n"),
            )
            .expect("the stand-in cargo");
            let made_executable = Command::new("chmod")
                .arg("+x")
                .arg(&cargo)
                .status()
                .is_ok_and(|status| status.success());
            assert!(
                made_executable,
                "the stand-in cargo could not be made executable"
            );
        }
        if let Some((planted, _)) = debug {
            let tree = sandbox.target().join("debug/deps");
            std::fs::create_dir_all(&tree).expect("a planted debug tree");
            // Bytes that are not zero, so no filesystem stores the file sparse and `du` counts it.
            std::fs::write(
                tree.join("planted"),
                vec![0x5a_u8; planted.saturating_mul(1024)],
            )
            .expect("a planted artefact");
        }
        // `tools/bounded.sh` beside it, because the script reads the agent's task budget there.
        for script in ["tools/batch.sh", "tools/bounded.sh"] {
            std::fs::copy(repository_root().join(script), sandbox.repo().join(script))
                .expect("the scripts copy");
        }
        std::fs::write(sandbox.repo().join("a.txt"), "a\n").expect("a tracked file");
        for (relative, contents) in files {
            let path = sandbox.repo().join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("a directory in the repository");
            }
            std::fs::write(path, contents).expect("a file in the repository");
        }
        sandbox.git(&sandbox.repo(), &["init", "-q", "-b", "main"]);
        sandbox.git(&sandbox.repo(), &["add", "-A"]);
        sandbox.git(&sandbox.repo(), &["commit", "-q", "-m", "base"]);
        let branch = if detaching {
            "batch-9001-9006"
        } else {
            "batch-test"
        };
        let opened = sandbox.batch(&["open", branch]);
        assert!(opened.status.success(), "open failed: {}", text(&opened));
        sandbox.opened = text(&opened);
        sandbox
    }

    /// The batch build directory `tools/batch.sh` is pointed at: inside the sandbox, so that no
    /// test reads the machine's own (a hundred-gigabyte `du` per test, and a finding about it).
    fn target(&self) -> PathBuf {
        self.base.join("target")
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
            .env("BATCH_TARGET_DIR", self.target())
            // The throwaway workspace names its own build directory; an inherited one would put a
            // stand-in called `quorra` beside the real one.
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_BUILD_TARGET_DIR")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@invalid")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@invalid");
        if let Some(rule) = self.debug_rule_kib {
            command.env("BATCH_DEBUG_RULE_KIB", rule.to_string());
        }
        if self.detaching {
            let mut path = std::ffi::OsString::from(self.base.join("bin"));
            if let Some(inherited) = std::env::var_os("PATH") {
                path.push(":");
                path.push(inherited);
            }
            command
                .env("PATH", path)
                .env("BATCH_ARMS_ROOT", &self.base)
                .env("HEAVY_WALK_LOCK", self.lock())
                .env("HEAVY_WALK_LOG", self.base.join("heavy-walk.log"));
        } else {
            // `open` warms a workspace it finds; a throwaway one is built by the test that needs it.
            command.env("BATCH_WARM", "0");
        }
        command
    }

    /// The heavy-walk lock of a detaching sandbox: its own file, so that no test queues behind a
    /// real walk and no real walk behind a test.
    fn lock(&self) -> PathBuf {
        self.base.join("heavy-walk.lock")
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
        for group in &self.detached {
            stop_group(group);
        }
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// How long the stand-in `cargo` of a detaching sandbox sleeps: far longer than `open` may take,
/// so that an `open` still holding its caller's pipe when it returns is a failure by a margin.
const STAND_IN_SECONDS: u64 = 60;

/// Sends `TERM` to the process group `group` leads, by its number: `open` gives each detached job
/// a session of its own, so the group is that job and nothing else.
fn stop_group(group: &str) {
    let _ = Command::new("kill")
        .args(["-TERM", "--", &format!("-{group}")])
        .status();
}

/// Whether the process group `group` leads still has a member.
fn group_runs(group: &str) -> bool {
    Command::new("kill")
        .args(["-0", "--", &format!("-{group}")])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The pid `open` printed after `what`, as `<what>: pid <n>, <log>`.
fn detached_pid(opened: &str, what: &str) -> Option<String> {
    let rest = opened.split_once(&format!("{what}: pid "))?.1;
    let pid: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (!pid.is_empty()).then_some(pid)
}

/// Polls `condition` every tenth of a second for at most `seconds`.
fn within(seconds: u64, condition: impl Fn() -> bool) -> bool {
    for _ in 0..seconds.saturating_mul(10) {
        if condition() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    condition()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `open` and `close` print the batch directory's `debug` and the prune once it is over the rule,
/// and nothing under it; neither prunes. Calibrated both ways (trap 13): the same planted tree
/// under a rule above it is silent.
#[test]
fn open_and_close_print_the_prune_once_debug_is_over_the_rule_and_prune_nothing() {
    let over = Sandbox::with_debug_tree("over", 2048, 1024);
    let prune = format!("rm -rf {}/debug", over.target().display());
    assert!(
        over.opened.contains("over the 100 GB rule") && over.opened.contains(&prune),
        "open did not print the prune for a debug tree over the rule: {}",
        over.opened
    );
    let closed = over.batch(&["close", "batch-test"]);
    let said = text(&closed);
    assert!(closed.status.success(), "close failed: {said}");
    assert!(
        said.contains(&prune),
        "close did not print the prune for a debug tree over the rule: {said}"
    );
    assert!(
        over.target().join("debug/deps/planted").exists(),
        "open or close pruned the tree it was only to report"
    );

    let under = Sandbox::with_debug_tree("under", 2048, 1024 * 1024);
    let closed = under.batch(&["close", "batch-test"]);
    for said in [under.opened.clone(), text(&closed)] {
        assert!(
            !said.contains("rm -rf"),
            "a debug tree under the rule printed a prune: {said}"
        );
    }
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

/// `check`'s added-file findings read `population`, the listing `commit` stages from, so a path
/// `git status --porcelain` would print quoted and escaped — a space and a non-ASCII character —
/// is excluded by the directory it is really in, and one outside `scratchpad/` is named as itself.
#[test]
fn check_reads_the_population_commit_stages_so_a_quoted_path_is_placed_by_its_real_directory() {
    let sandbox = Sandbox::new("quoted");
    sandbox.write("scratchpad/r1/w/\u{5bf9} page one.pdf", "scratch\n");
    sandbox.write("copies/\u{e9}t\u{e9} copy.bin", "a stray binary\n");
    let checked = sandbox.batch(&["check"]);
    let report = text(&checked);
    let line = report
        .lines()
        .find(|line| line.starts_with("untracked, unexpected extension"))
        .expect("check prints the unexpected-extension line");
    assert!(
        line.ends_with("1 file(s)"),
        "the scratchpad path was counted, or the stray one missed: {report}"
    );
    assert!(
        report.contains("    copies/\u{e9}t\u{e9} copy.bin"),
        "the stray file is not named by its real path: {report}"
    );
    assert!(
        !report.contains("scratchpad/"),
        "a path under scratchpad/ was reported: {report}"
    );
}

/// The line of `check`'s report that starts with `label`.
fn check_line(report: &str, label: &str) -> String {
    let found = report.lines().find(|line| line.starts_with(label));
    assert!(found.is_some(), "check prints no `{label}` line: {report}");
    found.unwrap_or_default().to_owned()
}

/// `check` prints the user's tasks and the limit the calling shell held as one line, and says the
/// limit is above the budget only when it is (trap 13): run under a soft limit of the budget it is
/// not, and run under a soft limit of 7000 against a planted budget of 6000 it is. Both limits sit at
/// or under 8192, so the test needs no more than the hard limit `ulimit -u 8192` leaves, and 6000 is
/// still far above what the user holds while it runs. The figure is read from the copy of
/// `tools/bounded.sh` beside the script, which is the one place it is written.
#[test]
fn check_prints_the_tasks_held_and_the_limit_where_called() {
    let sandbox = Sandbox::new("tasks");
    let bounded_script = sandbox.repo().join("tools/bounded.sh");
    let budget = Command::new("bash")
        .arg(&bounded_script)
        .arg("--task-budget")
        .output()
        .expect("bash runs tools/bounded.sh --task-budget");
    let budget = String::from_utf8_lossy(&budget.stdout).trim().to_owned();
    assert_eq!(
        budget, "8192",
        "tools/bounded.sh --task-budget is the figure trap 116 states"
    );
    let label = "tasks of ";
    let under = |limit: &str| -> String {
        let output = sandbox
            .command("bash", &sandbox.repo())
            .arg("-c")
            .arg(format!(
                "ulimit -S -u {limit} && exec bash {} check",
                sandbox.repo().join("tools/batch.sh").display()
            ))
            .output()
            .expect("bash runs tools/batch.sh check");
        check_line(&text(&output), label)
    };
    let bounded = under(&budget);
    assert!(
        bounded.contains(&format!("ulimit -u {budget} where called")) && !bounded.contains("ABOVE"),
        "{bounded}"
    );
    let held = bounded
        .split("the bound")
        .nth(1)
        .and_then(|rest| rest.split(';').next())
        .map(str::trim)
        .unwrap_or_default();
    assert!(
        held.parse::<u32>().is_ok_and(|count| count > 0),
        "the line carries no task count: {bounded}"
    );
    let script = std::fs::read_to_string(&bounded_script).expect("the copied wrapper reads");
    assert_eq!(
        script.matches("\ntask_budget=8192\n").count(),
        1,
        "the budget is written once"
    );
    std::fs::write(
        &bounded_script,
        script.replace("\ntask_budget=8192\n", "\ntask_budget=6000\n"),
    )
    .expect("a planted budget");
    let above = under("7000");
    assert!(
        above.contains("ulimit -u 7000 where called, ABOVE the budget of 6000"),
        "{above}"
    );
}

/// `check` reads the root `Cargo.toml`'s `members` the way cargo does and names every one that is
/// not a crate git tracks — a crate a round made under `scratchpad/` and a `__pycache__` a `tools/*`
/// glob matches, the two shapes of trap 114 — and names any `__pycache__` under `tools/` or
/// `crates/` on a line of its own. Each is planted, seen, and removed, and the line goes back to
/// `none`, so neither line is one that always fires (trap 13).
#[test]
fn check_names_a_member_that_is_not_a_tracked_crate_and_a_pycache_under_the_globs() {
    let manifest = "[workspace]\nmembers = [\"crates/*\", \"tools/*\"]\nresolver = \"3\"\n";
    let crate_manifest = |name: &str| {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
    };
    let (one, two) = (crate_manifest("one"), crate_manifest("two"));
    let sandbox = Sandbox::with_files(
        "members",
        &[
            ("Cargo.toml", manifest),
            ("crates/one/Cargo.toml", &one),
            ("crates/one/src/lib.rs", "//! One.\n"),
            ("tools/two/Cargo.toml", &two),
            ("tools/two/src/lib.rs", "//! Two.\n"),
        ],
    );
    let member = "workspace member not a tracked crate";
    let pycache = "__pycache__ under tools/ or crates/";
    let report = text(&sandbox.batch(&["check"]));
    assert!(check_line(&report, member).ends_with(" none"), "{report}");
    assert!(check_line(&report, pycache).ends_with(" none"), "{report}");

    // `cargo new scratchpad/r1/probe`'s shape: the crate, and its directory added to `members`.
    sandbox.write("scratchpad/r1/probe/Cargo.toml", &crate_manifest("probe"));
    sandbox.write(
        "Cargo.toml",
        &manifest.replace("\"tools/*\"]", "\"tools/*\", \"scratchpad/r1/probe\"]"),
    );
    let report = text(&sandbox.batch(&["check"]));
    assert!(
        check_line(&report, member).ends_with(" 1 member(s)"),
        "{report}"
    );
    assert!(
        report.contains("    scratchpad/r1/probe: under scratchpad/"),
        "the scratch crate is not named: {report}"
    );
    sandbox.write("Cargo.toml", manifest);
    let report = text(&sandbox.batch(&["check"]));
    assert!(check_line(&report, member).ends_with(" none"), "{report}");

    // A Python run under `tools/` without `PYTHONDONTWRITEBYTECODE=1`: one directory the glob reads
    // as a member, and the same leavings once more inside a crate, which no glob reads.
    sandbox.write(
        "tools/__pycache__/main-checkout.cpython-314.pyc",
        "bytecode\n",
    );
    sandbox.write(
        "crates/one/__pycache__/helper.cpython-314.pyc",
        "bytecode\n",
    );
    let report = text(&sandbox.batch(&["check"]));
    assert!(
        check_line(&report, member).ends_with(" 1 member(s)"),
        "{report}"
    );
    assert!(
        report.contains("    tools/__pycache__: no tracked Cargo.toml"),
        "the cache directory is not named as a member: {report}"
    );
    assert!(
        check_line(&report, pycache).ends_with(" 2 director(ies)"),
        "{report}"
    );
    std::fs::remove_dir_all(sandbox.worktree().join("tools/__pycache__")).expect("planted cache");
    std::fs::remove_dir_all(sandbox.worktree().join("crates/one/__pycache__"))
        .expect("planted cache");
    let report = text(&sandbox.batch(&["check"]));
    assert!(check_line(&report, member).ends_with(" none"), "{report}");
    assert!(check_line(&report, pycache).ends_with(" none"), "{report}");
}

/// Every `python3` a script under `tools/` runs is run with `PYTHONDONTWRITEBYTECODE=1`: on its own
/// line, or exported by the script before anything runs, so no run leaves the `__pycache__` the
/// workspace's `tools/*` glob reads as a member (trap 114). A comment, an `echo`, and
/// `command -v python3`, which run nothing, are not runs.
#[test]
fn every_python_run_in_tools_writes_no_bytecode() {
    let mut unguarded = Vec::new();
    let directory = repository_root().join("tools");
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("tools/ is in the tree")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sh"))
        .collect();
    scripts.sort();
    for script in &scripts {
        let source = std::fs::read_to_string(script).expect("a script under tools/ reads");
        if source
            .lines()
            .any(|line| line.trim_start() == "export PYTHONDONTWRITEBYTECODE=1")
        {
            continue;
        }
        for (number, line) in source.lines().enumerate() {
            let code = line.trim_start();
            let runs_nothing = code.starts_with('#')
                || code.starts_with("echo ")
                || code.contains("command -v python3");
            if runs_nothing || !code.contains("python3") {
                continue;
            }
            if !code.contains("PYTHONDONTWRITEBYTECODE=1") {
                unguarded.push(format!("{}:{}: {code}", script.display(), number + 1));
            }
        }
    }
    assert!(
        unguarded.is_empty(),
        "python3 run without PYTHONDONTWRITEBYTECODE=1:\n{}",
        unguarded.join("\n")
    );
}

/// Where a package's integration test called `name` is, if the package is in one of the tree's
/// three roots.
fn test_file(package: &str, name: &str) -> Option<PathBuf> {
    ["crates", "raster/crates", "tools"]
        .iter()
        .flat_map(|root| {
            let tests = repository_root().join(root).join(package).join("tests");
            [
                tests.join(format!("{name}.rs")),
                tests.join(name).join("main.rs"),
            ]
        })
        .find(|path| path.is_file())
}

/// How many `#[test]` functions a file holds, ignored and not.
///
/// A test's attributes are the lines between the item before it — the last line that is a lone
/// `}` — and its `fn`; one of them starting `#[ignore` makes it ignored. A doc comment that
/// *mentions* `#[ignore]` starts with `///` and is not an attribute, and `#[cfg_attr(miri, ignore
/// …)]` ignores a test under Miri alone, so neither counts.
fn test_shape(text: &str) -> (usize, usize) {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let (mut ignored, mut run) = (0usize, 0usize);
    let mut attributes_from = 0usize;
    for (at, line) in lines.iter().enumerate() {
        if *line == "}" {
            attributes_from = at.saturating_add(1);
            continue;
        }
        let is_fn =
            line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("async fn ");
        if !is_fn {
            continue;
        }
        let attributes = lines.get(attributes_from..at).unwrap_or_default();
        if attributes.iter().any(|line| line.starts_with("#[test]")) {
            if attributes.iter().any(|line| line.starts_with("#[ignore")) {
                ignored = ignored.saturating_add(1);
            } else {
                run = run.saturating_add(1);
            }
        }
        attributes_from = at.saturating_add(1);
    }
    (ignored, run)
}

/// Every `cargo test … -p <package> --test <name> …` a gate list writes, with the loops of
/// `tools/batch.sh`'s `gates()` expanded, as (where, package, name, whether `--ignored` is given).
fn gate_commands() -> Vec<(String, String, String, bool)> {
    let mut commands = Vec::new();
    let mut read = |source: &str, text: &str| {
        let mut looped: Vec<String> = Vec::new();
        for (number, line) in text.lines().enumerate() {
            if let Some(rest) = line.trim().strip_prefix("for t in ")
                && let Some((names, _)) = rest.split_once(';')
            {
                looped = names.split_whitespace().map(str::to_owned).collect();
                continue;
            }
            let mut from = 0;
            while let Some(found) = line.get(from..).and_then(|rest| rest.find("cargo test")) {
                let start = from.saturating_add(found);
                let command: String = line
                    .get(start..)
                    .unwrap_or_default()
                    .chars()
                    .take_while(|character| !matches!(character, '`' | '#' | '|'))
                    .collect();
                from = start.saturating_add(command.len().max(1));
                let words: Vec<&str> = command.split_whitespace().collect();
                let after = |flag: &str| {
                    words
                        .iter()
                        .position(|word| *word == flag)
                        .and_then(|at| words.get(at.saturating_add(1)))
                        .map(|word| word.trim_matches('"').to_owned())
                };
                let (Some(package), Some(name)) = (after("-p"), after("--test")) else {
                    continue;
                };
                // A build that names its test binary to run it later — `arms` runs the copy — is
                // not a gate line, and runs nothing by its own flag.
                if words.contains(&"--no-run") {
                    continue;
                }
                let ignored = words.contains(&"--ignored");
                let names = if name.starts_with('$') {
                    looped.clone()
                } else {
                    vec![name]
                };
                for name in names {
                    commands.push((
                        format!("{source}:{}", number.saturating_add(1)),
                        package.clone(),
                        name,
                        ignored,
                    ));
                }
            }
        }
    };
    for source in ["tools/batch.sh", "doc/todo/02-every-round.md"] {
        let text = std::fs::read_to_string(repository_root().join(source))
            .expect("the gate lists are in the tree");
        read(source, &text);
    }
    commands
}

/// A gate that runs no test is green while checking nothing: `--ignored` against a file with no
/// ignored test, or no `--ignored` against a file whose every test is ignored, exits 0 having run
/// zero. Every gate line in `tools/batch.sh`'s `gates()` and `doc/todo/02` names a test binary
/// whose shape matches the flag it is given (ADR 1392).
#[test]
fn every_gate_line_runs_at_least_one_test_of_the_file_it_names() {
    let commands = gate_commands();
    assert!(
        commands.len() > 20,
        "the gate lists were not read: {commands:?}"
    );
    let mut wrong = Vec::new();
    for (place, package, name, ignored) in &commands {
        let Some(path) = test_file(package, name) else {
            wrong.push(format!(
                "{place}: -p {package} --test {name} names no test file"
            ));
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("a test file is readable");
        let (ignored_tests, run_tests) = test_shape(&text);
        let selected = if *ignored { ignored_tests } else { run_tests };
        if selected == 0 {
            wrong.push(format!(
                "{place}: -p {package} --test {name} {} runs zero tests ({ignored_tests} ignored, \
                 {run_tests} not)",
                if *ignored {
                    "with --ignored"
                } else {
                    "without --ignored"
                }
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The shape reader's own calibration: an ignored test, a plain one, and the two spellings that
/// are not an ignore.
#[test]
fn a_test_files_shape_counts_the_ignore_attribute_and_nothing_that_mentions_it() {
    let text = "#[test]\n#[ignore = \"corpus\"]\nfn walk() {\n}\n\n/// Not `#[ignore]`d.\n#[test]\n\
                #[cfg_attr(miri, ignore = \"files\")]\nfn unit() {\n    let x = 1;\n}\n";
    assert_eq!(test_shape(text), (1, 1));
}

/// The words of `tools/batch.sh`'s `<variable>="…"` line.
fn batch_list(variable: &str) -> Vec<String> {
    let script = std::fs::read_to_string(repository_root().join("tools/batch.sh"))
        .expect("tools/batch.sh is in the tree");
    let prefix = format!("{variable}=\"");
    script
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .and_then(|rest| rest.split_once('"'))
        .map(|(words, _)| words.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// A workspace whose targets carry `install`'s names, built into its own `target/` — never the
/// build directory `~/.cargo/config.toml` names, where a stand-in called `quorra` would sit
/// beside the real one.
fn stand_in_workspace() -> Vec<(String, String)> {
    let libraries = batch_list("install_libraries");
    // Each program behind a feature is a package of its own, its `[[bin]]` requiring the feature
    // `install_featured` names, so that the stand-in's build fails exactly where install forgot it.
    let featured: Vec<(String, String, String)> = batch_list("install_featured")
        .iter()
        .filter_map(|entry| {
            let (name, features) = entry.split_once(':')?;
            let (package, feature) = features.split_once('/')?;
            Some((name.to_owned(), package.to_owned(), feature.to_owned()))
        })
        .collect();
    let mut members = vec!["\"programs\"".to_owned()];
    members.extend(libraries.iter().map(|name| format!("\"{name}\"")));
    members.extend(
        featured
            .iter()
            .map(|(_, package, _)| format!("\"{package}\"")),
    );
    let mut files = vec![
        (
            "Cargo.toml".to_owned(),
            format!(
                "[workspace]\nmembers = [{}]\nresolver = \"3\"\n",
                members.join(", ")
            ),
        ),
        (
            ".cargo/config.toml".to_owned(),
            "[build]\ntarget-dir = \"target\"\n".to_owned(),
        ),
        (".gitignore".to_owned(), "/target\nCargo.lock\n".to_owned()),
        (
            "programs/Cargo.toml".to_owned(),
            "[package]\nname = \"programs\"\nversion = \"0.1.0\"\nedition = \"2024\"\n".to_owned(),
        ),
    ];
    for name in batch_list("install_binaries") {
        files.push((
            format!("programs/src/bin/{name}.rs"),
            "fn main() {}\n".to_owned(),
        ));
    }
    for (name, package, feature) in &featured {
        files.push((
            format!("{package}/Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                 [features]\n{feature} = []\n\n[[bin]]\nname = \"{name}\"\npath = \"src/main.rs\"\n\
                 required-features = [\"{feature}\"]\n"
            ),
        ));
        files.push((
            format!("{package}/src/main.rs"),
            "fn main() {}\n".to_owned(),
        ));
    }
    for name in &libraries {
        files.push((
            format!("{name}/Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                 [lib]\ncrate-type = [\"cdylib\"]\n"
            ),
        ));
        files.push((
            format!("{name}/src/lib.rs"),
            "pub fn stand_in() {}\n".to_owned(),
        ));
    }
    files
}

/// `install` refuses until `main` names the batch's commit, then installs every file into the main
/// checkout's `target/`, records the commit beside them, and leaves the main checkout otherwise
/// exactly as it was.
#[test]
fn install_refuses_until_main_names_the_batch_then_writes_only_main_s_target() {
    let owned = stand_in_workspace();
    let files: Vec<(&str, &str)> = owned
        .iter()
        .map(|(path, contents)| (path.as_str(), contents.as_str()))
        .collect();
    let sandbox = Sandbox::with_files("install", &files);

    sandbox.write("b.txt", "b\n");
    let dirty = sandbox.batch(&["install"]);
    assert!(
        !dirty.status.success() && text(&dirty).contains("1 uncommitted path(s)"),
        "install built from uncommitted work: {}",
        text(&dirty)
    );

    sandbox.write("scratchpad/message", "the batch\n");
    let message = sandbox.worktree().join("scratchpad/message");
    let committed = sandbox.batch(&["commit", message.to_str().expect("a UTF-8 temporary path")]);
    assert!(
        committed.status.success(),
        "commit failed: {}",
        text(&committed)
    );
    let early = sandbox.batch(&["install"]);
    assert!(
        !early.status.success() && text(&early).contains("fast-forward main first"),
        "install ran before main named the batch: {}",
        text(&early)
    );
    assert!(
        !sandbox.repo().join("target").exists(),
        "a refusal wrote into the main checkout"
    );

    sandbox.git(&sandbox.repo(), &["merge", "-q", "--ff-only", "batch-test"]);
    let installed = sandbox.batch(&["install"]);
    let said = text(&installed);
    assert!(installed.status.success(), "install failed: {said}");

    let head = sandbox.git(&sandbox.repo(), &["rev-parse", "main"]);
    let head = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    let record = std::fs::read_to_string(sandbox.repo().join("target/installed-from"))
        .expect("install writes target/installed-from");
    assert!(
        record.starts_with(&format!("# commit {head}\n")),
        "the record does not name main's commit {head}:\n{record}"
    );
    let mut expected: Vec<String> = batch_list("install_binaries");
    expected.extend(
        batch_list("install_featured")
            .iter()
            .filter_map(|entry| entry.split_once(':').map(|(name, _)| name.to_owned())),
    );
    expected.extend(
        batch_list("install_libraries")
            .iter()
            .map(|name| format!("lib{}.so", name.replace('-', "_"))),
    );
    for name in &expected {
        let path = sandbox.repo().join("target").join(name);
        assert!(path.is_file(), "{name} was not installed: {said}");
        assert!(
            record
                .lines()
                .any(|line| line.ends_with(&format!("  {name}"))),
            "the record carries no checksum for {name}:\n{record}"
        );
        assert!(
            said.contains(&format!("target/{name}")),
            "{name} is not printed: {said}"
        );
    }

    // The main checkout's status, ignored files included: the only new thing is `target/`.
    let status = sandbox.git(
        &sandbox.repo(),
        &[
            "status",
            "--porcelain",
            "--ignored",
            "--untracked-files=normal",
        ],
    );
    let status = String::from_utf8_lossy(&status.stdout);
    assert_eq!(
        status.trim(),
        "!! target/",
        "install wrote into the main checkout outside target/"
    );
}

/// What one directory of packages builds: its programs, the packages that build a C library, and
/// each program behind a feature as `name:package/feature,…` — `install_featured`'s spelling.
type Built = (Vec<String>, Vec<String>, Vec<String>);

/// The binary targets of the packages directly under `directory`, and the packages that build a C
/// library, read from each manifest the way Cargo discovers them in this tree: a `[[bin]]` table's
/// `name`, every `src/bin/<name>.rs`, and `src/main.rs` as the package's own name.
fn programs_and_libraries(directory: &str) -> Built {
    let (mut programs, mut libraries, mut featured) = (Vec::new(), Vec::new(), Vec::new());
    let Ok(entries) = std::fs::read_dir(repository_root().join(directory)) else {
        return (programs, libraries, featured);
    };
    for entry in entries.flatten() {
        let Ok(manifest) = std::fs::read_to_string(entry.path().join("Cargo.toml")) else {
            continue;
        };
        let value = |line: &str| {
            line.split_once('=')
                .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        };
        let package = manifest
            .lines()
            .skip_while(|line| line.trim() != "[package]")
            .find(|line| line.trim_start().starts_with("name"))
            .and_then(value)
            .unwrap_or_default();
        let mut tables = 0usize;
        let mut in_bin = false;
        // A program behind a feature no build turns on by default is built in a Cargo run of its
        // own, so that the feature reaches nothing a window links: `pdf-script-worker` needs
        // `engine` (ADRs 1616, 1625). Each `[[bin]]` is read whole, its name and its
        // `required-features`, before it is counted.
        let mut gated: Vec<String> = Vec::new();
        let mut bin: (Option<String>, Option<Vec<String>>) = (None, None);
        let mut place =
            |bin: (Option<String>, Option<Vec<String>>), gated: &mut Vec<String>| match bin {
                (Some(name), Some(required)) => {
                    let spelled: Vec<String> = required
                        .iter()
                        .map(|feature| format!("{package}/{feature}"))
                        .collect();
                    featured.push(format!("{name}:{}", spelled.join(",")));
                    gated.push(name);
                }
                (Some(name), None) => programs.push(name),
                (None, _) => {}
            };
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                place(std::mem::take(&mut bin), &mut gated);
                in_bin = line == "[[bin]]";
                tables = tables.saturating_add(usize::from(in_bin));
            } else if in_bin && line.starts_with("required-features") {
                bin.1 = value(line).map(|list| {
                    list.trim_matches(['[', ']'])
                        .split(',')
                        .map(|feature| feature.trim().trim_matches('"').to_owned())
                        .filter(|feature| !feature.is_empty())
                        .collect()
                });
            } else if in_bin
                && line.starts_with("name")
                && let Some(name) = value(line)
            {
                bin.0 = Some(name);
            }
        }
        place(bin, &mut gated);
        if manifest.contains("\"cdylib\"") {
            libraries.push(package.clone());
        }
        if let Ok(bins) = std::fs::read_dir(entry.path().join("src/bin")) {
            for bin in bins.flatten() {
                let path = bin.path();
                if path.extension().is_some_and(|extension| extension == "rs")
                    && let Some(stem) = path.file_stem()
                    && !gated.iter().any(|name| *name == stem.to_string_lossy())
                {
                    programs.push(stem.to_string_lossy().into_owned());
                }
            }
        }
        if tables == 0 && entry.path().join("src/main.rs").is_file() {
            programs.push(package);
        }
    }
    (programs, libraries, featured)
}

/// `install`'s names against the workspace: every program of a package under `crates/` — the
/// programs a person runs, each worker among them — is installed, every name installed is a
/// binary target, and every C library is installed. A program added under `crates/` and not
/// here fails this rather than going stale under `target/` (`doc/todo/02` section 5).
#[test]
fn install_names_every_program_and_library_the_workspace_builds_for_a_person() {
    let (person_programs, libraries, featured) = programs_and_libraries("crates");
    let (tool_programs, _, _) = programs_and_libraries("tools");
    let installed = batch_list("install_binaries");
    let installed_libraries = batch_list("install_libraries");
    let installed_featured = batch_list("install_featured");
    assert!(
        installed.len() >= 6 && person_programs.len() >= 6 && tool_programs.len() >= 20,
        "a list was not read: installed {installed:?}, crates/ {person_programs:?}, tools/ \
         {tool_programs:?}"
    );
    let not_programs: Vec<&String> = installed
        .iter()
        .filter(|name| !person_programs.contains(name) && !tool_programs.contains(name))
        .collect();
    let not_installed: Vec<&String> = person_programs
        .iter()
        .filter(|name| !installed.contains(name))
        .collect();
    let libraries_missed: Vec<&String> = libraries
        .iter()
        .filter(|name| !installed_libraries.contains(name))
        .collect();
    assert!(
        not_programs.is_empty() && not_installed.is_empty() && libraries_missed.is_empty(),
        "tools/batch.sh install_binaries/install_libraries against the workspace:\n  installed and \
         no binary target: {not_programs:?}\n  a program under crates/ not installed: \
         {not_installed:?}\n  a C library not installed: {libraries_missed:?}"
    );
    let (mut wanted, mut named) = (featured, installed_featured);
    wanted.sort();
    named.sort();
    assert!(
        !wanted.is_empty() && wanted == named,
        "tools/batch.sh install_featured against the workspace's programs behind a feature, each as \
         `name:package/feature`: the manifests state {wanted:?} and install builds {named:?} \
         (ADRs 1616, 1625)"
    );
}

/// `tools/batch.sh raster-examples` runs what CI's `raster-examples` job runs: the names it reads
/// out of `.github/workflows/ci.yml`'s `--check` loop are every example `raster-gpu` has, one a
/// file under `examples/` or a directory with a `main.rs` (ADR 1575). `raster-gpu`'s own
/// `tests/example_checks.rs` holds the workflow to the directory; this holds the gate's reading
/// of the workflow, so a reformatted loop that the `awk` stops reading fails here rather than
/// as a green gate that ran nothing.
#[test]
fn the_raster_examples_gate_reads_every_example_ci_runs() {
    let listed = Command::new("bash")
        .arg(repository_root().join("tools/batch.sh"))
        .args(["raster-examples", "--list"])
        .env("BATCH_WORKTREE", repository_root())
        .output()
        .expect("bash runs tools/batch.sh");
    assert!(listed.status.success(), "--list failed: {}", text(&listed));
    let mut read: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    read.sort();
    let examples = repository_root().join("raster/crates/raster-gpu/examples");
    let mut on_disk: Vec<String> = std::fs::read_dir(&examples)
        .expect("raster-gpu has an examples directory")
        .filter_map(|entry| {
            let path = entry.expect("a readable directory entry").path();
            let stem = path.file_stem()?.to_str()?.to_owned();
            let is_example = if path.is_dir() {
                path.join("main.rs").is_file()
            } else {
                path.extension().is_some_and(|extension| extension == "rs")
            };
            is_example.then_some(stem)
        })
        .collect();
    on_disk.sort();
    assert_eq!(
        read, on_disk,
        "tools/batch.sh raster-examples reads these names out of ci.yml (left); raster-gpu's \
         examples/ holds these (right)"
    );
}

/// `arms` refuses every export that would not be HEAD's — a branch that does not name its first
/// session with no directory given, a worktree with uncommitted work, a directory holding another
/// commit's export — and writes nothing in each; keeps an export complete for this commit; and
/// takes an incomplete one for this commit as owed rather than done. Calibrated both ways (trap 13):
/// the same directory is kept once its six digest files are there and refused again once one goes.
#[test]
fn arms_exports_only_head_and_never_over_another_commits_export() {
    let sandbox = Sandbox::new("arms");
    assert!(
        !sandbox.opened.contains("exporting HEAD"),
        "open started an export where there is no workspace: {}",
        sandbox.opened
    );
    let arms = |arguments: &[&str]| {
        sandbox
            .command("bash", &sandbox.repo())
            .env("BATCH_ARMS_ROOT", &sandbox.base)
            .arg(sandbox.repo().join("tools/batch.sh"))
            .arg("arms")
            .args(arguments)
            .output()
            .expect("bash runs tools/batch.sh")
    };
    let unnamed = arms(&[]);
    assert!(
        !unnamed.status.success() && text(&unnamed).contains("does not name its first session"),
        "arms took a branch without its first session: {}",
        text(&unnamed)
    );

    let out = sandbox.base.join("given");
    let given = out.to_str().expect("a temporary path is UTF-8");
    sandbox.write("new.txt", "new\n");
    let dirty = arms(&[given]);
    assert!(
        !dirty.status.success() && text(&dirty).contains("1 uncommitted path(s)"),
        "arms exported from a dirty worktree: {}",
        text(&dirty)
    );
    assert!(!out.exists(), "a refused export wrote its directory");
    std::fs::remove_file(sandbox.worktree().join("new.txt")).expect("the planted file goes");

    let head = String::from_utf8_lossy(
        &sandbox
            .git(&sandbox.worktree(), &["rev-parse", "HEAD"])
            .stdout,
    )
    .trim()
    .to_owned();
    std::fs::create_dir_all(&out).expect("an export directory");
    let other = "HEAD arms of 0123456789abcdef0123456789abcdef01234567, exported then\ndone then\n";
    std::fs::write(out.join("README"), other).expect("another commit's README");
    let refused = arms(&[given]);
    assert!(
        !refused.status.success() && text(&refused).contains("never overwritten"),
        "arms wrote over another commit's export: {}",
        text(&refused)
    );
    assert_eq!(
        std::fs::read_to_string(out.join("README")).expect("the README stays"),
        other,
        "a refused export touched another commit's README"
    );

    // A complete export of this commit, in the directory the branch names.
    sandbox.git(
        &sandbox.worktree(),
        &["checkout", "-q", "-b", "batch-9001-9006"],
    );
    let named = sandbox.base.join("arms-9001");
    std::fs::create_dir_all(&named).expect("the named export directory");
    std::fs::write(
        named.join("README"),
        format!("HEAD arms of {head}, exported now from here\ndone now\n"),
    )
    .expect("this commit's README");
    for arm in [
        "cpu-1x",
        "gpu-1x",
        "compute-1x",
        "cpu-4x",
        "gpu-4x",
        "compute-4x",
    ] {
        std::fs::write(named.join(format!("{arm}.tsv")), "page.pdf\t1\t1\t0\t0\n")
            .expect("a digest file");
    }
    let kept = arms(&[]);
    assert!(
        kept.status.success() && text(&kept).contains("already"),
        "arms did not keep a complete export of this commit: {}",
        text(&kept)
    );
    std::fs::remove_file(named.join("gpu-4x.tsv")).expect("one arm goes");
    let owed = arms(&[]);
    assert!(
        !owed.status.success() && text(&owed).contains("no workspace"),
        "arms took an export missing an arm as done: {}",
        text(&owed)
    );
}

/// `open` starts the warm build and then the arms export, each detached into a session of its own,
/// and returns while both still run: the caller reads `open`'s output to its end, as an agent's
/// shell and `Command::output` do, so a job that kept the caller's pipe would hold `open` for as
/// long as the job ran. Both jobs here run a stand-in `cargo` that sleeps for a minute, so an `open`
/// that waited is a failure by fifty seconds; the export queues for the sandbox's own heavy-walk
/// lock through `tools/bounded.sh --lock`, so its hold is a line of the lock's log, written when
/// the job is stopped (ADR 1662). Calibrated by planting the defect (trap 13): with `cd … &&
/// setsid … &` restored in `detach`, `open` took the stand-in's whole minute and this test failed.
#[test]
fn open_returns_while_the_warm_build_and_the_arms_export_run_detached() {
    let started = std::time::Instant::now();
    let mut sandbox = Sandbox::detaching("detach");
    let took = started.elapsed();
    let warm = detached_pid(&sandbox.opened, "warming the build directory");
    let arms = detached_pid(&sandbox.opened, "exporting HEAD's six corpus arms");
    sandbox
        .detached
        .extend(warm.iter().chain(arms.iter()).cloned());
    assert!(
        took < std::time::Duration::from_secs(15),
        "open took {took:?} with two detached jobs of {STAND_IN_SECONDS} s: it waited for one of them: {}",
        sandbox.opened
    );
    assert!(
        warm.is_some() && arms.is_some(),
        "open did not start both jobs: {}",
        sandbox.opened
    );
    let (warm, arms) = (warm.unwrap_or_default(), arms.unwrap_or_default());
    let order = |what: &str| sandbox.opened.find(what).unwrap_or(usize::MAX);
    assert!(
        order("warming the build directory") < order("exporting HEAD's six corpus arms"),
        "open did not start the warm build first: {}",
        sandbox.opened
    );
    assert!(
        group_runs(&warm) && group_runs(&arms),
        "open returned and a job it detached is not running — it was never started, or ended at once: {}",
        sandbox.opened
    );
    let lock = sandbox.lock();
    let held = within(20, || {
        Command::new("flock")
            .args(["-n"])
            .arg(&lock)
            .arg("true")
            .status()
            .is_ok_and(|status| !status.success())
    });
    assert!(
        held,
        "the arms export never took the heavy-walk lock:\n{}",
        std::fs::read_to_string(sandbox.worktree().join("scratchpad/open/arms.log"))
            .unwrap_or_default()
    );
    // The export under the lock wrote its README's lock line, so `arms-held` found the
    // descriptor the wrapper handed down rather than refusing for want of it.
    let readme = sandbox.base.join("arms-9001/README");
    let under_the_lock = within(20, || {
        std::fs::read_to_string(&readme).is_ok_and(|text| text.contains("\nlock: queued "))
    });
    assert!(
        under_the_lock,
        "the export never wrote its lock line, so `arms-held` did not run under the lock:\n{}",
        std::fs::read_to_string(sandbox.worktree().join("scratchpad/open/arms.log"))
            .unwrap_or_default()
    );
    stop_group(&arms);
    let log = sandbox.base.join("heavy-walk.log");
    let logged = within(20, || {
        std::fs::read_to_string(&log).is_ok_and(|text| {
            text.lines()
                .any(|line| line.contains(" batch=batch-9001-9006 round=arms wait="))
        })
    });
    assert!(
        logged,
        "the arms export's hold of the lock is on no line of its log: {}",
        std::fs::read_to_string(&log).unwrap_or_default()
    );
}

/// The two clocks `gates` prints for each gate are read off the wrapper's last line, which names
/// the seconds the command ran once the lock was granted as `… after <n>s`: a wrapper whose last
/// line stopped saying so would leave every gate's queue at nought and its wait inside its wall.
#[test]
fn the_wrappers_last_line_names_the_seconds_gates_reads() {
    let output = Command::new("bash")
        .arg(repository_root().join("tools/bounded.sh"))
        .args([
            "--tree", "1", "--data", "1", "--nice", "0", "--", "sleep", "1",
        ])
        .output()
        .expect("bash runs tools/bounded.sh");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let last = stderr.lines().last().unwrap_or_default();
    let seconds = last
        .strip_prefix("bounded: ")
        .and_then(|rest| rest.rsplit_once(" after "))
        .map(|(_, after)| {
            after
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .filter(|digits| !digits.is_empty());
    assert!(
        output.status.success() && seconds.is_some_and(|digits| digits == "1" || digits == "2"),
        "the wrapper's last line does not name the seconds `run` reads (`bounded: … after <n>s`): {stderr}"
    );
    let source = std::fs::read_to_string(repository_root().join("tools/batch.sh"))
        .expect("tools/batch.sh is in the tree");
    assert!(
        source.contains("s/^bounded: .* after \\([0-9][0-9]*\\)s.*/\\1/p"),
        "tools/batch.sh's `run` no longer reads the wrapper's `after <n>s`, which this test holds"
    );
}
