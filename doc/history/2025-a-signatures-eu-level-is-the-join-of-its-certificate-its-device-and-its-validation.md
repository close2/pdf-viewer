# 2025 — A signature's EU level is the join of its certificate, its device and its validation

Session 2025, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
item 3; §12.8.3.4.5). ADR 2866; no question.

**The slice, and why.** TS 119 172-4 REQ-4.4.2-02 to -06, the join `doc/todo/77` ordered next and
ADR 2835's condition for any "qualified". It needs clause 4.2's `TOTAL-PASSED`, whose REQ-4.2-03 c)
constraints the verdict did not apply, so the path's revocation issuance bound came with it.

**The premise.** Held: nothing joined clauses 4.4 and 4.5, and `notes` said "not determined by this
build". The walk took no issuance bound. Nothing owner-blocked, so no Q852; no `A*` after A800.

**Built.** `verdict::level`: `join` (REQ-4.4.2-02, -05, -06; `PROCESS_PASSED` read as that value;
REQ-4.4.2-03 names no check), `Level` with its deciding requirement, Annex A's types and identifiers,
and `determine`, which runs the verdict again over the listed service under REQ-4.2-03 c): status
checked, none unknown accepted, information issued after the best signature time. A locally valid
path has no level. `trust::PathInputs::revocation_issuance`, `revocation::status_within`,
`Signature::trust_under`. `notes::level_sentence` words the level, and nothing else does.

**Tests.** 13 in `verdict/level/tests.rs`, a branch of each requirement each, two over the fixture
signature; one walk test in `revocation_checks/tests.rs`; one in `notes`. **Plants**: five (a
warning passing -02, the host's acceptance, the device ignored, Annex A without the validation, the
walk ignoring the bound): each failed 1 to 3; reversed, pass.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-core`): 0. nextest
of the two: 1093 of 1093, 7 skipped. Fuzz `cargo check --bins`: 0. Rustdoc
`-D warnings --document-private-items` on the two: 0. Conformance after `git add -N`: 472 passed.
Tier 2 (load 120 at the start): `corpus` 101, 974 documents, 61 incomplete, as at HEAD;
`raster_golden` 101, held 951, moved 23, as at HEAD, no drawing path reached;
`selection_census` 0 (974); `accessibility_census` 0; `awkward_classes` 0 (152 swept, none
killed; openpreserve and tika-issue-tracker absent); `state.sh --round 2025 docs` 0.

**Habit, and what is left.** A requirement that refers to itself is a silence; write it down as one
and make no check out of it. §12.8.3.4.5 stays `implemented`. `doc/todo/77`: REQ-4.5-01's report
contents, then fetching the lists, `TAsInTrustedList`, a C host's lists, time-stamping authorities.
