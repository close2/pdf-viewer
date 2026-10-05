# 1347 — The black frame takes its geometry from the chromatic frame

Raster and render-raster, from the clauses alone. ADR 1529. No question. `crates/render-cpu/` was
not opened (A76); its figure was taken through the `Rasterizer` trait.

**Why two frames, derived.** §11.3.3 composites per component and §11.3.4 admits only spaces where
that is meaningful, so a four-component group is two three-channel renders of one geometry. The
conversion out reads all four components through a grid, so it runs once, after both. One render
with four channels would be exact; raster has no such target. So the second render stays and
shares what is not a colour.

**Measured first.** Callgrind, dumps around `frame_of`, one thread, GPU lane, the 1.25× frame: the
chromatic render 1.142 G, the black 0.443 G. In the black: `coverage_tile` 0.115 G,
`plan_group_residue` 0.091 G (the big clip's region again), clips rebuilt and uploaded 0.038 G,
`draw_encoded` 0.114 G (as in the chromatic).

**Kept.** A finished coverage tile for the next render, keyed by tile, rule, polylines and chain
number; a chain's region with its admission price; one set of clip outlines for a group's frames.
Black render 0.443 → 0.249 G; chromatic 1.142 → 1.148 G.

**Fixture.** `raster-gpu/tests/coverage_kept_for_the_next_render.rs`, three cases on three arms,
watched failing three ways: rule out of the key, chain out, every region under one number.

**Figures.** Pinned to the eight fastest cores, minima of 3 × 5, interleaved, exported trees: the
GPU-lane step 148.2 → 136.2 ms, the 1× frame 153.1 → 145.4 ms. The CPU backend, re-taken by
`zoom_frame`'s new `ZOOM_FRAME_BACKEND=cpu`: 47.2 and 56.3 ms (74.2 and 90.0 unpinned at four
threads; the old 84 was that kind of figure). The step is 2.4× the CPU backend. `frame_budget`:
turn 222.1 → 214.0, step 144.7 → 132.3, for 1348's `doc/performance.md`.

**Gates.** Exit statuses: `rustfmt --check` on my files 0. Clippy `-D warnings --all-targets` on
`raster-gpu`, `render-raster`, `pdf-render` 0. `cargo test -p raster-gpu` 0 (682 passed),
`-p render-raster` 0 (100), `-p pdf-render` 0 (257), `-p conformance` 0 (376). Behind the lock, on
exported HEAD and change trees: the corpus at 1× and 4× on three lanes, 0 pages moved on six arms,
one-versus-many empty, verdicts equal HEAD's. `turn_path` 0 (16 judged, 0 outside).
`headless_gpu` 0 (39 passed). `raster_golden` 0, unchanged (held 974, moved 0). `launch_path` 0
(26 banded, 0 outside).

**Left.** Each render's pass recording (0.115 G) and the chromatic exact meet (0.80 G).
