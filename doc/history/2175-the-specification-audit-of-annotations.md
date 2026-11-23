# Round 2175 — the ninth specification audit: §12.5, annotations

An audit round in the `gwg` worktree, made in the shape of review 2038 and audits 2128 to 2171:
§12.5.1 to §12.5.6.25 (Tables 166 to 196) against `pdf-model`'s annotation reading, drawing and
construction, and what a pointer, a click, a tab and an edit reach in `pdf-model`'s view and
`viewer-core`. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2175-specification-audit-12-5.md` and its directory. Six findings: 6 B, 0 T, 0 D.
- B01 a NoZoom or NoRotate annotation (every Text note) is hit-tested at its unaltered `/Rect`;
  B02 Table 167's Hidden, NoView and ReadOnly are read by `annotation_at` alone — a hidden link
  turns the page on a click, `field_at` answers a hidden widget, Tab focuses hidden annotations;
  B03 a widget of a ReadOnly field interacts; B04 `LockedContents` on a widget does not restrict
  filling its field; B05 an `/AP` subdictionary with no `/AS` draws nothing unreported; B06
  `/Rect` and `/Matrix` of the wrong type or length are read by shifting numbers, in silence.
- Nine failing tests with controls: eight in `crates/pdf-model/tests/annotation_clause12_5_spec_audit.rs`
  (`annotations_suite`), one in `crates/viewer-core/tests/annotation_interaction_flags_spec_audit.rs`;
  listed in the audit's `expected-failures.json` and appended to `expected-failures-current.json`.
- Readings left: a stored appearance ignoring `/ca` and `/CA` (the item 2171 left; the standard
  contradicts itself and rows 12.5.2 and 12.5.5 are `departed` for it, ADRs 0030 and 0579); an
  annotation hidden by `/OC` still interacting (no sentence forbids it); `ToggleNoView`'s
  "selected"; a `/D` subdictionary lacking the state.

## Ledger

§12.5 rows before and after: 29 `implemented`, 3 `departed`. B findings contradict 12.5.3, 12.5.1,
12.5.5, 12.5.2, 12.5.6.4, 12.5.6.5, 12.7.4.1 and 7.9.5.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p viewer-qt`
exit 0 (101 first: `doc_markdown` on verbatim quotes; the file expects it). `pdf-sandbox` 0. nextest
`--lib --bins --tests` of the three (`xvfb-run -a`): 3662 run, 3653 passed, 9 failed, 26 skipped —
exactly this audit's nine; `audit-diff.py` prints "failed 9, expected 9". rustdoc `-D warnings
--document-private-items -p pdf-model -p viewer-core` exit 0. `cargo test -p conformance
--no-fail-fast` exit 0. Fuzz not owed: no public type changed.

## Habit

A flag that says "interact" is read on every route a person reaches an annotation by — hover,
click, field hit test, Tab, edit. List the routes first, then probe each with the flag set.
