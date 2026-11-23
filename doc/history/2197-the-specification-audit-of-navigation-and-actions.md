# Round 2197 — the thirteenth specification audit: §12.3, §12.4 and §12.6, navigation and actions

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2195:
every `shall` and `should` of §12.3, §12.4 and §12.6 (Tables 149 to 223) against `pdf-model`'s
destinations, outline, page labels, articles, transitions, navigation nodes and actions, and
`viewer-core`'s application of them, with the outline panel's rows in `viewer-host`. It fixes
nothing. No ledger row moves.

## What it found

- `doc/reviews/2197-specification-audit-12-3-12-4-12-6.md` and its directory: 4 B, 0 T, 2 D.
- B03 a `/Next` chain moves the view once (the first page any request names is kept, and
  `NextPage` counts from the chain's starting page); B04 no host draws an outline item's `/C`
  or italic and bold; B02 a remote go-to's `/SD` decides the page but drops its own view, and an
  element without content hands the page back to `/D`; B01 an outline item's go-to ignores `/SD`.
- D01 `outline::destination` says no action but a go-to could be followed; D02 `Jump`'s doc says
  a chain that jumps twice shows the second page and §12.6.2 states no rule.
- Five failing tests with controls: `pdf-model/tests/navigation_12_3_12_4_12_6_spec_audit.rs`
  (`navigation_suite`), `viewer-core/tests/navigation_12_3_12_4_12_6_spec_audit.rs`,
  `viewer-host/tests/outline_item_style_spec_audit.rs`; listed in the audit's
  `expected-failures.json` and appended to `expected-failures-current.json`.
- Readings left: a `/Threads` entry that is not a thread shifts Table 209's index; `/St` has no
  bound; when a transition action plays inside a chain was not traced.

## Ledger

§12.3, §12.4 and §12.6 rows before and after: unchanged. B findings contradict 12.3.3, 12.6.4.2,
12.3.2.3, 12.6.4.3, 12.6.2 and 12.6.4.12, all `implemented`.

**Gates.** clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p viewer-host -p
viewer-qt` exit 0; fmt exit 1 on two new files, reformatted, then 0. `pdf-sandbox` 0. nextest
`--lib --bins --tests` of the four (`xvfb-run -a`): 4112 run, 4107 passed, 5 failed, 28 skipped —
exactly this audit's five; `audit-diff.py` prints "failed 5, expected 5". rustdoc `-D warnings
--document-private-items` of the four, private target, exit 0. `cargo test -p conformance
--no-fail-fast` exit 0. Fuzz not owed: no public type changed.

## Habit

Every reader of one dictionary is a route to it: four read the same go-to, and the one that
never performs it — the outline item's page for `section_at` — read it differently.
