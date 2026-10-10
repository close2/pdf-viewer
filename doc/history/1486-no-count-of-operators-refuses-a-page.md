# 1486 — No count of operators refuses a page, and what a run keeps is charged

Slot 1 of batch seventy-seven, 2026-10-10, a parsers round, the project owner's own request ("as
long as it isn't a resource problem (only a time problem) we should allow it"). ADRs 1809, 1810.
Ledger: no status moves; §7.4.4.2's, §7.8.2's and §8.7.3.1's notes re-read and corrected where they
named the bound.

**Premise, held.** `MAX_OPERATIONS` had one check (`run.rs`), three in `repeat_cell` and the cap on
`list_budget.rs`'s `next_check`, as the brief said; it is gone, and the count stays as a figure —
round 1487's progress hook in the periodic branch (kept), `Interpretation::operators`, the census.

**Hypothesis, did not hold.** The list's bytes did not bound a runaway alone: painted paths were
"not charged: its segments each cost an operator", and the readback (text, placements, spans, their
`/Alt` strings) was charged to nothing, so a form re-read through a fan-out of `Do`s kept both
without bound once the count went. And the owner's A1 drawing stood at **504.7 MiB of 512 MiB**
(4 408 405 commands, 1 256.0 MiB peak). Both are now charged — paths at the painting operator,
the readback at the slow path — and `MAX_LIST_BYTES` is **2 GiB**, half the confined worker's 4 GiB
address space; the drawing charges **740.1 MiB**, `unsupported []`, rendered and looked at whole.

**What ends a hundred million `0 g`**: its stream's end, then Table 31's `max_stream_len` by name;
both tested. A stream that re-reads streams keeping nothing is the person's stop (ADR 0657);
`quorra-transform render`, `pdf-vfs` and the `page` fuzz target's `-timeout` wait (ADR 1809 §4).
Planted (trap 13): the path charge and the readback charge each removed fails the new charge test.

**Files.** `crates/pdf-model/src/content.rs`, `content/{run,pattern,list_budget,ledger,report}.rs`,
`src/accessibility.rs` (comment); `tests/{hostile_budgets,tiling,replacement}.rs`;
`examples/{content_budget_census,replacement_census}.rs`; `crates/viewer-core/src/report.rs`;
`doc/conformance/ledger.toml`; `doc/todo/49`; ADR 1808's status; ADRs 1809, 1810; this record.

**Gates.** `rustfmt --check` on every file touched: clean. `clippy -D warnings --all-targets` on
`pdf-model`, `viewer-core`: exit 0. `nextest -p pdf-model`: exit 0, 2035 passed; `-p viewer-core`:
exit 0, 374 passed. `cargo test -p conformance`: exit 0, 430 passed. `hostile_budgets` (`--tree 12`,
queued 351.7 s): exit 0, 34 passed, 3.19 GiB peak. `raster_golden` (gates, `--tree 12`, queued
193.3 s): exit 0, held 974, moved 0. The six arms not re-run: `display_list_census` before and
after over the corpus gate's 958 first pages differs on no page's commands or refusal, so no digest
can move by this change. Duration 3 450 s.
