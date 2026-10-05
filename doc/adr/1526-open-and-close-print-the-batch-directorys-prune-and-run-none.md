# 1526 — `open` and `close` print the batch directory's prune, and run none

Session 1345. Status: accepted. Context: ADR 1500 (`tools/state.sh disk`), ADR 1463 (`open` warms
`dev`), ADR 1440 (one build directory for every batch). Code: `tools/batch.sh`
(`debug_over_the_rule`). Test: `tools/conformance/tests/batch.rs`'s
`open_and_close_print_the_prune_once_debug_is_over_the_rule_and_prune_nothing`. Prose:
`doc/environment.md`, `doc/todo/02` section 8 step 5, `doc/PLAN.md`.

## 1. The question

After batch fifty-two, `tools/state.sh disk` printed the batch directory's `debug` at 172 GB, and
`doc/environment.md` sets the rule at 100 GB. `open` starts the warm build into that directory, so
a prune after `open` deletes what the rounds are about to read. The prune belongs between `close`
and `open`. But nothing told the orchestrator at either moment.

## 2. The decision

`close`, after it removes the worktree, and `open`, before it makes one, both read
`du -sk <batch directory>/debug`. When the figure is over the rule they print the size and the
`rm -rf` line. **They print it and never run it.** The prune is the orchestrator's decision, made
with no round running and with `doc/environment.md`'s list of what each directory costs to lose.
A script that deleted a build directory on a threshold would delete one a live round is reading.

The rule is `BATCH_DEBUG_RULE_KIB`, which defaults to 100 GiB in KiB, the unit `du -sh` prints. The
directory is `BATCH_TARGET_DIR` or its default, the same variable `open` writes into the worktree's
configuration. The test points both at a sandbox, so no test runs `du` over the machine's own
directory. It plants 2 MiB of real bytes, which no filesystem stores sparse. It then checks both
ways: under a 1 MiB rule both commands print the prune and the file survives, and under a 1 GiB
rule neither prints one.

Measured on 2026-10-05: `du -sk` over the 171 GiB tree takes 0.22 s warm, which is nothing beside
`open`'s warm build.
