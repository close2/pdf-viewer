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
//! The self-test's ten cases are the script's own (`tools/bounded.sh --self-test` prints one line
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
//! lane refused (ADR 1684) — and a `--build` run inside the hold before its command, a failed one
//! ending the run unstarted (ADR 1710). Every hand-off between a case's runs waits for the run to say
//! what it was asked, never for a hold of so many seconds, and the one cost it bounds is processor
//! time: a hold an idle machine always outlasted failed inside the merge's whole-workspace test run at
//! a load of 16 (ADR 1710). This test runs the script and repeats what it said.
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
//! more a large one) or `--clock` (ADR 1684). The next two tests hold every tracked instruction to
//! that, and the merge's list of clock gates to the gates the merge runs.
//!
//! **And no state section walks unlocked** (ADR 1698). `tools/state.sh` is run by rounds as often
//! as by a person, and a walk it ran bare took no lane and wrote no line. One test holds every
//! walk it runs to its `walk` helper and a declared kind, and a gate the merge also runs to the
//! merge's own answer on whether its verdict is a time.
//!
//! **Nor builds what a walk spawns outside the walk's hold** (trap 109, ADR 1710). A worker built
//! before the lock is as old as the moment the walk stopped queueing, and siblings edit the tree
//! meanwhile. The last test holds `tools/state.sh` to no `cargo build` of its own — each walk takes
//! its builds with `--build`, inside its hold — and holds the worker each walk derives, and the
//! `pdfref-hayro` a walk whose test asks `Reference::Hayro` declares, to what the script prints.

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
/// names no batch branch: a wrapper run from a detached export of HEAD writes `batch=HEAD`, and
/// such a line is placed by its round, which is where a round's queue would otherwise go uncounted
/// (ADR 1675). The planted log puts such a line inside a batch, one whose round
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
const HELD_UNDECLARED_LOCKS: [&str; 0] = [];

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
    merge_gates(source)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// Every gate `tools/batch.sh`'s `gates()` runs, as its name and its command line — a command
/// continued with `\` joined into one, and a loop's `$t` replaced by each of the loop's words in
/// both.
fn merge_gates(source: &str) -> Vec<(String, String)> {
    let mut gates = Vec::new();
    let mut inside = false;
    let mut looped: Vec<String> = Vec::new();
    for (_, line) in joined_lines(source) {
        let line = line.as_str();
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
                gates.extend(
                    looped
                        .iter()
                        .map(|each| (name.replace("$t", each), call.replace("$t", each))),
                );
            } else {
                gates.push((name.to_owned(), call.to_owned()));
            }
        }
        if call.contains("done") {
            looped.clear();
        }
    }
    gates
}

/// The lines of a shell script with every command continued by `\` joined into the line it starts
/// on, so that a reader of words sees one command per line, each with the number of that first line.
fn joined_lines(source: &str) -> Vec<(usize, String)> {
    let mut joined: Vec<(usize, String)> = Vec::new();
    let mut continuing = false;
    for (index, line) in source.lines().enumerate() {
        match joined.last_mut() {
            Some((_, last)) if continuing => {
                last.push(' ');
                last.push_str(line.trim_start());
            }
            _ => joined.push((index.saturating_add(1), line.to_owned())),
        }
        continuing = line.trim_end().ends_with('\\');
        if continuing && let Some((_, last)) = joined.last_mut() {
            let kept = last.trim_end().trim_end_matches('\\').len();
            last.truncate(kept);
        }
    }
    joined
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

/// One walk a shell script runs: the line it starts on, the kind its `walk` declares, and the gate
/// it is where the line states one literally.
#[derive(Debug, PartialEq, Eq)]
struct Walk {
    /// The number of the line the command starts on.
    line: usize,
    /// The word after `walk`, or `None` where the command runs no `walk`.
    kind: Option<String>,
    /// `<package> --test <target>` or `<package> --example <name>`, where both are literal.
    gate: Option<String>,
    /// The command's words, continued lines joined.
    text: String,
}

/// The `<package> --test <target>` or `<package> --example <name>` a command's words state, where
/// both are literal rather than a shell variable.
fn gate_of(words: &[&str]) -> Option<String> {
    let after = |flag: &str| {
        words
            .windows(2)
            .find(|pair| pair.first() == Some(&flag))
            .and_then(|pair| pair.get(1))
            .map(|word| word.trim_matches('"'))
            .filter(|word| !word.contains('$'))
    };
    let package = after("-p")?;
    ["--test", "--example"]
        .iter()
        .find_map(|flag| after(flag).map(|name| format!("{package} {flag} {name}")))
}

/// Every walk in a shell script: a command, continued lines joined, whose `cargo test`, `cargo
/// nextest run` or `cargo run` builds under `--profile gates` or `--release` (or a profile held in a
/// variable), or runs over the whole `--workspace`. Every other `cargo` a state section runs is a
/// `conformance` count or a dev-profile lookup that holds no corpus, and a `cargo build` is a build.
/// A comment is not a command.
fn state_walks(source: &str) -> Vec<Walk> {
    let mut walks = Vec::new();
    for (line, text) in joined_lines(source) {
        if text.trim_start().starts_with('#') {
            continue;
        }
        let words: Vec<&str> = text.split_whitespace().collect();
        let runs_cargo = words.windows(2).any(|pair| {
            pair.first()
                .is_some_and(|word| word.trim_matches('"').ends_with("cargo"))
                && pair
                    .get(1)
                    .is_some_and(|verb| matches!(*verb, "test" | "nextest" | "run"))
        });
        let profile = words.windows(2).any(|pair| pair == ["--profile", "gates"])
            || words
                .iter()
                .any(|word| matches!(*word, "--release" | "--workspace" | "$profile"));
        if !(runs_cargo && profile) {
            continue;
        }
        let at = words.iter().position(|word| *word == "walk");
        let kind = at
            .and_then(|at| words.get(at.saturating_add(1)))
            .map(|kind| (*kind).to_owned());
        // The gate is the command's, after the walk's own `--`: a `--build` before it names a
        // package of its own (`-p hayro-compare`), and that is a build rather than the gate.
        let command = at
            .and_then(|at| {
                words
                    .iter()
                    .skip(at)
                    .position(|word| *word == "--")
                    .map(|end| at.saturating_add(end).saturating_add(1))
            })
            .and_then(|start| words.get(start..))
            .unwrap_or(&words);
        walks.push(Walk {
            line,
            kind,
            gate: gate_of(command),
            text: text.clone(),
        });
    }
    walks
}

/// The kind a state section's walk owes beside the merge's own line for the same gate: `clock` where
/// `tools/batch.sh` runs the gate as a clock run, and anything but `clock` where it runs it as a walk.
fn merge_clocks(batch: &str) -> Vec<(String, bool)> {
    let clocks: Vec<&str> = batch
        .lines()
        .find_map(|line| line.strip_prefix("clock_gates=\""))
        .and_then(|rest| rest.strip_suffix('"'))
        .map(|list| list.split_whitespace().collect())
        .unwrap_or_default();
    merge_gates(batch)
        .into_iter()
        .filter_map(|(name, call)| {
            let words: Vec<&str> = call.split_whitespace().collect();
            gate_of(&words).map(|gate| (gate, clocks.contains(&name.as_str())))
        })
        .collect()
}

/// The findings for one script against the merge's gates: a walk with no `walk`, a kind that is not
/// one of the three, and a gate the merge runs as a clock run declared as a walk here, or the other
/// way round.
fn walk_findings(script: &str, merge: &[(String, bool)]) -> Vec<String> {
    let mut found = Vec::new();
    for walk in state_walks(script) {
        let Walk {
            line, kind, gate, ..
        } = &walk;
        let named = gate.as_deref().unwrap_or("a command with no literal gate");
        let Some(kind) = kind else {
            found.push(format!("line {line}: {named} walks unlocked"));
            continue;
        };
        if !matches!(kind.as_str(), "small" | "large" | "clock") {
            found.push(format!("line {line}: {named} declares `{kind}`"));
            continue;
        }
        let merge_clock = gate
            .as_ref()
            .and_then(|gate| merge.iter().find(|(merged, _)| merged == gate))
            .map(|(_, clock)| *clock);
        if let Some(clock) = merge_clock
            && clock != (kind == "clock")
        {
            let merge_kind = if clock { "a clock run" } else { "a walk" };
            found.push(format!(
                "line {line}: {named} is `{kind}` here and {merge_kind} in the merge"
            ));
        }
    }
    found
}

/// **No `tools/state.sh` section walks unlocked** (ADR 1698). Every walk it runs goes through its
/// `walk` helper, which takes the heavy-walk lock in the lane the declared kind takes (ADR 1684), and
/// a gate the merge also runs is a `clock` walk here exactly where `tools/batch.sh`'s `clock_gates`
/// makes it a clock run there — a gate whose verdict is a time is one whoever runs it. Calibrated by
/// planting (trap 13): the reader names a walk run bare across a continuation, a kind no lane
/// knows, and a merge clock gate declared `large`, and passes a declared walk, a walk whose gate is
/// a variable, a `conformance` count and a comment.
#[test]
fn every_walk_a_state_section_runs_is_locked_in_its_declared_lane() {
    let merge_planted = "gates() {\n    run t2-clocked cargo test --release -p e --test clocked -- --ignored\n    \
                         for t in d; do\n        run \"t2-$t\" cargo test --profile gates -p c --test \"$t\" -- --ignored; done\n}\n\
                         clock_gates=\"t2-clocked\"\n";
    let merge = merge_clocks(merge_planted);
    assert_eq!(
        merge,
        [
            ("e --test clocked".to_owned(), true),
            ("c --test d".to_owned(), false)
        ],
        "the merge's reader is not the shape it states"
    );
    let planted = "section_a() {\n    run \"a\" 'x' \\\n        cargo test --profile gates -p a --test b -- --ignored\n    \
                   run \"c\" 'x' walk small -- cargo test --profile gates -p c --test d -- --ignored\n    \
                   run \"e\" 'x' walk large -- cargo test --release -p e --test clocked -- --ignored\n    \
                   run \"f\" 'x' cargo test -p conformance --test f -- --nocapture\n    \
                   # cargo test --profile gates -p g --test h\n    \
                   run \"g\" 'x' walk tiny -- cargo nextest run --workspace\n    \
                   run \"h\" 'x' walk clock -- \"$cargo\" test $profile -p \"$package\" --test \"$target\"\n    \
                   run \"i\" 'x' walk large --build '--profile gates -p other --bin x' -- \\\n        \
                   cargo test --release -p e --test clocked -- --ignored\n}\n";
    assert_eq!(
        walk_findings(planted, &merge),
        [
            "line 2: a --test b walks unlocked",
            "line 5: e --test clocked is `large` here and a clock run in the merge",
            "line 8: a command with no literal gate declares `tiny`",
            "line 10: e --test clocked is `large` here and a clock run in the merge",
        ],
        "the reader is not the shape it states"
    );

    let read = |path: &str| {
        std::fs::read_to_string(repository_root().join(path))
            .unwrap_or_else(|why| panic!("{path} is this gate's population: {why}"))
    };
    let merge = merge_clocks(&read("tools/batch.sh"));
    let script = read("tools/state.sh");
    let walks = state_walks(&script);
    let count = |kind: &str| {
        walks
            .iter()
            .filter(|walk| walk.kind.as_deref() == Some(kind))
            .count()
    };
    let shared = walks
        .iter()
        .filter(|walk| {
            walk.gate
                .as_ref()
                .is_some_and(|gate| merge.iter().any(|(merged, _)| merged == gate))
        })
        .count();
    println!(
        "tools/state.sh runs {} walk(s): {} small, {} large, {} clock; {shared} of them gates the \
         merge's {} also run",
        walks.len(),
        count("small"),
        count("large"),
        count("clock"),
        merge.len()
    );
    assert!(
        walks.len() > 30 && merge.len() > 20 && shared > 20,
        "{} walk(s) in tools/state.sh, {} gate(s) in the merge, {shared} shared: the population is \
         not the scripts'",
        walks.len(),
        merge.len()
    );
    let found = walk_findings(&script, &merge);
    assert!(
        found.is_empty(),
        "tools/state.sh walks outside its `walk` helper, or in a lane the merge does not; run each \
         as `walk small|large|clock -- <command>` (ADR 1698):\n{}",
        found.join("\n")
    );
}

/// The lines of a shell script that run `cargo build` themselves, as `(line number, line)`, continued
/// lines joined; a comment, and a line that only prints, build nothing.
fn builds_of_its_own(source: &str) -> Vec<(usize, String)> {
    joined_lines(source)
        .into_iter()
        .filter(|(_, text)| {
            let code = text.trim_start();
            let words: Vec<&str> = code.split_whitespace().collect();
            !(code.starts_with('#') || code.starts_with("echo ") || code.starts_with("printf "))
                && words.windows(2).any(|pair| {
                    pair.first().is_some_and(|word| {
                        word.trim_start_matches(['$', '(', '"', '\''])
                            .trim_end_matches('"')
                            .ends_with("cargo")
                    }) && pair.get(1) == Some(&"build")
                })
        })
        .map(|(line, text)| (line, text.trim().to_owned()))
        .collect()
}

/// The test file a `<package> --test <target>` gate runs, in whichever of the tree's crate roots holds
/// it.
fn test_source(gate: &str) -> Option<String> {
    let mut words = gate.split_whitespace();
    let (package, flag, target) = (words.next()?, words.next()?, words.next()?);
    (flag == "--test")
        .then(|| {
            ["crates", "raster/crates", "tools"]
                .iter()
                .find_map(|root| {
                    std::fs::read_to_string(
                        repository_root().join(format!("{root}/{package}/tests/{target}.rs")),
                    )
                    .ok()
                })
        })
        .flatten()
}

/// The walks whose test asks `Reference::Hayro` and whose command builds no `pdfref-hayro`: that
/// reading is another package's binary, and a walk without it votes with three references and says
/// nothing (trap 10, ADR 0222). `source_of` reads a gate's test file.
fn hayro_findings(
    script: &str,
    source_of: impl Fn(&str) -> Option<String>,
) -> (Vec<String>, usize) {
    let mut found = Vec::new();
    let mut asking = 0_usize;
    for walk in state_walks(script) {
        let Some(gate) = walk.gate.as_deref() else {
            continue;
        };
        if !source_of(gate).is_some_and(|source| source.contains("Reference::Hayro")) {
            continue;
        }
        asking = asking.saturating_add(1);
        if !walk.text.contains("--bin pdfref-hayro") {
            found.push(format!(
                "line {}: {gate} asks Reference::Hayro and builds no pdfref-hayro",
                walk.line
            ));
        }
    }
    (found, asking)
}

/// **What a walk spawns is built inside the walk's hold** (trap 109, ADR 1710). A worker
/// `tools/state.sh` built before the lock is as old as the moment its walk stopped queueing, and five
/// siblings edit the tree meanwhile, so the script runs no `cargo build` of its own: `walk` hands
/// `tools/bounded.sh` a `--build` for the sandbox worker of the command's profile, which the wrapper
/// runs inside the hold before the command, and a walk that spawns another package's program declares
/// it. This test names a build outside a walk; the two after it hold the worker the script derives,
/// and a walk whose test asks `Reference::Hayro`. Calibrated by planting (trap 13): the reader names
/// a build on a line of its own, inside a command substitution and through a variable across a
/// continuation, and passes a comment, a printed command and a walk's `--build`.
#[test]
fn no_state_section_builds_outside_the_hold_of_the_walk_that_needs_it() {
    let planted = "section_a() {\n    cargo build --profile gates -p pdf-sandbox --bins >/dev/null 2>&1 || status=1\n    \
                   out=$(cargo build -p x 2>&1)\n    \"$cargo\" build \\\n        --release -p y\n    \
                   # cargo build -p z\n    printf 'cargo build -p w\\n'\n    \
                   run \"a\" 'x' walk small --build '-p v --bins' -- cargo test --profile gates -p a --test b\n}\n";
    let found: Vec<usize> = builds_of_its_own(planted)
        .iter()
        .map(|(line, _)| *line)
        .collect();
    assert_eq!(
        found,
        [2, 3, 4],
        "the build reader is not the shape it states"
    );

    let script = std::fs::read_to_string(repository_root().join("tools/state.sh"))
        .expect("tools/state.sh is in the tree");
    let builds: Vec<String> = builds_of_its_own(&script)
        .iter()
        .map(|(line, text)| format!("line {line}: {text}"))
        .collect();
    assert!(
        builds.is_empty(),
        "tools/state.sh builds outside the hold of the walk that spawns what it builds; give the walk \
         `--build '<cargo build arguments>'` (trap 109, ADR 1710):\n{}",
        builds.join("\n")
    );
}

/// The sandbox worker each walk takes is the one the script derives from the command's own words: the
/// command's profile, nothing for a whole-`--workspace` command, which builds every package's binaries,
/// and nothing for a command that is not cargo's. Asked of the script itself, `--walk-worker`, so the
/// derivation that runs is the one held (ADR 1710).
#[test]
fn a_walk_builds_the_sandbox_worker_of_its_own_profile() {
    let worker = |command: &[&str]| {
        let output = Command::new("bash")
            .arg(repository_root().join("tools/state.sh"))
            .arg("--walk-worker")
            .args(command)
            .output()
            .expect("bash runs tools/state.sh");
        assert!(
            output.status.success(),
            "tools/state.sh --walk-worker {command:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    for (command, wanted) in [
        (
            &[
                "cargo",
                "test",
                "--profile",
                "gates",
                "-p",
                "a",
                "--test",
                "b",
                "--",
                "--ignored",
            ][..],
            "--profile gates -p pdf-sandbox --bins",
        ),
        (
            &[
                "cargo",
                "run",
                "--release",
                "-q",
                "-p",
                "a",
                "--example",
                "b",
            ][..],
            "--release -p pdf-sandbox --bins",
        ),
        (
            &["cargo", "test", "-p", "a", "--", "--release"][..],
            "-p pdf-sandbox --bins",
        ),
        (&["cargo", "nextest", "run", "--workspace"][..], ""),
        (&["python3", "tools/x.py"][..], ""),
    ] {
        assert_eq!(
            worker(command),
            wanted,
            "the worker tools/state.sh derives for `{}`",
            command.join(" ")
        );
    }
}

/// A walk whose test asks `Reference::Hayro` builds `pdfref-hayro` inside its hold: the oracle's fourth
/// reading is another package's binary, and without it the walk votes with three references and says
/// nothing (trap 10, ADR 0222). Calibrated by planting (trap 13): the reader names a walk that asks and
/// builds nothing, and passes one that builds it and one that does not ask.
#[test]
fn a_walk_whose_test_asks_hayro_builds_it_inside_its_hold() {
    let script = std::fs::read_to_string(repository_root().join("tools/state.sh"))
        .expect("tools/state.sh is in the tree");
    let planted = "section_b() {\n    run \"a\" 'x' walk clock -- cargo test --profile gates -p m --test asks -- --ignored\n    \
                   run \"b\" 'x' walk clock --build '--profile gates -p hayro-compare --bin pdfref-hayro' -- \\\n        \
                   cargo test --profile gates -p m --test asks -- --ignored\n    \
                   run \"c\" 'x' walk small -- cargo test --profile gates -p m --test quiet -- --ignored\n}\n";
    let (found, asking) = hayro_findings(planted, |gate| {
        Some(
            if gate.ends_with("asks") {
                "Reference::Hayro"
            } else {
                "Reference::Poppler"
            }
            .to_owned(),
        )
    });
    assert_eq!(
        (found, asking),
        (
            vec![
                "line 2: m --test asks asks Reference::Hayro and builds no pdfref-hayro".to_owned()
            ],
            2
        ),
        "the hayro reader is not the shape it states"
    );
    let (found, asking) = hayro_findings(&script, test_source);
    println!("{asking} walk(s) in tools/state.sh ask Reference::Hayro");
    assert!(
        asking >= 1,
        "no walk in tools/state.sh asks Reference::Hayro, so the oracle is not in the population"
    );
    assert!(
        found.is_empty(),
        "these walks ask Reference::Hayro and build no pdfref-hayro inside their hold:\n{}",
        found.join("\n")
    );
}
