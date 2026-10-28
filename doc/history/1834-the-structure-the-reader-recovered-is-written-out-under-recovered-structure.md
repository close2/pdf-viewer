# 1834 — The structure the reader recovered is written out under `recovered-structure`

Slot 3 of batch one hundred and twenty-four, 2026-10-27, an archive slot (Sonnet, high), continued from a
handover at about 225k context; ADRs 2502 and 2503 used, Q700 left unused. I worked in `scratchpad/r1834/`.
**Ledger.** No status moves: the clauses I touch (§7.5.5 `departed`, §7.7.2, §7.7.3.2, §7.7.3.4 `implemented`)
were not `unreviewed`; I added `archive/recovered.rs` to §7.7.3.4's `code`. Annex C is informative and has no row.
**Premise** (the list cites "§C.4's recovery"): did not hold. Annex C.4, "File Limits", says only that a processor
"may attempt to rebuild the table by scanning all the objects in the file" for a damaged cross-reference table;
nothing defines recovering a missing catalog or page tree, so the recovery is this reader's own choice and the
loss says so (ADR 2502). The 18 documents decline on four causes: the catalog's `/Pages` resolves to no node (10),
`/Root` is no dictionary (4), the trailer states no `/Root` (3), a node states no `/Kids` (1).
**Built.** `archive/recovered.rs` rebuilds a catalog (the producer's where it resolves) and one flat `/Pages` node over
the pages the reader draws, each page the producer's with `/Parent` restated and §7.7.3.4's four inheritable
entries copied down; the rebuilt file is reopened and held to the three checks that declined the source. `Loss::
RecoveredStructure` (before `TextAsImage`), `[profile] recovered-structure = "discard"` in both shipped profiles
(my reading of ADR 2490's "both": the row states no action word), the report line and JSON entry, one
`xmpMM:History` event. `optimize` and `redact` keep the decline; encrypted, signed, direct-page and
damaged-page sources are refused by name, each with its elegant remedy written in ADR 2502.
**Census** (pack named, rank.py over both logs): 11 of the 18 convert at 2b and at part 4 under both profiles, veraPDF
compliant on all 44 outputs; documents refused or declined at every target 262 to 251 (`as-if-printed`) and 351 to 340
(`as-shown`); `UnknownFilter-xrefstm` now stops at the file header (R06b's), 6 refuse by name (ADR 2503), `027613`
among them: linearized, `/N 429`, one page reached, so a stated count above the pages drawn is refused.
**Found, not fixed:** `optimize.rs` and `redact.rs` still say "§C.4's recovery" in their comments; `tests/archive.rs` and slot 1 both
collapse the same `Authorisations` literal in `a_metadata_property_this_target_rejects_...` (a merge overlap).
**Gates.** `cargo fmt --check -p pdf-transform` exit 0, `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform
--all-targets` exit 0 and rustdoc `-D warnings --document-private-items` exit 0, all in my export of HEAD plus my
hunks (the worktree's own clippy exit 101 on `pdf-model/src/content/transparency/soft_mask.rs`, a sibling's mid-edit).
`cargo test -p pdf-transform --test archive` on the export with the debug worker built (habit 185): 377 passed, 0 failed
(my first run lacked the worker and read two JPX reds as HEAD's; they were the missing worker); `cargo nextest run -p
pdf-transform` 767 passed, 21 skipped; my 15 `recovered::` tests pass. Plants: `/Parent` not restated, the inheritable
copy off, the hook off, the value copied per page: 1, 1, 11 and 1 tests red. `--test gate -- --ignored` under
`--profile gates` exit 0; `archive_corpus -- --ignored` exit 0 (7 tests) and `cross_check -- --ignored` exit 0 on the
earlier export; the census (pack and none) re-run on the final export exit 0 twice. `cited` exit 0, `quotations` exit 0,
neither naming a line of mine; `cargo test -p conformance --no-fail-fast` exit 0 before the fixes, and `--test records
--test adr_status --test evidence` re-run after, exit 0. `tools/state.sh --round 1834 archive` exit 0. The validator's
two behaviour findings (027613 written as one page of 429; a direct inherited value copied per page) and four paperwork
findings (optimize.rs doc and two runtime strings, record figures, the depth test, the Table 30 wording) were fixed by me.
**Validation** (the orchestrator): FAIL, then the fixes above; re-validation pending. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on two behaviour findings — a linearized source archived as one page of its stated 429, and a direct inheritable value cloned once per page — and four of paperwork, and PASS on the construction, the provenance, the plants and veraPDF; the round fixed them and the validator's probes then held.
