//! `tools/bounded.sh` is the wrapper every corpus walk, census and build runs under, and the
//! memory ceiling it enforces is what keeps the machine up (`doc/environment.md`'s parallel-round
//! agreements, ADR 0798). Its `--self-test` exercises the half of it that a gate can see without
//! a corpus: the sampler that measures the tree, the ceiling's kill, the wrapper's own answer to
//! a sampler that stalls, and the task limit a fork loop meets.
//!
//! Not a conformance question, and it lives here for the reason `sandbox_gates.rs`,
//! `submodules.rs` and `workspaces.rs` do: this is the crate whose gates read the repository's
//! own files rather than a PDF, and `cargo test -p conformance` is the sequence's last line, so
//! a wrapper broken by an edit fails a round before that round walks anything under it.
//!
//! # What it guards
//!
//! A sampler that walks the process table with an inner loop over every process for every node of
//! the tree is quadratic — 6 s a sample over a tree of 8 000 processes — and one with no guard
//! against visiting a pid twice and no bound on how long the `ps` underneath it might take can hang
//! under exactly the memory pressure the ceiling exists to prevent (ADR 0807). **A bound that
//! is not being measured is a bound that is not there**, which is
//! `doc/traps/instruments-and-reports.md`'s
//! trap 18 read from the other side: there the limit destroyed the channel that reports it; here
//! the channel that measures the limit could stop, and nothing said so.
//!
//! The self-test's seven cases are the script's own (`tools/bounded.sh --self-test` prints one line
//! each): a synthetic table of a hundred thousand children sampled in a fraction of the interval,
//! a chain, a cycle and a duplicated row walked once each, a live tree that fans out, a child
//! over the ceiling stopped with exit 137, a sampler that never returns stopping the tree after
//! the stated number of missed samples, a fork loop of at most 128 children refused under a
//! task limit of 64, and the heavy-walk lock on a file of the case's own — a run queued behind a
//! holder logs its wait, one finds the lock free, one runs under its caller's own `flock` and
//! finishes rather than queueing behind it (ADR 1646). This test runs the script and repeats what
//! it said.
//!
//! **No memory bound sees a process count**, and trap 116 is the incident: a tool that forked a
//! task per package and never waited took the agent's scope to 52 259 tasks, and the OOM daemon
//! killed every round. `RLIMIT_NPROC` is the bound that acts at the `fork` itself, it counts every
//! task of the user, and its figure is the agent's budget, written once in the wrapper (ADR 1612).
//! The second test holds every heavy script under `tools/` to that figure.
//!
//! **A lock nobody logged is a wait nobody can count.** `tools/bounded.sh --lock` writes one line
//! per hold of the heavy-walk lock into `/home/AI/heavy-walk.log`, and `tools/state.sh gates-cost`
//! reads the batch's queue off those lines; a tool that takes the lock with a bare `flock` holds it
//! on no line, which is how the merge's gates and the arms export each held it for half an hour
//! unrecorded (ADR 1662). The third test holds every script under `tools/` but the wrapper, and the
//! rule line `doc/environment.md` gives the rounds, to taking it through `--lock`.

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

/// Every script under `tools/` that runs a build, a test, a walk or a display server holds itself to
/// the agent's task budget before anything runs: it reads the figure from `tools/bounded.sh
/// --task-budget`, the one place it is written, and sets `ulimit -u` to it (trap 116, ADR 1612). A
/// line that only prints a command — a comment, an `echo`, a `printf` — runs nothing.
#[test]
fn every_heavy_script_in_tools_runs_under_the_task_budget() {
    let heavy = [
        "cargo build",
        "cargo test",
        "cargo run",
        "cargo nextest",
        "cargo clippy",
        "fuzz run",
        "xvfb-run",
        "Xvfb ",
    ];
    let mut scripts: Vec<_> = std::fs::read_dir(repository_root().join("tools"))
        .expect("tools/ is in the tree")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sh"))
        .collect();
    scripts.sort();
    let mut checked = 0_usize;
    let mut unbounded = Vec::new();
    for script in &scripts {
        let name = script
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name == "bounded.sh" {
            continue;
        }
        let source = std::fs::read_to_string(script).expect("a script under tools/ reads");
        let runs_heavy = source.lines().map(str::trim_start).any(|code| {
            !(code.starts_with('#') || code.contains("echo ") || code.contains("printf "))
                && heavy.iter().any(|shape| code.contains(shape))
        });
        if !runs_heavy {
            continue;
        }
        checked = checked.saturating_add(1);
        let reads = source.lines().any(|line| {
            line.contains("task_budget=$(") && line.contains("bounded.sh\" --task-budget)")
        });
        let sets = source.contains("ulimit -u \"$task_budget\"");
        if !(reads && sets) {
            unbounded.push(name);
        }
    }
    println!("{checked} heavy script(s) under tools/ hold themselves to the task budget");
    assert!(
        checked >= 3,
        "{checked} heavy script(s): the shapes are measuring nothing"
    );
    assert!(
        unbounded.is_empty(),
        "these scripts under tools/ run builds, tests or walks without the agent's task budget \
         (`task_budget=$(\"<root>/tools/bounded.sh\" --task-budget)` and `ulimit -u \"$task_budget\"`): \
         {unbounded:?}"
    );
}

/// The lines of a shell script that take a lock with `flock` themselves rather than through
/// `tools/bounded.sh --lock`, as `(line number, line)`. A comment, and a line that only prints, take
/// nothing.
fn bare_locks(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.trim_start();
            !(code.starts_with('#') || code.starts_with("echo ") || code.starts_with("printf "))
                && code
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                    .any(|word| word == "flock")
        })
        .map(|(index, line)| (index.saturating_add(1), line.trim().to_owned()))
        .collect()
}

/// Every lock a tool takes is taken by `tools/bounded.sh --lock`, so that its wait and its hold are
/// a line of the lock's log: no script under `tools/` but the wrapper runs `flock` itself, and the
/// rule line `doc/environment.md` opens with spells the lock as the wrapper's flag, never as
/// `flock` on the lock's path (ADR 1662).
/// Calibrated by planting (trap 13): the reader names a bare `flock` on a line of its own, behind
/// `exec`, and inside a command substitution, and passes a comment and the wrapper's own spelling.
#[test]
fn every_lock_a_tool_takes_is_taken_by_the_wrapper_that_logs_it() {
    let planted = "# flock /home/AI/heavy-walk.lock is the old spelling\n\
                   out=$(flock /home/AI/heavy-walk.lock sh -c true)\n\
                   tools/bounded.sh --lock --round 1 -- true\n\
                   exec {lock}>/home/AI/heavy-walk.lock; flock \"$lock\"\n\
                   \x20   flock -n 9\n";
    let found: Vec<usize> = bare_locks(planted).iter().map(|(line, _)| *line).collect();
    assert_eq!(found, [2, 4, 5], "the reader is not the shape it states");

    let mut scripts: Vec<_> = std::fs::read_dir(repository_root().join("tools"))
        .expect("tools/ is in the tree")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sh"))
        .filter(|path| path.file_name().is_some_and(|name| name != "bounded.sh"))
        .collect();
    scripts.sort();
    assert!(
        scripts.len() >= 3,
        "{} script(s) under tools/: the population is not the tree",
        scripts.len()
    );
    let mut bare = Vec::new();
    for script in &scripts {
        let source = std::fs::read_to_string(script).expect("a script under tools/ reads");
        for (line, text) in bare_locks(&source) {
            bare.push(format!("{}:{line}: {text}", script.display()));
        }
    }
    println!(
        "{} script(s) under tools/ read for a lock taken outside tools/bounded.sh --lock",
        scripts.len()
    );
    assert!(
        bare.is_empty(),
        "these lines take a lock outside `tools/bounded.sh --lock`, so their wait is on no line of \
         the lock's log:\n{}",
        bare.join("\n")
    );

    let environment = std::fs::read_to_string(repository_root().join("doc/environment.md"))
        .expect("doc/environment.md is in the tree");
    let rule = environment
        .lines()
        .skip_while(|line| !line.starts_with("- **One heavy walk on the machine at a time**"))
        .take_while(|line| !line.is_empty() && !line.starts_with("- **No `git stash`"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rule.contains("tools/bounded.sh --lock --round <session>")
            && !rule.contains("flock /home/AI/heavy-walk.lock"),
        "doc/environment.md's rule line does not spell the lock as `tools/bounded.sh --lock \
         --round <session>`, or still runs a bare `flock`:\n{rule}"
    );
}
