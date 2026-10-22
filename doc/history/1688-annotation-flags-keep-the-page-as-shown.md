# 1688 — Annotation flags keep the page as shown

Archive round, batch one hundred and six, 2026-10-22. ADRs 2210–2211; no new question.
**Implemented; archive corpus and final batch gates pending.** PDF/A-2/-4 §6.3.2 and
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
veraPDF accepts both 2b/4 fixtures; 400×400 source/output CPU rasters are byte-identical.
The inspected output retains two blue marks and removes the two hidden annotations.
Canonical archive catalogue/census/validator/converter/resource checks are running.
Scoped warnings-denied rustdoc passes, 22 s; workspace conformance passes, 137 s.
Host caller clippy remains pending; root owns final six arms.

Owned paths: scratchpad/r1688/owned-paths.txt. Logs: scratchpad/r1688/.
Started 2026-10-22T04:17:17+02:00; final duration pending required corpus checks.
