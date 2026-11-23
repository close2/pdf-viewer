# Round 2184 — the eleventh specification audit: §8.6 and §8.7, colour spaces and patterns

An audit round in the `gwg` worktree, in the shape of review 2038 and audits 2128 to 2180: every
`shall` and `should` of §8.6 and §8.7 (Tables 61 to 84) against `pdf-colour`'s space, shading and
mesh reading and `pdf-model`'s colour operators, patterns, `sh` and §8.6.8's restriction. It fixes
nothing. No ledger row moves.

## What it found

- `doc/reviews/2184-specification-audit-8-6-8-7.md` and its directory: 4 B, 0 T, 2 D.
- B01 a `d1` glyph's `gs` stating `/BG2` and `/UCR2` converts the text's colour into a
  `DeviceCMYK` group, where §8.6.8 has them ignored; B02 an unreadable `ICCBased` profile with no
  `/Alternate` becomes the bare `/N` device space, starting at `DeviceCMYK`'s black and skipping
  `/Range`; B03 nonzero overprint mode skips an `Indexed` colour over `DeviceCMYK` (§11.7.4.3
  NOTE 2); B04 a `Pattern` space where none carries a colour — a shading's `/ColorSpace`, an
  `Indexed` base, a number given to `scn` — paints black and the page calls itself complete.
- D01 a comment says the black-generation entries are "never evaluated"; D02 a zero `/XStep` is
  tiled at the `/BBox`'s extent on the reason "what other readers assume".
- Seven failing tests with controls, all in `crates/pdf-model/tests/colour_8_6_8_7_spec_audit.rs`
  (`colour_suite`); listed in the audit's `expected-failures.json` and appended to
  `expected-failures-current.json`.
- Readings left: Table 65's "no tint transformation" and a `Separation` alternate; `All` in a group.

## Ledger

§8.6 and §8.7 rows before and after: 49 `implemented`. B findings contradict 8.7.4.3, 8.6.6.3,
8.6.8, 8.6.5.5 and 8.6.7.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model -p viewer-qt` exit 0
(101 first: `doc_markdown` on the verbatim quotes in the new test file, now expected there).
`pdf-sandbox` 0. nextest `--lib --bins --tests` of the two (`xvfb-run -a`): 3009 run, 3002 passed,
7 failed, 24 skipped — exactly this audit's seven; `audit-diff.py` prints "failed 7, expected 7".
rustdoc `-D warnings --document-private-items -p pdf-model -p viewer-qt` exit 0.
`cargo test -p conformance --no-fail-fast` exit 0 (101 first: a round number in the
test module's first line, and this record's gates paragraph unwritten). Fuzz not owed: no public type changed.

## Habit

A NOTE that defines a term (§11.7.4.3 NOTE 2's "current colour space") is a list of routes: a
route the code follows for one family it names, check for every other family it names.
