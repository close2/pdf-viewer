# Round 2237 — the sixteenth specification audit: §8.1 to §8.3, §8.8 and §10.1 to §10.4

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2234: every
`shall` and `should` of §8.1 to §8.3, §8.8 and §10.1 to §10.4 against `pdf-model`'s interpreter
(the CTM, default user space, forms, Type 3 glyphs, patterns, `Do`) and `pdf-colour`'s device
conversions, with clause 8's and clause 10's errata read first. It fixes one doc slip and nothing
else.

## What it found

- `doc/reviews/2237-specification-audit-8-1-8-3-8-8-10-1-10-4.md` and its directory: 3 B, 0 T, 2 D.
- B01 a pattern a Type 3 glyph paints with is placed against the page's (or enclosing form's)
  default space, not the glyph description's (§8.7.2's parent content stream): `Interpreter::base`
  is swapped at every other nested stream, and `pattern_initial` is reset for glyphs but `base` is
  not. B02 a page naming a spot colourant `/DeviceGray` has every `DeviceGray` colour painted on
  that plate on the simulated press under a CMYK intent, because the grey's construction is a
  `Separation` of that name (§10.2). B03 with no CMYK intent the simulated press is the assumed
  inks, yet `DeviceGray` is separated as a neutral rather than black `1 − gray` (§10.3.2, §10.4.2.3).
- D01 `device_ink_press` cited §10.4.2.2 for black alone; fixed to §10.4.2.3. D02 two comments say
  a pattern is placed against "the page's" space; left for B01's fix.
- Three failing tests with controls: `pdf-model/tests/coordinates_8_3_10_4_spec_audit.rs`
  (`content_suite`); listed in the audit's `expected-failures.json` and appended to
  `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (all `implemented`). B findings contradict 8.3.2.4, 8.7.2, 9.6.4,
10.2, 10.3.2, 10.4.2.3 and 10.8.3.

**Gates.** Over `-p pdf-model -p pdf-colour -p render-cpu -p viewer-qt`: fmt 1 on the new test's
line breaks, reformatted, then 0; clippy `-D warnings --all-targets` 101 on its pixel arithmetic and
quotations, fixed, then 0. `pdf-sandbox` 0. nextest `--lib --bins --tests` (`xvfb-run -a`): 3582
run, 6 failed — exactly the manifest's (`audit-diff.py`: "failed 6, expected 6"). rustdoc 0.
`cargo test -p conformance` 101 on this record's missing gates paragraph, then 0. Fuzz not owed.

## Habit

A construction that stands for a colour space by borrowing another family's shape (a `Separation`
named `DeviceGray`) is read by every rule keyed on that shape: list the readers of the borrowed
field (here, colourant names against spot planes) before trusting that "no name collides".
