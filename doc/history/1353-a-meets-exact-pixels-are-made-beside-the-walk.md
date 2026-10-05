# 1353 — A meet's exact pixels are made beside the walk

Raster slot. ADR 1541. No question. `crates/render-cpu/` was not opened. The round was cut by a
network outage while a corpus walk ran and was resumed from the worktree and `scratchpad/r1353/`.

**Measured first.** Callgrind by function on HEAD, the 1.25× frame, one thread, GPU lane: the
chromatic render 1.148 G, `meet_residue` 0.804 G, `exact_areas` 0.474 G, `draw_encoded` 0.115 G.
Counted per pixel (habit 63): 11 988 asked pixels a render, 17.8 edges through each, 38.4 partial
edges beside it, 34.5 k instructions a pixel spread over six steps, none a third of it, 9 889 of
the answers below `min`. The inputs are already small; the cost is that it is serial. Skipping
the exact pixels (a wrong-bytes probe) took the step from 145.7 to 107.0 ms.

**Kept.** The meet leaves the cut pixels at `min` and hands back what their areas read; the caller
packs the tile and records the meet at its place on the sheet. Helper threads make the areas as
meets are recorded; the settle at `finish` writes them over the sheet and keeps them for the next
render, in encounter order. A fan-out job's polylines and edges are taken, not copied (copied, the
stroked Type 3 page's turn read 13.3 ms against HEAD's 12.3). On the GPU lane a residue-clipped
mark now takes a fan-out job as on the others. Areas made all at `finish` on a scope: tried, slower.

**Fixtures.** `raster-gpu/tests/exact_meets_settled_after_the_walk.rs`: 256 diamonds under a
64-gon, four lane and thread arms at two budgets, every pixel to the closed form; watched failing
with the sheet write left out. `kept.rs` `a_key_kept_twice_is_charged_once`, watched failing
without each guard.

**Figures** (pinned, minima of 3 × 5, interleaved, exported trees). `zoom_frame` GPU lane: step
135.0 → 117.7 ms, 1× 146.5 → 123.7; the CPU backend 57.4 and 47.9. **The step is 2.05× the CPU
backend: at 2×, not inside it** (1.99× in an earlier sitting). `frame_budget`: turn 215.27 →
188.79, step 134.12 → 117.04 — the page's rows in `doc/performance.md` section 3e, which gained a
`taken` column, and banded in `turn-path.toml` from those figures. Left: the 3 515 small chains'
first fills (0.23 G) and the pass recording (0.115 G a render), ADR 1541 section 4.

**Gates.** `rustfmt --check` on my files 0. Clippy `-D warnings --all-targets` on `raster-gpu` 0.
`cargo test -p raster-gpu` 0 (684 passed), `-p conformance` 0 (380). Behind the lock on exported
HEAD and change trees: the corpus on CPU, GPU and compute at 1× and 4×, 0 pages moved on six arms
(final tree 1277 s), verdicts equal, one-versus-many 0. `turn_path` 0 (16 judged, 0 outside, the
new rows 192.31 and 116.38 ms; 61 s). `raster_golden` 0 (held 974, moved 0; 16 s).
