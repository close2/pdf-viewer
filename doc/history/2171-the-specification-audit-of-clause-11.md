# Round 2171 — the eighth specification audit: clause 11, transparency

An audit round in the `gwg` worktree, made in the shape of review 2038 and audits 2128 to 2161:
§11.1 to §11.7.5.3 (Tables 134 to 149) against `pdf-model`'s graphics state, groups, soft masks
and implicit groups, and the compositing of `render-cpu` (the oracle), `render-raster` (quorra,
on the RX 580) and `render-gpu` (Vello). It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2171-specification-audit-11.md` and its directory. Four findings: 4 B, 0 T, 0 D.
- B01 a `gs` with no `/SMask` entry removes the soft mask in force (an absent key is read as
  `/None`); B02 the page group's `/K` is never read; B03 a soft-mask dictionary that breaks
  Table 142 — a `/BC` of the wrong length, a `/Luminosity` group without `/CS` — is drawn in
  silence; B04 a `/BM` name outside Tables 134 and 135 is drawn as Normal unreported.
- Five failing `*_spec_audit` tests, each with a control, in
  `crates/pdf-model/tests/transparency_clause11_spec_audit.rs` (`transparency_suite`); listed in
  the audit's `expected-failures.json` and appended to `expected-failures-current.json`.
- About forty fixtures probed end to end; the same display lists drawn by all three backends agree
  at the probe point or are refused by name (Vello: non-isolated and four-component groups;
  quorra: non-isolated knockout). No backend disagreement is a finding. Two readings left: the
  CIE branch of §11.5.3 (the owner's A23, ADR 2552) and Vello's knockout at partial coverage.

## Ledger

Clause 11 rows before and after: 57 `implemented`, 1 `departed`. B findings contradict 11.6.4.3 and 8.4.5 (B01),
11.4.7 and 11.6.6 (B02), 11.6.5.1 (B03), 11.6.3 (B04).

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model -p viewer-qt` exit 0.
`pdf-sandbox --bins` exit 0. nextest `--lib --bins --tests -p pdf-model -p viewer-qt` (`xvfb-run
-a`): 2988 run, 2983 passed, 5 failed, 24 skipped — exactly this audit's five; `audit-diff.py`
prints "failed 5, expected 5". rustdoc `-D warnings --document-private-items -p pdf-model` exit 0.
`cargo test -p conformance --no-fail-fast` exit 0 (exit 101 first: `round_numbers` named this round in the test file's module comment, rewritten). Fuzz not owed: no public type changed.

## Habit

A parameter dictionary is read entry by entry, and an entry's absence is a third answer beside
its two values: probe a `gs` that names nothing about the parameter after one that set it.
