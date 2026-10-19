# 1593 — A line stated in the page's space is measured, and taken back

Slot 2 of batch ninety-four, 2026-10-19, a "pixels" round on `doc/todo/75` item 2 and the owner's
`A473`. ADR 2022; no `Q` file; no ledger row moves (§8.4.3.2, §8.4.3.3, §8.5.4 and §10.7.5 were cited
in the code that was built, and none of it stays).

**Premise, held**: `A473` accepts the bar; `line.wgsl` and its lane were as `doc/todo/75` item 2 and
ADR 2008 describe. **Hypothesis, did not hold**: the step lost its 264 MB of line bytes and 29–47 ms of
transfer, not 100–150 ms — the walk rose 60–80 ms in most processes with equal instructions (callgrind
933 M against 924 M on the walk's thread), equal simulated misses and branches, no wait on the makers,
equal page faults; the window's step rose 144–151 → 249–252 ms, every unculled line now written.
**Built and taken back**: page-space instances and a view in `Globals`, the width resolved and the cull
made in the vertex stage, `LineRuns` and the adopted stream; two tests, each failing planted back. The
patches are in `/home/AI/r1593-line-lane/`. **Kept**: `zoom_frame`'s `ZOOM_FRAME_DUMP`.

**Pixels, of the build**: arms gpu and compute at 1× and 4× against HEAD's arms, 968, 968,
965, 965 pages, 0 moved, none one-sided (the gate bakes the view); the witness's page-space frames
against HEAD's, 6 225 and 10 889 bytes, ≤ 3 and ≤ 6 levels, mean to the oracle 0.16427 against 0.16425.

**Files**: `crates/render-raster/examples/zoom_frame.rs` (the dump), ADR 2022, `doc/todo/75` item 2,
`doc/performance.md` 3d, this record. **Unfinished**: item 2 itself — its walk's cost unattributed (no
`perf` here) — and the host's cull owed until item 3; `t2-raster_golden` not run, since nothing that
draws changed in what stays.

**Gates.** Of the build: `rustfmt --check` 0 (15 files); `clippy -D warnings --all-targets` 0
(`raster-gpu`); `nextest` 734 passed (`raster-gpu`), 104 passed (`render-raster`); arms 0 ×4; one
`--clock` hold, 433 s, load 7.4 → 5.8, the turn gate 33 of 33 twice and `zoom_frame` both lanes twice.
Of what stays: `rustfmt` and `clippy -D warnings` 0 on `zoom_frame`; `cargo check` 0 on `raster-gpu`
and `render-raster`; `cargo test -p conformance` 442 passed, 0 failed; `state.sh docs` 0. 8 640 s.
