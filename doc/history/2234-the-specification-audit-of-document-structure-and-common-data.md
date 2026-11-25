# Round 2234 — the fifteenth specification audit: §7.7 to §7.12, document structure and common data

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2229: every
`shall` and `should` of §7.7 to §7.12 (Tables 29 to 49) against `pdf-model`'s page tree, resource,
Type 3, file-specification and extension readers and `pdf-syntax`'s trees, text strings, dates and
version, with clause 7's errata (H7.7 to H7.12) read first. It fixes one doc slip and nothing else.

## What it found

- `doc/reviews/2234-specification-audit-7-7-7-12.md` and its directory: 3 B, 0 T, 1 D.
- B02 `groups_referenced_by` (Table 99's `VisiblePages`) follows resources into forms only, so a
  group marked in a tiling pattern's cell or an annotation's appearance is not offered; B01 a Type 3
  glyph stream's own `/Resources` (Issue #128's first step) is walked by the interpreter but by
  neither `spot_colourants` nor `resources_blend`; B03 `encode_text_string` writes a text beginning
  `þÿ` or `ï»¿` in PDFDocEncoding, which reads back as UTF-16BE or UTF-8.
- D01 `text_string.rs`'s module doc spelled the UTF-8 prefix `¨»¿` (Issue #161: idieresis); fixed.
- Four failing tests with controls: `pdf-model/tests/resources_7_8_3_spec_audit.rs`
  (`content_suite`) and `pdf-syntax/tests/text_string_prefix_spec_audit.rs`; listed in the audit's
  `expected-failures.json` and appended to `expected-failures-current.json`.
- Readings left: `FileSpec::components` splits `/F` although `/UF` is present (pinned by its own
  test; no production caller); a zero-area crop box read as absent, against §7.9.5's NOTE, which
  is the silent-repair campaign's question.

## Ledger

§7.7 to §7.12 rows before and after: unchanged (54 `implemented`). B findings contradict 7.8.3,
7.9.2.2.1, 8.11.4.3, 10.8.3 and 11.6.3, all `implemented`.

**Gates.** fmt 0 after reformatting the new files; clippy `-D warnings --all-targets` of `-p
pdf-syntax -p pdf-model -p pdf-colour -p viewer-qt` 101 on the new test's doc quotation, fixed,
then 0. rustdoc 0. `cargo test -p conformance` 0. `pdf-sandbox` 0. nextest `--lib --bins --tests` of the four (`xvfb-run -a`): 3725 run,
4 failed — exactly this audit's four (`audit-diff.py`: "failed 4, expected 4"). Fuzz not owed: no
public type changed.

## Habit

A walk ahead of the interpreter is a second reader of every dictionary the interpreter reads: list
§7.8.2's content streams (forms, patterns, Type 3 glyphs, annotation appearances) against each
walk, not only against the interpreter.
