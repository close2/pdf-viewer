# 2098 — The GPU lock joins the server's queue; a stroke's `/Background` and an unflagged mesh stroke

Session 2098, a pixels round in `pdf-viewer-lane4`: the owner's pointer of 2026-11-19 to the server's FIFO
`with-gpu-lock`, then the two leftovers round 2093 recorded.

- **Premises held.** `gpu_lock::enter` took a bare `flock` on `/run/lock/gpu.lock` and so jumped the queue the
  wrapper adopted on 2026-11-18. pdf-model dropped a stroking pattern's `/Background` and reported it;
  render-cpu (`UnsupportedPaint` from `shading::shader`) and Vello (`brush_for`) refused a stroke of an
  unflagged mesh, quorra drew it. One premise was narrower than the brief: every backend's stroke already took
  the raster door for any `is_rastered()` shading (ADR 3074), so item 2 is pdf-model's alone.
- **ADR 3093**: `enter` writes the wrapper's ticket in the queue directory beside the lock file (`gpu.queue`,
  only where it stands), waits to be the earliest live ticket (pid + start ticks + deadline), removes its own
  user's stale tickets, then `flock`s; one `GPU_LOCK_TIMEOUT` bounds both; the ticket is a drop guard. Hold-log
  lines end `ticket=<name|none>`. Five tests in `render-raster/tests/gpu_lock.rs`, on a private lock and queue.
  Plant (queue off): four of them fail. A real run's holds (`holds-targeted.log`) were served in ticket order.
- **ADR 3094**: Table 77's wash fills "the area to be painted" for any operator but `sh`; `ShadingDefinition::
  paints_background` is removed, `ShadingBackground` now fires only for a wrong-length array. Tests:
  `test_scenes::shading_background_stroked`, `render-cpu/tests/all/stroked_shading_background.rs` (2), one each
  in `headless_quorra.rs` and `headless_gpu.rs`, and `shadings.rs::a_stroking_patterns_background_is_painted_
  along_the_stroke` (replacing the report test). Plant: the pdf-model test fails; the backend tests pass, as the
  doors were ADR 3074's.
- **ADR 3095**: render-cpu `shading::mesh_shader` (MeshRaster over the stroke's extent, through
  `device_raster_pattern`, shared with `raster_shader`); Vello `stroked_as_a_raster` and `stroke_filtered`
  drawing `fill_mesh`. Tests: one per backend on `shading_anti_alias_stroked(false)`; plant: all three fail.
- Looked at: a fixture with a washed axial stroke and a type 4 mesh stroke, rendered by `render_at`.
- Ledger: 8.7.4.3 stays `implemented` (note's report sentence rewritten, tests named); 8.7.4.5.5 stays
  `implemented` (a stroke sentence and three tests). No owner audit file edited.
- Golden: only pages stroking a washed shading or a mesh can move; `raster_golden` is the train's.

**Gates.** Over one `-p` set (pdf-model, render-cpu, render-raster, render-gpu, test-scenes, raster-gpu,
viewer-device, viewer-host, viewer-ui — the last four include `gpu_lock.rs`): fmt `--check` exit 0; clippy `-D
warnings --all-targets` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a`,
unpinned (the RX 580): 4740 passed, 0 failed, the manifest's 0; 361 holds, every one on a ticket; rustdoc `-D
warnings --document-private-items` exit 0 for the five source crates; conformance exit 0 after a quotation's
attribution and this paragraph were fixed. No public type changed, so no `fuzz` check.

Left: `pdf-model/tests/corpus.rs` classes `ShadingBackground` as this reader's, while the one case left (a
wrong-length array) is the document's — not edited here because round 2096 is moving that file.
