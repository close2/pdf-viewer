# 1525 — A pointer in a trap or a habit is kept current, and its story is not rewritten

Session 1345. Status: accepted. Context: ADR 1023 (the comment rule, which exempts traps and habits),
ADR 1511 (`doc/todo/02` section 5 rewritten: the merge installs), ADR 1440 (a batch builds apart from
the main checkout). Prose: `doc/habits/tests-gates-and-reports.md`, `doc/todo/02-every-round.md`.

## 1. The question

`CLAUDE.md` exempts a trap or a habit from the comment rule because each is an incident with a rule
attached, and the history is the lesson. But a trap or habit sentence can also send a reader
somewhere: a section to open, a command to run, or the line that rebuilds something. After ADR 1511
rewrote `doc/todo/02` section 5, one habit still said that section "rebuilds every fifth round" the
`--profile gates` worker. No command does that. Does the exemption cover that sentence?

## 2. The decision

**No. The exemption covers the story, not the pointer.** A sentence saying what happened, to whom,
and what it cost stays as written, in its tense. A sentence saying where to look or what does a
job now is a claim about the tree, and it is kept true. When the tree moves, the pointer moves with
it, and the story around it keeps its words. A pointer whose claim was true at the incident and is
still true stays, even if it names an old section. Three such pointers in
`doc/traps/instruments-and-reports.md` were read and left. Trap 10's third copy says that
`doc/todo/02`'s section 5 does not reach `release/examples/`, and the install does not. Trap 15 says
that section now derives its directory, and the install asks Cargo for it. Trap 39's fifth round is
still printed by `tools/round.sh`.

## 3. What the sweep found

Every habit and trap file was searched for `doc/todo/02` sections, `tools/` commands and their
sections, "fifth round" and `CARGO_TARGET_DIR`. Every `tools/state.sh`, `batch.sh`, `worktree.sh`,
`round.sh` and `fuzz.sh` sub-command named there exists. Two sentences were false:

- `doc/habits/tests-gates-and-reports.md`: the gates worker is now built by the first line of
  `doc/todo/02` section 2's tier-2 block and by `tools/batch.sh gates`' `build-sandbox` line. The
  same sentence's "the workspace shares one `CARGO_TARGET_DIR`" was the incident's state, so it is
  now in the past tense: since ADR 1440 each tree names its own build directory.
- `doc/todo/02` section 2: two sentences named section 5 as what rebuilds the binaries, one of them
  every fifth round. Section 5's `tools/batch.sh install` does it at the merge.

Trap 50's index line ("two exported trees that share a `CARGO_TARGET_DIR`") still describes the
position that springs it, and is kept.
