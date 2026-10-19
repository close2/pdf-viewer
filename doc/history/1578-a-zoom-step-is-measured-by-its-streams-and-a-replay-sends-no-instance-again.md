# 1578 — A zoom step is measured by its streams, and a replayed encode sends no instance again

Slot 2 of batch ninety-two, 2026-10-19, a pixels evaluation round on the owner's ask of that day that
a zoom "send only a matrix". ADRs 1992 (the evaluation) and 1993 (the step); `doc/questions/Q473`
(the line lane mapped on the device); `doc/todo/75` (the order); no ledger row moves.

**Premise, held**: ADR 0368 priced the processor's geometry, not a compute rasteriser; nothing divided
`bytes_uploaded`. The brief's "eight rows" of `turn-path.toml` are eleven; all were read. **Hypothesis,
held on the witness at its upper end, missed on the rows**: host encode and transfer are 50–55 % of
the witness's 1.25× step on both lanes, the line lane 95 % (`Cpu`) and 87 % (`Compute`) of its
277–305 MB, fills under `Compute` resident (80-byte records, arena 0); the rows' steps are host stroke
tiling (83–97 %), an image's finer level (41–89 %), a mesh (53 %) or a rebuilt scene (2 %).

**Built.** `raster_gpu::UploadSplit` in `Counters::uploads` (sums to `bytes_uploaded`, the compute
lane's device-only buffers apart), `QuorraRasterizer::last_counters`, `zoom_frame`'s `crossed:` and
`lanes:` lines; and ADR 1993's step: a retained encode keeps its instance streams on the device
(`KeptInstances` in `retained::Held`), `RetainedScene::resident_instance_bytes` says the 264 499 408 B
held on the witness. The witness's replay: transfer 27.6–33.8 → 0.7–1.2 ms, frame 157–170 → 127–141 ms
(one run 172). Every figure is the Radeon 890M through RADV, headless; the owner's loop was not used,
since nothing here needs the display (`doc/environment.md`).

**Proof that no pixel moved.** Every digest of the first hold equals HEAD's at the fit, the step and the
replay on both lanes. The four arms of the export against HEAD's arms: gpu 1× 968, compute 1×
968, gpu 4× 965, compute 4× 965 pages, 0 digests and 0 mean errors differ, none on one side only.
`raster_golden`: held 974, moved 0. A line, text and artwork page replay to their immediate bytes; the
cell planted out fails two `retained_replay` tests.

**Files.** `raster/crates/raster-gpu/src/{frame,lib,compute,retained,device}.rs`,
`raster-gpu/src/device/{staging,render}.rs`, `raster-gpu/tests/retained_replay.rs`,
`crates/render-raster/src/lib.rs`, `render-raster/examples/zoom_frame.rs`, ADRs 1992 and 1993, `Q473`,
`doc/todo/75` and its `README` row, `doc/{performance,state-of-play}.md`, this record. **Unfinished**:
`doc/todo/75` items 2–6 (2 waits for `A473`); `FrameCost` does not carry the split, since 96 bytes more
put it past clippy's by-value bound in `viewer-ui`'s files, so no window's trace reads it yet.

**Gates.** `rustfmt --check` 0 (8 files); `clippy -D warnings --all-targets` 0 for `raster-gpu`,
`render-raster`, `viewer-ui`; `nextest` 726 passed (2 skipped) and 104 passed (3 skipped); `cargo
test -p conformance` 440 passed, 0 failed; `state.sh docs` 0; arms 0 ×4; `raster_golden` 0. Clock:
hold 1, 405 s at load 6.3 → 2.5 — the turn gate 33 of 33 for the step twice, HEAD 30 then 33 of 33
(personwithdog's turn and step, issue12841's step, at load 3.7–4.2), every step within HEAD's spread;
hold 2, 7 s at 5.8 → 5.4, two rows' steps unchanged, their replays less their instances. 8 900 s.
