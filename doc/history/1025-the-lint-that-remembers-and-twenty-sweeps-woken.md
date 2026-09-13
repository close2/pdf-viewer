# 1025 — The lint that remembers, two merges, a demotion, and twenty sweeps woken

Instruments slot of batch 1020–1025, on `batch-1020-1025`, five siblings live in the same worktree.
Theme: a rule a round must **remember** is worth less than a rule a program **enforces**.

**1. Trap 7 is a gate.** `clippy::allow_attributes = "warn"` in `[workspace.lints.clippy]` and in
`fuzz/Cargo.toml`'s copy (trap 23). 221 outer `#[allow]` found — **215 of them under `raster/`**,
which review 1018's grep over `crates/` and `tools/` could not see (trap 25). 190 converted, **31
deleted because they silenced nothing**: 49 dead lint names in all — `expect_used`/`panic` in tests
that `clippy.toml` already exempts, `arithmetic_side_effects` on float-only code, `float_cmp`
against a zero literal, `too_many_lines` under the threshold. Four design sentences from deleted
`reason` fields are kept as doc comments. No honest `#[allow]` exception arose; none is kept. The
three `unsafe_code` lifts became `#[expect]` and the three `unsafe_position.rs` tests moved with
them, the cross-crate sweep now matching all four spellings. **Trap 7 is rewritten, not retired**:
the lint holds *outer* attributes only — calibrated, an outer plant failed (exit 101) and an inner
`#![allow]` plant passed silently (exit 0) — and 160 inner `#![allow]` stand. ADR 1042.

**2. Review 1018's other verdicts, applied.** Trap **4 → 8** and trap **29 → 13**, each as a
`####` sub-heading inside the trap that already named it as its mirror; trap **36 →
`doc/habits/measuring.md`**. Every incident kept whole, every number kept citable, and
`doc/traps/README.md` keeps a row for each saying where its incident now lives.

**3. All twenty conformance binaries run.** One defect found and fixed: `--bin quoted` and `--bin
unpriced` read the oracle log off standard input *whenever no path was given*, so the obvious
invocation blocked for ever having printed nothing — trap 18's shape one level up. Standard input
is now asked for by name (`-`); no argument is a refusal in a sentence, exit 1.

| verdict | binaries |
|---|---|
| **act — a reading list, cadence one per merge** | `blockers` (21 expired refusals), `counts` (5 double-counted families), `entries` (65 entries no note disposes of), `inapplicable` (43 confirmed, 252 with a contradicting cousin), `overstated` (2 unmarked contradictions), `overtaken` (1 member the prose does not name), `owed` (130 debts named in a word), `permitted` (50 rows resting on an optional entry), `quotations` (38 diverging), `tables` (7 denials a table contradicts), `unread` (114 keys the row's own code quotes) |
| **fix first — the level is noise (trap 39)** | `callers` (126, and its own tail says only the delta counts — it stores no baseline), `capabilities` (residue is true statements about deliberate boundaries), `parts` (403 of 603 sit in dated records), `pointers` (233 absent, monotone for 470 sessions), `undenominated` (155 impossible denominators inside 2971 claims). Each needs its population partitioned the way `--bin quotations` was (ADR 1036 §3) before a cadence buys anything. |
| **the merge's, with the oracle log** | `quoted`, `unpriced` |
| **on demand** | `retired` (wants the nouns the round retired), `ledger` (already inside `tools/state.sh`) |

The cadence is written into `doc/todo/02-every-round.md` §4, which is where a round reads.

**Gates.** Tier 1 run in full. Every failure belongs to a sibling's in-flight work in
`crates/pdf-signature/` and `crates/pdf-model/src/document_part.rs`; not one is in a file this
round touched. `cargo fmt --all` was run once before `--check` — a slip against this round's own
rule; it changed nothing of a sibling's, checked by diff.
