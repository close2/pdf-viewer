# 1689 — Pattern tints retain their source at paint

Session1689,2026-10-22. ADR2212/2213; §§8.7.3.3,11.6.7,11.7.5.3.

**Change.** Retain the selected underlying tint source and bind mark conversion
parameters separately from pattern definition state. Cell paint and stencils resolve
it in their current target. Nested admission requires retained source when uncoloured;
ordinary Type3 without a binding retains its refusal. Group Do parameters remain
separate. Actual-paint diagnostics use the same effective source. RGB tints reuse
existing complete-context SolidColours slots; no new cache/key/invalidation protocol.
Generic sources retain direct conversion; alpha and reporting remain outside memo.

**Gates.** Fixed-baseline arithmetic fails with exit 101 as required. Prior uncached
model+colour nextest:2283 passed,21 skipped; exit 0,178s,3.91GiB. Two isolated admission
mutations fail their targeted tests with exit 101; calibration exit 0,81s,0.63GiB.
Revised-cache all-target warnings-denied Clippy and fmt passed; ten tint controls,
one ICC-intent and one profile diagnostic passed; exit 0,48s,1.75GiB. Independent
review found no concrete cache/lifetime defect. Existing1687 controls remain unchanged.
Revised model+colour suite:2285 passed,21 skipped,58.871s tests; exit 0,156s,3.91GiB.
Revised paired cost results are pending; source is frozen for that comparison.

**Cost.** First immutable release builds passed200s,0.88GiB. All12 balanced pinned
pairs/all48 quiet guards passed,397s,0.07GiB; direct conversion is rejected: per20k
marks one-cell70.181→199.762ms, many-cell751.943→891.094, alternating130.386→172.443,
changed-pair142.430→149.081. Original exports and every sample remain preserved.
Revised comparison uses the same fixed b9d05ee8 baseline and identical instrumentation.
Inspected pictures confirm the clause-derived tint correction [51,102,154]→[52,62,79].
Final combined corpus/launch evidence remains an integration obligation.

**Residual.** Ordinary ICC selection-before-Perceptual still fails [116,111,133]
versus expected[253,0,0]; retain partial separately from deliberate profile bypass.
Reproducer/next contract:/home/AI/r1689-deferred-icc; no broader production fix.
Scope/report:/home/AI/paths-1689.txt,/home/AI/report-1689.md. Receipts:scratchpad/r1689/.
