# 1689 — Pattern tints retain their source at paint

Session1689,2026-10-22. ADR2212/2213; §§8.7.3.3,11.6.7,11.7.5.3.

**Change.** Retain underlying tint source and bind mark parameters separately from
pattern definition state. Cell paints/stencils resolve it in their current target.
Nested admission requires source when uncoloured; unbound Type3 retains its refusal.
Group Do parameters remain separate. Actual-paint diagnostics share effective source.
RGB tints reuse existing complete-context SolidColours slots; no new cache/key.
Generic sources convert directly; alpha/reporting remain outside memo.

**Gates.** Fixed-baseline arithmetic fails with exit 101 as required. Two isolated
admission mutations fail targeted controls; calibration exit 0,81s,0.63GiB. Revised
all-target warnings-denied Clippy/fmt and twelve focused controls pass,48s,1.75GiB.
Revised model+colour:2285 passed,21 skipped,58.871s tests; exit 0,156s,3.91GiB.
Independent source/cache review finds no concrete defect;1687 tests unchanged.
Four cached PNGs equal the correct uncached candidate; inspected tint changes
[51,102,154]→[52,62,79], matching independently derived arithmetic.

**Cost.** First candidate rejected: one/many/alternating/changed-pair medians per20k
marks70.181→199.762/751.943→891.094/130.386→172.443/142.430→149.081ms.
Revision build exit 0,114s,0.83GiB. Fixed b9d05ee8, identical instrumentation,
12 balanced CPU2 pairs/all48 quiet guards pass; exit 0,394s hold,499s elapsed,0.07GiB.
Revised paired median ratios1.01723/1.01093/1.02955/1.04436; deltas per20k marks
1.209/8.1575/3.874/6.267ms. Accept bounded correctness cost, not performance-neutral.
All samples retained, including214.685ms changed-pair outlier and noisy ordinaryRGB.
Original candidate/export receipts remain preserved. Final six arms from immutable 5cdd57c3
match baseline b9d05ee8 frames/errors: 968 pages per 1x arm, 965 per 4x; 1658 s hold, 3.73 GiB.
Final integration accepted (launch follow-up: ADR 2209); c76e3bdd precedes cost verdict.

**Residual.** Ordinary ICC selection-before-Perceptual still fails [116,111,133]
versus expected[253,0,0]; retain partial separately from deliberate profile bypass.
Reproducer/next contract:/home/AI/r1689-deferred-icc; no broader production fix.
Scope/report:/home/AI/paths-1689.txt,/home/AI/report-1689.md. Receipts:scratchpad/r1689/.
