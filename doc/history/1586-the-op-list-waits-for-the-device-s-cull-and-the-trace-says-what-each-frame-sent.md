# 1586 — The op list waits for the device's cull, and the trace says what each frame sent

Slot 2 of batch ninety-three, 2026-10-19, a "pixels" round on `doc/todo/75` item 3 and the
`UploadSplit` carried to the window. ADRs 2008 (the op list's ordering) and 2009 (the trace's
streams); no `Q` file; no ledger row moves.

**Premise, did not hold**: "only the view-dependent remainder changes — 9 474 of 4 408 406" is the
remainder *after* item 2; `doc/todo/75` item 3's own first sentence asks for "line batches [that]
are view-free", and item 2 waits on `Q473` (no `A473` in the owner's checkout). Today 40 of a line
instance's 60 bytes are device-space, and the op list is the view's through the host's cull: the
witness in the window (900 × 1100, `Compute`, scale 2.4 ×1, ×1.25, ×1.5) admitted 438 940 →
332 995 → 235 905 lines. A scratch probe found the witness leaves ADR 0087's record replay at its
twenty-sixth command for a residue clip, so every step is the full walk. **Hypothesis, did not
hold**: the view-independent part is not nine tenths of the encode this tree runs (step at 1.25×,
instrumented: encode 179.1 ms, walk 137.5, forms ahead 36.4). Item 3 is ordered after item 2.

**Built**: `render_raster::FrameCost::streams` (raster's `UploadSplit`, kept `Copy`; the two
by-value sites take `&FrameCost`, a `Box` refused — ADR 2009), `UploadSplit::saturating_add` for
`FrameCost::add`, `zoom_frame` reading the field, and the window's frame line's `| sent …` column
with its legend. Driven under Xvfb (lavapipe) on the EC3 specification and `AdobeBPC-2006.pdf`:
encoded frames print their streams, frames that adopted nothing print none.

**Proof that no pixel moved**: the export (HEAD plus these five files) against HEAD's arms
by page: gpu 1× 968, compute 1× 968, gpu 4× 965, compute 4× 965 pages, 0 digests and 0 mean errors
differ, none on one side only; `raster_golden` held 974, moved 0. One `--clock` hold (load 2.1 → 2.2),
the turn gate HEAD, this, HEAD, this: 33 of 33 inside every time, every figure of the second pair
within 1.1 ms of HEAD's (bug1721218's turn 134.2 against 135.3 ms).

**Files**: `raster/crates/raster-gpu/src/frame.rs`, `crates/render-raster/src/present.rs`,
`crates/render-raster/examples/zoom_frame.rs`, `crates/viewer-ui/src/bin/quorra/{timing,surface}.rs`,
ADRs 2008 and 2009, `doc/todo/75` and its `README` row, `doc/performance.md`, this record.
**Unfinished**: `doc/todo/75` items 2–6 (2 waits for `A473`; 3 after it, owing also a kept list
apart from the record list or a replayable residue clip); `doc/state-of-play.md` is slot 3's this
batch and names neither ADR.

**Gates.** `rustfmt --check` 0 (5 files); `clippy -D warnings --all-targets` 0 for `raster-gpu`,
`render-raster`, `viewer-ui` (in the export); `nextest` 1030 passed, 7 skipped, for the three; `cargo
test -p conformance` 440 passed, 0 failed; `state.sh docs` 0; arms 0 ×4; `raster_golden` 0. Clock: one
hold, 368 s at load 2.1 → 2.2, the turn gate 33 of 33 in all four runs. 9 800 s.
