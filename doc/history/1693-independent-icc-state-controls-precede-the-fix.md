# 1693 — Independent ICC state controls precede the fix

Stability clause round, batch107, 2026-10-22; ADRs2220–2221, no question.
ISO32000-2 §§8.6.5.8–9 and11.7.5.3; status remains partial pending integration.
The recurring defect is a converted result surviving changes to its inputs.
Source identity, current paint context and derived cache contents need separate
ownership; a smaller source file alone does not enforce that invariant.

Thirteen independent tests were committed before the production change.
Generated ICC midpoint tables distinguish intent; independent encoded XYZ,
D50-adapted sRGB matrix and transfer arithmetic supply expected colours.
Cold comparisons supplement rather than replace that oracle. Fill/stroke,
embedded text and one shared stencil have explicit painted-interior controls.
Save/restore, profiles, resource shadowing, defaults, resets, components,
BPC, supported target entry/return and warm alpha cross the state boundary.

On fixed0eb4d278, two tests passed and eleven failed: stale intent/BPC and
coloured Gray-group output differ from the independent answers. Both the
verbose initial failure and compact repeat are preserved in scratchpad/r1693.
Initial lint flagged manual midpoint arithmetic; explicit midpoint fixes it.
The later initial candidate passed twelve tests. The remaining assertion
wrongly skipped eight-bit premultiplied group storage; independently staged
integer arithmetic explains it exactly (ADR2220). Tolerance remains one level.
No renderer change or observed-raster golden was used to resolve that error.

**Gates.** With the corrected oracle and candidate implementation: 13 passed
(staged-alpha.log, exit0,1s; test runtime0.03s). Warnings-denied focused Clippy
passes,32.50s; root scoped rustfmt passes. An invalid cargo fmt --test invocation
was corrected to direct rustfmt on the owned file. Isolated mutation controls,
separate ICC/tint/mixed costs and combined gates remain subsequent obligations.
Round1694 owns production code and supplementary multi-channel/recovery tests.
Durations above are measured command times, not total round wall time.
