# 1687 — Mixed paint consumers restore their state

Session 1687, 2026-10-22. Stability controls for §11.7.5.3; ADR 2208.

**Change.** Test-only mixed sequences cross fill, stroke, text and stencil consumers,
changing BG/UCR after source selection and changing sources/alpha inside q/Q. Each
cell is compared with a separately interpreted cold page with its state established
before colour selection. A second test reuses one indirect ImageMask across changes
and restoration. Both fill/stroke source orders are covered; nonempty consumer regions
and an arithmetic pixel prevent agreement on absent artwork from passing silently.

**Gates.** Initial scoped tests: 2 passed, 24 s hold, 0.25 GiB peak. The earlier
attempt exited 101 during a sibling's temporary workspace-manifest edit; no test ran.
Final nonempty-region controls passed both tests in the isolated baseline. Removing
context matching failed both; bypassing stroke conversion failed the mixed-consumer
case while the stencil-only case passed (89 s hold, 0.44 GiB peak for all three runs).
Scoped formatting, warnings-denied all-targets clippy, sandbox binaries, model nextest
(2136 passed, 21 skipped), conformance and warnings-denied rustdoc passed: 439 s hold,
3.91 GiB peak (650 s including queue). No production or performance change.

**Limits.** No ledger status changes. Uncoloured tint, nested group planes and profile
bypass remain separate obligations. Sources: mixed_paint_context.rs and ADR 2208;
isolated exports and calibration receipts are under scratchpad/r1687/.

**Design review.** ADR 2209 records the observed paint-context, toolkit allocation and native
artifact boundaries, with narrow follow-ups and remaining hypotheses separated. File splitting
alone is not treated as proof of reduced coupling. No further production change in this slot.
