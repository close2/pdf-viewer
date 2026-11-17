# Continuing the specification audit

Checkpoint 133 closes this budgeted pass on branch
`audit/specification-evidence-2026-11-16`, against production baseline
`b901326c230faa5d2cb288016339b9d5c6288527`.
The final usage reading was 81% of the weekly allowance, leaving 19%.
No workers or test runs remain active. This is a stopping point for the budget,
not a declaration that the specification audit is complete.

The final eleven-package default-feature run executed 7,339 tests: 7,229 passed,
110 failed and 58 were skipped. Every observed failure matches an identity in
[expected-failures.json](expected-failures.json); see
[broad133-comparison.json](broad133-comparison.json). Five GPU failures, one
engine-only failure and one stale ignored-gate expectation are separately
recorded. Some passing corpus tests can return early when inputs are unavailable;
the D45 evidence preserves that distinction. Model all-target Clippy passes after
the final fixture changes.

All 883 ledger rows have partial current-source/assertion review. None is certified
in full. The 172 explicit table matrices are also partial: enumerated keys do not
prove every condition, interaction or public consumer. Selected ADR reviews do not
cover the entire ADR collection.

## First steps on a new baseline

1. Compare the new main revision with the recorded baseline and preserve these
   findings as provenance. Recheck affected evidence against the actual changes;
   a previous pass or failure is not a result on the new revision.
2. Reconcile the current failure identities with fixes and fresh execution.
   B08/B42 remain interpretation/policy probes, and some findings concern SHOULD
   guidance or product contracts. The manifest is not a count of mandatory ISO
   violations. Keep the 72 resolved prior-round failures distinct from this round.
3. Inspect the assertions and their public consumers when confirming a fix. Check
   producer validity, version, source overlays and independent expected values;
   passing a malformed or non-discriminating fixture does not settle a feature.
4. Continue with tests and evidence, regular commits and pushes. Production fixes,
   ledger status changes and historical ADR rewrites were outside this pass's
   scope; obtain a changed scope from the owner before doing those.

## Useful remaining work

- Recheck the narrowly qualified B77/B78 findings after fixes. B77 concerns loss of
  an active halftone transfer function when only screen loss was authorized;
  PDF/A-4 erratum 314 removes the transfer prohibition, not the Type6 restriction.
  B78 concerns visibility callbacks sent to an explicitly enabled recording runner,
  not execution by a JavaScript engine.
- Deepen untested combinations in the table matrices. Table143 ignored-entry
  behavior and depth/filter/interpolation combinations remain partial. Table146
  spot-color rows need assertions of the specific source/component case; command
  masks, rendered pixels and physical separations are different evidence.
- Resolve the watermark page-Rotate interpretation with a discriminating public
  output test. The existing FixedPrint Matrix rotation test does not settle it.
  The passive stale-TrapNet regeneration wording still has an actor ambiguity;
  writer validation requirements alone do not establish a reader duty.
- Continue independent peer review of older audit fixtures and claims. T166's
  parent-page fixtures required real sibling content even though the original
  tests passed. The repaired tests now pass with valid parent-page associations.
  The overprint negative control was strengthened to compare pixels independently
  of its metadata flag. These are concrete examples of why execution alone is
  insufficient.

Use [clause-inventory.json](clause-inventory.json),
[shard-index.json](shard-index.json), and
[table-cell-review-index.json](table-cell-review-index.json) to select a bounded
next review. Follow parent-review qualifications as well as agent draft records.
Primary specification files and local execution logs remain under `doc/` and
`scratchpad/r2038/spec-audit/` respectively; specification source files are not
redistributed in the audit commits.
