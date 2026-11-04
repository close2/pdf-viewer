# 1924 — The checker reads the parent tree back from every content item

Session 1924, a single archive round on the server VM. ADR 2673; trap 319; no question or habit.
Worked in the rounds worktree and its `scratchpad/r1924/`.

**Why.** Round 1923 withheld the parent tree's new leaf and `pdf_archive::check` still answered
`Conforms` at 2a and part 4: no row read `/StructParents`.

**The premise, measured.** It held: no row read it. The clause did not hold where the brief put it:
neither part of ISO 19005 states a parent tree rule (19005-2's 6.7.2.1 and 6.7.3.3 bind Level A to
§14.7 and §14.8; 19005-4's 6.8 is not normative), so the row is the base standard's, at section 5.1
of both parts, in the `file_structure` tranche beside the page-tree rows. Read from the elements
down, not from content-stream `/MCID`s: an identifier no element claims is no content item.

**Built (ADR 2673).** `table/file_structure/parent_tree.rs`: each holder of a claimed sequence
states `/StructParents`, the tree holds the key, the value is an array, the entry at each `/MCID`
names a claimant; each claimed object states `/StructParent` and its entry names a claimant. Refused
by name: a parent tree that is not a number tree, a structure tree past 64 levels or 2^20 visits.
The decision table answers it `NotBuiltYet`; `coverage` carries it under both 5.1 readings;
`editions` gains three shifts for the clauses the module cites.

**Tests.** Six unit tests (both forms found, the withheld key, each broken route, five unreadable
trees, the depth bound both sides, an unclaimed `/MCID`). In `tests/archive/statement.rs`, the
one-leaf output at 2a and part 4 conforms, and an incremental update withholding its new leaf fails
the row once per appended page.

**Census.** ADR 2669's command against round 1923's after-run: the row refuses `issue15340`,
`issue20516`, `pdfjs_wikipedia` and `prefilled_f1040` (pdf.js) under both stating profiles at 2b and
4, so 3 fewer converted in each of the four (the fourth was refused already); no corpora source
fails it. All 8636 shared outputs are byte-identical once dates, `/ID` and UUIDs are masked, so no
output changed and veraPDF is owed for none. Few, so decided: rebuild the entries, a later round.

**Ledger.** 14.7.5.4 stays `implemented`; its code, tests and note name ADR 2673.

**Gates.** `cargo fmt --check` of `pdf-archive` and `pdf-transform`, `clippy -D warnings
--all-targets` of both (`CARGO_INCREMENTAL=0`, after an incremental-cache ICE): exit 0. `nextest`
of both: 1338 passed, exit 0. Archive tier behind the walk lock: exit 0; the veraPDF-corpus
comparison, 0 `over` in every profile. `cargo test -p conformance` after `git add -N`: exit 0.
