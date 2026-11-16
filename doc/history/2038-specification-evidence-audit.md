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
63 failing assertions, grouped by behavior and qualified where interpretation is
contested. Passing controls and rejected preliminary hypotheses are recorded too.
The shard index links partial reviews to622 ledger rows;261 remain pending in
this round. No entire clause is certified merely because referenced tests pass.
A shared-parser date mutation was killed and restored. Independent JP2 fixture
regeneration matches embedded hashes. Selected ADR conclusions are retained,
qualified, or reconsidered with primary-source reasoning.

**Gates.** Wave31, eleven crates: nextest7,142 run,7,087 pass, exactly55 expected
failures by binary/test identity,58 skipped. No missing or additional failures.
Waves32–33 add five passing tests and three expected failing assertions. Final
model/core and syntax/render-cpu all-targets Clippy pass. Earlier gates retain their
recorded scope and do not certify subsequent changes.

**Latest findings.** Incomplete transparency surveys can yield false Met archive
results; valid deep DPart attachments can disappear without disclosure. Requirement
versions affect neither reports nor processing restrictions. A transparent stroke
composites overlapping same-object geometry twice. Literal measurement, join/fill,
WritingMode and new-file ID controls pass. The stroke failure also reaches PDF and three GPU lanes; CMYK-image OP passes.
Embedded-font clipping and corrected FDF import controls pass. Next scopes cover
signature revocation, FieldMDP identity, and transparency foundations.
UR annotation-save policy fails its signed control; withdrawal is a SHOULD. Latest weekly usage45%;ceiling85%.
