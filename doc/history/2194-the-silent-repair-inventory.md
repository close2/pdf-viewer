# Round 2194 — the silent-repair inventory

The first step of the owner's campaign of 2026-11-23, in the `gwg` worktree: every place
`pdf-syntax`, `pdf-model` and `pdf-font` repair, recover, substitute or tolerate instead of reading
what the file says, and which of them reach `pdf-archive` without a report. It fixes nothing.

## What it decided and found

- ADR 3477 (accepted, the owner's): one channel — a `Repair` with a static `RepairClass`, logged
  on the `Document`, one `CLASSES` slice per crate, the existing reports kept as detail;
  `pdf-archive` maps every class to section 5.1's row (to become `Implemented`) and to the row it
  touches; a conformance gate fails on an unmapped class.
- `doc/reviews/2194-silent-repairs.md`: 446 census sites (147 syntax, 213 model, 86 font), 33 not
  repairs, 239 reaching the validator silently, 27 reported to it, 10 caught by raw-byte rows,
  170 off its path. 8 B, 6 T, 0 D.
- The most serious: B02, an object that does not parse (or a misfiled one) is cached as null and
  the page's content leaves every content row, so the damaged file fails *fewer* rows than its
  twin; B05, a TrueType program with a rewritten `loca` or cut in half passes every font row.
- Section 5.1's row is `Unchecked` and the verdict counts only `Failed` rows, so every repair
  whose specific row does not re-read the bytes is certified `Conforms`.
- 18 failing tests in `crates/pdf-archive/tests/silent_repairs_spec_audit.rs`, each a damaged file
  against its twin; listed in the review's `expected-failures.json` and appended to
  `expected-failures-current.json`.

**Ledger.** No row moves: an audit changes no code.

**Gates.** fmt `--check` over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p
viewer-qt` exit 1 on the new test file, reformatted, then 0; clippy `-D warnings --all-targets`
over the same set exit 101 (one `doc_markdown`), fixed, then 0; `pdf-sandbox --bins` 0; nextest
`--lib --bins --tests` of the set (`xvfb-run -a`): 4292 run, 4273 passed, 19 failed, 28 skipped,
`audit-diff.py` "failed 19, expected 19" (this round's 18 and 2197-B01); rustdoc `-D warnings
--document-private-items` of `pdf-archive`, private target, 0; `cargo test -p conformance
--no-fail-fast` exit 101 on two of this round's own lines (a doc path, this record's gate
paragraph), fixed, then 0. Fuzz not owed: no public type changed.

## Habit

A twin is the oracle for a repair: the damaged file and the same file without the damage, held
to the validator, and the finding is a row the twin passes and the damaged file does not fail.
The twin need not conform.
