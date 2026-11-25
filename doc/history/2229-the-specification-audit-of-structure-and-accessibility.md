# Round 2229 — the fourteenth specification audit: §14.6 to §14.9, marked content, structure and accessibility

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2197:
every `shall` and `should` of §14.6 to §14.9 (Tables 352 to 385) against `pdf-model`'s structure
reading, marked-content reading and speech, `viewer-core`'s accessibility tree and `viewer-host`'s
panels. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2229-specification-audit-14-6-14-9.md` and its directory: 5 B, 0 T, 1 D.
- B01 a form `XObject` named by an `/OBJR` is called an artifact by absence, is missing from the
  logical text and leaves its element's accessible name empty; B03 reports and questions name a
  field by its qualified name although it states `/TU`; B02 a `Span`'s `/Lang` reaches structured
  content inside it when no element states one; B04 outline and layer rows carry no language; B05
  `Interpretation::speech` speaks an element's `/Alt` per sequence and not over child elements.
- D01 `by_absence`'s docs list three routes into the structure tree and omit Table 359's `XObject`.
- Six failing tests with controls: `pdf-model/tests/structure_14_6_14_9_spec_audit.rs`
  (`navigation_suite`), `viewer-core/tests/structure_objr_xobject_spec_audit.rs`,
  `viewer-host/tests/outline_language_spec_audit.rs`; listed in the audit's
  `expected-failures.json` and appended to `expected-failures-current.json`.
- Readings left: a stated SPACE plus a `TJ` gap reads back as two spaces; a role-map chain followed
  past its first standard type; user properties shown by no panel.

## Ledger

§14.6 to §14.9 rows before and after: unchanged (90 `implemented`, 3 `inapplicable`). B findings
contradict 14.7.5.3, 14.7.5.4, 14.8.2.2.2, 14.8.2.5.1, 14.9.2.1, 14.9.2.3 and 14.9.3, all
`implemented`.

**Gates.** fmt 0 after reformatting the new files; clippy `-D warnings --all-targets` of `-p
pdf-model -p viewer-core -p viewer-host -p viewer-qt` 101 on the new viewer-core test, fixed, then 0.
`pdf-sandbox` 0. nextest `--lib --bins --tests` of the four (`xvfb-run -a`): 4186 run, 6 failed —
exactly this audit's six (`audit-diff.py`: "failed 6, expected 6"). rustdoc 0. `cargo test -p
conformance` 0 once this record was cut to 40 lines. Fuzz not owed: no public type changed.

## Habit

An exception in a clause is a route: §14.9.2.3's "except for nested marked-content that is contained
in the structure hierarchy" held where an element stated the language and failed where the
structure inherited it from the catalog.
