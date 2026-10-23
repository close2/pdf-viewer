# 1707 — Tint and CIE solids follow the paint context

Session 1707, 2026-10-23. Batch 109, slot 4 (clause). ADRs 2248, 2249; §§8.6.6.4–5, 11.7.5.3.

**Controls first.** Seven independent controls (`tests/icc_paint_context/tints_and_cie.rs`)
against HEAD `ee365e4f`'s production code: six red on stale colours — `Separation`/`DeviceN`
tints into an `ICCBased` alternate under a late `ri`, `/RI` or `/UseBlackPtComp`, an initial
tint, q/Q, stroke/stencil/alpha; and a tint, `CalRGB` or `Lab` colour inherited into a
`DeviceGray` group, a `CalGray` one into a `CalGray` group. One green: a late `ri` over `CalRGB`
and `Lab` on the device, whose route reads no intent. With the inherited-paint assertions
removed, HEAD's cold selections meet every expected value. The hypothesis held for the tints
and not for the calibrated families: they fail the group-target control.

**Change.** `SolidSource::Space` keeps the resolved space and operands (`Rc`, compared by
allocation) for `Separation`, `DeviceN`, §10.8.3's simulation, `/All`, `CalGray`, `CalRGB`
and `Lab`; the two memos and their key are unchanged; `scn` moves the space it already clones.
A probe found plain `DeviceCMYK` inherited into a `DeviceGray` group stale ([236, 0, 140] for
105); it stays outside, named in the row and ADR 2248.

**Gates.** HEAD controls: 24 passed, 6 failed (exit 101); candidate 30 passed. Mutations in
an isolated export (`mutation-receipt.json`): no retention fails 6, none at `cs` 1, no source
identity 1, no context 6, each exit 101 on assertions; the oracle's hash unchanged. Colour
tests (6 files) exit 0. `cargo fmt --check` and warnings-denied clippy `-p pdf-model
--all-targets` exit 0 (worktree and clean export). `cargo nextest run -p pdf-model`: 2181
passed, 21 skipped, exit 0. Paired cost: 12 pairs, 24 sides, 48 guards passed, hold 384 s,
exit 0; table in ADR 2249. `cargo test -p conformance` in the worktree: one failure, `flags`
on `viewer-confined`'s new `protocol/tests/commands.rs:158`, then the ledger check on §10.7.4's
ADRs 2252/2253 not yet on disk — both siblings'; §11.7.5.3's row passes. Rustdoc `-D warnings
--document-private-items -p pdf-model` on the clean export: exit 0.
`raster_golden` on the export: 2 passed, exit 0, after a first run refused loudly for a
worker the wrapper's `--build` had made in the worktree rather than the export (trap 10).
Six arms on the export, by page against `/home/AI/arms-1704/`: 0 of 5799 digests differ, each
arm exit 0, 1573 s hold. `t2-turn_path` under `--clock`: exit 0, 33 figures banded, 0 judged
(device above 20% busy); the retry, 94 s hold: 33 judged, 0 outside, exit 0. Witness
pictures of HEAD and the candidate, rendered and looked at: `scratchpad/r1707/witness-*.png`.

**Remains.** Plain `DeviceCMYK`/`DeviceGray` solids and other Indexed bases; the row stays
`partial` with its boundary sentence rewritten to them.
