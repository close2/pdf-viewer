# Round 2282 — the twenty-fourth specification audit: ISO 19005-2 and ISO 19005-4 clause 6

An audit round in the `lane3` worktree, in the shape of review 2038 and audits 2128 to 2278, and
the first of ISO 19005: clause 6 of both parts against `pdf_archive`'s 268 rows and its coverage
table, and the converter's claim through its census and one output put through veraPDF. Made by
about eighty two-file probes held to 2b, 2a and 4 with `examples/one.rs`. It fixes nothing.

## What it found

- `doc/reviews/2282-specification-audit-19005.md` and its directory: 5 B, 0 T, 1 D.
- B02 a file specification written in place (an annotation's `/FS`, an action's `/F`) is never
  read, so a non-PDF/A attachment passes and the converter calls its output conforming (veraPDF
  fails it on 6.8). B03 colour ignored after `d1` and in an uncoloured tiling cell, and such a
  cell's initial space, are read as device colour. B04 a page's metadata stream without `/Type`
  is read by nothing. B05 `Order` is held to `/OCGs` rather than to the groups the file uses. B01
  twelve forbidden-key rules fail an entry naming no object, which §7.3.10 makes the null object.
  D01 a report names an object in Rust's `Debug` form.
- File syntax, actions and their `/Next` chains, the implementation limits, page boundaries,
  annotation flags and appearances, transparency-group spaces, language identifiers and font
  embedding routes hold.
- Five failing tests with controls in `pdf-archive/tests/pdfa_clause_6_spec_audit.rs`, listed in
  the audit's `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged. No ISO 32000-2 row is contradicted; the contradicted claims are
`pdf_archive::coverage`'s `Carried::By` for part 2 6.8, 6.2.4.3, 6.6.2.1, 6.9 (part 4 6.10) and the
forbidden-key sentences.

**Gates.** Over `-p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`: fmt 0;
clippy 101 on the new test (a field named after its struct, one long table test), fixed, then 0.
`pdf-sandbox` 0. nextest `--lib --bins --tests` (`xvfb-run -a`): 5478 run, 10 failed — the
manifest's eight in these crates and two citation gates (`ISO 32000-1 §` in the new test's
comments), fixed; both then pass alone, and `cargo test -p conformance --no-fail-fast` is 0.
Rustdoc of `pdf-archive` 0. Fuzz not owed (no public type changed).

## Habit

A population built from the cross-reference table misses every dictionary written in place, and
ISO 32000 lets most of them be either: before trusting a walk, probe with the dictionary direct.
