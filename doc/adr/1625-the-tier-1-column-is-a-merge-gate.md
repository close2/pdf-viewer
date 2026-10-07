# 1625 — The Tier 1 column is a merge gate, held to ceilings of its own

Session 1394. Status: **accepted** and **built**. Builds on RFC 0008 section 6.7 (accepted,
`doc/questions/A193`) and ADR 1602, which built the column as a census. Amends nothing.
Context: `CLAUDE.md` principle 3 (explicit budgets against pathological content); ADRs 1392 (a
green line that ran nothing), 1535 (`==` ratchets), 1590 (the budgets), 1612 (the task budget).
Code: `crates/pdf-script/tests/script_corpus.rs` (`HELD_*`), `tools/batch.sh` (`gates()`, `run()`,
`install_featured`), `tools/conformance/tests/batch.rs`,
`tools/state.sh` (`section_scripts`), `doc/todo/02-every-round.md` section 2.

## The question

`crates/pdf-script/tests/script_corpus.rs` runs every script Tier 0 does not run, over the census
population (90 763 documents), in that document's realm under `Budget::FIELD_EVENT`. It counts each
run as finished, finished-but-refused, over a budget, threw, or unparsed. It is the only
measurement of what the engine does with the scripts that exist. It ran in no gate. Its own header
said that no build `tools/batch.sh gates` makes turns the `engine` feature on, and nothing in it
was held. Should the merge run it?

## Against, priced

- **The engine is off by default, and the gate builds it at every merge.** Measured on this
  machine, 2026-10-07: the `gates`-profile build took 33 s with the batch's target directory warm.
  The walk took 107 s, against 1 459 s for the other 34 gates in the last merge's log. That is about
  a tenth more wall time. Feature unification does not leak: `--features engine` on
  `-p pdf-script --test script_corpus` turns the feature on for that test binary's build of
  `pdf-script`, and for nothing installed.
- **Its figures move as the bridge grows.** Every member bridged moves the `threw` and `refused`
  columns. A held `==` would fail every round that bridges a member. Answer: ceilings, not
  equalities (below).

## For, and why it wins

- **RFC 0008 section 6.7 names this gate.** It asks for budget exceedances, throws and refusals as
  three columns with a ratchet each, and says that a document which finishes under budget stays
  under budget. Section 6.3 puts the `on` question back to the owner only once the gate exists and
  has been green for a batch. A census no merge runs can never be green for a batch.
- **The budgets are what it proves.** Principle 3's budgets are claims about pathological content,
  and this is the one run that puts the world's scripts against them. Today no run is over a budget.
  A change that lets one through is exactly the regression the gate exists to catch.

## Decision

1. **`t2-script_corpus_engine`** in `tools/batch.sh gates`:
   `cargo test --profile gates -p pdf-script --features engine --test script_corpus -- --ignored`.
   It runs through `tools/bounded.sh --data 8 --tree 12`, because a realm's heap is the engine's to
   bound and the walk is ninety thousand documents of them. `run()`'s zero-tests check now finds
   `cargo test` anywhere in the line, so a gate behind the wrapper is held to ADR 1392 as well.
2. **What it holds**, as the column's own constants:
   - `HELD_EXCEEDED`, held at 0.
   - Ceilings: `HELD_THREW` 9 593, `HELD_FINISHED_REFUSED` 0, `HELD_UNPARSED` 8.
   - A floor, `HELD_RUNS` 20 789. Without it, a walk that reached fewer documents would pass the
     ceilings by running less (trap 13).

   These are ceilings rather than ADR 1535's `==`, because the column is meant to move as the
   bridge grows. A fall is printed as a `ratchet:` line, which asks for the ceiling to be lowered
   with it. A rise fails and names the column.
3. **`tools/state.sh scripts`** prints both columns, Tier 0's and this one's, and this one's `held:`
   line.
4. **`doc/todo/02` section 2** carries the line in tier 2, and a `pdf-script` row in the map.

## And the worker is installed

ADR 1609 held `pdf-script-worker` back until a host supplied a level for scripts. ADR 1616 is that
level, in every window. It names the build the worker needs: a Cargo run of its own, with
`engine`, so that feature unification puts no engine into a window. So `tools/batch.sh install`
now carries `install_featured="pdf-script-worker:pdf-script-worker/engine"`. Each entry is built
with `cargo build --release --bin <name> --features <package/feature>` and installed beside the
windows. `tests/batch.rs` holds that list equal to every `[[bin]]` under `crates/` that states
`required-features`, and its stand-in workspace builds each entry as a package whose binary
requires the feature.

## What it does not decide

Whether the level's default moves from `off`. That question is the owner's, asked once this gate
has been green for a batch (RFC 0008 section 6.3).
The tree as this batch branched read 20 734 runs and 9 575 throws. The same walk an hour later,
with this batch's other rounds' `pdf-script` and `pdf-model` changes in the worktree, read 20 789
runs and 9 593 throws, and the gate failed on the throws. That is the instrument doing its job, and
the constants are the second reading. The merge runs the gate on the merged tree. A figure that
moves there is read against the column's printed throws and sites before its constant moves, and
the reason is written beside it.
