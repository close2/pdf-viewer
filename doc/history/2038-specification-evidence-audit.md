# 2038 — Specification evidence audit after the fixes

The owner requested another audit after pulling `b901326c`, with15% of the weekly
allowance reserved. This round preserves the prior inventory and distinguishes
source/errata coverage, assertion quality, execution, and ADR interpretation.
The working branch is `audit/specification-evidence-2026-11-16`.

**Scope.** Tests and evidence only. No production fixes, ledger status changes,
or historical ADR rewrites. Luna-high reviewers supply findings/test drafts; the parent checks sources, controls, consumers and execution.
The report and machine-readable continuation are in
`doc/reviews/2038-specification-audit.md` and its evidence directory.

**Evidence so far.** All72 previous failing assertions pass. Current tests expose
106 failing test identities (one stale ignored gate), grouped and qualified where interpretation is
contested. Passing controls and rejected preliminary hypotheses are recorded too.
The shard index links partial reviews to all883 ledger rows, each still partial. Feature residuals remain
inside reviewed rows. No entire clause is certified merely because referenced tests pass.
Date, colour, TJ, Indexed and optional-content mutations were restored; stronger tests catch them. Independent JP2 fixture
regeneration matches embedded hashes. Selected ADR conclusions are retained,
qualified, or reconsidered with primary-source reasoning.

**Gates.** Wave99, eleven packages:7,283run,7,185pass, exactly98expected
failures by binary/test identity,58skipped. Five GPU, one engine-only and one ignored stale expectation are outside that scope.
Wave49 adds247passing engine-enabled script/worker tests; both crate Clippy checks pass.

**Latest findings.** Incomplete transparency surveys can yield false Met archive
results; valid deep DPart attachments can disappear without disclosure. Requirement
versions affect neither reports nor processing restrictions. A transparent stroke
composites overlapping same-object geometry twice. Literal measurement, join/fill,
WritingMode and new-file ID controls pass. The stroke failure also reaches PDF and three GPU lanes; CMYK-image OP passes.
Embedded-font, Type3 image and corrected FDF controls pass. Next scopes cover
remaining errata and consumer assertions; native print, xref and four transition-window checks pass.
UR annotation-save policy fails its signed control; withdrawal is a SHOULD. Latest weekly usage72%;ceiling85%.

TSA keyCompromise time selection fails with genuine signed material; FieldMDP
rename coverage is an unresolved policy probe. Independent colour/annotation tests pass.
Alpha, filtered-checksum and cross-Contents controls pass; archive path cap fails. Model/transform Clippy pass.
Revocation findings survive second review; older indexes omit four grouped errata IDs.
All69file pointers partially reviewed. Type1 wide-BBox spacing fails; outline mutation caught; Alpha omission passes; FitR finding narrowed to SHOULD.
Malformed XFA, near-similarity thick strokes and TM export naming and field-AA/DS/TI/I inheritance fail; save/font controls pass.
