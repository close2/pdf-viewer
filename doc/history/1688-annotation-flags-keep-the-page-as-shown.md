# 1688 — Annotation flags keep the page as shown

Archive round, batch one hundred and six, 2026-10-22. ADRs 2210–2211; no new question.
**Implemented and scoped gates passed; final batch gates pending.** PDF/A-2/-4 §6.3.2 and
ISO 32000-2 §12.5.3 Table 167 require a distinction between visibility and printing.
As-shown now discards Hidden/NoView with Invisible/ToggleNoView clear and preserves
visible missing-Print annotations in the same document by setting Print in place.
Typed optional answers survive configuration, profile, CLI and host transport until
object preparation. Without shape rows, previous site-wide behavior is unchanged.
Partial shapes inherit an unqualified answer or Stop; unsupported shape options reject.
Negative, Invisible/ToggleNoView and malformed flags refuse even with global removal.
Missing/null F keeps the separate default-zero flags-entry-present policy.
Discarded widgets and Popup/Parent relationships refuse; no field-tree repair is claimed.

One prepared population supplies retained and removed writer operations and reports.
Shared references on two pages retain one output object and both report occurrences.
Appearance streams stay byte-identical; already-conforming objects stay outside the split.
The public Rust ArchivePlan field is a source API addition; native ABI is unchanged.

**Gates.** Scoped transform nextest: 640 passed, 13 skipped; 2.803 s tests, 105 s including queue/build.
All nine new controls pass, covering mixed 2b/4, reordered/partial/absent policy, malformed F,
ambiguous flags, linked/widget refusal, shared identity, exact appearances and CPU pixels.
The old unqualified Stop and visible-only shape are refusal controls, not a baseline build.
Warnings-denied all-targets clippy passes, 43 s. An initial absent-F test incorrectly assumed
output object 6; following the emitted Annots reference corrected the test premise.
veraPDF 1.30.3 accepts both 2b/4 fixtures; 400×400 source/output CPU rasters are byte-identical.
The inspected output retains two blue marks and removes the two hidden annotations.
Canonical archive section passes, 1405 s including queues. Census: 1450/1477 opened,
334 s held; validator: no missed/over findings, 108 s. Converter: all six targets retain
conforming inputs; 195 s, 1.57 GiB. As-shown converts 522 failing 2b and 266 failing 4
veraPDF cases; pdf.js/corpora counts and named refusals remain in the detailed receipt.
Resource cross-check: 13547 selections, zero disagreements, 59 s; one-sided walks and
13 soft-mask selections remain explicitly outside that comparison.
Scoped warnings-denied rustdoc passes, 22 s; workspace conformance passes, 137 s.
Host caller all-targets clippy passes; formatting/diff and doctest invocation pass.
Root owns final six arms and combined integration.

Owned paths: scratchpad/r1688/owned-paths.txt. Logs: scratchpad/r1688/.
Duration: 2361 s from 2026-10-22T04:17:17+02:00, including queue and coordination.
