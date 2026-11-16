# 2033 — The trusted lists are fetched, and kept only once authenticated

Round 2033, a single ledger round, the signatures lane (A800 item 3, `doc/todo/77` items 4 and 8;
§12.8.3.4.4, §12.8.3.4.5). A host fetches the European Commission's list of trusted lists, its
pivots and every Member State's list it points at into the `--trusted-lists` directory, at A98's
`Submissions` level, and keeps only what ETSI TS 119 615 clauses 4.1 and 4.2 authenticate;
PRO-4.1.4-16 and -17's updates of the configuration are persisted (ADR 2882). Annex A's
`id-etsi-dst-euqtst` is read and recorded as a silence of TS 119 172-4 (ADR 2883).

- The premise held: nothing fetched a list (`TRUSTED_LISTS`' doc said "never fetched"), and ADR
  2842 left -16 and -17 to a host. Nothing owner-blocked, so no Q860; no A-file after A800 cites it.
- pdf-signature: `trusted_list::refresh` (a host's transport asked for PRO-4.1.4-01, -04, -05's
  files, then PRO-4.2.4-03's lists; clause 4.1 over them in front of the host's files; a Member
  State's list kept only past clause 4.2; stale stored files removed; -16 and -17 as files, b) 1)
  only, neither half without it). `authentication::stored_name`: the location percent-encoded
  (RFC 3986 sections 2.1, 2.3), looked for before the last path segment, in `Lists::read`,
  clause 4.1's lookups and a policy's list references — three Member States publish a `tsl.xml`.
- viewer-host: `trusted_list_fetch` (the level, refused at every level without a directory; the
  question; `submit::fetch`'s GETs, a non-2xx said; each file written and renamed into place).
  `restriction::SUBMITTING` names the lists.
- Evidence (the lists of 2026-11-12, untracked): the LOTL through five pivots, Austria's and
  Germany's lists kept, `OJEU-Loc` moved to the 2026 publication with pivot 378's six certificates;
  the second run passes with no pivot and removes the five.
- Tests: 10 in `trusted_list/refresh/tests.rs` (fixture transport, lists signed with the tree's
  test keys), 5 in `viewer-host/tests/trusted_list_fetch.rs` (loopback listener). Plants: a list
  clause 4.2 refused kept, and the last segment read before the stored name; each failed one test,
  reversed, pass.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-host`): 0. nextest
of the two: 920 of 920, 6 skipped. Fuzz `cargo check --bins`: 0. Rustdoc `-D warnings
--document-private-items` of the two: 0. Conformance after `git add -N`: 474 passed. Tier 2, on
3cd7426d: `corpus` 101, 974 documents, 61 incomplete (issue15053, mixedfonts joined), HEAD's;
`raster_golden` 101, held 951, moved 23, HEAD's; `selection_census` 0 (968 compared);
`accessibility_census` 0 (1020); `awkward_classes` 0 (152 swept of 2677, none killed).
`state.sh --round 2033 docs` 0. `launch_path` not run: no launch-path code is reached, load
12/26/37. Fast-forwarded onto f7e5dfe8 after; clippy 0, nextest 920 of 920 and conformance 474 passed again.

**What is left.** §12.8.3.4.5 stays `implemented`, §12.8.3.4.4 `partial`. No window calls the fetch
yet (`doc/todo/77` item 4: an act, its key, the question, the thread, `Command::Trust` again);
GPR-4.0-03's *should* is not taken; a policy's list outside the LOTL's tree is fetched by nothing.
