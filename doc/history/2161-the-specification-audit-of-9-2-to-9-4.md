# Round 2161 — the seventh specification audit: §9.2 to §9.4, text state and text objects

An audit round in the `gwg` worktree, made in the shape of review 2038 and audits 2128 to 2155:
§9.2.1 to §9.4.4 (Tables 102 to 107) against `pdf-model`'s text state, text objects, the
displacement arithmetic of §9.4.4, the rendering modes and the readback that reads the same
positions. The backends draw a glyph as an ordinary fill or stroke, so the trace stops at the
display list. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2161-specification-audit-9-2-9-4.md` and its directory. Five findings: 5 B, 0 T, 0 D.
- B01 a second `BT` discards what the interrupted text object owes at `ET` — its clip and its
  §11.7.4.4 knockout groups — in silence; B02 `TJ` reads operands outside its array as elements
  (a leftover number moves the text); B03 a text state operand of the wrong type is dropped in
  silence (`/F1 (x) Tf` draws nothing and says nothing, `1.5 Tr` keeps the old mode); B04 a show
  after `ET` is placed by the matrix `ET` discarded, unreported; B05 a Type 3 glyph's own text
  object puts a word break into the page's readback.
- Five failing `*_spec_audit` tests, each with a control, in
  `crates/pdf-model/tests/text_objects_spec_audit.rs` (`text_suite`); listed in the audit's
  `expected-failures.json` and appended to `expected-failures-current.json`.
- About thirty further candidates were probed end to end and hold; the review lists them, with two
  readings checked and left (zero-sized text in a clipping mode; §9.4.3's "same results").

## Ledger

Nineteen §9.2 to §9.4 rows `implemented` before and after. B findings contradict 9.3.1, 9.3.6,
9.4.1, 9.4.3 and 11.7.4.4.

**Gates.** fmt exit 0 (exit 1 on the first run, one `assert_eq!` rewrapped). clippy `-D warnings
--all-targets -p pdf-model -p viewer-qt` exit 0. `pdf-sandbox --bins` exit 0. nextest `--lib
--bins --tests -p pdf-model -p viewer-qt` (`xvfb-run -a`): 2973 run, 2966 passed, 7 failed, 24
skipped — the manifest's six and `viewer-qt`'s `unsafe_position` (Q979). rustdoc `-D warnings
--document-private-items -p pdf-model` exit 0. `cargo test -p conformance --no-fail-fast` exit 0.
Fuzz not owed: no public type changed.

## Habit

A text object is bounded by two operators, and each obligation it carries — a clip, a group, a
discarded matrix — is owed at the bound, so probe every way a bound can be crossed: a second `BT`,
an `ET` with nothing open, a show past `ET`, a text object inside a glyph description.
