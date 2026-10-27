# 1803 — The page boundaries are shown as guidelines on request, and review 1768's list is down to the owner's

Session 1803, a clause round in batch 120; 2026-10-27. ADR 2440; ADR 2441 unused; no Q (Q669 unused).

**Contract.** Slot 6: review 1768's accepted rows still `out-of-scope` or `inapplicable`, listed, and
the next two built. **The list on HEAD**: candidates 2–7 are built or written; what remains is
**§14.11.2.2** (1, no blocker — built here); §13.2.6.2, §13.2.6.3, §13.2.7.5 and §13.7's eighteen rows
(8, Tier C, closed until the owner builds the player, ADR 2428); §13.6's rows and §12.6.4.16 (9, Tier
D, A632); K, K.1, K.2 (10, the XFA slots').

**Hypothesis.** *Four to six remain; two move.* **Did not hold**: one unblocked row remained and it
moved. In place of a second, finding 10's note: §13.5 kept on the clause's own deprecation and "not
required to support", no status moved.

**Premises.** *§14.11.2.2 waits for a print panel* (RFC 0009 section 2.10): the clause asks for a
display "on the screen", so the window is the view and no panel was needed.

**Done.** `pdf_model::guidelines` reads Tables 396 and 397 for each boundary the page object defines
(an inherited crop box counts), drawn as strokes after the annotations under `Purpose::View` only,
`/D` through §8.4.3.6's dash function at phase 0, a value the table does not admit said. `b` in every
window, `Command::Guidelines` on the confined wire, `QUORRA_KEY_B` 33 and `QUORRA_ACT_GUIDELINES` 22
appended to the C ABI and the Windows constants. Default colours are a choice (ADR 2440); a page in
its own blending space gets none and a sentence. The rendered fixture was looked at (trap 1).

**Unfinished.** No window driven under Xvfb, no arm run (the default path returns at once).

**Files.** New: `pdf-model`'s `src/guidelines.rs`, `src/content/guidelines.rs`, `tests/guidelines.rs`;
ADR 2440; this record. Edited, `-U0` on my hunks: pdf-model (`lib`, `page`, `view`, `view/settings`,
`content`, `content/report`, `tests/corpus`), viewer-core (`command`, `viewer`, `viewer/opening`,
`report`), viewer-confined (`protocol/command`, its test), viewer-host (`keys`, `policy`, `lib`,
`native_window`), viewer-ffi (`keyboard`, `window`, `quorra.h`, two tests), viewer-gtk and viewer-qt
(input, keys, a test each), viewer-ui (four files), `hosts/windows/` (two), the ledger (three rows),
`doc/state-of-play.md`. I worked in `scratchpad/r1803/`, gates in its export.

**Gates.** In the export: `cargo fmt --check` over the eight crates exit 0; clippy `-D warnings`
`--no-deps --all-targets` exit 0; `cargo nextest run` over seven crates 1123 passed and 5 failed, the
five mine (counts and an unbound `b`), fixed and rerun 5 passed; pdf-model lib plus three tests 732
passed. In the worktree: `--bin quotations`, `--bin cited`, `--bin inapplicable` exit 0;
`tools/state.sh --round 1803 ledger` exit 0; `cargo test -p conformance --no-fail-fast` 467 passed, 2
failed, this record's before its gates were written; `--test records` rerun, 1805's red alone.
