# Round 2148 — the fifth specification audit: §12.7, interactive forms

An audit round in the `audit` worktree, made in the shape of review 2038 and audits 2128, 2131,
2136 and 2142, turned on where the reader *writes*: §12.7.1 to §12.7.7 (Tables 224 to 243) against
`pdf-model`'s field tree, inheritance, button and choice values, `set_field`, the save and the
submission, and `viewer-host`'s click. It fixes nothing but one doc slip. No ledger row moves.

## What it found

- `doc/reviews/2148-specification-audit-12-7.md` and its directory. Nine findings: 8 B, 0 T, 1 D.
- B01 a submission carries a choice's label, not its `/Opt` export value; B02 a button with Table
  230's `/Opt` is submitted by its state name; B03 a saved radio set with `RadiosInUnison` clear
  states both same-named buttons on, though the view drew one; B04 the second of two same-named
  buttons cannot be selected, because an edit names no widget; B05 typed text becomes a list box's
  value; B06 a push-button takes and saves a `/V`; B07 Comb with FileSelect is a comb; B08 a root
  field named `()` merges its child `a` with a top-level `a`.
- D01 (fixed) `inheritance.rs` named Tables 229 and 231 for `/MaxLen`, `/TI` and `/I`.
- Eight failing `*_spec_audit` tests in `crates/pdf-model/tests/forms_values_spec_audit.rs`, a
  `forms_suite` module (ADR 3086), each with a control; listed in the audit's
  `expected-failures.json` and appended to `2038-specification-audit/expected-failures-current.json`.

## Ledger

Twenty-five §12.7 rows (§12.7.8 aside) `implemented` before and after. B findings contradict
12.7.4.2, 12.7.5.2.2, 12.7.5.2.3, 12.7.5.2.4, 12.7.5.3, 12.7.5.4 and 12.7.6.2.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model` exit 0 on the second run;
the first found four unbackticked flag names and a `map().unwrap_or()` in the test. `pdf-sandbox
--bins` exit 0. nextest `--lib --bins --tests -p pdf-model` (`xvfb-run -a`): 2900 run, 2892
passed, 8 failed, 23 skipped, the manifest's 8 ("failed 8, expected 8"). rustdoc `-D warnings
--document-private-items` exit 0. `cargo test -p conformance --no-fail-fast`
exit 0 on the third run; the second wanted this paragraph to state that result, not point elsewhere.
Fuzz not owed: no public type changed.

## Habit

A value has a reader on the way out as on the way in: for each thing the reader writes — a body,
an `/AS`, a name — ask what the next reader of it reads, and whether it is what this one showed.
