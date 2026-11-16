# 2031 — A time stamp's authority, and its qualification, are found in the host's lists

Session 2031, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
items 7 and 6; §12.8.3.4.5, §12.8.3.4.8, §12.8.5.2). ADRs 2878 and 2879; no question.

**The slice, and why.** Item 7 whole — TS 119 615 clauses 4.6 and 4.7, a listed time-stamping
service anchoring a token's authority, and REQ-4.5-01 d) i) and d) iii) 3) — as the brief ordered;
the budget remained, so item 6 (a C host's lists), which is one entry point and needs nothing else.

**The premise.** Held: tokens asked the reader's anchors only, `eu_qualified` was always `None`,
and `quorra_trust_anchors` sent `Lists::none()`. No Q858; no `A*` after A800 cites either item.

**Built.** `trusted_list::time_stamp`: PRO-4.6.4-01 to -09 (the list by the subject's `countryName`,
the conversion shared with PRO-4.4.4-01), PRO-4.7.4-01 to -06, and `listed_authorities` (clause 4.3
for `TSA/QTST` and `TSA` at `genTime`). Clause 4.3's path test states the time-stamping purpose for
those types, without which RFC 3161's critical `extKeyUsage` refuses every authority.
`timestamp::Authority`: the reader's anchors first, then the listed services' identities; `chain`,
`established` and `signature_timestamp_established` take the lists. `applicability::EuQualified`
and `Assurance::Token::listed`; `notes` words both and names a listed authority. `viewer-ffi`:
`quorra_trusted_lists`, a session mirror of the policy so each entry point keeps the other's half
(ADR 1144's shape), names required, the clock carried with lists where no anchor was given; the
header, the C# declaration and both counts moved.

**Tests.** 8 in `time_stamp/tests.rs` over a root, three units and one `openssl ts` token made by
`r2031/gen/gen.sh`; 2 in `timestamp`, 1 in `applicability`, 1 in `notes`, 1 in `viewer-ffi`'s
session. **Plants**: five (-09's status, -06's comparison, -08's name check, clause 4.3's purpose,
the listed services never tried): each failed 1 to 8 tests; reversed, they pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-core`, `viewer-ffi`):
0. nextest: 1207, 1206 passed, the header test's count then moved, 9 of 9; 7 skipped. Fuzz `cargo
check --bins`, private rustdoc on the three, `state.sh docs`: 0. Conformance: 474 passed. Tier 2:
`corpus` 101, 974, 61 incomplete, as at HEAD; `raster_golden` 101, held 951, moved 23, HEAD's;
`selection_census` 0 (974); `accessibility_census` 0 (1020); `awkward_classes` 0 (152 of 2677).

**Habit, and what is left.** A clause run over a certificate whose use the service type names
states that use to the path test: RFC 5280 section 4.2 refuses a critical extension it cannot
process, and a search that never says why it asks finds no time-stamping authority at all. The rows
stay `implemented`; `doc/todo/77` keeps a)'s text (Q855), h)'s TS 119 312 (Q856), the fetch, and
whether Annex A's `id-etsi-dst-euqtst` is owed a document timestamp.
