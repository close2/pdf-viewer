# Round 2271 — the twenty-second specification audit: §12.10 and Annexes K, L, O and Q

An audit round in the `lane3` worktree, in the shape of review 2038 and audits 2128 to 2260: every
`shall` and `should` of §12.10 (geospatial features) and Annexes K (XFA, as ISO 32000-2 states it),
L (structure element nesting), O (fragment identifiers) and Q (the transparency method) against
`pdf-model`'s geospatial reading, `pdf-archive`'s survey, the writers of XFA forms and structure
trees, and `viewer-core`'s fragment handling. It fixes nothing.

## What it found

- `doc/reviews/2271-specification-audit-12-10-annexes-k-l-o-q.md` and its directory: 4 B, 0 T, 2 D.
- B04 a redaction removes a field's `/V` and carries the `/XFA` packet holding the same value
  (Annex K's first writer rule; §12.5.6.23). B02 a form a soft mask's group draws stays visited
  and is never opened when the page draws it (Q.3). B01 a Type 3 font a `gs` `/Font` selects never
  reaches Q.5. B03 a pattern cell is judged under the `scn`'s state, not §8.7.3.1's (Q.2).
  D01 the 12.10.3 row is `implemented` over an `/EPSG`-only refusal grounded on a corpus census;
  D02 the 12.10.2 row counts `/PDU`, which no display applies.
- Annex L in every tree this program writes, Annex O's eleven parameters and K.1's note hold.
- Four failing tests with controls: `pdf-archive/tests/annex_q_routes_spec_audit.rs`,
  `pdf-transform/tests/annex_k_xfa_spec_audit.rs`; listed in the audit's
  `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (12.10, 12.10.1, 12.10.3–12.10.5, K, K.1, K.2, L, O, O.1, O.2,
O.2.1, O.2.2, Q, Q.1–Q.5 `implemented`; 12.10.2 `departed`). The findings contradict Q, Q.2, Q.3,
Q.5, K, K.2, 12.10.2 and 12.10.3.

**Gates.** Over `-p pdf-model -p pdf-render -p render-cpu -p viewer-core -p viewer-qt -p
conformance -p pdf-archive -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 101 on
`doc_markdown` in the new test, fixed, then 0. `pdf-sandbox` 0. nextest `--lib --bins --tests`
(`xvfb-run -a`): 6706 run, 4 failed — exactly the manifest's (`audit-diff.py`: "failed 4,
expected 4"). Rustdoc of `pdf-archive` and `pdf-transform` 0. `cargo test -p conformance
--no-fail-fast` 0. Fuzz not owed (no public type changed).

## Habit

A walk's memory of what it has seen is a route too: the visited set that keeps a cycle finite
also decides which content a later route is allowed to see, and a walk that takes back what it
*found* must take back what it *visited* with it.
