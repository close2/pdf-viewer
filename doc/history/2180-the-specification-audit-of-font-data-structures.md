# Round 2180 — the tenth specification audit: §9.5 to §9.10, font data structures

An audit round in the `gwg` worktree, in the shape of review 2038 and audits 2128 to 2175: every
`shall` and `should` of §9.5 to §9.10 (Tables 108 to 127) against `pdf-font`'s encodings, widths,
`CMap`s, descriptor and `ToUnicode` reading and `pdf-model`'s Type 3 fonts. It fixes one doc slip
and nothing else. No ledger row moves.

## What it found

- `doc/reviews/2180-specification-audit-9-5-9-10.md` and its directory: 3 B, 0 T, 4 D.
- B01 a referencing `CMap`'s `cidrange` loses to the referenced `CMap`'s `cidchar` for the same
  code (`Mapping::get` asks singles before ranges across the `usecmap` boundary); B02 a symbolic
  `TrueType` font's ignored `/Encoding` still selects glyphs, above the reader-chosen tiers and
  unrecorded among them; B03 a Type 3 font's `/LastChar` is never read, so codes past it take
  `/Widths` elements Table 110 makes 0.
- D01 (fixed) `Type3Font::text`'s doc contradicted its body; D02 three history comments in the
  Type 3 path; D03 §9.10.2 step b)'s "the font's CMap" read as the descendant's, unwritten; D04 a
  dead `UnsupportedEncoding` arm in `truetype_code_table`.
- Three failing tests with controls: two in `crates/pdf-font/tests/fonts_9_5_9_10_spec_audit.rs`,
  one in `crates/pdf-model/tests/type3_widths_range_spec_audit.rs` (`text_suite`); listed in the
  audit's `expected-failures.json` and appended to `expected-failures-current.json`.
- Readings left: `usecmap` inheriting `/WMode` (Technical Note #5014); a (3, 0) table in two ranges.

## Ledger

§9.5 to §9.10 rows before and after: 45 `implemented`. B findings contradict 9.7.5.3, 9.7.6.2,
9.6.5.4, 9.10.2 and 9.6.4.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-font -p pdf-model -p viewer-qt`
exit 0 (101 twice first: `arithmetic_side_effects` and `doc_markdown` in the new test files, now
expected there). `pdf-sandbox` 0. nextest `--lib --bins --tests` of the three (`xvfb-run -a`):
3450 run, 3447 passed, 3 failed, 24 skipped — exactly this audit's three; `audit-diff.py` prints
"failed 3, expected 3". rustdoc `-D warnings --document-private-items -p pdf-font -p pdf-model`
exit 0. `cargo test -p conformance --no-fail-fast` exit 0 (101 first: a blockquote attributed to
§9.10 by the module's first line, now cited to §9.6.4). Fuzz not owed: no public type changed.

## Habit

An inherited table is a third answer beside "stated" and "absent": probe an override across the
inheritance with every operator pair (single over range, range over single), not one.
