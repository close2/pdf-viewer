# 1710 — A walk builds what it spawns inside its hold, and the self-test waits for events

Session 1437. Status: **accepted** and **built**. Answers ADR 1698 section 4's re-ask (the
`--bins` builds `tools/state.sh` ran before the lock), declares the fuzz walks' kinds from batch
sixty-seven's lock lines, gives `tools/batch.sh check` the fuzz workspace ADR 1694 found unbuilt, and
takes the wall clocks out of `tools/bounded.sh --self-test`. Supersedes nothing.
Context: ADRs 1659, 1674, 1684, 1694, 1698; traps 10, 13, 109, 130, 131.
Code: `tools/bounded.sh` (`--build`, self-test cases 1, 5, 7, 8, 9 and the new 10), `tools/state.sh`
(`walk`, `walk_worker`, `--walk-worker`, `gate_binaries` removed), `tools/batch.sh` (`check`),
`fuzz/seeds.sh`'s header, `doc/verify.md`'s two fuzz lock lines, `doc/environment.md`'s rule line.
Tests: `tools/conformance/tests/bounded.rs` (`no_state_section_builds_outside_the_hold_of_the_walk_
that_needs_it`, `a_walk_builds_the_sandbox_worker_of_its_own_profile`, `a_walk_whose_test_asks_hayro_
builds_it_inside_its_hold`, `state_walks` reads the gate after the walk's `--`), `read_only.rs`
(`first_command` skips a walk's options), `batch.rs` (`check_compiles_the_fuzz_workspace_…`).

## 1. The builds go inside the hold

**The premise, checked.** `grep -c -- --bins tools/state.sh` gives 8, but two of the eight are
comments; the builds were seven commands in five places — `gate_binaries` (the gates sandbox worker
and `pdfref-hayro`), `section_launch` (the release worker and `pdf-script-worker`), `section_vfs`,
`section_confined` and `section_ratchets` (the release worker) — and every one ran before its walk
queued, so a walk that waited spawned the tree as it was when the build ran (trap 109).

**Built.** `tools/bounded.sh --build '<cargo build arguments>'`, repeatable, runs `cargo build` inside
the hold and the tree bound, before the command, with the lanes' descriptors closed (trap 131); a
build that fails ends the run with its status and the command unstarted. The line's `cmd=` stays the
command's, so a gate keeps its name on the log and in `gates-cost`. `state.sh`'s `walk` hands it the
sandbox worker of the profile its command names, derived by `walk_worker` from the command's own
words — none for a `--workspace` command, which builds every package's binaries, and none for a
command that is not cargo's — and a section declares the rest: the oracle `pdfref-hayro`, the launch
path `pdf-script-worker`, and a ratchet whose test asks `Reference::Hayro` the same. **Two builds were
dropped rather than moved**: `pdf-vfs-worker` and `pdf-view-worker` are their packages' own binaries,
and Cargo builds those for the package's integration tests — the unit graph of `cargo test -p
viewer-confined --test awkward_classes --no-run` lists `pdf-view-worker` as a `bin` artefact. Trap 10
is about another package's binary. A warm no-op build costs a hold about 1 s (`--build '--profile gates
-p pdf-sandbox --bins'` held 1.2 s); a small walk is still killed at 6 GiB with a build inside it, as
`cargo test`'s own compile inside every walk already was (ADR 1698 section 4).

**Held.** The three tests above: no `cargo build` of `state.sh`'s own on any command line (planted: a
line of its own, a command substitution, a variable across a continuation; passed: a comment, a
printed command, a walk's `--build`); the worker `--walk-worker` prints for five shapes of command;
and a walk whose test file asks `Reference::Hayro` carries the `pdfref-hayro` build (one does, the
oracle). Self-test case 10 runs two builds through a `cargo` of its own and a third that fails;
calibrated (trap 13) by planting each defect in a scratch copy — the lanes left open, the failure not
stopping the run, the builds not run — and each failed the case.

## 2. The fuzz walks' kinds, from batch sixty-seven's lines

Every fuzz line of batch sixty-seven already declared a kind (`kind=small` on all of them), so the
change is to the instructions, which said `--tree 12`. Peaks: seeding 0.01–1.28 GiB and `seeds.sh
check` 0.04–1.29 GiB for ten targets, and 5.34 (seeding `jbig2`, which writes `jpx`), 5.36 and
5.55 GiB (`check jbig2`, `check jpx`); `cargo +nightly fuzz build -O -s none` 1.37 and 1.65 GiB; the
twelve 600-s campaigns 0.04–0.50 GiB. By ADR 1698's rule (small is half the second lane's kill or
less) every one is `--tree 6` but `jbig2` and `jpx`, which stay `--tree 12`, and the fuzz build goes
behind the lock as a small walk. **A campaign stays a small walk and is not a clock run**: its verdict
is a crash or none, and its edges and executions in 600 s are figures to state with the load beside
them, not a band. The cost is named: a clock run that finds one waits out its hold, and round 1428's
six clock runs queued 2 281.6 s behind round 1429's campaigns.

## 3. `check` compiles the fuzz workspace

`cargo check -q --manifest-path fuzz/Cargo.toml` under `--lock --round check --tree 12`, a large walk
because a cold one builds the targets' dependencies under the fuzz workspace's own lock file; it reads
`clean` on the batch's tree. The batch sandbox now gives every sandbox a lock of its own, and the test
plants a target with a type error: `fails`, with `error[E0308]` named, then `clean`, and `none` where
there is no manifest.

## 4. The self-test waits for events

The failure was case 7 or 9 (batch sixty-seven's record names no case); **both reproduce**. With the
old script pinned to two processors beside six busy loops it failed twice in two runs on case 7 (a
run that found the lock free logged `wait=0.1s` against `= 0.0`), and with two loops once on case 9
(the same against the small walk's `wait=0\.0s`); idle it passed three of three. Every hold is now
released by the case after the run under test has said it queued, every wait for a file is bounded at
a minute, "waited for nothing" is under one poll (0.5 s) rather than 0.0 s, case 1 bounds the walk's
processor time rather than its wall time (wall 1 333 ms and 1 583 ms at 12 and 24 loops, against the
old bound of 1 000), and case 5's bound is the leader's own 30 s life less a margin (25 s). **Held at**:
two processors with 6, 6, 12 and 24 busy loops (run queues of 4 to 13 a processor), four of four,
36–50 s a run — against the merge's 16 over 24 processors.

## 5. What stays

- `tools/batch.sh`'s merge still builds the four programs as gate lines of their own; the merge runs
  alone, so nothing edits the tree between them, and its `build-vfs` and `build-confined` lines are
  redundant by section 1's evidence and harmless.
- Traps 10 and 109 still tell a round to run `cargo build … --bins` inside its lock; the wrapper's
  `--build` is now the spelling, and the trap files are not this round's to edit.
