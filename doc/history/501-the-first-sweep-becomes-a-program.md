# 501 — The first sweep becomes a program, and learns a third noise shape

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The expired-blocker sweep — the
oldest of the fifteen, a description since session 118 — is `conformance --bin blockers` now,
and it carries the judgement each run used to redo by hand: a blocker sentence naming a clause
is checked against that clause's own ledger row, expired-first. Its first run found 0 defects
and taught the instrument a third noise shape: §12.10.2's "needs §12.10.3's external
references" printed as expired because §12.10.3's row is settled, and the wait was never on the
clause — it is on the EPSG registry and ISO 19162 grammar the clause's entries *point at*, so
the row names the registry now. The retired-claim sweep over the six-round wave's nouns paid
twice, both one file from a round's own correction: `free_text_census.rs` still called `/CL`
"the callout line `doc/todo/33` holds open" one round after 494 closed it, and
`doc/todo/README.md`'s item-31 line still listed the empty accessibility answer 490 had closed —
an index row decays at its item's pace, not its own. The capability sweep found §14.7.2 saying
"nothing in this program yet hands a structure tree to anybody" four sentences above its own
appended correction (shape 6, rewritten). And the blame list's next band — thirteen rows,
commits 165 to 185, minus §12.8.\*'s four, which a parallel round owns — held one more:
§12.3.5.2 said "`partial` for the panel" while the files tab has drawn its folder tree since
session 352 (ADR 0202); what is owed is named now (a folder's own `/Thumb`, `/CreationDate`,
`/ModDate`, with `/Free` named rather than owed).

**Date.** 2026-08-14.
**ADR.** [0336](../adr/0336-the-first-sweep-becomes-a-program-and-learns-a-third-noise-shape.md).

**Sweep results, verbatim from the runs.**

- `blockers` (new): ledger 20 sentences — 6 printed expired, 9 holding, 5 naming no clause;
  source 26 — 9, 10, 7. 0 defects; the printed-expired hits are corrections quoting retired
  wording and contrastive "while §X says", plus the §12.10.2 hit that became the third noise
  shape.
- `unread`: 61 rows claim, 167 keys; 52 confirmed, 115 quoted over 49 rows, 50 by the row's own
  code. 0 defects.
- `entries`: 222 rows in the population, 42 stating an entry their own `code` does not name,
  106 entries — 29 named nowhere, 77 only elsewhere, 36 not named by the row's own note. The
  known populations; nothing worked.
- `quotations`: 2951 quotations in 446 documents, 1398 verbatim, 22 diverging, 0 defects.
- Capability reasons (3): 43 ledger / 148 source; one defect (§14.7.2). Retired claim (4): two
  defects (`free_text_census.rs`'s `/CL`, `doc/todo/README.md`'s item 31). Caller sweep (5):
  284 distinct `pub fn` names in `pdf-model`, 92 unnamed by hosts, 76 by nothing — the known
  three populations. Arithmetic (6): §7.9.2 and §O, clean. Inapplicable (7): 27 of 80 name
  source vocabulary, none wrong. Citations (8): 4 hits, all the known
  correction-quoting-its-pointer shape. Table numbers (9): 1159 citations checked, 90 suspects,
  0 defects — the first fully clean run over ledger and source together. Parent counts (10): 6
  hits, 0 defects. Ledger quotation marks (11): 1224 spans, 848 verbatim, 40 diverging, 0
  defects. Sweep 14: 5 hits, 0 defects. Errata (12): "151 struck passage(s) of 4 words or more
  that doc/md/ still carries as current text" over all fourteen PDFs, unchanged — over one PDF
  alone it prints 150, so the invocation's document list is part of the count.

**Rows corrected in this commit.** §12.3.5.2 (the panel arrived in 352; the owed entries named),
§12.10.2 (the wait named against the registry, not the clause), §14.7.2 (shape 6 rewritten in
place). Kept with evidence recorded: §14.8.4.2 (the two-crate split behind `standard_role`),
§14.8.5.3, §12.10, §12.10.1, §12.9, §12.9.1, §12.3.6 (the fifth sweep's shape, stated),
§9.8.3.3, §12.7.7, §12.7.8.3.2, §12.7.8.3.3.

**Source corrected.** `crates/pdf-model/examples/free_text_census.rs` (the `/CL` sentence),
`doc/todo/README.md` (item 31's index line).

**Code.** `tools/conformance/src/blockers.rs` and `bin/blockers.rs` (new, with seven unit
tests); `unread::sentences` shared with it.

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's
record; four commands now), `doc/todo/02-every-round.md` §4 (the new command's line),
`doc/ledger-and-claims.md` (three programs → four), `doc/adr/0336-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent;
`cargo nextest run --workspace` all green; `cargo test --workspace --doc` green;
`cargo test -p conformance -- --nocapture` green. No change reaches a raster — the one source
edit outside `tools/conformance` is an example's doc comment — so the corpus and oracle gates
were not owed by this round's edits.
