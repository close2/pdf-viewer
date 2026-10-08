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
//! The self-test's nine cases are the script's own (`tools/bounded.sh --self-test` prints one line
//! each): a synthetic table of a hundred thousand children sampled in a fraction of the interval,
//! a chain, a cycle and a duplicated row walked once each, a live tree that fans out, a child
//! over the ceiling stopped with exit 137, a sampler that never returns stopping the tree after
//! the stated number of missed samples, a fork loop of at most 128 children refused under a
//! task limit of 64, and the heavy-walk lock on a file of the case's own — a run queued behind a
//! holder logs its wait, one finds the lock free, one runs under its caller's own `flock` and
//! finishes rather than queueing behind it (ADR 1646) — and the lock kept by the wrapper rather
//! than handed to its command: a daemon the command leaves running does not keep it, `--held`
//! reads the marker inside a hold and not outside it, and a `--lock` run nested in a hold runs
//! under it (ADR 1674) — and the lock's two lanes: a small walk granted the second lane beside a
//! large one, a clock run planted behind two lane holders that waits for both while a walk asked
//! after it is granted nothing until it ends, and a `--clock` outside a lock or inside a hold of one
//! lane refused (ADR 1684). This test runs the script and repeats what it said.
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
//!
//! **A lane is granted on what a run declared**, never on what it turns out to peak at, so every
//! `--lock` a person or a script is told to run says its kind: `--tree` (6 GiB or less a small walk,
//! more a large one) or `--clock` (ADR 1684). The last two tests hold every tracked instruction to
//! that, and the merge's list of clock gates to the gates the merge runs.

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

/// `tools/state.sh gates-cost` counts a lock line for the batch it belongs to even where the line
/// names no batch branch: a wrapper run from a detached export of HEAD writes `batch=HEAD`, and two
/// of round 1405's runs, 1 921.9 s of its queue, were counted for no batch until the line was
/// placed by its round (ADR 1675). The planted log puts such a line inside a batch, one whose round
/// no batch holds, and that one last, so the last batch is found by its branch and not by the last
/// line. Calibrated by planting (trap 13): the previous reading, which took the last line's branch
/// and counted only lines naming it, printed `batch HEAD` and nothing of the batch.
#[test]
fn the_lock_cost_counts_a_line_that_names_no_batch_for_the_batch_its_round_is_in() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let scratch = std::env::temp_dir().join(format!("lock-cost-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a temporary directory");
    let log = scratch.join("heavy-walk.log");
    std::fs::write(
        &log,
        "2026-10-07T10:00:00 batch=batch-100-105 round=101 wait=1.0s hold=2.0s exit=0 peak=0.10GiB behind=- cmd=walk one\n\
         2026-10-07T10:01:00 batch=HEAD round=103 wait=30.0s hold=4.0s exit=0 cmd=walk two\n\
         2026-10-07T10:03:00 batch=batch-100-105 round=batch-100-105 wait=0.0s hold=5.0s exit=0 cmd=a gate\n\
         2026-10-07T10:04:00 batch=HEAD round=999 wait=7.0s hold=1.0s exit=0 cmd=walk three\n",
    )
    .expect("a planted lock log");
    let gates = scratch.join("batch-gates.log");
    std::fs::write(&gates, "").expect("an empty gate log");
    let output = Command::new("bash")
        .arg(repository_root().join("tools/state.sh"))
        .arg("gates-cost")
        .env("HEAVY_WALK_LOG", &log)
        .env("BATCH_GATES_LOG", &gates)
        .output()
        .expect("bash runs tools/state.sh");
    let _ = std::fs::remove_dir_all(&scratch);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let by_round = |round: &str| {
        stdout
            .lines()
            .find(|line| {
                line.trim_start().starts_with(&format!("round {round} ")) && line.contains("run(s)")
            })
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    };
    assert!(
        stdout.contains("batch batch-100-105, by round:"),
        "the last batch was not found by its branch:\n{stdout}"
    );
    assert_eq!(
        by_round("103"),
        "round 103 1 run(s) queued 30.0s held 4.0s",
        "a line naming no batch was not counted for the batch its round is in:\n{stdout}"
    );
    assert_eq!(by_round("101"), "round 101 1 run(s) queued 1.0s held 2.0s");
    assert!(
        by_round("999").is_empty()
            && stdout.contains("[batch=HEAD, by its round]")
            && stdout.contains("1 of those line(s) name no batch branch and are counted by their round, 30.0s of queue")
            && stdout.contains("1 line(s) of the log name no batch branch and a round no batch on the log holds"),
        "a line counted by its round is not marked so, or one no batch holds was counted:\n{stdout}"
    );
}

/// Whether `line` tells a person to take the heavy-walk lock with `flock` on the lock's path, as
/// `flock /home/AI/heavy-walk.lock <command>` — options between the two allowed — rather than
/// through `tools/bounded.sh --lock`.
fn spells_a_bare_lock(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    words.iter().enumerate().any(|(at, word)| {
        let bare = word
            .rsplit(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.')))
            .next()
            .unwrap_or_default();
        (bare == "flock" || bare.ends_with("/flock"))
            && words
                .iter()
                .skip(at.saturating_add(1))
                .find(|next| !next.starts_with('-'))
                .is_some_and(|path| {
                    path.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ')' | ';' | ','))
                        .ends_with("heavy-walk.lock")
                })
    })
}

/// The files that still tell a person to take the lock with a bare `flock`, each another round's to
/// re-spell: a ratchet, so a file leaves this list the day it is re-spelled and none joins it.
const HELD_BARE_LOCK_INSTRUCTIONS: [&str; 1] =
    ["doc/rfc/0008-a-script-is-a-document-acting-on-its-reader.md"];

/// Every instruction a person reads spells the lock as the wrapper does, not only the scripts and
/// the rule line above: a census's doc comment, `fuzz/seeds.sh`'s header and `doc/verify.md` told a
/// person to run a walk under a bare `flock`, whose wait is on no line of the lock's log and whose
/// descriptor any daemon the walk starts keeps (ADR 1662 section 3, ADR 1674). The population is
/// every tracked text file but the records — `doc/adr/`, `doc/history/` and `doc/reviews/` keep the
/// spelling of their day — and this file, which plants the shape. Calibrated by planting (trap 13):
/// the reader names the bare spelling in a code span, behind `RAYON_NUM_THREADS=4`, with `-n`, and
/// inside `$(…)`, and passes the wrapper's spelling and a `flock` on another file.
#[test]
fn every_instruction_a_person_reads_spells_the_lock_as_the_wrapper() {
    for planted in [
        "run it behind the lock, `flock /home/AI/heavy-walk.lock fuzz/seeds.sh`.",
        "//! RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 12 -- \\",
        "flock -n /home/AI/heavy-walk.lock true",
        "out=$(flock \"/home/AI/heavy-walk.lock\" true)",
    ] {
        assert!(spells_a_bare_lock(planted), "the reader passed {planted:?}");
    }
    for clean in [
        "tools/bounded.sh --lock --round <session> --tree 12 -- fuzz/seeds.sh",
        "timeout 20 flock \"$scratch/lock\" env HEAVY_WALK_LOCK=\"$scratch/lock\"",
        "a bare `flock`, which is on no line",
    ] {
        assert!(!spells_a_bare_lock(clean), "the reader named {clean:?}");
    }

    let listed = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .args(["ls-files", "-z"])
        .output()
        .expect("git lists the tree");
    assert!(listed.status.success(), "git ls-files failed");
    let records = ["doc/adr/", "doc/history/", "doc/reviews/"];
    let text = ["rs", "md", "sh", "py", "toml", "txt", "yml", "yaml"];
    let mut read = 0_usize;
    let mut bare = Vec::new();
    let mut spelled: Vec<String> = Vec::new();
    for path in String::from_utf8_lossy(&listed.stdout).split('\0') {
        let is_text = Path::new(path)
            .extension()
            .is_some_and(|extension| text.iter().any(|kind| extension == *kind));
        if !is_text
            || records.iter().any(|record| path.starts_with(record))
            || path == "tools/conformance/tests/bounded.rs"
        {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(repository_root().join(path)) else {
            continue;
        };
        read = read.saturating_add(1);
        let lines: Vec<usize> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| spells_a_bare_lock(line))
            .map(|(index, _)| index.saturating_add(1))
            .collect();
        if lines.is_empty() {
            continue;
        }
        spelled.push(path.to_owned());
        if !HELD_BARE_LOCK_INSTRUCTIONS.contains(&path) {
            bare.push(format!("{path}:{lines:?}"));
        }
    }
    println!(
        "{read} tracked text file(s) read for a bare `flock` on the lock; {} held, {} owed",
        spelled.len().saturating_sub(bare.len()),
        bare.len()
    );
    assert!(
        read >= 1000,
        "{read} file(s) read: the population is not the tree"
    );
    assert!(
        bare.is_empty(),
        "these lines tell a person to take the heavy-walk lock with a bare `flock`; spell it \
         `tools/bounded.sh --lock --round <session> … --`:\n{}",
        bare.join("\n")
    );
    let fixed: Vec<&str> = HELD_BARE_LOCK_INSTRUCTIONS
        .iter()
        .copied()
        .filter(|held| !spelled.iter().any(|path| path == held))
        .collect();
    assert!(
        fixed.is_empty(),
        "these files no longer spell a bare `flock`, so they leave the held list: {fixed:?}"
    );
}

/// The `--lock` invocations in `source` that declare no kind — neither `--tree` nor `--clock` — as
/// `(line number, invocation)`. A command continued with `\` is read as one line, and the line it
/// starts on is the one named. Only the wrapper's path followed by the flag is an invocation, so
/// prose that names the flag on its own is not one.
fn undeclared_locks(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut at = 0;
    while let Some(first) = lines.get(at) {
        let start = at;
        let mut joined = (*first).to_owned();
        while joined.trim_end().ends_with('\\') {
            at = at.saturating_add(1);
            let Some(next) = lines.get(at) else { break };
            joined.push(' ');
            joined.push_str(next);
        }
        at = at.saturating_add(1);
        let words: Vec<&str> = joined.split_whitespace().collect();
        let invokes = words.windows(2).any(|pair| {
            pair.first().is_some_and(|word| {
                word.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | '(' | '$'))
                    .ends_with("bounded.sh")
            }) && pair.get(1).is_some_and(|word| *word == "--lock")
        });
        if invokes && !(joined.contains("--tree ") || joined.contains("--clock")) {
            found.push((start.saturating_add(1), joined.trim().to_owned()));
        }
    }
    found
}

/// The files that still tell a person to run `--lock` with no kind, each another round's to
/// declare: a ratchet, so a file leaves this list the day it says `--tree` or `--clock`.
const HELD_UNDECLARED_LOCKS: [&str; 1] = ["crates/pdf-model/examples/substitution_census.rs"];

/// Every `--lock` a tool runs or a document tells a person to run declares its kind, so the lane it
/// is granted is a decision written down rather than the wrapper's default (ADR 1684). The
/// population is every tracked text file but the records and this file, as the bare-`flock` test's
/// is. Calibrated by planting (trap 13): the reader names an undeclared invocation in a code span,
/// behind a quoted path, and across a continuation, and passes `--tree 6`, `--clock` on the
/// continued line, and prose that names the flag.
#[test]
fn every_lock_a_caller_takes_declares_its_kind() {
    let planted = "run it as `tools/bounded.sh --lock --round <session> -- walk`.\n\
                   \"$wt/tools/bounded.sh\" --lock --round arms -- \\\n\
                   \x20   tools/batch.sh arms-held\n\
                   tools/bounded.sh --lock --round 1 --tree 6 -- walk\n\
                   tools/bounded.sh --lock --round 1 \\\n\
                   \x20   --clock -- gate\n\
                   the wrapper's `--lock` takes the lock\n";
    let found: Vec<usize> = undeclared_locks(planted)
        .iter()
        .map(|(line, _)| *line)
        .collect();
    assert_eq!(found, [1, 2], "the reader is not the shape it states");

    let listed = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .args(["ls-files", "-z"])
        .output()
        .expect("git lists the tree");
    assert!(listed.status.success(), "git ls-files failed");
    let records = ["doc/adr/", "doc/history/", "doc/reviews/"];
    let text = ["rs", "md", "sh", "py", "toml", "txt", "yml", "yaml"];
    let mut read = 0_usize;
    let mut invoking = 0_usize;
    let mut owed = Vec::new();
    let mut undeclared: Vec<String> = Vec::new();
    for path in String::from_utf8_lossy(&listed.stdout).split('\0') {
        let is_text = Path::new(path)
            .extension()
            .is_some_and(|extension| text.iter().any(|kind| extension == *kind));
        if !is_text
            || records.iter().any(|record| path.starts_with(record))
            || path == "tools/conformance/tests/bounded.rs"
            || path == "tools/bounded.sh"
        {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(repository_root().join(path)) else {
            continue;
        };
        read = read.saturating_add(1);
        if source.contains("bounded.sh\" --lock") || source.contains("bounded.sh --lock") {
            invoking = invoking.saturating_add(1);
        }
        let lines: Vec<usize> = undeclared_locks(&source)
            .iter()
            .map(|(line, _)| *line)
            .collect();
        if lines.is_empty() {
            continue;
        }
        undeclared.push(path.to_owned());
        if !HELD_UNDECLARED_LOCKS.contains(&path) {
            owed.push(format!("{path}:{lines:?}"));
        }
    }
    println!(
        "{read} tracked text file(s) read, {invoking} of them run or name `--lock`; {} held, {} owed",
        undeclared.len().saturating_sub(owed.len()),
        owed.len()
    );
    assert!(
        read >= 1000 && invoking >= 5,
        "{read} file(s) read, {invoking} naming `--lock`: the population is not the tree"
    );
    assert!(
        owed.is_empty(),
        "these `--lock` runs declare no kind; give each `--tree <GiB>` (6 or less for a small walk) or \
         `--clock` (ADR 1684):\n{}",
        owed.join("\n")
    );
    let fixed: Vec<&str> = HELD_UNDECLARED_LOCKS
        .iter()
        .copied()
        .filter(|held| !undeclared.iter().any(|path| path == held))
        .collect();
    assert!(
        fixed.is_empty(),
        "these files now declare every `--lock`'s kind, so they leave the held list: {fixed:?}"
    );
}

/// The names `tools/batch.sh`'s `gates()` runs, with its `for t in …` loops expanded, whether the
/// loop's `run` is on the `for` line or the next.
fn merge_gate_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut inside = false;
    let mut looped: Vec<String> = Vec::new();
    for line in source.lines() {
        if line.starts_with("gates() {") {
            inside = true;
            continue;
        }
        if inside && line.starts_with('}') {
            break;
        }
        if !inside {
            continue;
        }
        let mut call = line.trim_start();
        if let Some(rest) = call.strip_prefix("for t in ")
            && let Some((list, after)) = rest.split_once(';')
        {
            looped = list.split_whitespace().map(str::to_owned).collect();
            call = after
                .trim_start()
                .strip_prefix("do")
                .unwrap_or(after)
                .trim_start();
        }
        if let Some(name) = call
            .strip_prefix("run ")
            .and_then(|rest| rest.split_whitespace().next())
            .map(|name| name.trim_matches('"'))
        {
            if name.contains("$t") {
                names.extend(looped.iter().map(|each| name.replace("$t", each)));
            } else {
                names.push(name.to_owned());
            }
        }
        if call.contains("done") {
            looped.clear();
        }
    }
    names
}

/// The merge's clock gates — the ones `tools/batch.sh` runs `--clock`, alone on both lanes — are
/// gates the merge runs: a name in `clock_gates` that no gate carries would leave the gate it meant
/// to name on one lane beside a walk, and say nothing (trap 25). And the list holds `t2-turn_path`,
/// whose bands are clocks by construction (ADR 1513). Calibrated by planting (trap 13): the reader
/// expands a loop and reads a quoted name.
#[test]
fn every_clock_gate_the_merge_names_is_a_gate_it_runs() {
    let planted = "gates() {\n    run build cargo build\n    for t in a b; do\n        \
                   run \"t3-$t\" cargo test; done\n    for t in c d; do run \"t2-$t\" cargo test; done\n\
                   \x20   run last cargo test\n}\nrun outside\n";
    assert_eq!(
        merge_gate_names(planted),
        ["build", "t3-a", "t3-b", "t2-c", "t2-d", "last"],
        "the reader is not the shape it states"
    );
    let source = std::fs::read_to_string(repository_root().join("tools/batch.sh"))
        .expect("tools/batch.sh is in the tree");
    let names = merge_gate_names(&source);
    assert!(
        names.len() > 20,
        "{} gate name(s) read from tools/batch.sh: the population is not the merge's",
        names.len()
    );
    let clocks: Vec<&str> = source
        .lines()
        .find_map(|line| line.strip_prefix("clock_gates=\""))
        .and_then(|rest| rest.strip_suffix('"'))
        .map(|list| list.split_whitespace().collect())
        .unwrap_or_default();
    assert!(
        clocks.contains(&"t2-turn_path"),
        "tools/batch.sh's clock_gates does not hold t2-turn_path: {clocks:?}"
    );
    let strangers: Vec<&&str> = clocks
        .iter()
        .filter(|clock| !names.iter().any(|name| name == *clock))
        .collect();
    assert!(
        strangers.is_empty(),
        "tools/batch.sh's clock_gates names gates the merge does not run: {strangers:?}"
    );
}
