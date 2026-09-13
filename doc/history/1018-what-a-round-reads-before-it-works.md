# 1018 — What a round reads before it works

Date: 2026-09-13. Branch `batch-1006-1011`, five sibling rounds live in the same worktree. The
owner's contract, on `doc/reviews/1012-where-the-effort-goes.md`: **cut what every round pays before
it does any work, without deleting a single reason.** Output: ADR 1036, `doc/traps/README.md`,
`doc/reviews/1018-what-retiring-a-trap-would-cost.md`.

## What was done

1. **Re-derived 1012's figures** rather than trusting them; the trap skew held, the habit one did not.
2. **Built the trap index**, `doc/traps/README.md`: one line per trap, the position that springs it
   and the rule. `doc/HANDOVER.md`'s 41-row table moved into it and gained the condition column;
   HANDOVER and the five group files point at it, and not a line of their 2,509 lines of incident,
   evidence and argument was touched.
3. **Measured the habits and declined an index** — six files cited two to five times, flat; a
   per-bullet index would add ~200 lines to the standing read (ADR 1036 §1).
4. **Tiered `doc/todo/02` §2** into three fenced blocks, one sentence per tier saying why, and
   retiered the map's six rules. No gate deleted; tier 3 moves to the merge, which rule 5 required.
5. **Decided what `doc/history/` is**: a record the sweep still reads and reports **apart** —
   `prose::RECORDS`, `prose::is_a_record`, a test, a third tally in `--bin quotations`.
6. **Evaluated the sixteen cold traps**: 1 retire (conditional), 2 merge, 1 demote, 12 keep.

## What it measured

- Standing read for a round that writes Rust: **2,868 → 2,035 lines**, and now the same 2,035
  whatever the round is about. The five core files are 1,951 both ways (HANDOVER −60, §2 +60); what
  came off is the group file a round opened for one trap.
- Tier 1 is 7 of the 39 gate lines and carries 17 of the ~18 self-catches; tier 3's ~25 minutes go to
  the merge, once per batch rather than once per round.
- `--bin quotations`: the figure a round moves falls **51 → 38**, with 13 in 566 record files.

**Found on the way — trap 7 is violated three times in the committed tree**: two
`#[allow(clippy::too_many_arguments)]` in `crates/pdf-transform/src/archive/mod.rs` and one in
`archive/remedies.rs` carrying a `reason`; `clippy::allow-attributes` exists in this toolchain and is
not in `Cargo.toml`'s lint table. Reported, not fixed — five rounds are editing that crate.

**Gates.** `sandbox_gates`, `state_sections`, `workspaces`, `questions`, `submodules` and 244
library tests pass, which is what says splitting §2's one fence into three broke no parser. Three `conformance.rs`
tests fail on siblings' mid-edit work: a `structure.rs` blockquote, and an unparseable `ledger.toml`.
