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
22 failing assertions, grouped by behavior and qualified where interpretation is
contested. Passing controls and rejected preliminary hypotheses are recorded too.
The shard index links partial reviews to 189 ledger rows; 694 remain pending in
this round. No entire clause is certified merely because referenced tests pass.
A shared-parser date mutation was killed and restored. Independent JP2 fixture
regeneration matches the embedded hashes. Selected ADR conclusions are retained,
qualified, or reconsidered with primary-source reasoning.

**Gates.** Six touched crates: nextest 5,358 run, 5,341 pass, exactly the 17 expected
failures, 50 skipped. Clippy all-targets, fmt, rustdoc with warnings denied, and
full conformance package tests pass. Earlier baseline and GTK runs are recorded
separately; they do not certify later changes. The audit continues into encodings,
shading, signatures and remaining feature coverage, with periodic commits/pushes.

The next checkpoint adds AGL sequence/unknown-component failures and public text/search
controls, four passing mesh pixel assertions, and genuine PAdES report/tamper tests.
The PAdES Open-trigger finding is static: no verifier execution seam is exposed.
The new 11-test run has six passes and five expected failures; three-crate Clippy passes.
