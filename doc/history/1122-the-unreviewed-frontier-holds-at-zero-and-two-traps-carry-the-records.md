# 1122 — The unreviewed frontier holds at zero, and two traps carry the records

2026-09-16. Instruments slot. Files: this record only. No code edit is owed; no ADR; no ledger
row moved (there was none to move). Worktree clean at open; no sibling paths in `git status`.

## 1. The unreviewed frontier — closed, so the campaign is pointed at `partial`

`cargo run -q -p conformance --bin ledger`: `883 rows, 0 new` — `implemented 559, partial 108,
departed 14, reported 11, inapplicable 67, writer-side 9, out-of-scope 115`. **Zero `unreviewed`,
and zero new** (the bin invents an `unreviewed` row for any subclause with none; it invented none,
so every subclause the spec tree carries has a read row). The frontier was read to zero in round
1010 (ADR 1010) and has stayed shut for 112 rounds; nothing was moved because there is no
`unreviewed` row at all. The next frontier is the read-but-incomplete one: **`partial` (108)**
concentrates in clause 12 (45), 11 (24), 8 (16), 7 (12), 10 (5), 14 (4), then 6 and 9 singletons;
**`reported` (11)** is clause 12 (6), 7 (4), 10 (1) — a batch wanting a target takes clause 12's
`partial` block first, 42% of the frontier.

## 2. §-checker and record budget — both hold over every current document

`cargo test -p conformance` green, all suites: `tests/documents.rs` `every_section_sign_in_an_
instruction_document_is_iso_32000_2s` passes, so `§` still means ISO 32000-2 and nothing else over
every committed instruction document; `tests/records.rs` `every_record_since_the_budget_was_
counted_fits_inside_it` passes. Re-measured by hand: every record numbered ≥ 1086 is ≤ 40 lines
(the over-40 records all predate the budget; last is 1085; highest record present is 1117). No new
`§`-on-a-non-ISO clause to report for the merge — the worktree was clean, so no sibling's in-flight
file is visible here to check.

## 3. The review/trap frontier

**984's findings are mostly exhausted by consequence.** F2's comment rule is now in `CLAUDE.md`
with four documents rewritten as *what is* (round 1003); F1's first extraction is done
(`crates/pdf-signature` exists; `pdf-colour` still stands); F3's three ungated walks are in
`doc/todo/02` tier 3 with `state_sections.rs` holding the population; F4's own-output golden exists
(`raster_golden`, ADR 1016); F5's first step is in (`Examination::reaches()`, `examination.rs:117`),
its interpreter-observer question and `survey.rs`'s 3840-line second state machine still standing;
F6's Q53–Q60 are answered. Standing: `pdf-colour`, the observer, Finding 7's small inversions.
**Trap citations** over the last ~30 records (109*–111*): 28 across 6 distinct traps, two carrying
it — **trap 13 (13) and trap 8 (10) = 82%**, the measuring-round pair. Full-history "8 traps carry
most" still holds (13, 8, 9, 1, 11, 10, 5, 15 lead), but the recent cadence narrowed to those two.
Gates: `cargo test -p conformance` ok (all suites, 0 failed); `tools/state.sh ledger` exit 0.
