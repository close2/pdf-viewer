# 1696 — Confined codecs move without changing the wire

Docs round, batch one hundred and seven, 2026-10-22. ADRs 2226–2227; no question.
**Implemented; scoped gates pass; final conformance pending.** Four script dialogue and two published
signature-policy helpers move from protocol.rs into two private children.
Parent imports are explicit; helper visibility becomes pub(super), while Reader,
Writer and ProtocolError retain their existing visibility. No public API changes.
Every router and original test remains in the root; PDFVCF18, tags, field order,
allocation limits, error strings and refusal ordering are unchanged.
Root:8169→7951 lines; script_dialogues:145; published_policy:91.

**Proof.** Hashed pre-edit source, syn item inventory and exact function-body
bytes match. Complete function tokens match after normalising added visibility
and exactly two rustfmt trailing signature commas: read_script_answer and
write_published. No body punctuation is excluded. Dialogue54/policy29 literals,
including documentation, match; remaining-root tokens and original tests match.
Changed-tag, changed-error and omitted-helper scratch controls each fail101.
The positive proof passes; no new behavioural test or expected wire regeneration.

**Gates.** Nextest117 passed,1 skipped (3.479s); warnings-denied all-target
clippy29s/private rustdoc5s pass. Doctests0 run,1 ignored; viewer-host/viewer-ffi
all-target caller compilation passes20s. Whole hold116s,1.29GiB; queued wall273s.
Formatting/diff pass. Shared pointers updated by root; final conformance running.
No GPU or raster update is needed for a source-only extraction.
Owned paths and proof receipts: scratchpad/r1696/owned-paths.txt and proof.log.
Scoped duration348s from 2026-10-22T08:08:23+02:00. Final integration is root-owned.
