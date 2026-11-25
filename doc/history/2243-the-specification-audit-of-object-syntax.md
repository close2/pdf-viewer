# Round 2243 — the eighteenth specification audit: §7.1 to §7.3

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2239: every
`shall` and `should` of §7.1 to §7.3 (lexical conventions, the nine object types, streams and
indirect objects) and Annex C's Table C.1 against `pdf-syntax`'s lexer, parser and resolution, and
against every writer of the syntax — `write.rs`, `serialize.rs`, `linearize.rs` and the content and
`/DA` writers of `pdf-model` and `pdf-transform`, the last by a Sonnet census. It fixes nothing.

## What it found

- `doc/reviews/2243-specification-audit-7-1-7-3.md` and its directory: 6 B, 0 T, 0 D.
- B01 the incremental update writes a stream's `/Length` as handed, not as written. B02 a stream
  written as a direct value is read in silence and written back nested. B03 a `#00` escape puts a
  null in a name in silence. B04 object identifiers outside issue #379's grammar are read in
  silence. B05 a script's `textSize` re-states the `/DA` font name unescaped. B06 the `/DA` replay
  writes a real past `f32`'s range as `inf`.
- Six failing tests with controls: `pdf-syntax/tests/objects_7_3_spec_audit.rs` and
  `pdf-model/tests/default_appearance_syntax_7_3_spec_audit.rs` (`forms_suite`); listed in the
  audit's `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (all §7.1 to §7.3 rows `implemented`). The findings contradict
7.3.3, 7.3.5, 7.3.6, 7.3.7, 7.3.8.1, 7.3.8.2, 7.3.10 and 12.7.4.3.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform -p viewer-qt`: fmt 0 after rustfmt of
the two new files; clippy `-D warnings --all-targets` 101 twice in the new tests (an unchecked
`+`, an unfulfilled `expect`, an unused import), fixed, then 0. `pdf-sandbox` 0. nextest `--lib
--bins --tests` (`xvfb-run -a`): 4903 run, 6 failed — exactly the manifest's (`audit-diff.py`:
"failed 6, expected 6"), re-run on the two binaries after the lint fixes. rustdoc of `pdf-syntax`
and `pdf-model` 0. `cargo test -p conformance` 101 on this record's missing gates paragraph, then
101 on `bounded`'s self-test alone (the load flake the habits name), which passes re-run alone. Fuzz not owed (no public type changed).

## Habit

A rule about how a token is written binds every writer that formats one itself. `write::object`
and `Name::escaped` were right; the two `/DA` writers that format a name and a number by hand
beside them were not. Census the hand-formatted sites, not the shared formatter.
