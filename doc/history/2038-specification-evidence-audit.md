# 2038 — Specification evidence audit after the fixes

The owner requested another audit after pulling `b901326c`, with 15% of the weekly
allowance reserved. This round preserves the prior inventory and distinguishes
source/errata coverage, assertion quality, execution, and ADR interpretation.
The working branch is `audit/specification-evidence-2026-11-16`.

**Scope.** Tests and evidence only. No production fixes, ledger status changes,
or historical ADR rewrites. Luna-high reviewers supply bounded findings and test
drafts; the parent checks sources, controls, actual consumers, and execution.
The report and machine-readable continuation are in
`doc/reviews/2038-specification-audit.md` and its evidence directory.

**Evidence so far.** All 72 previous failing assertions pass. Current tests expose
24 failing assertions, grouped by behavior and qualified where interpretation is
contested. Passing controls and rejected preliminary hypotheses are recorded too.
The shard index links partial reviews to 260 ledger rows; 623 remain pending in
this round. No entire clause is certified merely because referenced tests pass.
A shared-parser date mutation was killed and restored. Independent JP2 fixture
regeneration matches the embedded hashes. Selected ADR conclusions are retained,
qualified, or reconsidered with primary-source reasoning.

**Gates.** Before wave14 additions, eight crates: nextest 6,381 run, 6,358 pass, exactly the 23 expected
failures, 55 skipped. Current touched-crate all-targets Clippy and formatting pass.
Earlier rustdoc/conformance, baseline and GTK runs are recorded separately; they
do not certify later changes. The audit continues into fonts, metadata and
remaining feature coverage, with periodic commits/pushes.

The next checkpoint adds AGL sequence/unknown-component failures and public text/search
controls, four passing mesh pixel assertions, and genuine PAdES report/tamper tests.
The PAdES Open-trigger finding is static: no verifier execution seam is exposed.
Further passing controls cover all four named actions, packed/16-bit PNG arithmetic,
Type6 inherited edges, Type7 asymmetric control order and non-unit shading domains.

Clip timing/rules, optional-content composition and public media selection controls pass.
Annex D comparisons are reproduced; EC3#549 rejects the preliminary path-error hypothesis.
A genuine DTS-only DocMDP update is unclassified; the separately timestamped page-change control passes.
Disjoint boundaries reveal content despite required intersection; ADR0414 reconsideration is recorded.
Type3 restoration, popup, FDF pixels, crop inheritance and metadata controls pass; current Clippy passes.
