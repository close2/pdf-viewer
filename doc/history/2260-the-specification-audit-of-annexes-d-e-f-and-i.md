# Round 2260 — the twenty-first specification audit: Annexes D, E, F and I

An audit round in the `lane3` worktree, in the shape of review 2038 and audits 2128 to 2253: every
`shall` and `should` of Annexes D (character sets and encodings), E (extending PDF), F (linearized
PDF) and I (versions and compatibility) against `pdf-font`, `pdf-syntax`, the transform verbs, the
linearizer and the archive converter. It fixes nothing.

## What it found

- `doc/reviews/2260-specification-audit-annexes-d-e-f-i.md` and its directory: 3 B, 0 T, 4 D.
- B01 `StandardEncoding` is a predefined encoding in `pdf-font`, which Table D.1 forbids; an
  embedded program's built-in encoding loses to Table D.2's STD column. B02 the converter's rebuild
  and `pdf-archive`'s page check take `/N` from a linearization dictionary whose `/L` no longer
  matches (Table F.1: ignore it). B03 `merge`'s page-label writer puts ASCII into an unprefixed
  string, where Table D.3 makes 0x1F a tilde. D01 to D04: the D.1 and E.2 rows, a doc comment on
  the encoding names, and an unrecorded 1.8-to-1.7 lowering in the converter.
- Annex D's six tables diffed against the code: no disagreement. Annex E: no extension written.
- Three failing tests with controls: `pdf-font/tests/annex_d_encodings_spec_audit.rs`,
  `pdf-transform/tests/annexes_d_f_i_spec_audit.rs`; listed in the audit's
  `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (D, D.1–D.6, E, E.1, F, F.1–F.4.7, I, I.1–I.3 `implemented`;
E.2 `writer-side`). The findings contradict D.1, D.3, F, F.3.3, 7.9.2.2, E.2 and I.2.

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-transform -p viewer-qt`: fmt 0; clippy
`-D warnings --all-targets` 101 twice on lints in the new tests, fixed, then 0. `pdf-sandbox` 0.
nextest `--lib --bins --tests` (`xvfb-run -a`): 2378 run, 3 failed — exactly the manifest's
(`audit-diff.py`: "failed 3, expected 3"). Rustdoc of `pdf-font` and `pdf-transform` 0. `cargo
test -p conformance --no-fail-fast` 101 only on this record's missing gates paragraph; `records` re-run, 0.
Fuzz not owed (no public type changed).

## Habit

The annexes are read where their tables are *used*, not where they are transcribed: every table
matched the code, and the defects were a name the code knows that the annex says it shall not, a
reader of Table F.1 that skipped the function holding Table F.1's rule, and a writer that encoded
a text string by hand beside the function that does it right.
