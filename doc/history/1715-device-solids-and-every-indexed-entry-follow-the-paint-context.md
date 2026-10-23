# 1715 — Device solids and every Indexed entry follow the paint context

Session 1715, 2026-10-23. Batch 110, slot 4 (clause). ADRs 2264, 2265; no `Q`. §§8.6.6.3,
10.4.2.3, 11.6.6, 11.7.5.3. This session worked under `scratchpad/r1715/`.

**Controls first.** Seven independent controls (`tests/icc_paint_context/device_solids.rs`)
against HEAD `e883ed13`'s production code: six red on stale colours — `0 1 0 0 k` inherited
into a `DeviceGray` group magenta [236, 0, 140] where §10.4.2.3 gives 105, `0.6 g` into a
`DeviceCMYK` group [167, 156, 68] where the cold selection gives [202, 197, 100], both into a
gamma-one `ICCBased` group (246 and 203 for 105 and 153), `Indexed` over `DeviceCMYK`,
`DeviceGray`, `CalRGB` and a `Separation` under a late intent and black point, q/Q,
stroke/stencil/alpha. One green as it must be: a late `ri` or `/UseBlackPtComp` over `g`/`k`
on the device. HEAD's cold selections meet every expected value. The premise that an opaque
fill into a `DeviceCMYK` group is stale did not hold — such a group gives its space up and
draws in the parent's — so that control paints at half alpha over a yellow.

**Change.** `SolidSource::Gray(f32)` and `Cmyk([f32; 4])`, inline beside `Rgb`, compared by
value; an `Indexed` entry over any base is a selection in that base. Memos unchanged.

**Found, not built.** A scratch probe sees §11.7.2's redefinition read at selection: an
output-intent `k` inherited into an `ICCBased` CMYK group differs from a cold `k` there (ADR
2264). The row's note names it as what stands before `departed`.

**Gates.** HEAD controls: 31 passed, 6 failed (exit 101); candidate 37 passed, exit 0. Four
mutations in an isolated export each exit 101 on assertions (ADR 2265). `cargo fmt --check` and
warnings-denied clippy `-p pdf-model --all-targets` in the worktree: exit 0. `cargo nextest run
-p pdf-model`: 2188 passed, 21 skipped, exit 0. `cargo test -p conformance --no-fail-fast` in
the worktree: exit 101 on two siblings' gates only (`names`, a viewer-ffi path; `variables`,
`tools/zoom-probe-drive.py`); the ledger, `evidence` and `adr_status` tests pass. `raster_golden`
on the export: 2 passed, exit 0. Six arms on the export by page against HEAD's six arms
(`tools/batch.sh arms`): 0 of 5799 digests differ, each arm exit 0, 1893 s hold. One `--clock`
hold: 12 cost pairs, 24 sides exit 0 (table in ADR 2265), then `t2-turn_path` exit 0, 33 figures
banded, 33 judged, 0 outside. Rustdoc `-D warnings
--document-private-items -p pdf-model`: exit 0 (the workspace `state.sh docs` walk was queued
behind two arms and withdrawn before its grant). Witness pictures of HEAD and the candidate,
rendered and looked at: magenta becomes grey in both grey groups, salmon in the CMYK group.

**Remains.** §11.7.2's redefinition at the paint, then the row is `departed`; it stays
`partial`, its boundary sentence rewritten.
