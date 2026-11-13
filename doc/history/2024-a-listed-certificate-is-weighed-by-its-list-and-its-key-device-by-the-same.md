# 2024 — A listed certificate is weighed by its list, and its key's device by the same

Session 2024, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
items 2 and 3's first half; §12.8.3.4.5). ADRs 2864 and 2865; no question.

**The slice, and why.** TS 119 615 clause 4.4 with TS 119 612 clause 5.5.9.2's reader first, as the
brief ordered; it was done with the budget mostly left, so clause 4.5 followed, which reads clause
4.4's own outputs and has nothing else to wait for. The join (TS 119 172-4 REQ-4.4.2-06) is left.

**The premise.** Held: the reader kept `additionalServiceInformation` and no `Qualifications`
extension; nothing ran clause 4.4 or 4.5. Nothing owner-blocked, so no Q852; no `A*` after A800.

**Built.** `trusted_list::qualifications`: clause 5.5.9.2 read (criteria lists, key usage, policy
sets, clause 5.5.9.2.2.3's two, nesting), three-valued `identifies`. `trusted_list::determination`:
PRO-4.4.4-01 to -36, Tables 1 to 3 cell by cell, 4 and 5, the run at `notBefore`;
`determination::qscd`: PRO-4.5.4-01 to -05, Tables 6 and 7. `Provider::names`;
`Listing::qualification` at the best signature time; `notes` in the clauses' indications;
`KeyUsage::asserts` (`pdf_pki`); `trust::names` attribute and name matching; `fuzz/`'s
`trusted_list` runs clause 4.4. Five of the clauses' sentences cannot be taken literally; each is
read by its siblings, in the module.

**Tests.** 19 in `determination/tests.rs` over a CA and three signers `r2024/gen/gen.sh` made, one
in `verdict`, one in `notes`. **Plants**: eleven (a Table 1 cell, PRO-4.4.4-12 a) 7), `none`, the
critical rule, the `notBefore` run, PRO-4.4.4-06, the purpose filter, Table 7's row, the unknown
critical qualifier, the Directive regime, PRO-4.5.4-04 c) 1)): each failed 1 to 4; reversed, pass.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-pki`, `pdf-signature`, `viewer-core`): 0.
nextest of the three: 1135 of 1135, 7 skipped; doctests 0. Fuzz fmt and clippy `--bins`: 0. Rustdoc
`-D warnings --document-private-items`: 0. Conformance after `git add -N`: 472 passed. Evidence:
Germany's list reads with 1057 `Qualifications` extensions (45 critical), none undecidable. Tier 2:
`corpus` 101, 974 documents, 61 incomplete, round 2023's; `raster_golden` 101, held 951, moved 23,
round 2023's; `selection_census` 0 (974); `accessibility_census` 0 (1020);
`awkward_classes` 0 (152 swept of 2677, none killed); `state.sh --round 2024 docs` 0.

**Habit, and what is left.** A table the clause prints is transcribed cell by cell and pinned by its
EXAMPLEs, not derived from the pattern its cells happen to follow. §12.8.3.4.5 stays `implemented`;
`doc/todo/77` item 3 is now the join into a level, and only then does a report say "qualified".
