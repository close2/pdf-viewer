# Round 2239 — the seventeenth specification audit: §12.1, §12.2 and §12.9 to §12.11

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2237: every
`shall` and `should` of §12.1, §12.2 (Tables 147 and 148), §12.9 (Tables 265 to 268, §12.9.2's
algorithm), §12.10 (Tables 269 to 272) and §12.11 (Tables 273 to 276) against `pdf-model`,
`viewer-core` and `viewer-host`, with §12.3.5's collection display read where 2038 had not, and
clause 12's errata read first. It fixes nothing.

## What it found

- `doc/reviews/2239-specification-audit-12-1-12-2-12-9-12-11.md` and its directory: 3 B, 0 T, 1 D.
- B01 a decimal that rounds up to a whole unit drops the carry: 2.999 with `/D 100` reads `2 u`
  (Table 268 `/F` `D`). B02 `/RD ()` is used as an empty decimal position, so 12.5 reads `125 u`;
  Table 268 makes an empty `/RD` the default period. B03 a slope is taken along the page's axes and
  not the measuring axes Table 265's `/BBox` corner order fixes.
- D01 three of `requirements::Kind::unmet`'s reasons (`EnableJavaScripts`, `Multimedia`,
  `Geospatial2D`) are claims about the tree that have decayed.
- Three failing tests with controls: `pdf-model/tests/measurement_12_9_spec_audit.rs`
  (`navigation_suite`); listed in the audit's `expected-failures.json` and appended to
  `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (`12.9.1`, `12.9.2`, `12.11.2` `implemented`). B findings
contradict 12.9.1 and 12.9.2; D01 12.11.2.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-host -p viewer-qt`: fmt 1 on one assertion's
line breaks and clippy `-D warnings --all-targets` 101 on an unticked `BBox` in a quoted doc line,
both in the new test, fixed, then 0. `pdf-sandbox` 0. nextest `--lib --bins --tests` (`xvfb-run
-a`): 4228 run, 6 failed — exactly the manifest's (`audit-diff.py`: "failed 6, expected 6"; 2237's
three and these three). rustdoc of `pdf-model` 0. `cargo test -p conformance` 0. Fuzz not owed.

## Habit

Entries that share a shape do not share a meaning: Table 268's four text entries are one type with
one default rule in the code, and the table gives `/RD`'s empty string the opposite meaning from the
other three. Read each entry's own sentence about the empty value before folding them into one
reader.
