# 2038 — Specification evidence audit after the fixes

The owner requested another audit after pulling `b901326c`, with15% of the weekly
allowance reserved. This round preserves the prior inventory and distinguishes
source/errata coverage, assertion quality, execution, and ADR interpretation.
The working branch is `audit/specification-evidence-2026-11-16`.

**Scope.** Tests and evidence only. No production fixes, ledger status changes,
or historical ADR rewrites. Luna-high reviewers supply bounded findings and test
drafts; the parent checks sources, controls, actual consumers, and execution.
The report and machine-readable continuation are in
`doc/reviews/2038-specification-audit.md` and its evidence directory.

**Evidence so far.** All72 previous failing assertions pass. Current tests expose
81 failing assertions, grouped by behavior and qualified where interpretation is
contested. Passing controls and rejected preliminary hypotheses are recorded too.
The shard index links partial reviews to all883 ledger rows, each still partial. Feature residuals remain
inside reviewed rows. No entire clause is certified merely because referenced tests pass.
Date and colour mutations were restored; independent tests catch them. Independent JP2 fixture
regeneration matches embedded hashes. Selected ADR conclusions are retained,
qualified, or reconsidered with primary-source reasoning.

**Gates.** Wave64, eleven packages:7,218run,7,142pass, exactly76expected
failures by binary/test identity,58skipped. Three GPU and one engine-only failure are outside that scope.
Wave49 adds247passing engine-enabled script/worker tests; both crate Clippy checks pass.

**Latest findings.** Incomplete transparency surveys can yield false Met archive
results; valid deep DPart attachments can disappear without disclosure. Requirement
versions affect neither reports nor processing restrictions. A transparent stroke
composites overlapping same-object geometry twice. Literal measurement, join/fill,
WritingMode and new-file ID controls pass. The stroke failure also reaches PDF and three GPU lanes; CMYK-image OP passes.
Embedded-font, Type3 image and corrected FDF controls pass. Next scopes cover
remaining errata and consumer assertions; native print, xref and four transition-window checks pass.
UR annotation-save policy fails its signed control; withdrawal is a SHOULD. Latest weekly usage61%;ceiling85%.

TSA keyCompromise time selection fails with genuine signed material; FieldMDP
rename coverage is an unresolved policy probe. Independent colour/annotation tests pass.
Alpha, filtered-checksum and cross-Contents controls pass; archive path cap fails. Model/transform Clippy pass.
Revocation findings survive second review; older indexes omit four grouped errata IDs.
Matrix2D/header search/linearization/Warichu split fail; two fixture skips hide behind PASS.
